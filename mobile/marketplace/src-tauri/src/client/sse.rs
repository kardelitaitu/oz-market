use crate::auth;
use crate::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const CLAIMS_HEADER: &str = "x-marketplace-claims";

#[derive(Debug, Clone, PartialEq)]
pub struct SseMessage {
    pub event_type: String,
    pub data: String,
}

pub fn parse_sse_events(input: &str) -> Vec<SseMessage> {
    let mut messages = Vec::new();
    let mut event_type = String::new();
    let mut event_data = String::new();

    for raw_line in input.split('\n') {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            if !event_data.is_empty() {
                let ev = if event_type.is_empty() {
                    "update"
                } else {
                    &event_type
                };
                messages.push(SseMessage {
                    event_type: ev.to_string(),
                    data: event_data.clone(),
                });
            }
            event_type.clear();
            event_data.clear();
        } else if let Some(val) = line.strip_prefix("event: ") {
            event_type = val.to_string();
        } else if let Some(val) = line.strip_prefix("data: ") {
            event_data = val.to_string();
        }
    }
    messages
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListenerStatus {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
    Error,
}

impl ListenerStatus {
    fn as_str(&self) -> &'static str {
        match self {
            ListenerStatus::Connecting => "connecting",
            ListenerStatus::Connected => "connected",
            ListenerStatus::Reconnecting => "reconnecting",
            ListenerStatus::Disconnected => "disconnected",
            ListenerStatus::Error => "error",
        }
    }
}

pub(crate) trait SseEventCollector: Send + Sync {
    fn emit_event(&self, event_type: &str, data: &str);
    fn emit_status(&self, negotiation_id: &str, status: ListenerStatus);
    fn emit_error(&self, message: &str);
}

impl SseEventCollector for AppHandle {
    fn emit_event(&self, event_type: &str, data: &str) {
        let payload = serde_json::json!({
            "event_type": event_type,
            "data": data,
        });
        let _ = self.emit("negotiation-update", payload);
    }

    fn emit_status(&self, negotiation_id: &str, status: ListenerStatus) {
        let _ = self.emit(
            "negotiation-listener-status",
            serde_json::json!({
                "negotiation_id": negotiation_id,
                "status": status.as_str(),
            }),
        );
    }

    fn emit_error(&self, message: &str) {
        let _ = self.emit("negotiation-listener-error", message);
    }
}

pub async fn listen_negotiation(
    app_handle: AppHandle,
    state: AppState,
    negotiation_id: String,
    cancelled: Arc<AtomicBool>,
) {
    let claims = match auth::load_claims() {
        Ok(c) => c,
        Err(e) => {
            app_handle.emit_error(&format!("auth: {e}"));
            return;
        }
    };

    let base_url = state.base_url.read().await.clone();
    let url = format!("{base_url}/v1/events/negotiations/{negotiation_id}");
    let claims_value = serde_json::to_string(&claims).unwrap_or_default();

    listen_negotiation_impl(
        &app_handle,
        state,
        &negotiation_id,
        &url,
        &claims_value,
        cancelled,
    )
    .await;
}

async fn listen_negotiation_impl(
    collector: &impl SseEventCollector,
    state: AppState,
    negotiation_id: &str,
    url: &str,
    claims_value: &str,
    cancelled: Arc<AtomicBool>,
) {
    let mut retry_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(30);

    while !cancelled.load(Ordering::Relaxed) {
        if retry_delay > Duration::from_secs(1) {
            collector.emit_status(negotiation_id, ListenerStatus::Reconnecting);
            tokio::select! {
                _ = tokio::time::sleep(retry_delay) => {}
                _ = delay_cancel(&cancelled) => {
                    collector.emit_status(negotiation_id, ListenerStatus::Disconnected);
                    return;
                }
            }
        }

        collector.emit_status(negotiation_id, ListenerStatus::Connecting);

        let response = match state
            .client
            .get(url)
            .header(CLAIMS_HEADER, claims_value)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                collector.emit_error(&format!("connect: {e}"));
                retry_delay = (retry_delay * 2).min(max_delay);
                continue;
            }
        };

        retry_delay = Duration::from_secs(1);

        if !response.status().is_success() {
            collector.emit_status(negotiation_id, ListenerStatus::Error);
            collector.emit_error(&format!("status: {}", response.status()));
            return;
        }

        collector.emit_status(negotiation_id, ListenerStatus::Connected);
        read_sse_stream(collector, negotiation_id, response, &cancelled).await;
    }

    collector.emit_status(negotiation_id, ListenerStatus::Disconnected);
}

