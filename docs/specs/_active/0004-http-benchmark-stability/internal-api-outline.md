# Internal API Outline

## Benchmarked Routes

- `POST /v1/listings`
- `GET /v1/listings/{id}`
- `POST /v1/listings/search`
- `POST /v1/negotiations`
- `GET /v1/negotiations/{id}`
- `POST /v1/negotiations/{id}/request-contact-reveal`

## Expected Status Highlights

- `POST /v1/listings` => `201`
- `POST /v1/negotiations` => `201`
- `POST /v1/negotiations/{id}/request-contact-reveal` => `202`
