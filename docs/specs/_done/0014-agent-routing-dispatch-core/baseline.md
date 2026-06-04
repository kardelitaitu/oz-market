# Baseline - Agent Routing and Dispatch Core Layer

## Current State

As of the start of Phase 4:
- The system connects directly to a single configured agent endpoint defined in configuration variables.
- There is no dynamic registry, pool, or capability routing; all requests default to a single static agent.
- A multi-agent dispatcher struct or pool layer does not exist.
