# Spectral Rules for OpenAPI Validation

This document defines custom Spectral rules for validating the Oz Market OpenAPI specification.

## Overview

We use **Spectral** (from Stoplight) to lint our OpenAPI spec and catch issues early.

## Installation

```bash
npm install -g @stoplight/spectral-cli
# or
yarn global add @stoplight/spectral-cli
```

## Usage

```bash
# Lint the OpenAPI spec
spectral lint docs/specs/openapi.yaml

# Use custom ruleset
spectral lint docs/specs/openapi.yaml --ruleset docs/specs/.spectral.yaml
```

## Custom Ruleset (`.spectral.yaml`)

```yaml
extends: [[spectral:oas, all]]

rules:
  # Require operationId for all operations
  operation-id-required:
    severity: error
    message: "Every operation must have an operationId"
    given: "$.paths.*.*"
    then:
      field: operationId
      function: truthy

  # Require description for all operations
  operation-description-required:
    severity: warning
    message: "Every operation should have a description"
    given: "$.paths.*.*"
    then:
      field: description
      function: truthy

  # Require tags for all operations
  operation-tags-required:
    severity: error
    message: "Every operation must have at least one tag"
    given: "$.paths.*.*"
    then:
      field: tags
      function: schema
      functionOptions:
        type: array
        minItems: 1

  # Require responses for all operations
  operation-responses-required:
    severity: error
    message: "Every operation must define responses"
    given: "$.paths.*.*"
    then:
      field: responses
      function: schema
      functionOptions:
        type: object
        minProperties: 1

  # Require 200 response for successful operations
  operation-success-response:
    severity: error
    message: "Operation should have a 200 or 201 response"
    given: "$.paths.*.*"
    then:
      function: xor
      functionOptions:
        properties:
          - field: responses["200"]
            function: truthy
          - field: responses["201"]
            function: truthy
          - field: responses["204"]
            function: truthy

  # Require authentication for all operations except health
  operation-auth-required:
    severity: error
    message: "Operation must have security defined (except /health)"
    given: "$.paths[?(@property !== '/health')].*"
    then:
      field: security
      function: schema
      functionOptions:
        type: array
        minItems: 1

  # Require request body for POST/PUT operations
  operation-request-body-required:
    severity: error
    message: "POST/PUT operations must have a requestBody"
    given: "$.paths.*[?(@property === 'post' || @property === 'put')]"
    then:
      field: requestBody
      function: truthy

  # Require pagination parameters for list endpoints
  list-endpoint-pagination:
    severity: warning
    message: "List endpoints should have page and page_size parameters"
    given: "$.paths.*.get[?(@.operationId && @.operationId.match(/search|list/))]"
    then:
      field: parameters
      function: schema
      functionOptions:
        type: array
        contains:
          type: object
          required: ["name"]
          properties:
            name:
              enum: ["page", "page_size"]

  # Require consistent error response schema
  error-response-schema:
    severity: error
    message: "Error responses must use ErrorResponse schema"
    given: "$.paths.*.*.responses[?(@property.match(/4[0-9]{2}|5[0-9]{2}/))].content.application/json.schema"
    then:
      field: '$ref'
      function: pattern
      functionOptions:
        match: ".*ErrorResponse"

  # No examples in schema (keep spec clean)
  no-examples-in-schema:
    severity: warning
    message: "Avoid embedding examples in schema; use examples folder"
    given: "$.components.schemas.*.example"
    then:
      function: undefined

  # Require contact reveal approval flow
  contact-reveal-approval-required:
    severity: error
    message: "Contact reveal endpoints must follow approval flow"
    given: "$.paths[?(@property.match(/contact-reveals/))].*"
    then:
      field: operationId
      function: pattern
      functionOptions:
        match: ".*(request|approve|reject).*"

  # Enforce naming convention for operationIds
  operation-id-naming:
    severity: warning
    message: "operationId should follow: {resource}_{action}"
    given: "$.paths.*.*.operationId"
    then:
      function: pattern
      functionOptions:
        match: "^[a-z_]+$"

  # Require seller_id in listing creation
  listing-creation-requires-seller:
    severity: error
    message: "Listing creation must not include seller_id (set by server)"
    given: "$.paths./api/listings.post.requestBody.content.application/json.schema.properties"
    then:
      field: seller_id
      function: undefined

  # Enforce price precision (max 2 decimal places)
  price-precision:
    severity: error
    message: "Price fields must have maximum 2 decimal places"
    given: "$.components.schemas.*.properties[?(@property.match(/price/))].multipleOf"
    then:
      function: truthy

  # Require ratings to be between 1 and 5
  rating-range:
    severity: error
    message: "Rating must be between 1 and 5"
    given: "$.components.schemas.*.properties[?(@property === 'rating')]"
    then:
      field: minimum
      function: equal
      functionOptions:
        value: 1
    also:
      field: maximum
      function: equal
      functionOptions:
        value: 5
```

## Running in CI

Add to your GitHub Actions workflow:

```yaml
- name: Lint OpenAPI spec
  run: |
    npm install -g @stoplight/spectral-cli
    spectral lint docs/specs/openapi.yaml --ruleset docs/specs/.spectral.yaml
```

## Common Errors and Fixes

### Error: "Every operation must have an operationId"
**Fix**: Add `operationId` to each operation in `openapi.yaml`.

### Error: "Operation must have security defined"
**Fix**: Add `security` block to the operation or path.

### Warning: "List endpoints should have page parameters"
**Fix**: Add `page` and `page_size` query parameters.

## Custom Rules Explained

1. **operation-auth-required**: Ensures all endpoints (except `/health`) require authentication
2. **contact-reveal-approval-required**: Enforces the contact reveal approval flow
3. **listing-creation-requires-seller**: Prevents clients from setting `seller_id` (security)
4. **price-precision**: Ensures prices don't have more than 2 decimal places
5. **rating-range**: Validates that ratings are between 1 and 5

---

**Keep the API spec clean and consistent!** 🧹
