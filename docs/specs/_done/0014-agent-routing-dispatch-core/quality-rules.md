# Quality Rules - Agent Routing and Dispatch Core Layer

- **Thread-Safety Guarantees**: `AgentRegistry` modifications and reads must be completely lock-free or use low-contention read-heavy synchronization mechanisms (like `DashMap` or `parking_lot::RwLock`).
- **Connection Reuse**: The HTTP dispatcher must reuse connection pools (e.g., using a shared `reqwest::Client` instance) rather than instantiating a new client per request.
- **Fail-Safe Dispatches**: In case of routing failures, the dispatcher must return structured `DispatchError` variants instead of generic panic or raw network errors.
