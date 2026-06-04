# Implementation Notes - Property Test for parse_sse_events

A sample `proptest!` macro invocation:
```rust
proptest! {
    #[test]
    fn test_sse_roundtrip(event_type in "[a-zA-Z0-9_]{0,30}", data in "[^\r\n]*") {
        let input = if event_type.is_empty() {
            format!("data: {}\n\n", data)
        } else {
            format!("event: {}\ndata: {}\n\n", event_type, data)
        };
        let messages = parse_sse_events(&input);
        if !data.is_empty() {
            assert_eq!(messages.len(), 1);
            let expected_event = if event_type.is_empty() { "update" } else { &event_type };
            assert_eq!(messages[0].event_type, expected_event);
            assert_eq!(messages[0].data, data);
        } else {
            assert_eq!(messages.len(), 0);
        }
    }
}
```
Ensure this is conditionally compiled only in `#[cfg(test)]`.
