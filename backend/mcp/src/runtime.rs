use std::error::Error;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use marketplace_api_contract::{
    AcceptNegotiationRequest, AgentQueryRequest, AgentQueryResponse, ContactRevealResponse,
    CreateListingRequest, NegotiationResponse, OpenNegotiationRequest, RejectNegotiationRequest,
    RequestContactRevealRequest, SearchRequest, SearchResponse, SubmitOfferRequest,
};
use marketplace_auth_core::Claims;
use marketplace_server::app::MarketplaceApp;
use marketplace_server::http::handlers::HandlerError;
use marketplace_server::http::runtime::current_time_marker;
use marketplace_server::repositories::audit_events::{
    InMemoryAuditEventRepository, PostgresAuditEventRepository,
};
use marketplace_server::repositories::contact_reveals::{
    InMemoryContactRevealRepository, PostgresContactRevealRepository,
};
use marketplace_server::repositories::listings::{
    InMemoryListingRepository, PostgresListingRepository,
};
use marketplace_server::repositories::negotiations::PostgresNegotiationRepository;
use marketplace_server::repositories::outbox_events::{
    InMemoryOutboxEventRepository, PostgresOutboxEventRepository,
};
use marketplace_server::repositories::reservations::{
    InMemoryReservationLeaseRepository, PostgresReservationLeaseRepository,
};
use marketplace_server::repositories::seller_accounts::{
    InMemorySellerAccountRepository, PostgresSellerAccountRepository,
};
use marketplace_server::repositories::{
    ContactRevealRepository, IdempotencyKeyRepository, ListingRepository, RepositoryErrorKind,
    ReservationLeaseRepository, SellerAccountRepository,
};
use marketplace_server::services::idempotency::IdempotencyErrorKind;
use marketplace_server::services::idempotency::InMemoryIdempotencyRepository;
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};

type ProductionApp = MarketplaceApp<
    PostgresListingRepository,
    InMemoryIdempotencyRepository,
    PostgresReservationLeaseRepository,
    PostgresContactRevealRepository,
>;

type InMemoryApp = MarketplaceApp<
    InMemoryListingRepository,
    InMemoryIdempotencyRepository,
    InMemoryReservationLeaseRepository,
    InMemoryContactRevealRepository,
>;

const DEFAULT_TOOL_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct MarketplaceMcpAgent<LR, IR, RR, CR>
where
    LR: ListingRepository + Send + Sync + 'static,
    IR: IdempotencyKeyRepository + Send + Sync + 'static,
    RR: ReservationLeaseRepository + Send + Sync + 'static,
    CR: ContactRevealRepository + Send + Sync + 'static,
{
    app: Arc<MarketplaceApp<LR, IR, RR, CR>>,
    claims: Claims,
    tool_router: ToolRouter<Self>,
    tool_timeout: Duration,
}

