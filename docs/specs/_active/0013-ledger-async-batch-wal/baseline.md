# Baseline - Write-Ahead Log (WAL) and Async Batch Committer

## Current State

As of the completion of Specs 0010 and 0011:
- The system uses a synchronous write-through ledger. Every spend or deposit waits for a blocking PostgreSQL round-trip, which limits maximum transaction throughput to the database's round-trip write rate.
- There is no background thread task processing batched operations.
- No local file system writes are executed for logging transaction progress before database persistence.
