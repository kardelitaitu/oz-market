# CI Commands

Run from repository root:

```powershell
./check.ps1
```

Benchmark profiles:

```powershell
cargo run --manifest-path backend/server/Cargo.toml --bin bench_concurrent -- --url http://127.0.0.1:8080 --requests 1200 --concurrency 48 --threads 8 --claims-mode public
cargo run --manifest-path backend/server/Cargo.toml --bin bench_concurrent -- --url http://127.0.0.1:8080 --requests 1200 --concurrency 48 --threads 8 --claims-mode rotating
cargo run --manifest-path backend/server/Cargo.toml --bin bench_concurrent -- --url http://127.0.0.1:8080 --requests 1200 --concurrency 48 --threads 8 --claims-mode fixed
```
