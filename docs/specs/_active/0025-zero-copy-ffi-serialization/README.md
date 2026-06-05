---
id: 0025-zero-copy-ffi-serialization
title: Zero-Copy FFI Serialization
status: active
owner: mobile-team
implementer: agent
priority: P2
---

# Zero-Copy FFI Serialization

Status: `active`
Implementer: `agent`

## Summary

This specification governs the optimization of the Tauri-Svelte FFI bridge, replacing standard JSON string serialization with binary MessagePack serialization for high-volume transactions, preventing battery drain and UI frame drops.

## Scope

### In Scope
- Configuring `rmp-serde` inside the Tauri Rust workspace.
- Transferring binary arrays (`Vec<u8>`) across Tauri commands.
- Incorporating `@msgpack/msgpack` JavaScript decoding library on Svelte frontend.
- Setting up performance validation benchmarks for FFI transfers.

### Out of Scope
- Rewriting backend HTTP server endpoint payload types (applies strictly to the local client webview FFI bridge).

## Proposed Direction
1. Rust Commands:
   - Compile structures to MsgPack bytes using `rmp_serde::to_vec`.
   - Return raw `Vec<u8>` or `tauri::ipc::Response` binary frames.
2. Svelte Frontend:
   - Read FFI returns as `Uint8Array`.
   - Deserialize in Svelte using `decode()` from `@msgpack/msgpack`.
