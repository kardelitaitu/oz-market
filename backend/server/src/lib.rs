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

pub fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}
