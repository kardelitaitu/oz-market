use std::time::{Duration, Instant};
use uuid::Uuid;

use oz_market_api_contract::{AgentQueryRequest, AgentQueryResponse};

use super::agent_dispatcher::{AgentDispatcher, DispatchError};
use super::agent_metrics::AgentMetricsCollector;
use super::agent_registry::AgentRegistry;
use super::circuit_breaker::CircuitBreakerRegistry;

#[derive(Debug)]
pub enum RoutingError {
    Dispatch(DispatchError),
    NoAgentsAvailable,
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingError::Dispatch(e) => write!(f, "dispatch error: {e}"),
            RoutingError::NoAgentsAvailable => write!(f, "no available agents found"),
        }
    }
}

impl std::error::Error for RoutingError {}

/// Default per-agent dispatch timeout.
pub const DEFAULT_AGENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Route a query to registered agents.
///
/// 1. Discovers capabilities mentioned in the query text and finds matching
///    active agents from the registry.
/// 2. Skips agents whose circuit breaker is open.
/// 3. Dispatches the query payload to each eligible agent, recording telemetry
///    samples and updating circuit breaker state per attempt.
/// 4. Returns an aggregated response with dispatch results.
///
/// Each agent dispatch is bounded by `timeout`. If an agent times out or
/// returns an error, the next eligible agent is tried (retry-on-failure).
pub async fn route_agent_query(
    registry: &AgentRegistry,
    breaker_registry: &CircuitBreakerRegistry,
    metrics_collector: &AgentMetricsCollector,
    dispatcher: &dyn AgentDispatcher,
    request: &AgentQueryRequest,
    timeout: Duration,
) -> Result<AgentQueryResponse, RoutingError> {
    let query_lower = request.query.to_lowercase();
    let matched_capabilities: Vec<String> = detect_requested_capabilities(registry, &query_lower);

    let agents = if matched_capabilities.is_empty() {
        registry.list_agents()
    } else {
        registry.get_matching_agents(&matched_capabilities)
    };

    let mut eligible: Vec<_> = Vec::new();
    let mut skipped = 0u32;

    for agent in &agents {
        if breaker_registry.is_open(agent.id) {
            skipped += 1;
        } else {
            eligible.push(agent.clone());
        }
    }

    if eligible.is_empty() {
        return Err(RoutingError::NoAgentsAvailable);
    }

    let payload = serde_json::to_vec(request).unwrap_or_default();
    let mut dispatch_count = 0u32;
    let mut messages: Vec<String> = Vec::new();

    for agent in &eligible {
        let start = Instant::now();
        let result = match tokio::time::timeout(timeout, dispatcher.dispatch_query(agent, &payload))
            .await
        {
            Ok(inner) => inner,
            Err(_) => {
                let duration = start.elapsed();
                metrics_collector.record_sample(agent.id, duration, false);
                breaker_registry.record_result(agent.id, false, duration.as_secs_f64() * 1000.0);
                continue;
            }
        };
        let duration = start.elapsed();
        let is_success = result.is_ok();

        metrics_collector.record_sample(agent.id, duration, is_success);
        breaker_registry.record_result(agent.id, is_success, duration.as_secs_f64() * 1000.0);

        if is_success {
            dispatch_count += 1;
            if let Ok(bytes) = result {
                if let Ok(text) = String::from_utf8(bytes) {
                    messages.push(text);
                }
            }
        }
    }

    let message = if dispatch_count > 0 {
        let mut msg = format!("Dispatched to {count} agent(s)", count = dispatch_count);
        if skipped > 0 {
            msg.push_str(&format!(" ({} skipped, circuit open)", skipped));
        }
        msg
    } else {
        "Query dispatched but no agent returned a successful response".to_string()
    };

    Ok(AgentQueryResponse {
        message,
        actions: vec![],
        conversation_id: request
            .conversation_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        listing_ids: None,
    })
}

/// Reset the circuit breaker and metrics for a given agent.
pub fn reset_agent_breaker(
    breaker_registry: &CircuitBreakerRegistry,
    metrics_collector: &AgentMetricsCollector,
    agent_id: &Uuid,
) {
    breaker_registry.reset(agent_id);
    metrics_collector.clear_metrics(agent_id);
}

