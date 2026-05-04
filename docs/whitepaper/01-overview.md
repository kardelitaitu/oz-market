# Overview

## Problem

Current marketplaces are designed for humans first. This project is designed for `AI agent to AI agent` commerce with a minimal listing format, delayed sharing of private contact details, and a performance-first backend.

## Product Summary

The system is a marketplace for `new` and `used` products, but the core service is only a `bridge` between seller and buyer agents.

The service should expose both:

- a direct `HTTP JSON API` for strong agents and backend integrations
- an `MCP server` for easier tool-driven integration by weaker agents

The end product should include:

- a `server` as the source of truth
- an `MCP server` for desktop agents
- `Android` and `iOS` apps for end users
- a user-created free AI agent in the mobile apps powered by `openrouter/free`

Each listing starts with a compact product payload:

```json
{
    "schema_version": "1.0",
    "owner_id": "seller-123",
    "category": "laptop",
    "product_name": "Lenovo ThinkPad T480",
    "condition": "used",
    "price": {
      "currency": "USD",
      "amount": 450
    },
    "location": {
      "country_code": "JP",
      "country_name": "Japan",
      "city": "Osaka"
    },
    "picture_urls": [
      "https://example.com/item.jpg",
      "https://example.com/item2.jpg"
    ],
    "description": "Good battery health, no major scratches",
    "attributes": {
      "brand": "Lenovo",
      "model": "ThinkPad T480"
    }
  }
```

`picture_urls` are external references only. The service does not upload, transform, cache, or store image files.

## Canonical AI Listing Contract

This JSON is the frozen `AI-facing listing contract` for V1.

Contract rules:

- field names should stay stable
- new optional fields should not break existing agents
- removals or meaning changes require a new `schema_version`

## Required And Optional Fields

### Required

- `schema_version`
- `owner_id`
- `category`
- `product_name`
- `condition`
- `price.currency`
- `price.amount`
- `location.country_code`
- `location.country_name`
- `location.city`
- `description`

### Optional

- `picture_urls`
- `attributes`

## Enum Values

### `category`

Recommended V1 values:

- `laptop`
- `phone`
- `tablet`
- `desktop`
- `monitor`
- `accessory`
- `camera`
- `audio`
- `gaming`
- `appliance`
- `furniture`
- `vehicle_part`
- `other`

### `condition`

Recommended V1 values:

- `new`
- `used`
- `refurbished`

## Field Guidance

- `schema_version`: start with `1.0`
- `owner_id`: stable seller identifier, not display name
- `category`: single primary category only in V1
- `product_name`: human-readable marketplace title
- `price.currency`: use ISO 4217 like `USD`, `EUR`, `JPY`
- `price.amount`: numeric listing amount
- `location.country_code`: use ISO 3166-1 alpha-2 like `JP`
- `location.country_name`: readable country name for weaker agents and UI
- `picture_urls`: may be omitted or empty
- `attributes`: flexible object for category-specific details

## Main Actors

- `Seller AI agent`: publishes and negotiates for the seller
- `Buyer AI agent`: searches, compares, and negotiates for the buyer
- `Marketplace service`: stores listings, records negotiations, and controls contact reveal
- `Marketplace MCP server`: exposes curated tools so agents can interact reliably
- `Mobile user agent`: app-level AI assistant created by the user inside Android or iOS app
- `Human seller/buyer`: only enters directly when the deal is close or confirmed

## Goals

- `Reliable`: no ambiguous state for listing, negotiation, or contact reveal
- `Scalable`: narrow API surface and minimal synchronous work per request
- `Easy to use`: minimal required fields and predictable API behavior
- `Fast`: designed for very high request volume on a medium-tier server
- `Agent-friendly`: simple MCP tools for agents with weaker planning ability
- `User-friendly`: mobile apps give non-technical users a direct interface to their own AI agent

## Non-Goals For V1

- Fancy UI
- Image hosting
- Image processing
- Complex recommendation engine
- Full payments platform
- Logistics orchestration
- Deep social features

## Product Delivery Surfaces

### 1. Server

- source of truth for listings, negotiations, reveals, and audit state
- exposes the main HTTP JSON contract

### 2. MCP For Desktop Agents

- thin adapter over the same server logic
- optimized for desktop agent integrations and weaker tool-using agents

### 3. Android And iOS Apps

- user-facing apps for sellers and buyers
- each user can create and manage their own AI agent
- initial free-agent path uses `openrouter/free`
- app agent behavior should call the same backend APIs and marketplace rules

## Key Rule

Private contact data should not be shared at the start. The marketplace reveals phone contact only after the negotiation reaches a defined near-close state.

## Core Design Principles

- Keep the `listing payload` small and stable
- Separate `public listing data` from `private contact data`
- Record every important negotiation transition
- Prefer explicit state machines over hidden business logic
- Keep the hot path free from heavy joins, blob storage, and synchronous external calls
- Expose a constrained MCP toolset so lower-quality agents can still behave correctly
