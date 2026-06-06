/*
last audited 06-06-26 by RSA-Agent + clippy
crate: oz-market-server | status: SAFE | lint: CLEAN
findings: async_run() refactored from 185-line monolith into 8 named helpers
           (resolve_bind_address, init_tracing, build_moka_caches, init_event_bus,
            init_ledger_system, init_agent_system, resolve_server_config,
            build_http_server, setup_graceful_shutdown, run_migrations).
           AppDependencies struct groups all 15 actix app_data values.
           AgentSystemDeps type alias for complex return type.
           clippy -- -D warnings clean, 420 lib tests + 72 integration tests pass.
next: wire check.ps1 into pre-commit hook | docs-auditor sync complete
*/
pub mod app;
pub mod auth;
pub mod background;
pub mod bench;
pub mod bootstrap;
pub mod config;
pub mod domain;
pub mod errors;
pub mod http;
pub mod models;
pub mod observability;
pub mod openapi;
pub mod repositories;
pub mod services;
pub mod test_support;

#[cfg(not(test))]
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    http::actix_runtime::run()
}
