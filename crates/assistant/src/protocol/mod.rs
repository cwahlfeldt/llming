pub mod handler;
pub mod mapping;

use crate::error::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle_message(&self, message: &[u8]) -> Result<Vec<u8>>;
    async fn initialize(&self) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
}