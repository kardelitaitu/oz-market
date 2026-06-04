use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use uuid::Uuid;

use super::agent_metrics::AgentMetricsCollector;
use super::agent_registry::AgentRegistry;
use super::latency_scorer::{AgentScore, LatencyScorer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct AgentCircuitBreaker {
    state: CircuitState,
    failure_count: usize,
    last_state_change: Instant,
    _error_threshold_pct: f64,
    latency_threshold_ms: f64,
    cooldown_period: Duration,
}

impl AgentCircuitBreaker {
    pub fn new(
        error_threshold_pct: f64,
        latency_threshold_ms: f64,
        cooldown_period: Duration,
    ) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_state_change: Instant::now(),
            _error_threshold_pct: error_threshold_pct,
            latency_threshold_ms,
            cooldown_period,
        }
    }

    pub fn state(&mut self) -> CircuitState {
        if self.state == CircuitState::Open
            && self.last_state_change.elapsed() >= self.cooldown_period
        {
            self.state = CircuitState::HalfOpen;
            self.last_state_change = Instant::now();
        }
        self.state
    }

    pub fn peek_state(&self) -> CircuitState {
        self.state
    }

    pub fn record_result(&mut self, is_success: bool, latency_ms: f64) {
        match self.state {
            CircuitState::Closed => {
                if !is_success || latency_ms > self.latency_threshold_ms {
                    self.failure_count += 1;
                    if self.failure_count >= 5 {
                        self.state = CircuitState::Open;
                        self.last_state_change = Instant::now();
                    }
                } else {
                    self.failure_count = 0;
                }
            }
            CircuitState::Open => {}
            CircuitState::HalfOpen => {
                if is_success && latency_ms <= self.latency_threshold_ms {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                } else {
                    self.state = CircuitState::Open;
                    self.last_state_change = Instant::now();
                }
            }
        }
    }

    pub fn cooldown_remaining(&self) -> Duration {
        let elapsed = self.last_state_change.elapsed();
        if elapsed >= self.cooldown_period {
            Duration::ZERO
        } else {
            self.cooldown_period - elapsed
        }
    }

    pub fn failure_count(&self) -> usize {
        self.failure_count
    }
}

impl Default for AgentCircuitBreaker {
    fn default() -> Self {
        Self::new(0.2, 2000.0, Duration::from_secs(30))
    }
}

pub struct CircuitBreakerRegistry {
    breakers: DashMap<Uuid, Arc<Mutex<AgentCircuitBreaker>>>,
    error_threshold_pct: f64,
    latency_threshold_ms: f64,
    cooldown_period: Duration,
}

impl CircuitBreakerRegistry {
    pub fn new(
        error_threshold_pct: f64,
        latency_threshold_ms: f64,
        cooldown_period: Duration,
    ) -> Self {
        Self {
            breakers: DashMap::new(),
            error_threshold_pct,
            latency_threshold_ms,
            cooldown_period,
        }
    }

    pub fn get_or_create(&self, agent_id: Uuid) -> Arc<Mutex<AgentCircuitBreaker>> {
        if let Some(entry) = self.breakers.get(&agent_id) {
            return Arc::clone(entry.value());
        }
        let breaker = Arc::new(Mutex::new(AgentCircuitBreaker::new(
            self.error_threshold_pct,
            self.latency_threshold_ms,
            self.cooldown_period,
        )));
        self.breakers.insert(agent_id, Arc::clone(&breaker));
        breaker
    }

    pub fn record_result(&self, agent_id: Uuid, is_success: bool, latency_ms: f64) {
        let breaker = self.get_or_create(agent_id);
        let mut guard = breaker.lock().expect("circuit breaker lock poisoned");
        guard.record_result(is_success, latency_ms);
    }

    pub fn state(&self, agent_id: Uuid) -> CircuitState {
        let breaker = self.get_or_create(agent_id);
        let mut guard = breaker.lock().expect("circuit breaker lock poisoned");
        guard.state()
    }