/// Detect which capabilities the query text is requesting by matching
/// capability keywords against the query text.
fn detect_requested_capabilities(registry: &AgentRegistry, query: &str) -> Vec<String> {
    let mut matched: Vec<String> = Vec::new();
    for agent in registry.list_agents() {
        for cap in &agent.capabilities {
            let cap_lower = cap.to_lowercase();
            if query.contains(&cap_lower) && !matched.contains(&cap_lower) {
                matched.push(cap.clone());
            }
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::agent_dispatcher::MockAgentDispatcher;
    use crate::services::agent_registry::AgentMetadata;

    fn agent_with_cap(id: Uuid, capabilities: Vec<String>) -> AgentMetadata {
        AgentMetadata {
            id,
            endpoint: format!("http://agent-{}.local", id),
            capabilities,
            is_active: true,
        }
    }

    #[tokio::test]
    async fn route_query_to_single_agent() {
        let id = Uuid::new_v4();
        let registry = AgentRegistry::new();
        registry.register_agent(agent_with_cap(id, vec!["search".into()]));

        let breaker_registry = CircuitBreakerRegistry::default();
        let metrics = AgentMetricsCollector::default();
        let dispatcher = MockAgentDispatcher::with_response(id, b"ok".to_vec());

        let req = AgentQueryRequest {
            query: "search for listings".into(),
            conversation_id: None,
        };

        let result = route_agent_query(
            &registry,
            &breaker_registry,
            &metrics,
            &dispatcher,
            &req,
            Duration::from_secs(5),
        )
        .await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.message.contains("Dispatched to 1 agent(s)"));
        assert!(!resp.conversation_id.is_empty());
    }

    #[tokio::test]
    async fn route_query_skips_open_circuit() {
        let id = Uuid::new_v4();
        let registry = AgentRegistry::new();
        registry.register_agent(agent_with_cap(id, vec!["search".into()]));

        let breaker_registry = CircuitBreakerRegistry::default();
        let metrics = AgentMetricsCollector::default();

        // Trip the breaker
        for _ in 0..5 {
            breaker_registry.record_result(id, false, 100.0);
        }
        assert!(breaker_registry.is_open(id));

        let dispatcher = MockAgentDispatcher::with_response(id, b"ok".to_vec());

        let req = AgentQueryRequest {
            query: "search".into(),
            conversation_id: None,
        };

        let result = route_agent_query(
            &registry,
            &breaker_registry,
            &metrics,
            &dispatcher,
            &req,
            Duration::from_secs(5),
        )
        .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            RoutingError::NoAgentsAvailable => {}
            other => panic!("expected NoAgentsAvailable, got {other}"),
        }
    }

    #[tokio::test]
    async fn reset_clears_breaker_and_metrics() {
        let id = Uuid::new_v4();
        let registry = AgentRegistry::new();
        registry.register_agent(agent_with_cap(id, vec!["test".into()]));

        let breaker_registry = CircuitBreakerRegistry::default();
        let metrics = AgentMetricsCollector::default();

        // Trip the breaker
        for _ in 0..5 {
            breaker_registry.record_result(id, false, 100.0);
        }
        metrics.record_sample(id, std::time::Duration::from_millis(50), false);

        assert!(breaker_registry.is_open(id));
        assert!(metrics.sample_count(&id) > 0);

        reset_agent_breaker(&breaker_registry, &metrics, &id);

        assert!(!breaker_registry.is_open(id));
        assert_eq!(metrics.sample_count(&id), 0);
    }

    #[tokio::test]
    async fn detect_capabilities_matches_query_keywords() {
        let registry = AgentRegistry::new();
        registry.register_agent(agent_with_cap(
            Uuid::new_v4(),
            vec!["search".into(), "nlp".into()],
        ));
        registry.register_agent(agent_with_cap(Uuid::new_v4(), vec!["translation".into()]));

        let matched = detect_requested_capabilities(&registry, "i need translation help");
        assert!(matched.contains(&"translation".to_string()));
        assert!(!matched.contains(&"search".to_string()));
    }

    #[tokio::test]
    async fn route_query_returns_error_when_no_agents_registered() {
        let registry = AgentRegistry::new();
        let breaker_registry = CircuitBreakerRegistry::default();
        let metrics = AgentMetricsCollector::default();
        let dispatcher = MockAgentDispatcher::default();

        let req = AgentQueryRequest {
            query: "hello".into(),
            conversation_id: None,
        };

        let result = route_agent_query(
            &registry,
            &breaker_registry,
            &metrics,
            &dispatcher,
            &req,
            Duration::from_secs(5),
        )
        .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            RoutingError::NoAgentsAvailable => {}
            other => panic!("expected NoAgentsAvailable, got {other}"),
        }
    }

    #[tokio::test]
    async fn route_query_with_multiple_agents_all_succeed() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let registry = AgentRegistry::new();
        registry.register_agent(agent_with_cap(id1, vec!["search".into()]));
        registry.register_agent(agent_with_cap(id2, vec!["search".into()]));

        let breaker_registry = CircuitBreakerRegistry::default();
        let metrics = AgentMetricsCollector::default();
        let dispatcher = MockAgentDispatcher::new();
        dispatcher
            .mock_responses
            .insert(id1, Ok(b"reply-1".to_vec()));
        dispatcher
            .mock_responses
            .insert(id2, Ok(b"reply-2".to_vec()));

        let req = AgentQueryRequest {
            query: "search listings".into(),
            conversation_id: Some("conv-multi".into()),
        };

        let result = route_agent_query(
            &registry,
            &breaker_registry,
            &metrics,
            &dispatcher,
            &req,
            Duration::from_secs(5),
        )
        .await
        .unwrap();

        assert!(result.message.contains("Dispatched to 2 agent(s)"));
        assert_eq!(result.conversation_id, "conv-multi");
    }

    #[tokio::test]
    async fn route_query_one_succeeds_one_fails_reports_dispatched_to_one() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let registry = AgentRegistry::new();
        registry.register_agent(agent_with_cap(id1, vec!["search".into()]));
        registry.register_agent(agent_with_cap(id2, vec!["search".into()]));

        let breaker_registry = CircuitBreakerRegistry::default();
        let metrics = AgentMetricsCollector::default();
        let dispatcher = MockAgentDispatcher::new();
        dispatcher.mock_responses.insert(id1, Ok(b"ok".to_vec()));
        dispatcher
            .mock_responses
            .insert(id2, Err(DispatchError::Network("down".into())));

        let req = AgentQueryRequest {
            query: "search".into(),
            conversation_id: None,
        };

        let result = route_agent_query(
            &registry,
            &breaker_registry,
            &metrics,
            &dispatcher,
            &req,
            Duration::from_secs(5),
        )
        .await
        .unwrap();

        assert!(result.message.contains("Dispatched to 1 agent(s)"));
        // Failure must be recorded in metrics for the failed agent.
        assert!(
            metrics.sample_count(&id2) > 0,
            "failed agent must have a sample"
        );
        assert!(
            metrics.sample_count(&id1) > 0,
            "success agent must have a sample"
        );
    }

    #[tokio::test]
    async fn route_query_with_no_capability_match_falls_back_to_all_agents() {
        // The query doesn't mention any registered capability keyword —
        // routing falls back to dispatching to all active agents.
        let id = Uuid::new_v4();
        let registry = AgentRegistry::new();
        registry.register_agent(agent_with_cap(id, vec!["translation".into()]));

        let breaker_registry = CircuitBreakerRegistry::default();
        let metrics = AgentMetricsCollector::default();
        let dispatcher = MockAgentDispatcher::with_response(id, b"fallback".to_vec());

        let req = AgentQueryRequest {
            query: "something unrelated to translation".into(),
            conversation_id: None,
        };

        let result = route_agent_query(
            &registry,
            &breaker_registry,
            &metrics,
            &dispatcher,
            &req,
            Duration::from_secs(5),
        )
        .await
        .unwrap();

        assert!(result.message.contains("Dispatched to 1 agent(s)"));
    }

    #[tokio::test]
    async fn route_query_respects_short_timeout_and_records_failure() {
        let id = Uuid::new_v4();
        let registry = AgentRegistry::new();
        registry.register_agent(agent_with_cap(id, vec!["slow".into()]));

        let breaker_registry = CircuitBreakerRegistry::default();
        let metrics = AgentMetricsCollector::default();
        // Dispatcher sleeps longer than the route timeout.
        let dispatcher = SlowMockDispatcher {
            delay: Duration::from_millis(500),
        };

        let req = AgentQueryRequest {
            query: "slow".into(),
            conversation_id: None,
        };

        let result = route_agent_query(
            &registry,
            &breaker_registry,
            &metrics,
            &dispatcher,
            &req,
            Duration::from_millis(50),
        )
        .await
        .unwrap();

        assert!(
            result
                .message
                .contains("no agent returned a successful response"),
            "got: {}",
            result.message
        );
        assert_eq!(
            metrics.sample_count(&id),
            1,
            "timeout must record exactly one failure sample"
        );
    }

    #[tokio::test]
    async fn route_query_all_agents_have_open_breaker_returns_no_agents_available() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let registry = AgentRegistry::new();
        registry.register_agent(agent_with_cap(id1, vec!["search".into()]));
        registry.register_agent(agent_with_cap(id2, vec!["search".into()]));

        let breaker_registry = CircuitBreakerRegistry::default();
        for _ in 0..5 {
            breaker_registry.record_result(id1, false, 100.0);
            breaker_registry.record_result(id2, false, 100.0);
        }
        assert!(breaker_registry.is_open(id1));
        assert!(breaker_registry.is_open(id2));

        let metrics = AgentMetricsCollector::default();
        let dispatcher = MockAgentDispatcher::default();

        let req = AgentQueryRequest {
            query: "search".into(),
            conversation_id: None,
        };

        let err = route_agent_query(
            &registry,
            &breaker_registry,
            &metrics,
            &dispatcher,
            &req,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err();
        match err {
            RoutingError::NoAgentsAvailable => {}
            other => panic!("expected NoAgentsAvailable, got {other}"),
        }
    }

    /// Mock dispatcher that sleeps before responding, for timeout tests.
    struct SlowMockDispatcher {
        delay: Duration,
    }

    #[async_trait::async_trait]
    impl AgentDispatcher for SlowMockDispatcher {
        async fn dispatch_query(
            &self,
            _agent: &AgentMetadata,
            _payload: &[u8],
        ) -> Result<Vec<u8>, DispatchError> {
            tokio::time::sleep(self.delay).await;
            Ok(b"slow-ok".to_vec())
        }
    }
}
