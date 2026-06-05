# Decisions - Distributed Ledger Cache Synchronization

## Architecture Decisions

### 1. Redis Cache write-through with Postgres as Source of Truth
- **Decision**: All balance modifications must commit to Postgres first. Upon success, they update the Redis key.
- **Rationale**: Postgres provides strict ACID transaction guarantees. Using Redis only as a cache ensures we do not lose financial records in the event of Redis node crashes.

### 2. Cache Fail-Open
- **Decision**: In the event of a Redis connection loss, the server must fail-open and fetch data directly from Postgres.
- **Rationale**: Prevents a Redis failure from causing a total outage of the credit and checkouts system.
