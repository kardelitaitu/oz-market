use std::fmt;

use async_trait::async_trait;
use dashmap::DashMap;
use uuid::Uuid;

use super::agent_registry::AgentMetadata;

#[derive(Debug)]
pub enum DispatchError {
    Network(String),
    Timeout,
    Parse(String),
    Registry(String),
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DispatchError::Network(msg) => write!(f, "Network error: {}", msg),
            DispatchError::Timeout => write!(f, "Timeout error"),
            DispatchError::Parse(msg) => write!(f, "Parsing error: {}", msg),
            DispatchError::Registry(msg) => write!(f, "Registry error: {}", msg),
        }
    }
}

impl std::error::Error for DispatchError {}

#[async_trait]
pub trait AgentDispatcher: Send + Sync {
    async fn dispatch_query(
        &self,
        agent: &AgentMetadata,
        payload: &[u8],
    ) -> Result<Vec<u8>, DispatchError>;
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
                .expect("reqwest::Client::builder().build() should not fail with default settings"),
        }
    }

    /// Construct a dispatcher with a caller-provided reqwest client.
    /// Useful in tests that need to bypass system proxies.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl AgentDispatcher for HttpAgentDispatcher {
    async fn dispatch_query(
        &self,
        agent: &AgentMetadata,
        payload: &[u8],
    ) -> Result<Vec<u8>, DispatchError> {
        let response = self
            .client
            .post(&agent.endpoint)
            .header("Content-Type", "application/json")
            .body(payload.to_vec())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DispatchError::Timeout
                } else {
                    DispatchError::Network(e.to_string())
                }
            })?;

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

pub struct MockAgentDispatcher {
    pub mock_responses: DashMap<Uuid, Result<Vec<u8>, DispatchError>>,
}

impl MockAgentDispatcher {
    pub fn new() -> Self {
        Self {
            mock_responses: DashMap::new(),
        }
    }

    pub fn with_response(agent_id: Uuid, data: Vec<u8>) -> Self {
        let map = DashMap::new();
        map.insert(agent_id, Ok(data));
        Self {
            mock_responses: map,
        }
    }

    pub fn with_error(agent_id: Uuid, err: DispatchError) -> Self {
        let map = DashMap::new();
        map.insert(agent_id, Err(err));
        Self {
            mock_responses: map,
        }
    }
}

