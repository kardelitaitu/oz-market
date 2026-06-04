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
}
