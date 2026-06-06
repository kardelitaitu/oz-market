# Quality Rules - Benchmark Component Drivers

- **Connection Pool Re-use**: Drivers must re-use the existing configured connection pools (e.g. `Arc<PgPool>`) rather than spawning new connection instances per operation.
- **Strict File Sandboxing**: All WAL testing files must be restricted to a configured sandbox temp directory and never touch production log folders.
- **Resource Recovery**: `teardown()` must execute successfully even if previous operations failed, ensuring database records are cleaned and connections returned cleanly.
