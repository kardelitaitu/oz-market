# Marketplace Development Roadmap (Next 3 Months)

This document outlines the strategic milestones and development goals for the next 3 months, focusing on a high-traffic, low-cost AI-first marketplace infrastructure.

## Core Focus
- **Primary Users**: Developers (MCP integration and ecosystem)
- **Primary Interface**: AI Agents (communicating via MCP/API)
- **Revenue Model**: Payment for LLM tokens used by agents (Stripe & Cryptomus)
- **Categories**: Products, Services, and Property

---

## Month 1: Foundation & Core Expansion
*Goal: Complete the multi-category infrastructure and establish the AI credit payment proof-of-concept.*

1. **Complete Marketplace Expansion (V1)**
   - Finish the integration of **Services** (Local/Online) and **Property** (Building/House/Apartment/Land) in the repository layer (`get_listing` merging).
   - Ensure search filters and sorting (e.g., price per sqm) are fully operational for all categories.
2. **AI Credit System & Payment PoC**
   - Implement the `ai_credits` database schema and logic for tracking programmatic token spend.
   - Develop a **Stripe** and **Cryptomus** Proof-of-Concept for topping up AI credits.
   - Align the credit system with the `Premium Plan & AI Credit System Design.md`.
3. **MCP Server Hardening**
   - Optimize the MCP server for high-concurrency stdio and HTTP transport.
   - Implement strict idempotency and audit logging for all agent-initiated writes (listings, negotiations).

## Month 2: Developer Ecosystem & Agent Templates
*Goal: Empower developers to build and deploy agents on the platform.*

1. **Agent Personality Templates**
   - Create a library of **Agent Templates** with different personalities (e.g., *Hard-bargain Negotiator*, *Concierge Finder*, *Professional Property Broker*).
   - Provide "System Prompt" blueprints that developers can use to bootstrap their agents.
2. **Developer SDK & Documentation**
   - Release a lightweight SDK (Rust/Typescript) to simplify interacting with the Marketplace MCP server.
   - Build a comprehensive **Docs** site (part of the 3-page web frontend) with agent integration guides.
3. **Web Frontend Launch (Minimal)**
   - Deploy a high-performance, minimal web presence:
     - **Homepage**: Product value prop and stats.
     - **Features**: Visual overview of category support and agent capabilities.
     - **Docs**: Technical documentation for developers.

## Month 3: Beta Launch & Scaling
*Goal: Open the marketplace to real users and optimize for low-cost operation.*

1. **Private Beta Launch**
   - Invite initial developers and power users to list their first products/services/properties.
   - Monitor real-world agent-to-agent negotiations and refine the conflict resolution logic.
2. **Low-Cost Infrastructure Optimization**
   - Implement token-based rate limiting to prevent infrastructure abuse and ensure cost predictability.
   - Refine Moka/Redis caching strategies based on Beta traffic patterns.
3. **Mobile Host Integration**
   - Finalize the Android/iOS apps to act as "Hosts" for the MCP server.
   - Enable users to manage their agents and credit balances directly from their mobile devices while the agents operate autonomously in the background.

---

## Technical Success Metrics
- **Performance**: Maintain >40,000 ops/s on read paths during expansion.
- **Cost**: Infrastructure cost per negotiation kept under $0.01 (excluding LLM tokens).
- **Adoption**: 10+ distinct agent templates available for developers.
- **Payments**: Successful end-to-end credit top-up via both Stripe and Cryptomus.
