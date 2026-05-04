# Search Indexing

## Goal

Add `fast product search` through `smart multi-dimensional indexing` without breaking the compact architecture.

The search path should be optimized for how agents actually query:

- product text
- category
- country code
- city
- condition
- price range
- listing status

## Recommendation

Start with `PostgreSQL-native indexing`.

Do not start with Elasticsearch, OpenSearch, or a custom ranking service unless measurements prove PostgreSQL is not enough.

## Why Multi-Dimensional Indexing

Agent search is rarely one-dimensional. A buyer agent may want:

- `thinkpad`
- in category `laptop`
- in `Jakarta`
- under `500`
- only `active` listings

That means the search layer should combine:

- structured filters
- text matching
- partial indexes for active inventory

## Recommended Search Dimensions

| Dimension | Source | Purpose |
| --- | --- | --- |
| `category` | typed column | Product grouping and narrowing |
| `status` | typed column | Fast filtering to active inventory |
| `country_code` | typed column | Geographic narrowing |
| `city` | typed column | Local search narrowing |
| `condition` | typed column | Condition filtering |
| `currency` | typed column | Currency-safe price filtering |
| `price_amount` | typed column | Price range filtering |
| `product_name` | typed column | Exact or prefix matching |
| `search_text` | derived/search column | Full-text or trigram matching |

## Recommended PostgreSQL Strategy

Use a hybrid approach:

- typed columns for structured filters
- `search_text` column for normalized searchable text
- `GIN` full-text or `trigram` index for fuzzy text matching
- composite indexes for common filter combinations
- partial indexes for `active` listings

## Suggested Query Shape

The V1 API should encourage predictable search:

```json
{
  "query": "thinkpad",
  "category": "laptop",
  "condition": "used",
  "price": {
    "currency": "USD",
    "min_amount": 300,
    "max_amount": 500
  },
  "location": {
    "country_code": "JP",
    "city": "Osaka"
  },
  "status": "active",
  "limit": 20
}
```

This is easier to index than fully free-form search.

## Comparison

| Approach | Pros | Cons |
| --- | --- | --- |
| PostgreSQL multi-dimensional indexes | Compact architecture, fewer moving parts, strong reliability | Requires careful query design |
| External search engine from day one | More advanced ranking and fuzzy search | More ops cost, more sync complexity, bigger codebase |

## Best Practice Rules

- normalize searchable text into `search_text`
- only allow indexed filter fields in V1 search
- keep result size capped
- support pagination or cursoring
- benchmark search separately from listing reads
- avoid sorting patterns that bypass indexes

## Future Upgrade Path

Only consider an external search engine if:

- search latency becomes the main bottleneck
- ranking complexity grows beyond PostgreSQL support
- write amplification from index maintenance becomes too costly

Until then, `PostgreSQL-first search` is the better engineering choice.
