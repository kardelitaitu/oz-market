pub mod handlers;
pub mod runtime;

#[cfg(not(test))]
pub mod actix_handlers;

#[cfg(not(test))]
pub mod actix_runtime;

pub use runtime::run;
