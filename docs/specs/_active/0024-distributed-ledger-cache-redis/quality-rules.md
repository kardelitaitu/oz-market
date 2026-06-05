# Quality Rules - Distributed Ledger Cache Synchronization

- **Fail-Open Safety**: A failure in Redis connection, parsing, or command timeout must never crash the server thread or return a 500 error to clients; it must fallback silently to Postgres.
- **Lease Timeout (TTL)**: All keys set in Redis must have a lease timeout (e.g. 1 hour) to ensure garbage collection of dead keys.
- **Connection Reuse**: Use a shared `ConnectionManager` client pool to ensure socket connections are reused efficiently across tasks.
