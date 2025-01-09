use crate::error::Result;
use async_trait::async_trait;

pub mod anthropic;
// pub mod factory;s

#[async_trait]
pub trait Model: Send + Sync {
    /// Initialize the model
    async fn initialize(&self) -> Result<()>;

    /// Send a message and get a response
    async fn send_message(&self, message: &str) -> Result<String>;

    /// Get model identifier
    fn id(&self) -> &str;

    /// Check if model supports streaming
    fn supports_streaming(&self) -> bool;
}
