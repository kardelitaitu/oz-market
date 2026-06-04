# Implementation Notes - Reusable Counted Responder

The responder will be declared inside `#[cfg(test)] mod integration` in `sse.rs`.
```rust
struct CountedResponder {
    responses: Vec<ResponseTemplate>,
    count: AtomicUsize,
}

impl wiremock::Respond for CountedResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let idx = self.count.fetch_add(1, Ordering::Relaxed);
        if idx < self.responses.len() {
            self.responses[idx].clone()
        } else {
            ResponseTemplate::new(500)
        }
    }
}
```
Use `std::sync::atomic::Ordering::Relaxed` since we only need basic thread-safe increments without memory ordering guarantees.
