# Decisions - Zero-Copy FFI Serialization

## Architecture Decisions

### 1. MessagePack for FFI Payload Transfer
- **Decision**: Use MessagePack (`rmp-serde`) binary format instead of custom binary encoding or Protobuf.
- **Rationale**: MessagePack has excellent Rust library support (`rmp-serde`) and extremely lightweight, fast JavaScript decoding implementations that preserve key-value object mappings natively without requiring schema generation files on Svelte's side.

### 2. High-Frequency / Large Payload Scope only
- **Decision**: Restrict FFI MsgPack transfer to commands dealing with large arrays or high-frequency telemetry (like SSE updates and searches); keep simple configuration or status commands as standard JSON.
- **Rationale**: Maximizes return-on-investment by focusing optimization on actual performance hotspots, avoiding unnecessary complexity for trivial commands.