    pub fn all_states(&self) -> Vec<(Uuid, CircuitState)> {
        self.breakers
            .iter()
            .map(|entry| {
                let breaker = Arc::clone(entry.value());
                let mut guard = breaker.lock().expect("circuit breaker lock poisoned");
                let state = guard.state();
                (*entry.key(), state)
            })
            .collect()
    }

    pub fn is_open(&self, agent_id: Uuid) -> bool {
        self.state(agent_id) == CircuitState::Open
    }

    pub fn breaker_count(&self) -> usize {
        self.breakers.len()
    }

    pub fn reset(&self, agent_id: &Uuid) {
        self.breakers.remove(agent_id);
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new(0.2, 2000.0, Duration::from_secs(30))
    }
}

pub struct AgentHealthSummary {
    pub agent_id: Uuid,
    pub state: CircuitState,
    pub failure_count: usize,
    pub cooldown_remaining_secs: u64,
    pub score: AgentScore,
}

pub fn collect_health_summaries(
    registry: &AgentRegistry,
    breaker_registry: &CircuitBreakerRegistry,
    metrics_collector: &AgentMetricsCollector,
    scorer: &LatencyScorer,
) -> Vec<AgentHealthSummary> {
    registry
        .list_agents()
        .into_iter()
        .map(|agent| {
            let breaker_arc = breaker_registry.get_or_create(agent.id);
            let mut breaker = breaker_arc.lock().expect("circuit breaker lock poisoned");
            let state = breaker.state();
            let failure_count = breaker.failure_count();
            let cooldown_remaining_secs = breaker.cooldown_remaining().as_secs();
            drop(breaker);

            let samples = metrics_collector.get_samples(&agent.id);
            let score = scorer.calculate_score(&samples);

            AgentHealthSummary {
                agent_id: agent.id,
                state,
                failure_count,
                cooldown_remaining_secs,
                score,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::services::agent_registry::AgentMetadata;

    fn breaker() -> AgentCircuitBreaker {
        AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_secs(30))
    }

    #[test]
    fn initial_state_is_closed() {
        let b = breaker();
        assert_eq!(b.peek_state(), CircuitState::Closed);
        assert_eq!(b.failure_count(), 0);
    }

    #[test]
    fn single_success_resets_failures() {
        let mut b = breaker();
        b.record_result(false, 100.0);
        assert_eq!(b.failure_count(), 1);
        b.record_result(true, 50.0);
        assert_eq!(b.failure_count(), 0);
        assert_eq!(b.peek_state(), CircuitState::Closed);
    }

    #[test]
    fn five_failures_trips_to_open() {
        let mut b = AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_secs(3600));
        for _ in 0..5 {
            b.record_result(false, 100.0);
        }
        assert_eq!(b.peek_state(), CircuitState::Open);
    }

    #[test]
    fn slow_response_counts_as_failure() {
        let mut b = breaker();
        b.record_result(true, 3000.0);
        assert_eq!(b.failure_count(), 1);
    }

    #[test]
    fn half_open_success_closes_circuit() {
        let mut b = AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_secs(0));
        // Trip to open
        for _ in 0..5 {
            b.record_result(false, 100.0);
        }
        assert_eq!(b.peek_state(), CircuitState::Open);

        // Cooldown expired -> moves to HalfOpen
        assert_eq!(b.state(), CircuitState::HalfOpen);

        // Success in HalfOpen closes
        b.record_result(true, 50.0);
        assert_eq!(b.peek_state(), CircuitState::Closed);
        assert_eq!(b.failure_count(), 0);
    }

    #[test]
    fn half_open_failure_reopens() {
        let mut b = AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_secs(0));
        for _ in 0..5 {
            b.record_result(false, 100.0);
        }
        assert_eq!(b.state(), CircuitState::HalfOpen);

        b.record_result(false, 100.0);
        assert_eq!(b.peek_state(), CircuitState::Open);
    }

    #[test]
    fn open_ignores_results() {
        let mut b = AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_secs(3600));
        for _ in 0..5 {
            b.record_result(false, 100.0);
        }
        assert_eq!(b.peek_state(), CircuitState::Open);
        // Should not change state
        b.record_result(true, 50.0);
        assert_eq!(b.peek_state(), CircuitState::Open);
    }

    #[test]
    fn cooldown_remaining_reports_correctly() {
        let mut b = AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_secs(60));
        // Inject a past state change
        b.last_state_change = Instant::now() - Duration::from_secs(10);
        b.state = CircuitState::Open;

        let remaining = b.cooldown_remaining();
        assert!(remaining > Duration::from_secs(45));
        assert!(remaining <= Duration::from_secs(50));
    }

    #[test]
    fn registry_creates_breaker_on_demand() {
        let registry = CircuitBreakerRegistry::default();
        let id = Uuid::new_v4();
        assert_eq!(registry.state(id), CircuitState::Closed);
        assert_eq!(registry.breaker_count(), 1);
    }

    #[test]
    fn registry_records_results() {
        let registry = CircuitBreakerRegistry::new(0.2, 2000.0, Duration::from_secs(3600));
        let id = Uuid::new_v4();
        for _ in 0..5 {
            registry.record_result(id, false, 100.0);
        }
        assert_eq!(registry.state(id), CircuitState::Open);
        assert!(registry.is_open(id));
    }

    #[test]
    fn registry_reset_removes_breaker() {
        let registry = CircuitBreakerRegistry::default();
        let id = Uuid::new_v4();
        registry.record_result(id, false, 100.0);
        assert_eq!(registry.breaker_count(), 1);
        registry.reset(&id);
        assert_eq!(registry.breaker_count(), 0);
    }

    #[test]
    fn circuit_state_serializes() {
        assert_eq!(
            serde_json::to_string(&CircuitState::Closed).unwrap(),
            "\"Closed\""
        );
        assert_eq!(
            serde_json::to_string(&CircuitState::Open).unwrap(),
            "\"Open\""
        );
        assert_eq!(
            serde_json::to_string(&CircuitState::HalfOpen).unwrap(),
            "\"HalfOpen\""
        );
    }

    #[test]
    fn collect_health_summaries_returns_all_agents_with_state_and_score() {
        let registry = AgentRegistry::new();
        let breaker_registry = CircuitBreakerRegistry::default();
        let metrics = AgentMetricsCollector::default();
        let scorer = LatencyScorer::default();

        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        registry.register_agent(AgentMetadata {
            id: id_a,
            endpoint: "http://a.local".into(),
            capabilities: vec!["search".into()],
            is_active: true,
        });
        registry.register_agent(AgentMetadata {
            id: id_b,
            endpoint: "http://b.local".into(),
            capabilities: vec!["translate".into()],
            is_active: true,
        });

        for _ in 0..5 {
            breaker_registry.record_result(id_a, false, 100.0);
        }
        metrics.record_sample(id_a, Duration::from_millis(200), false);

        let summaries = collect_health_summaries(&registry, &breaker_registry, &metrics, &scorer);
        assert_eq!(summaries.len(), 2);

        let a = summaries.iter().find(|s| s.agent_id == id_a).unwrap();
        assert_eq!(a.state, CircuitState::Open);
        assert_eq!(a.failure_count, 5);
        assert!(a.score.ewma_latency_ms > 0.0);

        let b = summaries.iter().find(|s| s.agent_id == id_b).unwrap();
        assert_eq!(b.state, CircuitState::Closed);
        assert_eq!(b.failure_count, 0);
    }

    #[test]
    fn registry_reset_removes_breaker_and_next_record_starts_fresh() {
        let registry = CircuitBreakerRegistry::default();
        let id = Uuid::new_v4();
        for _ in 0..5 {
            registry.record_result(id, false, 100.0);
        }
        assert!(registry.is_open(id));
        assert_eq!(registry.breaker_count(), 1);

        registry.reset(&id);
        assert_eq!(registry.breaker_count(), 0);

        // A few failures after reset must NOT trip the breaker
        // (4 < 5 threshold since the breaker is freshly created).
        for _ in 0..4 {
            registry.record_result(id, false, 100.0);
        }
        assert!(
            !registry.is_open(id),
            "after reset, breaker must start from 0 failures"
        );
    }

    #[test]
    fn record_result_increments_failure_count_on_failure() {
        let mut b = AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_secs(60));
        b.record_result(false, 100.0);
        b.record_result(false, 100.0);
        assert_eq!(b.failure_count, 2);
        assert_eq!(
            b.state,
            CircuitState::Closed,
            "below threshold stays closed"
        );
    }

    #[test]
    fn record_result_success_keeps_breaker_closed() {
        let mut b = AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_secs(60));
        for _ in 0..10 {
            b.record_result(true, 50.0);
        }
        assert_eq!(b.state, CircuitState::Closed);
        assert_eq!(
            b.failure_count, 0,
            "success must not increment failure_count"
        );
    }

    #[test]
    fn registry_state_returns_closed_for_unknown_agent() {
        let registry = CircuitBreakerRegistry::default();
        let id = Uuid::new_v4();
        assert_eq!(registry.state(id), CircuitState::Closed);
        // The registry lazily creates a breaker on first access.
        assert_eq!(registry.breaker_count(), 1);
    }

    #[test]
    fn is_open_returns_false_for_breaker_with_zero_failures() {
        let registry = CircuitBreakerRegistry::default();
        let id = Uuid::new_v4();
        registry.record_result(id, true, 50.0);
        assert!(!registry.is_open(id));
    }

    #[test]
    fn cooldown_remaining_is_full_period_for_fresh_breaker() {
        // Note: cooldown_remaining() does not gate on state — for a fresh
        // breaker (last_state_change = now) it returns the full cooldown period.
        let b = AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_secs(60));
        let remaining = b.cooldown_remaining();
        assert!(remaining > Duration::from_secs(59));
        assert!(remaining <= Duration::from_secs(60));
    }

    #[test]
    fn peek_state_does_not_transition_breaker() {
        let mut b = AgentCircuitBreaker::new(0.2, 2000.0, Duration::from_millis(100));
        for _ in 0..5 {
            b.record_result(false, 100.0);
        }
        assert_eq!(b.state, CircuitState::Open);

        // Force a peek — must not trigger a state transition.
        for _ in 0..100 {
            assert_eq!(b.peek_state(), CircuitState::Open);
        }
        assert_eq!(b.state, CircuitState::Open);
    }

    #[test]
    fn error_threshold_pct_is_accepted_but_consults_consecutive_failures_only() {
        // Per decisions.md (Decision 3), the `error_threshold_pct` parameter
        // is stored but not consulted. The breaker trips only on
        // 5 consecutive failures. Pin this contract so a future change
        // to error-rate-based tripping requires a deliberate test update.
        let mut b = AgentCircuitBreaker::new(0.01, 2000.0, Duration::from_secs(3600));
        // 1 success + 4 failures = 80% error rate, which would exceed 0.01,
        // but the consecutive-failure policy must NOT trip.
        b.record_result(true, 50.0);
        b.record_result(false, 50.0);
        b.record_result(false, 50.0);
        b.record_result(false, 50.0);
        b.record_result(false, 50.0);
        assert_eq!(
            b.peek_state(),
            CircuitState::Closed,
            "5 results of which only 4 are consecutive failures must not trip"
        );
        assert_eq!(
            b.failure_count(),
            4,
            "counter reflects the current 4-failure streak (not the 80% error rate)"
        );
    }
}
