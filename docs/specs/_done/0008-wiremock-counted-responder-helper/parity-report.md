# Parity Report - Reusable Counted Responder

| Item | Status | Details |
|------|--------|---------|
| Reusable Helper | ✅ **DONE** | `CountedResponder` struct at `sse.rs:573`, `wiremock::Respond` impl at line 578, `counted_responder` factory at line 591; used by `listen_negotiation_impl_retries_after_timeout_then_succeeds` test |
| Spec Validation | ✅ **DONE** | 17 test suite passes; out-of-bounds returns 500 fallback; thread-safe via AtomicUsize |
