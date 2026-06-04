# Implementation Notes - Agent Routing and Dispatch Core Layer

## Core Structs and Traits

Below is the design for the in-memory `AgentRegistry` and the dispatching traits:

```rust
use std::sync::Arc;
use dashmap::DashMap;
use uuid::Uuid;
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentMetadata {
    pub id: Uuid,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub is_active: bool,
}

pub struct AgentRegistry {
    agents: DashMap<Uuid, AgentMetadata>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
        }
    }

    pub fn register_agent(&self, agent: AgentMetadata) {
        self.agents.insert(agent.id, agent);
    }

    pub fn deregister_agent(&self, agent_id: &Uuid) -> Option<AgentMetadata> {
        self.agents.remove(agent_id).map(|(_, v)| v)
    }

    pub fn get_matching_agents(&self, capabilities: &[String]) -> Vec<AgentMetadata> {
        self.agents
            .iter()
            .filter(|r| {
                let val = r.value();
                val.is_active && capabilities.iter().all(|c| val.capabilities.contains(c))
            })
            .map(|r| r.value().clone())
            .collect()
    }
}
```

## Agent Dispatcher Traits

```rust
#[async_trait]
pub trait AgentDispatcher: Send + Sync {
    async fn dispatch_query(
        &self,
        agent: &AgentMetadata,
        payload: &[u8],
    ) -> Result<Vec<u8>, DispatchError>;
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Timeout error")]
    Timeout,
    #[error("Parsing error: {0}")]
    Parse(String),
    #[error("Registry error: {0}")]
    Registry(String),
}

pub struct HttpAgentDispatcher {
    client: reqwest::Client,
}

impl HttpAgentDispatcher {
    pub fn new(timeout: std::time::Duration) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl AgentDispatcher for HttpAgentDispatcher {
    async fn dispatch_query(
        &self,
        agent: &AgentMetadata,
        payload: &[u8],
    ) -> Result<Vec<u8>, DispatchError> {
        let response = self.client
            .post(&agent.endpoint)
            .header("Content-Type", "application/json")
            .body(payload.to_vec())
            .send()
            .await
            .map_err(|e| DispatchError::Network(e.to_string()))?;

        if response.status().is_success() {
            let bytes = response
                .bytes()
                .await
                .map_err(|e| DispatchError::Parse(e.to_string()))?;
            Ok(bytes.to_vec())
        } else {
            Err(DispatchError::Network(format!(
                "HTTP failure: {}",
                response.status()
            )))
        }
    }
}
```

## Mock Dispatcher for Integration Testing

```rust
pub struct MockAgentDispatcher {
    pub mock_responses: DashMap<Uuid, Vec<u8>>,
}

#[async_trait]
impl AgentDispatcher for MockAgentDispatcher {
    async fn dispatch_query(
        &self,
        agent: &AgentMetadata,
        _payload: &[u8],
    ) -> Result<Vec<u8>, DispatchError> {
        if let Some(res) = self.mock_responses.get(&agent.id) {
            Ok(res.clone())
        } else {
            Err(DispatchError::Network("No mock response for agent".into()))
        }
    }
}
```
