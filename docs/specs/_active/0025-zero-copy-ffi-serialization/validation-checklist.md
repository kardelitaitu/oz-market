# Validation Checklist - Zero-Copy FFI Serialization

This checklist is used to confirm the completion of Spec 0025:

- [ ] `rmp-serde` is declared in `mobile/marketplace/src-tauri/Cargo.toml`.
- [ ] `@msgpack/msgpack` npm dependency is installed in `mobile/marketplace/package.json`.
- [ ] MsgPack Rust serialization helper successfully serializes target datasets into bytes.
- [ ] TypeScript/JS deserialization correctly reconstructs objects without data corruption.
- [ ] Benchmark measurements confirm FFI roundtrip timings remain below 1ms.
