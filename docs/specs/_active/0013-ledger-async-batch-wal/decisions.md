# Decisions - Write-Ahead Log (WAL) and Async Batch Committer

## Architecture Decisions

### 1. Synchronous Disk Flush on WAL Entry Creation
- **Decision**: A transaction is acknowledged to the client only after its metadata is written to the local WAL file and flushed to disk using `std::fs::File::sync_all()`.
- **Rationale**: Ensures durability of credits even if power is lost immediately after acknowledgment.

### 2. Multi-Row Batch Updates via Single SQL Statement
- **Decision**: The background task will consolidate individual agent balance adjustments and submit them via a single dynamic SQL statement utilizing `INSERT INTO agent_balances ... ON CONFLICT (agent_id) DO UPDATE SET balance_credits = agent_balances.balance_credits + EXCLUDED.balance_credits`.
- **Rationale**: Significantly reduces database execution locks and network latency compared to serial transaction executions.
