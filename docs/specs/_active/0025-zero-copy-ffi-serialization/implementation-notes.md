# Implementation Notes - Zero-Copy FFI Serialization

## Rust Command Serialization

```rust
#[tauri::command]
pub async fn get_large_dataset() -> Result<Vec<u8>, String> {
    let dataset = fetch_dataset().await?;
    let bytes = rmp_serde::to_vec(&dataset).map_err(|e| e.to_string())?;
    Ok(bytes)
}
```

## Svelte/JS MsgPack Deserialization

```typescript
import { decode } from "@msgpack/msgpack";
import { invoke } from "@tauri-apps/api/core";

async function fetchLargeDataset(): Promise<any> {
  const binaryData: Uint8Array = await invoke("get_large_dataset");
  const data = decode(binaryData);
  return data;
}
```