async fn read_sse_stream(
    collector: &impl SseEventCollector,
    negotiation_id: &str,
    mut response: reqwest::Response,
    cancelled: &Arc<AtomicBool>,
) {
    let mut buf = String::new();

    loop {
        if cancelled.load(Ordering::Relaxed) {
            return;
        }

        match response.chunk().await {
            Ok(Some(bytes)) => {
                let text = String::from_utf8_lossy(&bytes);
                buf.push_str(&text);

                let mut consumed = 0;
                while let Some(pos) = buf[consumed..].find('\n') {
                    consumed += pos + 1;
                }
                let ready = buf[..consumed].to_string();
                buf = buf[consumed..].to_string();

                for msg in parse_sse_events(&ready) {
                    collector.emit_event(&msg.event_type, &msg.data);
                }
            }
            Ok(None) => {
                collector.emit_status(negotiation_id, ListenerStatus::Reconnecting);
                return;
            }
            Err(e) => {
                collector.emit_error(&format!("stream: {e}"));
                collector.emit_status(negotiation_id, ListenerStatus::Reconnecting);
                return;
            }
        }
    }
}

async fn delay_cancel(cancelled: &Arc<AtomicBool>) {
    while !cancelled.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_event_with_event_and_data() {
        let input = "event: negotiation_updated\ndata: {\"key\":\"value\"}\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].event_type, "negotiation_updated");
        assert_eq!(msgs[0].data, "{\"key\":\"value\"}");
    }

    #[test]
    fn parse_sse_bare_data_defaults_to_update() {
        let input = "data: hello\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].event_type, "update");
        assert_eq!(msgs[0].data, "hello");
    }

    #[test]
    fn parse_sse_multiple_messages() {
        let input = "data: first\n\ndata: second\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].data, "first");
        assert_eq!(msgs[1].data, "second");
    }

    #[test]
    fn parse_sse_heartbeat_ignored() {
        let input = ": heartbeat\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn parse_sse_empty_input() {
        let msgs = parse_sse_events("");
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn parse_sse_only_event_no_data_is_empty() {
        let input = "event: foo\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn parse_sse_carriage_returns() {
        let input = "data: hello\r\n\r\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].data, "hello");
    }

    #[test]
    fn parse_sse_reuses_event_type_across_messages() {
        let input = "event: update\ndata: first\n\ndata: second\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].event_type, "update");
        assert_eq!(msgs[1].event_type, "update");
    }

    #[test]
    fn parse_sse_dangling_data_without_blank_line() {
        let input = "data: hello";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 0);
    }

    #[test]
    fn test_listener_status_serde_roundtrip() {
        let cases = vec![
            (ListenerStatus::Connecting, "\"connecting\""),
            (ListenerStatus::Connected, "\"connected\""),
            (ListenerStatus::Reconnecting, "\"reconnecting\""),
            (ListenerStatus::Disconnected, "\"disconnected\""),
            (ListenerStatus::Error, "\"error\""),
        ];

        for (status, expected_str) in cases {
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, expected_str);

            let deserialized: ListenerStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn test_listener_status_as_str_matches_serde() {
        let variants = [
            ListenerStatus::Connecting,
            ListenerStatus::Connected,
            ListenerStatus::Reconnecting,
            ListenerStatus::Disconnected,
            ListenerStatus::Error,
        ];
        for v in &variants {
            let serde_json_str = serde_json::to_string(v).unwrap();
            let serde_lowercase = serde_json_str.trim_matches('"');
            assert_eq!(v.as_str(), serde_lowercase);
        }
    }

    proptest::proptest! {
        #[test]
        fn parse_sse_events_property_roundtrip(
            ref event_type in "[a-zA-Z0-9_]{0,30}",
            ref data in "[^\r\n]*"
        ) {
            let input = if event_type.is_empty() {
                format!("data: {}\n\n", data)
            } else {
                format!("event: {}\ndata: {}\n\n", event_type, data)
            };
            let messages = parse_sse_events(&input);
            if !data.is_empty() {
                assert_eq!(messages.len(), 1);
                let expected_event = if event_type.is_empty() { "update" } else { event_type };
                assert_eq!(messages[0].event_type, expected_event.to_string());
                assert_eq!(messages[0].data, data.to_string());
            } else {
                assert_eq!(messages.len(), 0);
            }
        }
    }
}

#[cfg(test)]
mod integration {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex;
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    /// Build a reqwest::Client that ignores system HTTP_PROXY/HTTPS_PROXY
    /// env vars. Without this, tests run on Windows hosts with a corporate
    /// proxy intercept all traffic to local wiremock servers and return 403.
    fn no_proxy_client() -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("no_proxy reqwest client should build")
    }

    /// Same as `no_proxy_client` but with a per-request timeout.
    fn no_proxy_client_with_timeout(timeout: std::time::Duration) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(timeout)
            .no_proxy()
            .build()
            .expect("no_proxy reqwest client with timeout should build")
    }

    /// Convenience: GET `url` via a no_proxy client and return the response.
    async fn no_proxy_get(url: impl reqwest::IntoUrl) -> reqwest::Response {
        no_proxy_client()
            .get(url)
            .send()
            .await
            .expect("no_proxy GET should succeed")
    }

    struct TestSseCollector {
        events: Arc<Mutex<Vec<(String, String)>>>,
        statuses: Arc<Mutex<Vec<(String, ListenerStatus)>>>,
        errors: Arc<Mutex<Vec<String>>>,
    }

    impl TestSseCollector {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                statuses: Arc::new(Mutex::new(Vec::new())),
                errors: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl SseEventCollector for TestSseCollector {
        fn emit_event(&self, event_type: &str, data: &str) {
            self.events
                .lock()
                .unwrap()
                .push((event_type.to_string(), data.to_string()));
        }
        fn emit_status(&self, negotiation_id: &str, status: ListenerStatus) {
            self.statuses
                .lock()
                .unwrap()
                .push((negotiation_id.to_string(), status));
        }
        fn emit_error(&self, message: &str) {
            self.errors.lock().unwrap().push(message.to_string());
        }
    }

    impl SseEventCollector for std::sync::Arc<TestSseCollector> {
        fn emit_event(&self, event_type: &str, data: &str) {
            self.events
                .lock()
                .unwrap()
                .push((event_type.to_string(), data.to_string()));
        }
        fn emit_status(&self, negotiation_id: &str, status: ListenerStatus) {
            self.statuses
                .lock()
                .unwrap()
                .push((negotiation_id.to_string(), status));
        }
        fn emit_error(&self, message: &str) {
            self.errors.lock().unwrap().push(message.to_string());
        }
    }

    #[tokio::test]
    async fn read_sse_stream_forwards_single_event() {
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::any())
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("event: negotiation_updated\ndata: {\"id\":\"n1\"}\n\n"),
            )
            .mount(&server)
            .await;

        let response = no_proxy_get(server.uri()).await;
        let collector = TestSseCollector::new();
        let cancelled = Arc::new(AtomicBool::new(false));

        read_sse_stream(&collector, "n1", response, &cancelled).await;

        let events = collector.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "negotiation_updated");
        assert_eq!(events[0].1, "{\"id\":\"n1\"}");
    }

    #[tokio::test]
    async fn read_sse_stream_forwards_multiple_events() {
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::any())
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(
                        "event: update\ndata: {\"seq\":1}\n\nevent: update\ndata: {\"seq\":2}\n\n",
                    ),
            )
            .mount(&server)
            .await;

        let response = no_proxy_get(server.uri()).await;
        let collector = TestSseCollector::new();
        let cancelled = Arc::new(AtomicBool::new(false));

        read_sse_stream(&collector, "n2", response, &cancelled).await;

        let events = collector.events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].1, "{\"seq\":1}");
        assert_eq!(events[1].1, "{\"seq\":2}");
    }

    #[tokio::test]
    async fn listen_negotiation_impl_emits_error_on_bad_status() {
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::any())
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let state = crate::state::AppState::with_client(no_proxy_client());
        let collector = TestSseCollector::new();
        let cancelled = Arc::new(AtomicBool::new(false));

        listen_negotiation_impl(
            &collector,
            state,
            "n3",
            &server.uri(),
            "fake-claims",
            cancelled,
        )
        .await;

        let statuses = collector.statuses.lock().unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0].1, ListenerStatus::Connecting);
        assert_eq!(statuses[1].1, ListenerStatus::Error);

        let errors = collector.errors.lock().unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("500"));
    }

    #[tokio::test]
    async fn listen_negotiation_impl_emits_disconnected_when_cancelled_early() {
        let collector = TestSseCollector::new();
        let cancelled = Arc::new(AtomicBool::new(true));
        let state = crate::state::AppState::new();

        listen_negotiation_impl(
            &collector,
            state,
            "n4",
            "http://127.0.0.1:1/x",
            "claims",
            cancelled,
        )
        .await;

        let statuses = collector.statuses.lock().unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].1, ListenerStatus::Disconnected);
        assert!(collector.events.lock().unwrap().is_empty());
        assert!(collector.errors.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn listen_negotiation_impl_retries_after_timeout_then_succeeds() {
        let responder = counted_responder(vec![
            ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(5)),
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string("event: negotiation_updated\ndata: {\"retried\":true}\n\n"),
        ]);
        let server = MockServer::start().await;

        Mock::given(wiremock::matchers::any())
            .respond_with(responder)
            .mount(&server)
            .await;

        // Client with 50ms timeout — first request (5s delay) will timeout
        let state = crate::state::AppState {
            client: no_proxy_client_with_timeout(std::time::Duration::from_millis(50)),
            base_url: std::sync::Arc::new(tokio::sync::RwLock::new(
                "http://placeholder".to_string(),
            )),
            rate_limiter: std::sync::Arc::new(tokio::sync::RwLock::new(
                crate::client::rate_limit::RateLimitTracker::new(),
            )),
            negotiation_listeners: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        };

        let collector = std::sync::Arc::new(TestSseCollector::new());
        let cancelled = std::sync::Arc::new(AtomicBool::new(false));

        let c = collector.clone();
        let cancel = cancelled.clone();
        let url = format!("{}/events", server.uri());
        let handle = tokio::spawn(async move {
            listen_negotiation_impl(&*c, state, "n5", &url, "claims", cancel).await;
        });

        // Backoff: error at ~50ms, then 1s check skip → 2s sleep. After ~2.5s total, retry succeeds.
        // Give it 4s to be safe, then cancel.
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), handle).await;

        let errors = collector.errors.lock().unwrap();
        assert!(!errors.is_empty(), "expected at least one timeout error");

        let events = collector.events.lock().unwrap();
        assert_eq!(events.len(), 1, "expected one forwarded SSE event");
        assert_eq!(events[0].0, "negotiation_updated");
        assert_eq!(events[0].1, "{\"retried\":true}");

        let statuses = collector.statuses.lock().unwrap();
        let connected = statuses
            .iter()
            .any(|(_, s)| *s == ListenerStatus::Connected);
        assert!(connected, "expected Connected status after retry");
        let error_status = statuses.iter().any(|(_, s)| *s == ListenerStatus::Error);
        assert!(error_status, "expected Error status from 500 response");
        let reconnecting = statuses
            .iter()
            .any(|(_, s)| *s == ListenerStatus::Reconnecting);
        assert!(reconnecting, "expected Reconnecting after stream ends");
        // Disconnected is only emitted after the while loop exits;
        // the 500 error handler does a return inside the loop.
    }

    pub struct CountedResponder {
        responses: Vec<ResponseTemplate>,
        count: AtomicUsize,
    }

    impl wiremock::Respond for CountedResponder {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            let idx = self
                .count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if idx < self.responses.len() {
                self.responses[idx].clone()
            } else {
                ResponseTemplate::new(500)
            }
        }
    }

    pub fn counted_responder(responses: Vec<ResponseTemplate>) -> CountedResponder {
        CountedResponder {
            responses,
            count: AtomicUsize::new(0),
        }
    }

    #[tokio::test]
    async fn read_sse_stream_midstream_cancellation() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}/events", addr);

        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
                .await
                .unwrap();

            socket
                .write_all(b"event: update\ndata: first\n\n")
                .await
                .unwrap();
            socket.flush().await.unwrap();

            let _ = rx.await;

            socket
                .write_all(b"event: update\ndata: second\n\n")
                .await
                .unwrap();
            socket.flush().await.unwrap();
        });

        let response = no_proxy_get(&url).await;
        let collector = TestSseCollector::new();
        let cancelled = Arc::new(AtomicBool::new(false));

        let c = collector.events.clone();
        let cancel = cancelled.clone();
        let handle = tokio::spawn(async move {
            let collector = TestSseCollector {
                events: c,
                statuses: Arc::new(Mutex::new(Vec::new())),
                errors: Arc::new(Mutex::new(Vec::new())),
            };
            read_sse_stream(&collector, "n6", response, &cancel).await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        {
            let events = collector.events.lock().unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].1, "first");
        }

        cancelled.store(true, Ordering::Relaxed);
        let _ = tx.send(());

        let _ = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .unwrap();

        let events = collector.events.lock().unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn counted_responder_serves_in_order() {
        let server = MockServer::start().await;
        let responder = counted_responder(vec![
            ResponseTemplate::new(200).set_body_string("first"),
            ResponseTemplate::new(201).set_body_string("second"),
            ResponseTemplate::new(202).set_body_string("third"),
        ]);
        Mock::given(wiremock::matchers::any())
            .respond_with(responder)
            .mount(&server)
            .await;

        let resp1 = no_proxy_get(&server.uri()).await;
        let resp2 = no_proxy_get(&server.uri()).await;
        let resp3 = no_proxy_get(&server.uri()).await;
        // Consume the first two responses from the counted_responder so
        // the third (202) is the one we assert on.
        let _ = resp1;
        let _ = resp2;
        assert_eq!(resp3.status(), 202);
        assert_eq!(resp3.text().await.unwrap(), "third");
    }

    #[tokio::test]
    async fn counted_responder_returns_500_when_exhausted() {
        let server = MockServer::start().await;
        let responder = counted_responder(vec![ResponseTemplate::new(200).set_body_string("ok")]);
        Mock::given(wiremock::matchers::any())
            .respond_with(responder)
            .mount(&server)
            .await;

        let resp1 = no_proxy_get(&server.uri()).await;
        assert_eq!(resp1.status(), 200);

        let resp2 = no_proxy_get(&server.uri()).await;
        assert_eq!(resp2.status(), 500);
    }

    #[tokio::test]
    async fn listen_negotiation_impl_emits_disconnected_on_cancel_during_active_stream() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}/events", addr);

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
                .await
                .unwrap();
            socket
                .write_all(b"event: update\ndata: hello\n\n")
                .await
                .unwrap();
            socket.flush().await.unwrap();
            let _ = rx.await;
            socket
                .write_all(b"event: update\ndata: ignored\n\n")
                .await
                .unwrap();
            socket.flush().await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        let state = crate::state::AppState::with_client(no_proxy_client());
        let collector = TestSseCollector::new();
        let cancelled = Arc::new(AtomicBool::new(false));

        let c = collector.events.clone();
        let s = collector.statuses.clone();
        let e = collector.errors.clone();
        let cancel = cancelled.clone();
        let handle = tokio::spawn(async move {
            let collector = TestSseCollector {
                events: c,
                statuses: s,
                errors: e,
            };
            listen_negotiation_impl(&collector, state, "n8", &url, "claims", cancel).await;
        });

        tokio::time::sleep(Duration::from_millis(200)).await;
        cancelled.store(true, Ordering::Relaxed);
        let _ = tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .unwrap();

        let statuses = collector.statuses.lock().unwrap();
        let has_connecting = statuses
            .iter()
            .any(|(_, s)| *s == ListenerStatus::Connecting);
        let has_connected = statuses
            .iter()
            .any(|(_, s)| *s == ListenerStatus::Connected);
        let has_disconnected = statuses
            .iter()
            .any(|(_, s)| *s == ListenerStatus::Disconnected);
        assert!(has_connecting, "expected Connecting");
        assert!(has_connected, "expected Connected");
        assert!(has_disconnected, "expected Disconnected");
    }

    #[tokio::test]
    async fn listen_negotiation_impl_cancels_during_reconnect_backoff() {
        let responder = counted_responder(vec![
            ResponseTemplate::new(200).set_delay(Duration::from_secs(5))
        ]);
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::any())
            .respond_with(responder)
            .mount(&server)
            .await;

        let state = crate::state::AppState {
            client: no_proxy_client_with_timeout(Duration::from_millis(50)),
            base_url: std::sync::Arc::new(tokio::sync::RwLock::new(
                "http://placeholder".to_string(),
            )),
            rate_limiter: std::sync::Arc::new(tokio::sync::RwLock::new(
                crate::client::rate_limit::RateLimitTracker::new(),
            )),
            negotiation_listeners: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        };

        let collector = std::sync::Arc::new(TestSseCollector::new());
        let cancelled = std::sync::Arc::new(AtomicBool::new(false));

        let c = collector.clone();
        let cancel = cancelled.clone();
        let url = format!("{}/events", server.uri());
        let handle = tokio::spawn(async move {
            listen_negotiation_impl(&*c, state, "n9", &url, "claims", cancel).await;
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
        cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .unwrap();

        let statuses = collector.statuses.lock().unwrap();
        let has_connecting = statuses
            .iter()
            .any(|(_, s)| *s == ListenerStatus::Connecting);
        let has_reconnecting = statuses
            .iter()
            .any(|(_, s)| *s == ListenerStatus::Reconnecting);
        let has_disconnected = statuses
            .iter()
            .any(|(_, s)| *s == ListenerStatus::Disconnected);
        assert!(has_connecting, "expected Connecting");
        assert!(has_reconnecting, "expected Reconnecting");
        assert!(has_disconnected, "expected Disconnected");
    }

    // ---------------------------------------------------------------------
    // Spec 0006 — Additional mid-stream cancellation coverage
    // ---------------------------------------------------------------------

    #[tokio::test]
    async fn read_sse_stream_cancel_drops_partial_buffer() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}/events", addr);

        let (tx, rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
                .await
                .unwrap();
            socket.write_all(b"event: a\ndata: 1\n\n").await.unwrap();
            socket.flush().await.unwrap();
            let _ = rx.await;
            socket.write_all(b"data: 2\n").await.unwrap();
            socket.flush().await.unwrap();
        });

        let response = no_proxy_get(&url).await;
        let collector = TestSseCollector::new();
        let cancelled = Arc::new(AtomicBool::new(false));

        let c = collector.events.clone();
        let cancel = cancelled.clone();
        let handle = tokio::spawn(async move {
            let collector = TestSseCollector {
                events: c,
                statuses: Arc::new(Mutex::new(Vec::new())),
                errors: Arc::new(Mutex::new(Vec::new())),
            };
            read_sse_stream(&collector, "n_partial", response, &cancel).await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await;
        cancelled.store(true, Ordering::Relaxed);
        let _ = tx.send(());
        let _ = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .unwrap();

        let events = collector.events.lock().unwrap();
        assert_eq!(events.len(), 1, "only the complete event should be emitted");
        assert_eq!(events[0].1, "1");
    }

    // ---------------------------------------------------------------------
    // Spec 0007 — SSE parser additional unit + property tests
    // ---------------------------------------------------------------------

    #[test]
    fn parse_sse_ignores_id_and_retry_fields() {
        let input = "id: 42\nretry: 3000\ndata: hello\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].event_type, "update");
        assert_eq!(msgs[0].data, "hello");
    }

    #[test]
    fn parse_sse_heartbeat_does_not_reset_event_type() {
        // Heartbeat lines (starting with `:`) match neither prefix, so they're skipped.
        // The first message sets event_type=foo, then a blank line clears event_type
        // (per the parser's blank-line semantics). The second message therefore
        // defaults to "update". This pins the current behavior.
        let input = "event: foo\ndata: a\n\n: keepalive\n\ndata: b\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].event_type, "foo");
        assert_eq!(msgs[0].data, "a");
        assert_eq!(msgs[1].event_type, "update");
        assert_eq!(msgs[1].data, "b");
    }

    #[test]
    fn parse_sse_event_type_with_spaces() {
        // `strip_prefix("event: ")` captures everything after the first space,
        // including additional spaces. Pin this so any change to the prefix is
        // an explicit decision.
        let input = "event: user logged in\ndata: x\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].event_type, "user logged in");
        assert_eq!(msgs[0].data, "x");
    }

    #[test]
    fn parse_sse_data_without_space_is_ignored() {
        // The parser requires "data: " (with trailing space). Lines like "data:foo"
        // are silently dropped. Pin the exact prefix semantics.
        let input = "data:foo\n\n";
        let msgs = parse_sse_events(input);
        assert_eq!(msgs.len(), 0);
    }

    proptest::proptest! {
        #[test]
        fn parse_sse_preserves_message_order_property(
            ref event_a in "[a-zA-Z0-9_]{1,20}",
            ref data_a in "[a-zA-Z0-9_]{1,20}",
            ref event_b in "[a-zA-Z0-9_]{1,20}",
            ref data_b in "[a-zA-Z0-9_]{1,20}",
        ) {
            let input = format!(
                "event: {}\ndata: {}\n\nevent: {}\ndata: {}\n\n",
                event_a, data_a, event_b, data_b
            );
            let msgs = parse_sse_events(&input);
            assert_eq!(msgs.len(), 2);
            assert_eq!(msgs[0].event_type, *event_a);
            assert_eq!(msgs[0].data, *data_a);
            assert_eq!(msgs[1].event_type, *event_b);
            assert_eq!(msgs[1].data, *data_b);
        }
    }

    // ---------------------------------------------------------------------
    // Spec 0008 — CountedResponder additional coverage
    // ---------------------------------------------------------------------

    #[tokio::test]
    async fn counted_responder_thread_safe_under_concurrent_requests() {
        let server = MockServer::start().await;
        let responder = counted_responder(vec![
            ResponseTemplate::new(200).set_body_string("first"),
            ResponseTemplate::new(201).set_body_string("second"),
            ResponseTemplate::new(202).set_body_string("third"),
        ]);
        Mock::given(wiremock::matchers::any())
            .respond_with(responder)
            .mount(&server)
            .await;

        let url = server.uri();
        let mut handles = Vec::new();
        for _ in 0..20 {
            let u = url.clone();
            handles.push(tokio::spawn(async move {
                let r = no_proxy_get(&u).await;
                r.status().as_u16()
            }));
        }

        let mut statuses = Vec::new();
        for h in handles {
            statuses.push(h.await.unwrap());
        }

        let mut counts = std::collections::HashMap::new();
        for s in &statuses {
            *counts.entry(*s).or_insert(0u32) += 1;
        }

        assert_eq!(*counts.get(&200).unwrap_or(&0), 1, "exactly one 200");
        assert_eq!(*counts.get(&201).unwrap_or(&0), 1, "exactly one 201");
        assert_eq!(*counts.get(&202).unwrap_or(&0), 1, "exactly one 202");
        assert_eq!(
            *counts.get(&500).unwrap_or(&0),
            17,
            "remaining 17 must be 500 (exhausted)"
        );
    }

    #[tokio::test]
    async fn counted_responder_empty_vec_always_500() {
        let server = MockServer::start().await;
        let responder = counted_responder(vec![]);
        Mock::given(wiremock::matchers::any())
            .respond_with(responder)
            .mount(&server)
            .await;

        let resp = no_proxy_get(&server.uri()).await;
        assert_eq!(resp.status(), 500);
    }

    #[tokio::test]
    async fn counted_responder_preserves_response_headers() {
        // Pin: CountedResponder returns a clone of the configured
        // ResponseTemplate, so custom headers (anything not touched by
        // wiremock defaults) must round-trip.
        //
        // NOTE on content-type: wiremock 0.6's `set_body_string` sets the
        // response's content-type to "text/plain" regardless of any prior
        // `insert_header("content-type", ...)` call, so we don't assert
        // content-type here. The SSE parser tests (`read_sse_stream_*`)
        // already cover the production content-type contract end-to-end.
        let server = MockServer::start().await;
        let responder = counted_responder(vec![ResponseTemplate::new(200)
            .insert_header("x-custom", "foo")
            .insert_header("x-another", "bar")
            .set_body_string("ok")]);
        Mock::given(wiremock::matchers::any())
            .respond_with(responder)
            .mount(&server)
            .await;

        let resp = no_proxy_get(&server.uri()).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers().get("x-custom").and_then(|v| v.to_str().ok()),
            Some("foo")
        );
        assert_eq!(
            resp.headers()
                .get("x-another")
                .and_then(|v| v.to_str().ok()),
            Some("bar")
        );
        assert_eq!(resp.text().await.unwrap(), "ok");
    }

    #[tokio::test]
    async fn counted_responder_serves_sse_body() {
        let server = MockServer::start().await;
        let responder = counted_responder(vec![ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string("event: x\ndata: 1\n\n")]);
        Mock::given(wiremock::matchers::any())
            .respond_with(responder)
            .mount(&server)
            .await;

        let resp = no_proxy_get(&server.uri()).await;
        let body = resp.text().await.unwrap();
        let msgs = parse_sse_events(&body);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].event_type, "x");
        assert_eq!(msgs[0].data, "1");
    }

    // ---------------------------------------------------------------------
    // Spec 0009 — ListenerStatus serde additional coverage
    // ---------------------------------------------------------------------

    #[test]
    fn test_listener_status_rejects_uppercase_variants() {
        assert!(serde_json::from_str::<ListenerStatus>("\"Connected\"").is_err());
        assert!(serde_json::from_str::<ListenerStatus>("\"CONNECTED\"").is_err());
        assert!(serde_json::from_str::<ListenerStatus>("\"Connecting\"").is_err());
    }

    #[test]
    fn test_listener_status_rejects_unknown_variant() {
        assert!(serde_json::from_str::<ListenerStatus>("\"pending\"").is_err());
        assert!(serde_json::from_str::<ListenerStatus>("\"\"").is_err());
        assert!(serde_json::from_str::<ListenerStatus>("\"unknown\"").is_err());
    }

    #[test]
    fn test_listener_status_rejects_non_string_payload() {
        assert!(serde_json::from_str::<ListenerStatus>("123").is_err());
        assert!(serde_json::from_str::<ListenerStatus>("null").is_err());
        assert!(serde_json::from_str::<ListenerStatus>("true").is_err());
        assert!(serde_json::from_str::<ListenerStatus>("[]").is_err());
    }

    #[test]
    fn test_listener_status_roundtrips_inside_outer_struct() {
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
        struct Wrap {
            status: ListenerStatus,
        }

        let cases = [
            (ListenerStatus::Connecting, "{\"status\":\"connecting\"}"),
            (ListenerStatus::Connected, "{\"status\":\"connected\"}"),
            (
                ListenerStatus::Reconnecting,
                "{\"status\":\"reconnecting\"}",
            ),
            (
                ListenerStatus::Disconnected,
                "{\"status\":\"disconnected\"}",
            ),
            (ListenerStatus::Error, "{\"status\":\"error\"}"),
        ];
        for (status, expected) in cases {
            let serialized = serde_json::to_string(&Wrap { status }).unwrap();
            assert_eq!(serialized, expected);
            let parsed: Wrap = serde_json::from_str(expected).unwrap();
            assert_eq!(parsed.status, status);
        }
    }

    #[test]
    fn test_listener_status_deserialize_each_variant_independently() {
        let cases = [
            ("\"connecting\"", ListenerStatus::Connecting),
            ("\"connected\"", ListenerStatus::Connected),
            ("\"reconnecting\"", ListenerStatus::Reconnecting),
            ("\"disconnected\"", ListenerStatus::Disconnected),
            ("\"error\"", ListenerStatus::Error),
        ];
        for (json, expected) in cases {
            let parsed: ListenerStatus = serde_json::from_str(json).unwrap();
            assert_eq!(parsed, expected);
        }
    }
}
