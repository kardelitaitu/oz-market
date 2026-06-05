# Baseline - Zero-Copy FFI Serialization

## Current State

As of starting Phase 4:
- All commands in the Tauri app serialize response data into JSON strings on the Rust side and deserialize them back in Svelte.
- No binary FFI transfer or MessagePack decoders are utilized.
- Massive responses (like search results or full offer histories) show measurable delays and CPU peaks.
