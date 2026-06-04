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
    ledger_cache_hit_total: AtomicU64,
    ledger_cache_miss_total: AtomicU64,
    ledger_batch_lag_milliseconds: AtomicU64,
    ledger_batch_size: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerObservabilitySnapshot {
    pub requests_total: u64,
    pub internal_requests_total: u64,
    pub internal_writes_total: u64,
    pub conflict_responses_total: u64,
    pub quota_rejections_total: u64,
    pub error_responses_total: u64,
    pub ledger_cache_hit_total: u64,
    pub ledger_cache_miss_total: u64,
    pub ledger_batch_lag_milliseconds: u64,
    pub ledger_batch_size: u64,
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

    pub fn record_ledger_cache_hit(&self) {
        self.ledger_cache_hit_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_ledger_cache_miss(&self) {
        self.ledger_cache_miss_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_ledger_batch(&self, size: u64, lag_milliseconds: u64) {
        self.ledger_batch_size.store(size, Ordering::Relaxed);
        self.ledger_batch_lag_milliseconds
            .store(lag_milliseconds, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> ServerObservabilitySnapshot {
        ServerObservabilitySnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            internal_requests_total: self.internal_requests_total.load(Ordering::Relaxed),
            internal_writes_total: self.internal_writes_total.load(Ordering::Relaxed),
            conflict_responses_total: self.conflict_responses_total.load(Ordering::Relaxed),
            quota_rejections_total: self.quota_rejections_total.load(Ordering::Relaxed),
            error_responses_total: self.error_responses_total.load(Ordering::Relaxed),
            ledger_cache_hit_total: self.ledger_cache_hit_total.load(Ordering::Relaxed),
            ledger_cache_miss_total: self.ledger_cache_miss_total.load(Ordering::Relaxed),
            ledger_batch_lag_milliseconds: self
                .ledger_batch_lag_milliseconds
                .load(Ordering::Relaxed),
            ledger_batch_size: self.ledger_batch_size.load(Ordering::Relaxed),
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

    #[test]
    fn records_ledger_cache_hit_and_miss() {
        let observability = ServerObservability::new();
        observability.record_ledger_cache_hit();
        observability.record_ledger_cache_hit();
        observability.record_ledger_cache_hit();
        observability.record_ledger_cache_miss();

        let snapshot = observability.snapshot();
        assert_eq!(snapshot.ledger_cache_hit_total, 3);
        assert_eq!(snapshot.ledger_cache_miss_total, 1);
    }

    #[test]
    fn records_ledger_batch_size_and_lag_with_latest_values() {
        let observability = ServerObservability::new();
        observability.record_ledger_batch(50, 100);
        observability.record_ledger_batch(75, 200);
        observability.record_ledger_batch(120, 350);

        let snapshot = observability.snapshot();
        assert_eq!(snapshot.ledger_batch_size, 120);
        assert_eq!(snapshot.ledger_batch_lag_milliseconds, 350);
    }
}
