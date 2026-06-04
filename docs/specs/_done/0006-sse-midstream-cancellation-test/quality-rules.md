# Quality Rules - SSE Mid-stream Cancellation

- Tests must not introduce flaky behavior. Sleep durations must be calibrated to run reliably in CI.
- The test must clean up after itself (e.g. handle terminates and socket/server resources are released).
