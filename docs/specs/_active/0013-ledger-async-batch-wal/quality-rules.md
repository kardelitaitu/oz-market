# Quality Rules - Write-Ahead Log (WAL) and Async Batch Committer

- **Atomic WAL Truncation**: Truncating or deleting the WAL file must only happen after database confirm acknowledgments have succeeded.
- **Batch Lag Alerts**: The system should log warnings or trigger alerts if the batch lag exceeds 1 second under sustained load, indicating database contention.
- **Graceful Shutdown**: The server must listen for termination signals (`SIGTERM`, `SIGINT`) and execute a flush of all remaining queued memory events to the database and truncate the WAL cleanly before exit.
