use crate::{error::Result, models::Model};
use std::sync::Arc;

#[allow(clippy::module_name_repetitions)]
pub struct AssistantClient {
    model: Arc<dyn Model>,
}

impl AssistantClient {
    /// Creates a new client instance
    #[must_use]
    pub fn new(model: Arc<dyn Model>) -> Self {
        Self { model }
    }

    /// Initialize the client and underlying model
    /// # Errors
    /// Returns error if model initialization fails
    pub async fn initialize(&self) -> Result<()> {
        self.model.initialize().await
    }

    /// Send a message and get a response
    /// # Errors
    /// Returns error if message sending fails
    pub async fn send_message(&self, message: &str) -> Result<String> {
        self.model.send_message(message).await
    }
}