impl<LR, IR, RR, CR> MarketplaceMcpAgent<LR, IR, RR, CR>
where
    LR: ListingRepository + Send + Sync + 'static,
    IR: IdempotencyKeyRepository + Send + Sync + 'static,
    RR: ReservationLeaseRepository + Send + Sync + 'static,
    CR: ContactRevealRepository + Send + Sync + 'static,
{
    pub fn new(app: MarketplaceApp<LR, IR, RR, CR>, claims: Claims) -> Self {
        Self {
            app: Arc::new(app),
            claims,
            tool_router: Self::tool_router(),
            tool_timeout: Self::resolve_timeout(),
        }
    }

    #[allow(dead_code)]
    pub fn with_timeout(
        app: MarketplaceApp<LR, IR, RR, CR>,
        claims: Claims,
        tool_timeout: Duration,
    ) -> Self {
        Self {
            app: Arc::new(app),
            claims,
            tool_router: Self::tool_router(),
            tool_timeout,
        }
    }

    fn resolve_timeout() -> Duration {
        std::env::var("MCP_TOOL_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(DEFAULT_TOOL_TIMEOUT)
    }

    fn claims(&self) -> &Claims {
        &self.claims
    }

    async fn run_with_timeout<F, T>(&self, future: F) -> Result<T, McpToolError>
    where
        F: Future<Output = Result<T, McpToolError>>,
    {
        tokio::time::timeout(self.tool_timeout, future)
            .await
            .map_err(|_| McpToolError::timeout())?
    }
}

#[tool_router]
impl<LR, IR, RR, CR> MarketplaceMcpAgent<LR, IR, RR, CR>
where
    LR: ListingRepository + Send + Sync + 'static,
    IR: IdempotencyKeyRepository + Send + Sync + 'static,
    RR: ReservationLeaseRepository + Send + Sync + 'static,
    CR: ContactRevealRepository + Send + Sync + 'static,
{
    #[tool(description = "Create a new listing")]
    async fn create_listing(
        &self,
        Parameters(request): Parameters<CreateListingRequest>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let fingerprint = serde_json::to_string(&request)
                .map_err(|error| McpToolError::internal(error.to_string()))?;
            let result = self
                .app
                .create_listing(
                    self.claims(),
                    &request,
                    &fingerprint,
                    &current_time_marker(),
                )
                .await
                .map_err(McpToolError::from)?;
            let response = result.0;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Search listings")]
    async fn search_listings(
        &self,
        Parameters(request): Parameters<SearchRequest>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let response: SearchResponse = self
                .app
                .search_listings(Some(self.claims()), &request)
                .await
                .map_err(McpToolError::from)?;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Query the marketplace with natural language")]
    async fn agent_query(
        &self,
        Parameters(request): Parameters<AgentQueryRequest>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let response: AgentQueryResponse = self
                .app
                .agent_query(Some(self.claims()), &request)
                .await
                .map_err(McpToolError::from)?;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Get a listing by id")]
    async fn get_listing(
        &self,
        Parameters(request): Parameters<GetListingInput>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let response = self
                .app
                .get_listing(Some(self.claims()), &request.listing_id)
                .await
                .map_err(McpToolError::from)?;
            let response = response.ok_or_else(|| McpToolError::not_found("listing not found"))?;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Open a negotiation")]
    async fn open_negotiation(
        &self,
        Parameters(request): Parameters<OpenNegotiationRequest>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let fingerprint = serde_json::to_string(&request)
                .map_err(|error| McpToolError::internal(error.to_string()))?;
            let result = self
                .app
                .open_negotiation(
                    self.claims(),
                    &request,
                    &fingerprint,
                    &current_time_marker(),
                )
                .await
                .map_err(McpToolError::from)?;
            let response = result.0;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Submit an offer")]
    async fn submit_offer(
        &self,
        Parameters(request): Parameters<SubmitOfferInput>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let fingerprint = serde_json::to_string(&request)
                .map_err(|error| McpToolError::internal(error.to_string()))?;
            let offer = SubmitOfferRequest {
                offer_currency: request.offer_currency,
                offer_amount: request.offer_amount,
                idempotency_key: request.idempotency_key,
            };
            let response: NegotiationResponse = self
                .app
                .submit_offer(
                    self.claims(),
                    &request.negotiation_id,
                    &offer,
                    &fingerprint,
                    &current_time_marker(),
                )
                .await
                .map_err(McpToolError::from)?;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Get negotiation status")]
    async fn get_negotiation_status(
        &self,
        Parameters(request): Parameters<GetNegotiationStatusInput>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let response: NegotiationResponse = self
                .app
                .get_negotiation_status(self.claims(), &request.negotiation_id)
                .await
                .map_err(McpToolError::from)?;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Request contact reveal for a reserved negotiation")]
    async fn request_contact_reveal(
        &self,
        Parameters(request): Parameters<RequestContactRevealInput>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let fingerprint = serde_json::to_string(&request)
                .map_err(|error| McpToolError::internal(error.to_string()))?;
            let payload = RequestContactRevealRequest {
                idempotency_key: request.idempotency_key,
            };
            let response: ContactRevealResponse = self
                .app
                .request_contact_reveal(
                    self.claims(),
                    &request.negotiation_id,
                    &payload,
                    &fingerprint,
                    &current_time_marker(),
                )
                .await
                .map_err(McpToolError::from)?;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Approve a contact reveal")]
    async fn approve_contact_reveal(
        &self,
        Parameters(request): Parameters<ApproveContactRevealInput>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let response: ContactRevealResponse = self
                .app
                .approve_contact_reveal(self.claims(), &request.reveal_id)
                .await
                .map_err(McpToolError::from)?;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Accept a negotiation")]
    async fn accept_negotiation(
        &self,
        Parameters(request): Parameters<AcceptNegotiationInput>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let fingerprint = serde_json::to_string(&request)
                .map_err(|error| McpToolError::internal(error.to_string()))?;
            let payload = AcceptNegotiationRequest {
                idempotency_key: request.idempotency_key,
            };
            let response: NegotiationResponse = self
                .app
                .accept_negotiation(
                    self.claims(),
                    &request.negotiation_id,
                    &payload,
                    &fingerprint,
                    &current_time_marker(),
                )
                .await
                .map_err(McpToolError::from)?;
            json_string(&response)
        })
        .await
    }

    #[tool(description = "Reject a negotiation")]
    async fn reject_negotiation(
        &self,
        Parameters(request): Parameters<RejectNegotiationInput>,
    ) -> Result<String, McpToolError> {
        self.run_with_timeout(async {
            let fingerprint = serde_json::to_string(&request)
                .map_err(|error| McpToolError::internal(error.to_string()))?;
            let payload = RejectNegotiationRequest {
                idempotency_key: request.idempotency_key,
            };
            let response: NegotiationResponse = self
                .app
                .reject_negotiation(
                    self.claims(),
                    &request.negotiation_id,
                    &payload,
                    &fingerprint,
                    &current_time_marker(),
                )
                .await
                .map_err(McpToolError::from)?;
            json_string(&response)
        })
        .await
    }
}

#[tool_handler(router = self.tool_router)]
impl<LR, IR, RR, CR> ServerHandler for MarketplaceMcpAgent<LR, IR, RR, CR>
where
    LR: ListingRepository + Send + Sync + 'static,
    IR: IdempotencyKeyRepository + Send + Sync + 'static,
    RR: ReservationLeaseRepository + Send + Sync + 'static,
    CR: ContactRevealRepository + Send + Sync + 'static,
{
    fn get_info(&self) -> ServerInfo {
        let timeout_ms = self.tool_timeout.as_millis();
        ServerInfo {
            instructions: Some(format!(
                "Marketplace desktop MCP sidecar. Tool execution timeout: {timeout_ms}ms. Use MCP_TOOL_TIMEOUT_MS env var to configure."
            )),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct GetListingInput {
    listing_id: String,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct GetNegotiationStatusInput {
    negotiation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct SubmitOfferInput {
    negotiation_id: String,
    offer_currency: marketplace_api_contract::CurrencyCode,
    offer_amount: f64,
    idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct RequestContactRevealInput {
    negotiation_id: String,
    idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct ApproveContactRevealInput {
    reveal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct AcceptNegotiationInput {
    negotiation_id: String,
    idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
struct RejectNegotiationInput {
    negotiation_id: String,
    idempotency_key: String,
}

#[derive(Debug, Clone)]
pub struct McpToolError {
    code: &'static str,
    message: String,
    details: Option<serde_json::Value>,
}

impl McpToolError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    fn with_details(
        code: &'static str,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details),
        }
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self::new("forbidden", message)
    }

    fn invalid_field(message: impl Into<String>) -> Self {
        Self::new("invalid_field", message)
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self::new("conflict", message)
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    fn quota_exceeded(message: impl Into<String>) -> Self {
        Self::new("quota_exceeded", message)
    }

    fn internal(message: impl Into<String>) -> Self {
        let msg = message.into();
        eprintln!("[mcp] internal_error: {msg}");
        Self::new("internal_error", msg)
    }

    fn timeout() -> Self {
        eprintln!("[mcp] tool execution timed out");
        Self::new("timeout", "Tool execution timed out")
    }
}

impl std::fmt::Display for McpToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for McpToolError {}

impl rmcp::model::IntoContents for McpToolError {
    fn into_contents(self) -> Vec<Content> {
        let payload = serde_json::json!({
            "error": {
                "code": self.code,
                "message": self.message,
                "details": self.details,
            }
        });
        vec![Content::text(payload.to_string())]
    }
}

impl From<HandlerError> for McpToolError {
    fn from(value: HandlerError) -> Self {
        match value {
            HandlerError::Authz(error) => Self::forbidden(error.to_string()),
            HandlerError::Idempotency(error) => match error.kind {
                IdempotencyErrorKind::InvalidKey => Self::invalid_field(error.message),
                IdempotencyErrorKind::Conflict => Self::with_details(
                    "conflict",
                    error.message,
                    serde_json::json!({"source": "idempotency"}),
                ),
                IdempotencyErrorKind::Storage => Self::internal(error.message),
            },
            HandlerError::Search(error) => match error {
                marketplace_server::services::search::SearchError::Authz(error) => {
                    Self::forbidden(error.to_string())
                }
                marketplace_server::services::search::SearchError::Storage(error) => {
                    Self::internal(error.to_string())
                }
            },
            HandlerError::Repository(error) => match error.kind {
                RepositoryErrorKind::Conflict => Self::conflict(error.message),
                RepositoryErrorKind::NotFound => Self::not_found(error.message),
                RepositoryErrorKind::PermissionDenied => Self::forbidden(error.message),
                RepositoryErrorKind::Validation => Self::invalid_field(error.message),
                RepositoryErrorKind::Storage | RepositoryErrorKind::Unknown => {
                    Self::internal(error.message)
                }
            },
            HandlerError::QuotaExceeded { message } => Self::quota_exceeded(message),
            HandlerError::Agent(error) => Self::invalid_field(error.to_string()),
            HandlerError::Validation { message, .. } => Self::invalid_field(message),
        }
    }
}

fn json_string<T: Serialize>(value: &T) -> Result<String, McpToolError> {
    serde_json::to_string(value).map_err(|error| McpToolError::internal(error.to_string()))
}

fn env_flag_is_truthy(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("on")
    )
}

fn parse_claims_payload(key: &str, raw: &str) -> Result<Claims, Box<dyn Error + Send + Sync>> {
    let claims = serde_json::from_str::<Claims>(raw).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("invalid {key} payload: {error}"),
        )
    })?;
    Ok(claims)
}

fn load_claims_from_env(
    claims_json: Option<String>,
    allow_dev_claims: bool,
) -> Result<Claims, Box<dyn Error + Send + Sync>> {
    if let Some(raw) = claims_json {
        return parse_claims_payload("MARKETPLACE_MCP_CLAIMS_JSON", &raw);
    }

    if allow_dev_claims {
        eprintln!("MARKETPLACE_MCP_ALLOW_DEV_CLAIMS=1; using built-in dev claims");
        return Ok(crate::dev_launcher_claims());
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "missing MARKETPLACE_MCP_CLAIMS_JSON; pass explicit launcher claims or set MARKETPLACE_MCP_ALLOW_DEV_CLAIMS=1 for local smoke tests",
    )
    .into())
}

fn load_claims() -> Result<Claims, Box<dyn Error + Send + Sync>> {
    // Try shared HTTP server API key first
    if let Ok(key) = std::env::var("MARKETPLACE_API_KEY") {
        if !key.is_empty() {
            eprintln!("using MARKETPLACE_API_KEY for MCP agent auth");
            return Ok(Claims {
                sub: "mcp-agent".to_string(),
                roles: vec![
                    marketplace_auth_core::Role::SellerListingWriter,
                    marketplace_auth_core::Role::SellerNegotiator,
                    marketplace_auth_core::Role::SellerContactRevealApprover,
                    marketplace_auth_core::Role::BuyerSearcher,
                    marketplace_auth_core::Role::BuyerNegotiator,
                    marketplace_auth_core::Role::Admin,
                ],
                scopes: vec![
                    marketplace_auth_core::Scope::ListingCreate,
                    marketplace_auth_core::Scope::ListingRead,
                    marketplace_auth_core::Scope::ListingSearch,
                    marketplace_auth_core::Scope::NegotiationCreate,
                    marketplace_auth_core::Scope::NegotiationRead,
                    marketplace_auth_core::Scope::NegotiationOfferSubmit,
                    marketplace_auth_core::Scope::NegotiationRevealRequest,
                    marketplace_auth_core::Scope::RevealApprove,
                ],
                seller_account_id: Some("demo-seller".to_string()),
                buyer_agent_id: Some("demo-buyer".to_string()),
                hardware_id: None,
                exp: None,
            });
        }
    }

    load_claims_from_env(
        std::env::var("MARKETPLACE_MCP_CLAIMS_JSON").ok(),
        env_flag_is_truthy("MARKETPLACE_MCP_ALLOW_DEV_CLAIMS"),
    )
}

fn load_database_url(raw: Option<String>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_run())
}

async fn async_run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let claims = load_claims()?;
    match load_database_url(std::env::var("MARKETPLACE_MCP_DATABASE_URL").ok()) {
        Some(database_url) => {
            let app = build_postgres_app(&database_url).await?;
            run_agent(app, claims).await
        }
        None => {
            let app = build_in_memory_app();
            run_agent(app, claims).await
        }
    }
}

async fn run_agent<LR, IR, RR, CR>(
    app: MarketplaceApp<LR, IR, RR, CR>,
    claims: Claims,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    LR: ListingRepository + Send + Sync + 'static,
    IR: IdempotencyKeyRepository + Send + Sync + 'static,
    RR: ReservationLeaseRepository + Send + Sync + 'static,
    CR: ContactRevealRepository + Send + Sync + 'static,
{
    let service = MarketplaceMcpAgent::new(app, claims);
    let running = service.serve(rmcp::transport::io::stdio()).await?;
    running.waiting().await?;
    Ok(())
}

fn build_in_memory_app() -> InMemoryApp {
    MarketplaceApp::new(
        InMemoryListingRepository::new(),
        InMemoryIdempotencyRepository::new(),
        InMemoryReservationLeaseRepository::new(),
        InMemoryContactRevealRepository::new(),
        Arc::new(
            marketplace_server::repositories::negotiations::InMemoryNegotiationRepository::new(),
        ),
        Arc::new(InMemoryAuditEventRepository::new()),
        Arc::new(InMemoryOutboxEventRepository::new()),
        Arc::new(InMemorySellerAccountRepository::new()),
    )
}

async fn build_postgres_app(
    database_url: &str,
) -> Result<ProductionApp, Box<dyn Error + Send + Sync>> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(
            std::env::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(200),
        )
        .connect(database_url)
        .await?;

    let audit_repo: Arc<dyn marketplace_server::repositories::AuditEventRepository> =
        Arc::new(PostgresAuditEventRepository::new(pool.clone()));
    let outbox_repo: Arc<dyn marketplace_server::repositories::OutboxEventRepository> =
        Arc::new(PostgresOutboxEventRepository::new(pool.clone()));
    let seller_account_repo: Arc<dyn SellerAccountRepository> =
        Arc::new(PostgresSellerAccountRepository::new(pool.clone()));

    Ok(MarketplaceApp::new(
        PostgresListingRepository::new(pool.clone()),
        InMemoryIdempotencyRepository::new(),
        PostgresReservationLeaseRepository::new(pool.clone()),
        PostgresContactRevealRepository::new(pool.clone()),
        Arc::new(PostgresNegotiationRepository::new(pool)),
        audit_repo,
        outbox_repo,
        seller_account_repo,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use marketplace_auth_core::{Role, Scope};
    use marketplace_server::repositories::audit_events::InMemoryAuditEventRepository;
    use marketplace_server::repositories::contact_reveals::InMemoryContactRevealRepository;
    use marketplace_server::repositories::listings::InMemoryListingRepository;
    use marketplace_server::repositories::negotiations::InMemoryNegotiationRepository;
    use marketplace_server::repositories::outbox_events::InMemoryOutboxEventRepository;
    use marketplace_server::repositories::reservations::InMemoryReservationLeaseRepository;
    use marketplace_server::repositories::seller_accounts::InMemorySellerAccountRepository;
    use std::sync::Arc;

    #[tokio::test]
    async fn public_tools_are_listed_and_internal_helpers_are_hidden() {
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let server = MarketplaceMcpAgent::new(app, crate::dev_launcher_claims());
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server_task = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
        });

        let client = rmcp::serve_client((), client_transport).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();
        let names: Vec<String> = tools
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect();

        assert_eq!(names.len(), 11);
        assert!(names.contains(&"create_listing".to_string()));
        assert!(names.contains(&"agent_query".to_string()));
        assert!(names.contains(&"accept_negotiation".to_string()));
        assert!(names.contains(&"reject_negotiation".to_string()));
        assert!(names.contains(&"approve_contact_reveal".to_string()));
        assert!(!names.contains(&"get_contact_reveal".to_string()));

        client.cancel().await.unwrap();
        server_task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn search_tool_calls_shared_app_through_stdio_transport() {
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let server = MarketplaceMcpAgent::new(app, crate::dev_launcher_claims());
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server_task = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
        });

        let client = rmcp::serve_client((), client_transport).await.unwrap();
        let result = client
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "search_listings".into(),
                arguments: Some(
                    serde_json::json!({
                        "query": "ThinkPad",
                        "limit": 10,
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            })
            .await
            .unwrap();

        let text = result
            .content
            .first()
            .and_then(|content| content.raw.as_text())
            .map(|text| text.text.clone())
            .expect("expected json text content");
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("items").is_some());

        client.cancel().await.unwrap();
        server_task.await.unwrap().unwrap();
    }

    #[test]
    fn load_claims_from_env_requires_explicit_payload_by_default() {
        let error = load_claims_from_env(None, false).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("MARKETPLACE_MCP_CLAIMS_JSON"));
        assert!(message.contains("MARKETPLACE_MCP_ALLOW_DEV_CLAIMS"));
    }

    #[test]
    fn load_claims_from_env_accepts_explicit_json_payload() {
        let claims_json = serde_json::to_string(&crate::dev_launcher_claims()).unwrap();
        let claims = load_claims_from_env(Some(claims_json), false).unwrap();
        assert_eq!(claims.sub, "mcp-agent-dev");
        assert!(claims.has_scope(Scope::ListingCreate));
    }

    #[test]
    fn load_claims_from_env_can_opt_in_to_dev_claims() {
        let claims = load_claims_from_env(None, true).unwrap();
        assert_eq!(claims.sub, "mcp-agent-dev");
        assert!(claims.has_role(Role::BuyerNegotiator));
    }

    #[test]
    fn load_database_url_trims_and_rejects_blank_values() {
        assert_eq!(
            load_database_url(Some(" postgres://example ".to_string())),
            Some("postgres://example".to_string())
        );
        assert_eq!(load_database_url(Some("   ".to_string())), None);
        assert_eq!(load_database_url(None), None);
    }

    #[test]
    fn timeout_error_has_correct_code_and_message() {
        let err = McpToolError::timeout();
        assert_eq!(err.code, "timeout");
        assert_eq!(err.message, "Tool execution timed out");
        assert!(err.details.is_none());
    }

    #[test]
    fn resolve_timeout_uses_default_when_env_not_set() {
        let previous = std::env::var("MCP_TOOL_TIMEOUT_MS").ok();
        std::env::remove_var("MCP_TOOL_TIMEOUT_MS");
        let timeout = MarketplaceMcpAgent::<
            InMemoryListingRepository,
            InMemoryIdempotencyRepository,
            InMemoryReservationLeaseRepository,
            InMemoryContactRevealRepository,
        >::resolve_timeout();
        assert_eq!(timeout, DEFAULT_TOOL_TIMEOUT);
        if let Some(value) = previous {
            std::env::set_var("MCP_TOOL_TIMEOUT_MS", value);
        }
    }

    #[tokio::test]
    async fn with_timeout_returns_timeout_error_for_slow_operation() {
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let agent = MarketplaceMcpAgent::with_timeout(
            app,
            crate::dev_launcher_claims(),
            Duration::from_millis(5),
        );

        let result: Result<i32, McpToolError> = agent
            .run_with_timeout(async {
                tokio::time::sleep(Duration::from_secs(1)).await;
                Ok(42)
            })
            .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "timeout");
    }

    #[tokio::test]
    async fn with_timeout_allows_fast_operation_to_complete() {
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let agent = MarketplaceMcpAgent::with_timeout(
            app,
            crate::dev_launcher_claims(),
            Duration::from_secs(10),
        );

        let result: Result<i32, McpToolError> = agent.run_with_timeout(async { Ok(42) }).await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn tool_execution_succeeds_with_timeout_enabled() {
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let server = MarketplaceMcpAgent::with_timeout(
            app,
            crate::dev_launcher_claims(),
            Duration::from_secs(10),
        );
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server_task = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
        });

        let client = rmcp::serve_client((), client_transport).await.unwrap();
        let result = client
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "search_listings".into(),
                arguments: Some(
                    serde_json::json!({
                        "query": "ThinkPad",
                        "limit": 10,
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            })
            .await
            .unwrap();

        let text = result
            .content
            .first()
            .and_then(|content| content.raw.as_text())
            .map(|text| text.text.clone())
            .expect("expected json text content");
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("items").is_some());

        client.cancel().await.unwrap();
        server_task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn get_listing_returns_not_found_with_timeout_enabled() {
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let server = MarketplaceMcpAgent::with_timeout(
            app,
            crate::dev_launcher_claims(),
            Duration::from_secs(10),
        );
        let (server_transport, client_transport) = tokio::io::duplex(4096);

        let server_task = tokio::spawn(async move {
            server.serve(server_transport).await?.waiting().await?;
            Result::<(), Box<dyn Error + Send + Sync>>::Ok(())
        });

        let client = rmcp::serve_client((), client_transport).await.unwrap();
        let result = client
            .call_tool(rmcp::model::CallToolRequestParam {
                name: "get_listing".into(),
                arguments: Some(
                    serde_json::json!({
                        "listing_id": "non-existent-id",
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            })
            .await
            .unwrap();

        let content = result.content.first().unwrap();
        let text = content.raw.as_text().unwrap().text.clone();
        assert!(
            text.contains("not_found") || text.contains("not found"),
            "expected not_found error, got: {text}"
        );

        client.cancel().await.unwrap();
        server_task.await.unwrap().unwrap();
    }

    // ---------------------------------------------------------------------
    // Coverage added during the spec 0006-0017 audit.
    // ---------------------------------------------------------------------

    #[test]
    fn env_flag_is_truthy_accepts_canonical_truthy_values() {
        // Use a unique env var per test to avoid cross-test pollution
        // (env vars are process-global and cargo runs tests in parallel).
        for value in ["1", "true", "TRUE", "yes", "on"] {
            std::env::set_var("MCP_TEST_FLAG_TRUTHY", value);
            assert!(
                env_flag_is_truthy("MCP_TEST_FLAG_TRUTHY"),
                "{value:?} must be truthy"
            );
        }
        std::env::remove_var("MCP_TEST_FLAG_TRUTHY");
    }

    #[test]
    fn env_flag_is_truthy_rejects_empty_and_other_values() {
        for value in ["", "0", "false", "no", "off", "enabled"] {
            std::env::set_var("MCP_TEST_FLAG_FALSY", value);
            assert!(
                !env_flag_is_truthy("MCP_TEST_FLAG_FALSY"),
                "{value:?} must be falsy"
            );
        }
        std::env::remove_var("MCP_TEST_FLAG_FALSY");
    }

    #[test]
    fn env_flag_is_truthy_rejects_unset() {
        std::env::remove_var("MCP_TEST_FLAG_UNSET_XYZ");
        assert!(!env_flag_is_truthy("MCP_TEST_FLAG_UNSET_XYZ"));
    }

    #[test]
    fn parse_claims_payload_rejects_invalid_json() {
        let result = parse_claims_payload("MARKETPLACE_MCP_CLAIMS_JSON", "not json");
        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("MARKETPLACE_MCP_CLAIMS_JSON"));
    }

    #[test]
    fn parse_claims_payload_accepts_valid_json() {
        let claims_json = serde_json::to_string(&crate::dev_launcher_claims()).unwrap();
        let claims = parse_claims_payload("MARKETPLACE_MCP_CLAIMS_JSON", &claims_json).unwrap();
        assert_eq!(claims.sub, "mcp-agent-dev");
    }

    #[test]
    fn mcp_tool_error_display_includes_code_and_message() {
        let err = McpToolError::new("custom_code", "custom message");
        let s = format!("{err}");
        assert_eq!(s, "custom_code: custom message");
    }

    #[test]
    fn mcp_tool_error_into_contents_serializes_json_envelope() {
        let err = McpToolError::with_details(
            "conflict",
            "duplicate",
            serde_json::json!({"source": "idempotency"}),
        );
        let contents = <McpToolError as rmcp::model::IntoContents>::into_contents(err);
        assert_eq!(contents.len(), 1);
        let text = match &contents[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("expected text content"),
        };
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["error"]["code"], "conflict");
        assert_eq!(parsed["error"]["message"], "duplicate");
        assert_eq!(parsed["error"]["details"]["source"], "idempotency");
    }

    #[test]
    fn mcp_tool_error_into_contents_with_no_details_serializes_null() {
        let err = McpToolError::forbidden("denied");
        let contents = <McpToolError as rmcp::model::IntoContents>::into_contents(err);
        let text = match &contents[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("expected text content"),
        };
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["error"]["code"], "forbidden");
        assert!(parsed["error"]["details"].is_null());
    }

    #[test]
    fn mcp_tool_error_from_repository_error_maps_variants() {
        use marketplace_server::repositories::{RepositoryError, RepositoryErrorKind};
        let cases: Vec<(RepositoryErrorKind, &str)> = vec![
            (RepositoryErrorKind::Conflict, "conflict"),
            (RepositoryErrorKind::NotFound, "not_found"),
            (RepositoryErrorKind::PermissionDenied, "forbidden"),
            (RepositoryErrorKind::Validation, "invalid_field"),
        ];
        for (kind, expected) in cases {
            let repo_err = RepositoryError::new(kind, format!("test {expected}"));
            let mcp_err: McpToolError = HandlerError::Repository(repo_err).into();
            assert_eq!(mcp_err.code, expected, "kind {kind:?}");
            assert!(mcp_err.message.contains(expected));
        }
    }

    #[test]
    fn mcp_tool_error_from_repository_storage_maps_to_internal() {
        use marketplace_server::repositories::{RepositoryError, RepositoryErrorKind};
        let repo_err = RepositoryError::new(RepositoryErrorKind::Storage, "db down");
        let mcp_err: McpToolError = HandlerError::Repository(repo_err).into();
        assert_eq!(mcp_err.code, "internal_error");
        assert_eq!(mcp_err.message, "db down");
    }

    #[test]
    fn mcp_tool_error_from_quota_exceeded_preserves_message() {
        let handler_err = HandlerError::QuotaExceeded {
            message: "limit hit".to_string(),
        };
        let mcp_err: McpToolError = handler_err.into();
        assert_eq!(mcp_err.code, "quota_exceeded");
        assert_eq!(mcp_err.message, "limit hit");
    }

    #[test]
    fn get_info_includes_timeout_in_instructions() {
        let app = MarketplaceApp::new(
            InMemoryListingRepository::new(),
            InMemoryIdempotencyRepository::new(),
            InMemoryReservationLeaseRepository::new(),
            InMemoryContactRevealRepository::new(),
            Arc::new(InMemoryNegotiationRepository::new()),
            Arc::new(InMemoryAuditEventRepository::new()),
            Arc::new(InMemoryOutboxEventRepository::new()),
            Arc::new(InMemorySellerAccountRepository::new()),
        );
        let agent = MarketplaceMcpAgent::with_timeout(
            app,
            crate::dev_launcher_claims(),
            Duration::from_millis(7500),
        );
        let info = agent.get_info();
        let instructions = info.instructions.expect("instructions should be set");
        assert!(
            instructions.contains("7500ms"),
            "instructions must mention tool timeout, got: {instructions}"
        );
        assert!(instructions.contains("MCP_TOOL_TIMEOUT_MS"));
        // Tools capability must be enabled.
        assert!(info.capabilities.tools.is_some());
    }
}
