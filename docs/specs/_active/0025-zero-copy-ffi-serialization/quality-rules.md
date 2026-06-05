# Quality Rules - Zero-Copy FFI Serialization

- **Fast Decoding**: Frontend decoding must execute in-place over the transferred FFI typed array to avoid memory duplicating.
- **Polyfill Safety**: Ensure that fallback polyfills for TypedArray are loaded on older Webview context environments (e.g. legacy Android devices) to prevent javascript crashes.
- **Error Boundaries**: If decoding fails in the Svelte code, catch the error and throw a clear user-facing validation exception.
