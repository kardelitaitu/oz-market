# Internal API Outline

## Service Entry

- service: `listing_service::get_listing(listing_id, actor_ctx) -> Result<Listing, DomainError>`
- repository calls stay behind service boundaries

## HTTP Mapping

- `GET /v1/listings/{listing_id}` -> `listing_service::get_listing`
- legacy routes (`/v1/product/{listing_id}`, `/v1/service/{listing_id}`, `/v1/property/{listing_id}`) -> same service + deprecation behavior

## MCP Mapping

- listing read tool -> `listing_service::get_listing`
- keep request validation and auth context normalization in MCP transport only

