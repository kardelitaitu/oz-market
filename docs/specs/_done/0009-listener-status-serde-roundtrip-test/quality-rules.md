# Quality Rules - ListenerStatus Serde Round-trip

- No spelling mistakes in expected JSON strings. They must exactly match the frontend's listener state names.
- Keep the `as_str()` method synchronized with the serde implementation if any variant changes.
