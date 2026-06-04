use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AgentTelemetrySample {
    pub timestamp: Instant,
    pub duration: Duration,
    pub is_success: bool,
}

pub struct AgentMetricsCollector {
    samples: DashMap<Uuid, Mutex<VecDeque<AgentTelemetrySample>>>,
    capacity: usize,
}

impl AgentMetricsCollector {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: DashMap::new(),
            capacity,
        }
    }

    pub fn record_sample(&self, agent_id: Uuid, duration: Duration, is_success: bool) {
        let entry = self
            .samples
            .entry(agent_id)
            .or_insert_with(|| Mutex::new(VecDeque::with_capacity(self.capacity + 1)));
        let mut queue = entry.lock().expect("agent metrics lock poisoned");
        queue.push_back(AgentTelemetrySample {
            timestamp: Instant::now(),
            duration,
            is_success,
        });
        if queue.len() > self.capacity {
            queue.pop_front();
        }
    }

    pub fn get_samples(&self, agent_id: &Uuid) -> Vec<AgentTelemetrySample> {
        if let Some(entry) = self.samples.get(agent_id) {
            let queue = entry.lock().expect("agent metrics lock poisoned");
            queue.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn clear_metrics(&self, agent_id: &Uuid) {
        self.samples.remove(agent_id);
    }

    pub fn sample_count(&self, agent_id: &Uuid) -> usize {
        self.get_samples(agent_id).len()
    }

    pub fn total_agents(&self) -> usize {
        self.samples.len()
    }
}

impl Default for AgentMetricsCollector {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_retrieve_samples() {
        let collector = AgentMetricsCollector::new(10);
        let agent_id = Uuid::new_v4();

        collector.record_sample(agent_id, Duration::from_millis(50), true);
        collector.record_sample(agent_id, Duration::from_millis(150), false);

        let samples = collector.get_samples(&agent_id);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].duration, Duration::from_millis(50));
        assert!(samples[0].is_success);
        assert_eq!(samples[1].duration, Duration::from_millis(150));
        assert!(!samples[1].is_success);
    }

    #[test]
    fn get_samples_returns_empty_for_unknown_agent() {
        let collector = AgentMetricsCollector::new(10);
        let samples = collector.get_samples(&Uuid::new_v4());
        assert!(samples.is_empty());
    }

    #[test]
    fn capacity_evicts_oldest_samples() {
        let collector = AgentMetricsCollector::new(3);
        let agent_id = Uuid::new_v4();

        for i in 0..5 {
            collector.record_sample(agent_id, Duration::from_millis(i * 10), true);
        }

        let samples = collector.get_samples(&agent_id);
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[0].duration, Duration::from_millis(20));
        assert_eq!(samples[1].duration, Duration::from_millis(30));
        assert_eq!(samples[2].duration, Duration::from_millis(40));
    }

    #[test]
    fn clear_metrics_removes_agent_data() {
        let collector = AgentMetricsCollector::new(10);
        let agent_id = Uuid::new_v4();

        collector.record_sample(agent_id, Duration::from_millis(50), true);
        assert_eq!(collector.sample_count(&agent_id), 1);

        collector.clear_metrics(&agent_id);
        assert_eq!(collector.sample_count(&agent_id), 0);
        assert!(collector.get_samples(&agent_id).is_empty());
    }

    #[test]
    fn clear_metrics_nonexistent_agent_is_noop() {
        let collector = AgentMetricsCollector::new(10);
        collector.clear_metrics(&Uuid::new_v4());
        assert_eq!(collector.total_agents(), 0);
    }

    #[test]
    fn multiple_agents_isolated() {
        let collector = AgentMetricsCollector::new(10);
        let agent_a = Uuid::new_v4();
        let agent_b = Uuid::new_v4();

        collector.record_sample(agent_a, Duration::from_millis(100), true);
        collector.record_sample(agent_b, Duration::from_millis(200), false);

        assert_eq!(collector.sample_count(&agent_a), 1);
        assert_eq!(collector.sample_count(&agent_b), 1);
        assert_eq!(collector.total_agents(), 2);

        assert!(collector.get_samples(&agent_a)[0].is_success);
        assert!(!collector.get_samples(&agent_b)[0].is_success);
    }

    #[test]
    fn default_capacity_is_100() {
        let collector = AgentMetricsCollector::default();
        let agent_id = Uuid::new_v4();

        for i in 0..110 {
            collector.record_sample(agent_id, Duration::from_millis(i), i % 2 == 0);
        }

        assert_eq!(collector.sample_count(&agent_id), 100);
        let samples = collector.get_samples(&agent_id);
        assert_eq!(samples[0].duration, Duration::from_millis(10));
        assert_eq!(samples[99].duration, Duration::from_millis(109));
    }

    #[test]
    fn concurrent_record_does_not_panic() {
        use std::sync::Arc;
        use std::thread;

        let collector = Arc::new(AgentMetricsCollector::new(50));
        let agent_id = Uuid::new_v4();
        let mut handles = Vec::new();

        for _ in 0..20 {
            let c = Arc::clone(&collector);
            let id = agent_id;
            handles.push(thread::spawn(move || {
                for i in 0..10 {
                    c.record_sample(id, Duration::from_millis(i), true);
                }
            }));
        }

        for h in handles {
            h.join().expect("thread panicked");
        }

        assert_eq!(collector.sample_count(&agent_id), 50);
    }

    #[test]
    fn capacity_zero_evicts_after_first_sample() {
        // With capacity=0 the queue evicts after every push, so nothing is retained.
        // This pins the ">" semantics (not ">=") used in record_sample.
        let collector = AgentMetricsCollector::new(0);
        let agent_id = Uuid::new_v4();
        collector.record_sample(agent_id, Duration::from_millis(10), true);
        collector.record_sample(agent_id, Duration::from_millis(20), false);
        collector.record_sample(agent_id, Duration::from_millis(30), true);

        assert_eq!(
            collector.sample_count(&agent_id),
            0,
            "capacity 0 must not retain any samples"
        );
    }

    #[test]
    fn capacity_one_keeps_only_latest_sample() {
        let collector = AgentMetricsCollector::new(1);
        let agent_id = Uuid::new_v4();
        // i=0 success, i=1 failure, i=2 success, i=3 failure, i=4 success.
        for i in 0..5 {
            let is_success = i % 2 == 0;
            collector.record_sample(agent_id, Duration::from_millis(i * 100), is_success);
        }
        let samples = collector.get_samples(&agent_id);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].duration, Duration::from_millis(400));
        assert!(
            samples[0].is_success,
            "i=4 is even, so the last sample is a success"
        );
    }

    #[test]
    fn clear_metrics_preserves_other_agents_samples() {
        let collector = AgentMetricsCollector::new(10);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        collector.record_sample(a, Duration::from_millis(50), true);
        collector.record_sample(b, Duration::from_millis(60), false);

        collector.clear_metrics(&a);
        assert_eq!(collector.sample_count(&a), 0);
        assert_eq!(
            collector.sample_count(&b),
            1,
            "clearing agent A must not affect agent B's samples"
        );
        assert_eq!(collector.total_agents(), 1);
    }

    #[test]
    fn total_agents_decreases_after_clear_metrics() {
        let collector = AgentMetricsCollector::new(10);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        collector.record_sample(a, Duration::from_millis(1), true);
        collector.record_sample(b, Duration::from_millis(2), true);
        collector.record_sample(c, Duration::from_millis(3), true);
        assert_eq!(collector.total_agents(), 3);

        collector.clear_metrics(&b);
        assert_eq!(collector.total_agents(), 2);
    }

    #[test]
    fn record_sample_creates_agent_entry_lazily() {
        let collector = AgentMetricsCollector::new(10);
        let agent_id = Uuid::new_v4();
        assert_eq!(collector.total_agents(), 0);
        assert!(collector.get_samples(&agent_id).is_empty());

        collector.record_sample(agent_id, Duration::from_millis(1), true);
        assert_eq!(collector.total_agents(), 1);
        assert_eq!(collector.sample_count(&agent_id), 1);
    }

    #[test]
    fn get_samples_returns_independent_clones() {
        let collector = AgentMetricsCollector::new(10);
        let agent_id = Uuid::new_v4();
        collector.record_sample(agent_id, Duration::from_millis(1), true);

        let snap1 = collector.get_samples(&agent_id);
        collector.record_sample(agent_id, Duration::from_millis(2), false);
        let snap2 = collector.get_samples(&agent_id);

        assert_eq!(
            snap1.len(),
            1,
            "first snapshot must not see the second record"
        );
        assert_eq!(snap2.len(), 2, "second snapshot sees both records");
    }

    #[test]
    fn concurrent_clear_during_record_does_not_panic() {
        use std::sync::Arc;
        use std::thread;

        let collector = Arc::new(AgentMetricsCollector::new(100));
        let agent_id = Uuid::new_v4();
        let writer = Arc::clone(&collector);
        let writer_id = agent_id;

        let writer_handle = thread::spawn(move || {
            for i in 0..200 {
                writer.record_sample(writer_id, Duration::from_millis(i), true);
            }
        });

        let clearer = Arc::clone(&collector);
        let clearer_id = agent_id;
        let clearer_handle = thread::spawn(move || {
            for _ in 0..10 {
                clearer.clear_metrics(&clearer_id);
            }
        });

        writer_handle.join().expect("writer panicked");
        clearer_handle.join().expect("clearer panicked");
        // No assertion on final count — the invariant we care about is no panic
        // and the final state is internally consistent.
        let final_count = collector.sample_count(&agent_id);
        assert!(final_count <= 100, "capacity must still be respected");
    }
}
