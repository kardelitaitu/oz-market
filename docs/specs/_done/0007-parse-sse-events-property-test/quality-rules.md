# Quality Rules - Property Test for parse_sse_events

- Proptest case count should be kept reasonable (default is fine, e.g. 256 cases) to avoid dragging down general cargo test times.
- Ensure no panic scenarios can be triggered by arbitrary strings of printable characters.
