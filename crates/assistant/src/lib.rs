pub mod client;
pub mod error;
pub mod models;
pub mod protocol;

pub use client::AssistantClient;
pub use error::{AssistantError, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
