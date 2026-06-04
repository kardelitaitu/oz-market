# TODO

Project state: near-production-ready. All foundation work is complete.
Remaining items are deployment, demo validation, and future-facing extensions.

## Ready

- [x] Rust backend (Actix-web, 12MB release binary, 60k+ ops/s search)
- [x] MCP server (10 tools, full transaction flow verified via stdio)
- [x] PostgreSQL schema (17 migrations, auto-applied on boot)
- [x] Auth: API key fallback, JWT support wired, role/permission matrix
- [x] Rate limiting (per-IP search, per-token writes)
- [x] Idempotency (Postgres-backed)
- [x] Graceful shutdown (SIGINT/SIGTERM, configurable drain timeout)
- [x] Structured JSON logging (LOG_FORMAT=json)
- [x] Docker + docker-compose (one-command deploy)
- [x] CI (fmt, clippy, 184 lib tests, 15 MCP smoke tests, Postgres integration)
- [x] Actix integration tests (4 tests exercising full HTTP stack with in-memory repos)
- [x] Caches byte-limited via Moka weigher, sized for 4GB VPS
- [x] Deployment runbook (docs/deploy.md) with 8-step demo transaction

## Next (for production launch)

- [ ] Deploy to a Linux VPS (recommended: Hetzner CX22 €4/mo or Railway.app)
- [ ] Run the 8-step demo transaction against live server
- [ ] Configure domain name + reverse proxy (Caddy / Nginx)
- [ ] Set up PostgreSQL backups (pg_dump cron job or managed DB)
- [ ] Set up log aggregation (Loki / Datadog) via LOG_FORMAT=json

## Future

- [ ] Mobile apps (Android/iOS — currently design docs only)
- [ ] Stripe / Cryptomus payment integration
- [ ] AI credit system and premium plans
- [ ] Developer SDK (Rust / TypeScript)
- [ ] Agent personality templates
- [ ] Web frontend (minimal 3-page site)
- [ ] MCP HTTP/SSE transport (currently stdio-only)
- [ ] End-to-end benchmark CI step (bench_concurrent regression check)
