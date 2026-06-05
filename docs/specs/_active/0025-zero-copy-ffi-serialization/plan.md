# Plan - Zero-Copy FFI Serialization

## Implementation Steps

1. **Rust MsgPack Config**:
   - Add `rmp-serde` dependency to `mobile/marketplace/src-tauri/Cargo.toml`.
   - Update Tauri command return type declarations to `Result<Vec<u8>, String>`.

2. **JS MsgPack Decoder Config**:
   - Install `@msgpack/msgpack` npm package under `mobile/marketplace/`.
   - Create helper `decode_ffi_payload(array: Uint8Array)` in mobile TypeScript files.

3. **FFI command migration**:
   - Migrate target high-frequency queries (e.g. `get_negotiations`, `search_listings`) to MsgPack returns.
   - Deserialize on Svelte 5 frontend component loaders.
