use crate::services::agent_metrics::AgentTelemetrySample;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentScore {
    pub ewma_latency_ms: f64,
    pub ewma_error_rate: f64,
}

pub struct LatencyScorer {
    alpha: f64,
    default_latency_ms: f64,
}

impl LatencyScorer {
    pub fn new(alpha: f64, default_latency_ms: f64) -> Self {
        Self {
            alpha,
            default_latency_ms,
        }
    }

    pub fn calculate_score(&self, samples: &[AgentTelemetrySample]) -> AgentScore {
        if samples.is_empty() {
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
            ewma_latency_ms: clamp_non_negative(current_latency),
            ewma_error_rate: clamp_zero_to_one(current_error_rate),
        }
    }
}

impl Default for LatencyScorer {
    fn default() -> Self {
        Self::new(0.2, 200.0)
    }
}

fn clamp_non_negative(v: f64) -> f64 {
    if v.is_nan() || v.is_infinite() || v < 0.0 {
        0.0
    } else {
        v
    }
}

fn clamp_zero_to_one(v: f64) -> f64 {
    if v.is_nan() || v.is_infinite() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn sample(duration_ms: u64, is_success: bool) -> AgentTelemetrySample {
        AgentTelemetrySample {
            timestamp: std::time::Instant::now(),
            duration: Duration::from_millis(duration_ms),
            is_success,
        }
    }

    #[test]
    fn cold_start_returns_defaults() {
        let scorer = LatencyScorer::default();
        let score = scorer.calculate_score(&[]);
        assert_eq!(score.ewma_latency_ms, 200.0);
        assert_eq!(score.ewma_error_rate, 0.0);
    }

    #[test]
    fn cold_start_custom_defaults() {
        let scorer = LatencyScorer::new(0.3, 500.0);
        let score = scorer.calculate_score(&[]);
        assert_eq!(score.ewma_latency_ms, 500.0);
        assert_eq!(score.ewma_error_rate, 0.0);
    }

    #[test]
    fn single_sample_uses_its_values() {
        let scorer = LatencyScorer::default();
        let score = scorer.calculate_score(&[sample(150, true)]);
        assert_eq!(score.ewma_latency_ms, 150.0);
        assert_eq!(score.ewma_error_rate, 0.0);
    }

    #[test]
    fn single_error_sample() {
        let scorer = LatencyScorer::default();
        let score = scorer.calculate_score(&[sample(100, false)]);
        assert_eq!(score.ewma_latency_ms, 100.0);
        assert_eq!(score.ewma_error_rate, 1.0);
    }

    #[test]
    fn ewma_converges_toward_recent_samples() {
        let scorer = LatencyScorer::new(0.5, 200.0);
        let score =
            scorer.calculate_score(&[sample(100, true), sample(100, true), sample(200, true)]);
        // i=0: lat=100, err=0
        // i=1: 0.5*100 + 0.5*100 = 100, err=0
        // i=2: 0.5*200 + 0.5*100 = 150, err=0
        assert!((score.ewma_latency_ms - 150.0).abs() < 0.001);
        assert_eq!(score.ewma_error_rate, 0.0);
    }

    #[test]
    fn ewma_alpha_0_ignores_new_values() {
        let scorer = LatencyScorer::new(0.0, 200.0);
        let score = scorer.calculate_score(&[sample(500, false), sample(999, true)]);
        // i=0: lat=500, err=1
        // i=1: 0.0*999 + 1.0*500 = 500, err stays 1
        assert!((score.ewma_latency_ms - 500.0).abs() < 0.001);
        assert!((score.ewma_error_rate - 1.0).abs() < 0.001);
    }

    #[test]
    fn ewma_alpha_1_uses_only_newest() {
        let scorer = LatencyScorer::new(1.0, 200.0);
        let score = scorer.calculate_score(&[sample(50, true), sample(300, false)]);
        // i=0: lat=50, err=0
        // i=1: 1.0*300 + 0.0*50 = 300, err = 1.0
        assert!((score.ewma_latency_ms - 300.0).abs() < 0.001);
        assert!((score.ewma_error_rate - 1.0).abs() < 0.001);
    }

    #[test]
    fn error_rate_is_bounded_between_0_and_1() {
        let scorer = LatencyScorer::new(0.5, 200.0);
        let samples: Vec<_> = (0..20).map(|_| sample(100, false)).collect();
        let score = scorer.calculate_score(&samples);
        assert!(score.ewma_error_rate >= 0.0);
        assert!(score.ewma_error_rate <= 1.0);
        assert!(score.ewma_latency_ms >= 0.0);
    }

    #[test]
    fn mixed_success_failure_produces_intermediate_error_rate() {
        let scorer = LatencyScorer::new(0.5, 200.0);
        let score = scorer.calculate_score(&[sample(100, true), sample(100, false)]);
        // i=0: lat=100, err=0
        // i=1: 0.5*100 + 0.5*100 = 100, err = 0.5*1 + 0.5*0 = 0.5
        assert!((score.ewma_latency_ms - 100.0).abs() < 0.001);
        assert!((score.ewma_error_rate - 0.5).abs() < 0.001);
    }

    #[test]
    fn clamp_handles_nan_and_inf_gracefully() {
        assert_eq!(clamp_non_negative(f64::NAN), 0.0);
        assert_eq!(clamp_non_negative(f64::INFINITY), 0.0);
        assert_eq!(clamp_non_negative(f64::NEG_INFINITY), 0.0);
        assert_eq!(clamp_non_negative(-1.0), 0.0);
        assert_eq!(clamp_non_negative(42.5), 42.5);

        assert_eq!(clamp_zero_to_one(f64::NAN), 0.0);
        assert_eq!(clamp_zero_to_one(f64::INFINITY), 0.0);
        assert_eq!(clamp_zero_to_one(f64::NEG_INFINITY), 0.0);
        assert_eq!(clamp_zero_to_one(-0.5), 0.0);
        assert_eq!(clamp_zero_to_one(1.5), 1.0);
        assert_eq!(clamp_zero_to_one(0.5), 0.5);
        assert_eq!(clamp_zero_to_one(0.0), 0.0);
        assert_eq!(clamp_zero_to_one(1.0), 1.0);
    }

    #[test]
    fn agent_score_serializes() {
        let score = AgentScore {
            ewma_latency_ms: 150.5,
            ewma_error_rate: 0.05,
        };
        let json = serde_json::to_string(&score).unwrap();
        assert!(json.contains("150.5"));
        assert!(json.contains("0.05"));
    }
}
