# Quality Rules - Transactional Outbox Pattern

- **At-Least-Once Delivery**: Events must only be updated to `published` if the external publishing stream responds with a valid acknowledgment.
- **Index Optimization**: The outbox table must have a composite index on `(status, created_at)` to keep polling query execution times $< 1$ms.
- **No Transaction Overlap**: The background worker must wait for the current batch processing to complete before starting the next poll tick to prevent task overlap.
