**Storage Engine**
	PostgreSQL
	The primary relational database for marketplace data.
	*Why we choose it: Mature reliability, strong transactions, and a proven scaling path.*

**Access Layer**
	SQLx
	Explicit PostgreSQL access from Rust without a heavy ORM layer.
	*Why we choose it: Keeps SQL visible, predictable, and easy to tune.*

**Migration Style**
	Versioned SQL migrations
	Ordered SQL files for schema changes and rollouts.
	*Why we choose it: Repeatable upgrades and simple review flow.*

**Schema Style**
	Typed columns + JSONB
	Hot fields stay in columns while flexible payload parts stay in JSONB.
	*Why we choose it: Balances query speed with payload flexibility.*

**Search Extension**
	pg_trgm
	PostgreSQL trigram support for text search.
	*Why we choose it: Useful search matching without adding a second search engine too early.*

**Connection Pool**
	PgPool
	The database connection pool used by the Rust backend.
	*Why we choose it: Stable async pooling and easy integration with SQLx.*

**Extra Data Layer**
	None in V1
	No Redis or secondary database layer for the first release.
	*Why we choose it: Fewer moving parts and lower operational overhead.*
