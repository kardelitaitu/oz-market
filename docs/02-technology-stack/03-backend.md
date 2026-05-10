**Core Language**
	Rust 2021
	The primary language for the backend workspace.
	*Why we choose it: Strong safety, good performance, and a clean fit for server work.*

**HTTP Runtime**
	Actix-web
	The main HTTP server framework for the backend.
	*Why we choose it: Mature, fast, and already integrated in the current codebase.*

**Async Runtime**
	Tokio
	The async runtime used for server I/O and background work.
	*Why we choose it: Stable ecosystem support and standard Rust async tooling.*

**Serialization**
	Serde + serde_json
	Canonical JSON handling for request and response payloads.
	*Why we choose it: Deterministic serialization with broad Rust ecosystem support.*

**Persistence Layer**
	SQLx + PostgreSQL
	The backend data access path for typed SQL and database operations.
	*Why we choose it: Explicit queries, good performance, and strong PostgreSQL support.*

**Caching**
	Moka
	In-process caching for hot backend reads.
	*Why we choose it: Lightweight and easy to integrate without a separate cache service.*

**Observability**
	Tracing + metrics
	Logging and metrics support for backend visibility.
	*Why we choose it: Improves debugging and production monitoring.*

**API Docs**
	Utoipa + Swagger UI
	OpenAPI generation and interactive API documentation.
	*Why we choose it: Keeps docs close to the implementation.*