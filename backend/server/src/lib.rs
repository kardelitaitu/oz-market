pub mod app;
pub mod auth;
pub mod background;
pub mod config;
pub mod domain;
pub mod errors;
pub mod http;
pub mod models;
pub mod observability;
pub mod repositories;
pub mod services;
#[cfg(test)]
pub mod test_support;

pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    http::runtime::run()
}