impl Default for MockAgentDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentDispatcher for MockAgentDispatcher {
    async fn dispatch_query(
        &self,
        agent: &AgentMetadata,
        _payload: &[u8],
    ) -> Result<Vec<u8>, DispatchError> {
        match self.mock_responses.get(&agent.id) {
            Some(entry) => {
                let value = entry.value();
                match value {
                    Ok(data) => Ok(data.clone()),
                    Err(e) => match e {
                        DispatchError::Network(s) => Err(DispatchError::Network(s.clone())),
                        DispatchError::Timeout => Err(DispatchError::Timeout),
                        DispatchError::Parse(s) => Err(DispatchError::Parse(s.clone())),
                        DispatchError::Registry(s) => Err(DispatchError::Registry(s.clone())),
                    },
                }
            }
            None => Err(DispatchError::Network("No mock response for agent".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent(id: Uuid) -> AgentMetadata {
        AgentMetadata {
            id,
            endpoint: format!("http://agent-{}.local:8080", id),
            capabilities: vec!["test".into()],
            is_active: true,
        }
    }

    #[tokio::test]
    async fn mock_dispatcher_returns_expected_response() {
        let id = Uuid::new_v4();
        let agent = make_agent(id);
        let expected = b"hello from agent".to_vec();
        let dispatcher = MockAgentDispatcher::with_response(id, expected.clone());

        let result = dispatcher.dispatch_query(&agent, b"ping").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }

    #[tokio::test]
    async fn mock_dispatcher_returns_error_for_unregistered_agent() {
        let dispatcher = MockAgentDispatcher::new();
        let unknown_agent = make_agent(Uuid::new_v4());

        let result = dispatcher.dispatch_query(&unknown_agent, b"ping").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            DispatchError::Network(msg) => assert_eq!(msg, "No mock response for agent"),
            other => panic!("expected Network error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn mock_dispatcher_returns_error_type() {
        let id = Uuid::new_v4();
        let agent = make_agent(id);
        let dispatcher = MockAgentDispatcher::with_error(id, DispatchError::Timeout);

        let result = dispatcher.dispatch_query(&agent, b"ping").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            DispatchError::Timeout => {}
            other => panic!("expected Timeout, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn mock_dispatcher_network_error_propagation() {
        let id = Uuid::new_v4();
        let agent = make_agent(id);
        let dispatcher = MockAgentDispatcher::with_error(
            id,
            DispatchError::Network("connection refused".into()),
        );

        let result = dispatcher.dispatch_query(&agent, b"ping").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            DispatchError::Network(msg) => assert_eq!(msg, "connection refused"),
            other => panic!("expected Network error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn mock_dispatcher_parse_error_propagation() {
        let id = Uuid::new_v4();
        let agent = make_agent(id);
        let dispatcher =
            MockAgentDispatcher::with_error(id, DispatchError::Parse("bad json".into()));

        let result = dispatcher.dispatch_query(&agent, b"ping").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            DispatchError::Parse(msg) => assert_eq!(msg, "bad json"),
            other => panic!("expected Parse error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn dispatch_error_display() {
        assert_eq!(
            DispatchError::Network("reset".into()).to_string(),
            "Network error: reset"
        );
        assert_eq!(DispatchError::Timeout.to_string(), "Timeout error");
        assert_eq!(
            DispatchError::Parse("EOF".into()).to_string(),
            "Parsing error: EOF"
        );
        assert_eq!(
            DispatchError::Registry("not found".into()).to_string(),
            "Registry error: not found"
        );
    }

    // ----- Local HTTP test server helpers (no third-party mocking libs) -----

    use std::time::Duration as StdDuration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Spawn a one-shot HTTP server that always responds with the given status
    /// and body. The first request completes the response and the listener
    /// then closes — fine for single-shot tests.
    async fn start_one_shot_http_server(
        status: u16,
        body: &'static str,
    ) -> (String, tokio::task::JoinHandle<()>) {
        start_one_shot_http_server_delayed(status, body, StdDuration::ZERO).await
    }

    async fn start_one_shot_http_server_delayed(
        status: u16,
        body: &'static str,
        delay: StdDuration,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}/");

        let handle = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0u8; 8192];
                let _ = socket.read(&mut buf).await;
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                let response = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        (url, handle)
    }

    #[tokio::test]
    async fn http_dispatcher_returns_response_body_on_200() {
        let (url, _handle) = start_one_shot_http_server(200, "{\"ok\":true}").await;
        let client = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(2))
            .no_proxy()
            .build()
            .unwrap();
        let dispatcher = HttpAgentDispatcher::with_client(client);
        let agent = make_agent(Uuid::new_v4());
        let mut agent = agent;
        agent.endpoint = url;

        let result = dispatcher.dispatch_query(&agent, b"{}").await.unwrap();
        assert_eq!(result, b"{\"ok\":true}".to_vec());
    }

    #[tokio::test]
    async fn http_dispatcher_returns_network_error_on_500() {
        let (url, _handle) = start_one_shot_http_server(500, "boom").await;
        let client = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(2))
            .no_proxy()
            .build()
            .unwrap();
        let dispatcher = HttpAgentDispatcher::with_client(client);
        let agent = make_agent(Uuid::new_v4());
        let mut agent = agent;
        agent.endpoint = url;

        let result = dispatcher.dispatch_query(&agent, b"{}").await;
        match result.unwrap_err() {
            DispatchError::Network(msg) => assert!(msg.contains("500"), "got: {msg}"),
            other => panic!("expected Network(500), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn http_dispatcher_timeout_returns_timeout_error() {
        let (url, _handle) =
            start_one_shot_http_server_delayed(200, "late", StdDuration::from_millis(500)).await;
        let dispatcher = HttpAgentDispatcher::new(StdDuration::from_millis(50));
        let agent = make_agent(Uuid::new_v4());
        let mut agent = agent;
        agent.endpoint = url;

        let result = dispatcher.dispatch_query(&agent, b"{}").await;
        match result.unwrap_err() {
            DispatchError::Timeout => {}
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn mock_dispatcher_concurrent_calls_are_safe() {
        use std::sync::Arc;

        let id = Uuid::new_v4();
        let dispatcher = Arc::new(MockAgentDispatcher::with_response(id, b"shared".to_vec()));
        let agent = Arc::new(make_agent(id));

        let mut handles = Vec::new();
        for _ in 0..32 {
            let d = Arc::clone(&dispatcher);
            let a = Arc::clone(&agent);
            handles.push(tokio::spawn(async move {
                d.dispatch_query(&a, b"ping").await.unwrap()
            }));
        }

        for h in handles {
            let result = h.await.unwrap();
            assert_eq!(result, b"shared".to_vec());
        }
    }
}
