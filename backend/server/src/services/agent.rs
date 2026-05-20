use marketplace_api_contract::{
    AgentAction, AgentQueryRequest, AgentQueryResponse, SearchRequest, SearchResponse,
};
use marketplace_auth_core::Claims;
use crate::repositories::ListingRepository;
use crate::services::search::SearchService;

#[derive(Debug)]
pub enum AgentError {
    Search(String),
    Parse(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::Search(msg) => write!(f, "search error: {msg}"),
            AgentError::Parse(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

pub struct AgentService<R> {
    search: SearchService<R>,
}

impl<R> AgentService<R>
where
    R: ListingRepository + Send + Sync,
{
    pub fn new(search: SearchService<R>) -> Self {
        Self { search }
    }

    pub async fn query(
        &self,
        claims: Option<&Claims>,
        request: &AgentQueryRequest,
    ) -> Result<AgentQueryResponse, AgentError> {
        let conv_id = request
            .conversation_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let search_request = parse_query(&request.query)?;
        let mut search_response = SearchResponse {
            items: Vec::new(),
            applied_sort_by: marketplace_api_contract::SearchSort::Relevance,
            next_cursor: None,
        };

        if let Some(ref sr) = search_request {
            search_response = self
                .search
                .search_listings(claims, sr)
                .await
                .map_err(|e| AgentError::Search(e.to_string()))?;
        }

        let (message, actions, listing_ids) =
            build_response(&request.query, &search_response);

        Ok(AgentQueryResponse {
            message,
            actions,
            conversation_id: conv_id,
            listing_ids: if listing_ids.is_empty() {
                None
            } else {
                Some(listing_ids)
            },
        })
    }
}

fn parse_query(query: &str) -> Result<Option<SearchRequest>, AgentError> {
    let lower = query.to_lowercase();

    // Detect price constraints
    let max_price = extract_amount(&lower, &["under", "less than", "below", "max", "up to", "cheaper than", "<"]);
    let min_price = extract_amount(&lower, &["over", "more than", "above", "min", "from", "at least", ">"]);
    let exact_price = extract_amount(&lower, &["for", "around", "about", "~"]);

    // Detect listing type
    let listing_type = if lower.contains("property")
        || lower.contains("house")
        || lower.contains("apartment")
        || lower.contains("land")
        || lower.contains("real estate")
        || lower.contains("rent")
    {
        Some(marketplace_api_contract::ListingType::Property)
    } else if lower.contains("service")
        || lower.contains("consulting")
        || lower.contains("repair")
        || lower.contains("plumber")
        || lower.contains("electrician")
        || lower.contains("cleaning")
        || lower.contains("tutoring")
    {
        Some(marketplace_api_contract::ListingType::Service)
    } else if lower.contains("product")
        || lower.contains("laptop")
        || lower.contains("phone")
        || lower.contains("camera")
        || lower.contains("furniture")
        || lower.contains("accessory")
        || lower.contains("gaming")
        || lower.contains("audio")
        || lower.contains("vehicle")
        || lower.contains("appliance")
        || lower.contains("tablet")
        || lower.contains("monitor")
        || lower.contains("desktop")
    {
        Some(marketplace_api_contract::ListingType::Product)
    } else {
        None
    };

    let query_text = query
        .trim()
        .trim_start_matches("find ")
        .trim_start_matches("search ")
        .trim_start_matches("show ")
        .trim_start_matches("list ")
        .trim_start_matches("i want ")
        .trim_start_matches("i need ")
        .trim_start_matches("looking for ")
        .trim_start_matches("help me find ")
        .trim_start_matches("where can i find ")
        .to_string();

    if query_text.is_empty() && listing_type.is_none() && max_price.is_none() && min_price.is_none() {
        return Err(AgentError::Parse(
            "I couldn't understand your request. Try something like 'find laptops under $1000'".to_string(),
        ));
    }

    let price_filter = max_price.or(min_price).or(exact_price).map(|_| {
        marketplace_api_contract::SearchPriceFilter {
            currency: Some("USD".to_string()),
            min_amount: min_price,
            max_amount: max_price.or(exact_price),
        }
    });

    let location = detect_location(&lower);

    let category = detect_category(&lower);

    let condition = detect_condition(&lower);

    Ok(Some(SearchRequest {
        query: if query_text.is_empty() { None } else { Some(query_text) },
        category,
        condition,
        price: price_filter,
        location,
        listing_type,
        status: Some(marketplace_api_contract::ListingStatus::Active),
        // Defaults for rest
        sort_by: marketplace_api_contract::SearchSort::Relevance,
        limit: Some(10),
        ..SearchRequest::default()
    }))
}

fn extract_amount(text: &str, prefixes: &[&str]) -> Option<f64> {
    for prefix in prefixes {
        if let Some(idx) = text.find(prefix) {
            let after = &text[idx + prefix.len()..];
            // Try to find a number after the prefix
            if let Some(num_str) = after
                .trim_start()
                .trim_start_matches(['$', '€', '£', '¥'])
                .split_whitespace()
                .next()
            {
                if let Ok(amount) = num_str.replace(',', "").parse::<f64>() {
                    return Some(amount);
                }
            }
        }
    }
    None
}

fn detect_location(text: &str) -> Option<marketplace_api_contract::SearchLocationFilter> {
    let keywords = [
        "in ", "near ", "around ",
    ];
    for kw in &keywords {
        if let Some(idx) = text.find(kw) {
            let after = text[idx + kw.len()..].trim().to_string();
            if !after.is_empty() && after.len() < 50 {
                // Simple: treat everything after "in" as a city name
                return Some(marketplace_api_contract::SearchLocationFilter {
                    country_code: None,
                    city: Some(after.split_whitespace().next().unwrap_or("").to_string()),
                });
            }
        }
    }
    None
}

fn detect_category(text: &str) -> Option<marketplace_api_contract::Category> {
    use marketplace_api_contract::Category;
    let lower = &text.to_lowercase();
    if lower.contains("laptop") || lower.contains("notebook") || lower.contains("macbook") || lower.contains("thinkpad") {
        Some(Category::Laptop)
    } else if lower.contains("phone") || lower.contains("iphone") || lower.contains("smartphone") || lower.contains("pixel") || lower.contains("galaxy") {
        Some(Category::Phone)
    } else if lower.contains("tablet") || lower.contains("ipad") {
        Some(Category::Tablet)
    } else if lower.contains("desktop") || lower.contains("pc") || lower.contains("computer") || lower.contains("mac") {
        Some(Category::Desktop)
    } else if lower.contains("monitor") || lower.contains("screen") || lower.contains("display") {
        Some(Category::Monitor)
    } else if lower.contains("camera") || lower.contains("dslr") || lower.contains("lens") || lower.contains("gopro") {
        Some(Category::Camera)
    } else if lower.contains("headphone") || lower.contains("earphone") || lower.contains("speaker") || lower.contains("audio") || lower.contains("sound") {
        Some(Category::Audio)
    } else if lower.contains("gaming") || lower.contains("xbox") || lower.contains("playstation") || lower.contains("nintendo") || lower.contains("ps5") {
        Some(Category::Gaming)
    } else if lower.contains("furniture") || lower.contains("chair") || lower.contains("table") || lower.contains("desk") || lower.contains("sofa") || lower.contains("bed") {
        Some(Category::Furniture)
    } else if lower.contains("appliance") || lower.contains("fridge") || lower.contains("washer") || lower.contains("oven") || lower.contains("microwave") {
        Some(Category::Appliance)
    } else if lower.contains("vehicle") || lower.contains("car") || lower.contains("bike") || lower.contains("tire") || lower.contains("auto") {
        Some(Category::VehiclePart)
    } else if lower.contains("accessory") || lower.contains("charger") || lower.contains("case") || lower.contains("cable") || lower.contains("adapter") {
        Some(Category::Accessory)
    } else {
        None
    }
}

fn detect_condition(text: &str) -> Option<marketplace_api_contract::Condition> {
    use marketplace_api_contract::Condition;
    let lower = text.to_lowercase();
    if lower.contains("new") && !lower.contains("new york") && !lower.contains("new jersey") {
        Some(Condition::New)
    } else if lower.contains("used") || lower.contains("second") || lower.contains("pre-owned") || lower.contains("gently") {
        Some(Condition::Used)
    } else if lower.contains("refurbished") || lower.contains("refurb") || lower.contains("renewed") || lower.contains("certified") {
        Some(Condition::Refurbished)
    } else {
        None
    }
}

fn build_response(
    query: &str,
    search_result: &SearchResponse,
) -> (String, Vec<AgentAction>, Vec<String>) {
    let items = &search_result.items;
    let count = items.len();
    let listing_ids: Vec<String> = items.iter().map(|l| l.listing_id.clone()).collect();

    if count == 0 {
        let message = format!(
            "I searched for \"{query}\" but couldn't find any matching listings. Try a different search term or browse our categories."
        );
        return (message, Vec::new(), listing_ids);
    }

    let top = &items[0];
    let message = format!(
        "I found {} listing{} matching \"{query}\". The best match is \"{}\" for {} {}. {}",
        count,
        if count == 1 { "" } else { "s" },
        top.listing.title,
        top.listing.price.currency,
        top.listing.price.amount,
        if let Some(ref seller) = top.seller_name {
            format!("Sold by {}.", seller)
        } else {
            String::new()
        }
    );

    let mut actions = Vec::new();

    // Action: view the top result
    if let Some(top_id) = listing_ids.first() {
        actions.push(AgentAction {
            action_type: "view_listing".to_string(),
            label: format!("View \"{}\"", top.listing.title),
            params: serde_json::json!({ "listing_id": top_id }),
        });
    }

    // Action: refine search
    actions.push(AgentAction {
        action_type: "search".to_string(),
        label: if count > 1 {
            format!("Show all {} results", count)
        } else {
            "Show result".to_string()
        },
        params: serde_json::json!({}),
    });

    (message, actions, listing_ids)
}
