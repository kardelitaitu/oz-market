/*
last audited 06-06-26 by RSA-Agent
crate: oz-market-server | status: SAFE | lint: CLEAN
findings: 5 clippy issues fixed (bench/), 2 .lock().unwrap→.expect(), 1 unnecessary clone.
           async_run() is 185-line monolith — top refactor candidate.
next: split async_run() into named helpers | perf: no regressions; WAL uses Mutex but is perf-path
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
