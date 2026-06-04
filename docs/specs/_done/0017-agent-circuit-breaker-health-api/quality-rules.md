# Quality Rules - Agent Circuit-Breaker and Health API

- **Fast Fallback**: When an agent's circuit breaker is in the `Open` state, any attempt to route a query to it must return a fast error response immediately, without triggering network socket calls or hitting internal timeouts.
- **Fail-Open Safe Route**: If all matching registry agents are circuit-broken (Open), the router should trigger a graceful failover fallback strategy (e.g. route to default search database agent directly) rather than returning 500 Server Errors.
- **Valid REST Responses**: The health JSON responses must match the schemas defined in `docs/specs/openapi.yaml` exactly.
