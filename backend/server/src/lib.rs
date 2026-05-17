pub mod app;
pub mod auth;
pub mod background;
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

#[cfg(test)]
pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    http::runtime::run()
}
