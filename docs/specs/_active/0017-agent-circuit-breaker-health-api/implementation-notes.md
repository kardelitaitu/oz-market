# Implementation Notes - Agent Circuit-Breaker and Health API

## Circuit Breaker State Machine

Below is the design for the in-memory agent circuit-breaker:

```rust
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum CircuitState {
    Closed,     // Normal routing
    Open,       // Tripped, bypass queries
    HalfOpen,   // Probe routing
}

pub struct AgentCircuitBreaker {
    state: CircuitState,
    failure_count: usize,
    last_state_change: Instant,
    error_threshold_pct: f64,
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
            error_threshold_pct,
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
}
```

## API Endpoint Controllers

```rust
// Proposed Actix Web handler structures
pub async fn get_agents_health(
    registry: web::Data<AgentRegistry>,
    scorers: web::Data<AgentMetricsCollector>,
) -> HttpResponse {
    // Collect and format statuses
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}
```
