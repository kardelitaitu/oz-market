use std::sync::atomic::{AtomicU64, Ordering};

pub const MODULE_NAME: &str = "observability";

#[derive(Debug, Default)]
pub struct ServerObservability {
    requests_total: AtomicU64,
    internal_requests_total: AtomicU64,
    internal_writes_total: AtomicU64,
    conflict_responses_total: AtomicU64,
    quota_rejections_total: AtomicU64,
    error_responses_total: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerObservabilitySnapshot {
    pub requests_total: u64,
    pub internal_requests_total: u64,
    pub internal_writes_total: u64,
    pub conflict_responses_total: u64,
    pub quota_rejections_total: u64,
    pub error_responses_total: u64,
}

impl ServerObservability {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_request(&self, path: &str, status: u16) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        if path.starts_with("/internal/v1/") {
            self.internal_requests_total.fetch_add(1, Ordering::Relaxed);
        }
        if path.starts_with("/internal/v1/") && matches!(status, 200 | 201 | 204) {
            self.internal_writes_total.fetch_add(1, Ordering::Relaxed);
        }
        if status == 409 {
            self.conflict_responses_total
                .fetch_add(1, Ordering::Relaxed);
        }
        if status == 429 {
            self.quota_rejections_total.fetch_add(1, Ordering::Relaxed);
        }
        if status >= 400 {
            self.error_responses_total.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn snapshot(&self) -> ServerObservabilitySnapshot {
        ServerObservabilitySnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            internal_requests_total: self.internal_requests_total.load(Ordering::Relaxed),
            internal_writes_total: self.internal_writes_total.load(Ordering::Relaxed),
            conflict_responses_total: self.conflict_responses_total.load(Ordering::Relaxed),
            quota_rejections_total: self.quota_rejections_total.load(Ordering::Relaxed),
            error_responses_total: self.error_responses_total.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_internal_and_conflict_requests() {
        let observability = ServerObservability::new();
        observability.record_request("/v1/listings", 200);
        observability.record_request("/internal/v1/listings/1/release-reservation", 200);
        observability.record_request("/internal/v1/listings/1/release-reservation", 409);
        observability.record_request("/v1/listings", 429);
        observability.record_request("/v1/listings", 500);

        let snapshot = observability.snapshot();
        assert_eq!(snapshot.requests_total, 5);
        assert_eq!(snapshot.internal_requests_total, 2);
        assert_eq!(snapshot.internal_writes_total, 1);
        assert_eq!(snapshot.conflict_responses_total, 1);
        assert_eq!(snapshot.quota_rejections_total, 1);
        assert_eq!(snapshot.error_responses_total, 3);
    }
}
