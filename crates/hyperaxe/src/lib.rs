//! HTTP functionality for the LLM file system.

mod client;
mod server;

pub use client::HttpClient;
pub use server::HttpServer;

/// Common Result type for HTTP operations
pub type Result<T> = anyhow::Result<T>;
