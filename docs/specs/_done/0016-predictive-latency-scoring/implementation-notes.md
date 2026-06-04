# Implementation Notes - Predictive Latency Scoring

## Scoring Engine Design

Below is the design for the predictive score calculation module:

```rust
use crate::services::agent_metrics::AgentTelemetrySample;

pub struct LatencyScorer {
    alpha: f64,
    default_latency_ms: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentScore {
    pub ewma_latency_ms: f64,
    pub ewma_error_rate: f64, // Range 0.0 (perfect) to 1.0 (failed)
}

impl LatencyScorer {
    pub fn new(alpha: f64, default_latency_ms: f64) -> Self {
        Self {
            alpha,
            default_latency_ms,
        }
    }

    /// Calculates current score using EWMA based on metrics sliding window.
    pub fn calculate_score(&self, samples: &[AgentTelemetrySample]) -> AgentScore {
        if samples.is_empty() {
            // Cold start fallback
            return AgentScore {
                ewma_latency_ms: self.default_latency_ms,
                ewma_error_rate: 0.0,
            };
        }

        let mut current_latency = self.default_latency_ms;
        let mut current_error_rate = 0.0;

        for (i, sample) in samples.iter().enumerate() {
            let lat_val = sample.duration.as_secs_f64() * 1000.0;
            let err_val = if sample.is_success { 0.0 } else { 1.0 };

            if i == 0 {
                current_latency = lat_val;
                current_error_rate = err_val;
            } else {
                current_latency = self.alpha * lat_val + (1.0 - self.alpha) * current_latency;
                current_error_rate = self.alpha * err_val + (1.0 - self.alpha) * current_error_rate;
            }
        }

        AgentScore {
            ewma_latency_ms: current_latency,
            ewma_error_rate: current_error_rate,
        }
    }
}
```
