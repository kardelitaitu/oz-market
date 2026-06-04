# Quality Rules - Reusable Counted Responder

- The struct must be thread-safe as `wiremock` handles incoming HTTP requests on a multi-threaded tokio executor.
- Cloning `ResponseTemplate`s is required since `respond` takes `&self` and must return a clean template instance.
