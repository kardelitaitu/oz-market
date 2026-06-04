# Implementation Notes - Agent Metrics Collector

## Metrics Structs and Implementation

Below is the design for the in-memory `AgentMetricsCollector` service:

```rust
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};
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

    /// Record a single query event's duration and success status.
    pub fn record_sample(&self, agent_id: Uuid, duration: Duration, is_success: bool) {
        let entry = self.samples.entry(agent_id).or_insert_with(|| {
            Mutex::new(VecDeque::with_capacity(self.capacity + 1))
        });
        let mut queue = entry.lock().unwrap();
        queue.push_back(AgentTelemetrySample {
            timestamp: Instant::now(),
            duration,
            is_success,
        });
        if queue.len() > self.capacity {
            queue.pop_front();
        }
    }

    /// Retrieves all samples currently collected in the window.
    pub fn get_samples(&self, agent_id: &Uuid) -> Vec<AgentTelemetrySample> {
        if let Some(entry) = self.samples.get(agent_id) {
            let queue = entry.lock().unwrap();
            queue.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Reset/clear metrics for a specific agent (e.g. on manual recovery or reboot)
    pub fn clear_metrics(&self, agent_id: &Uuid) {
        self.samples.remove(agent_id);
    }
}
```
