use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub message_type: String,
    pub content: serde_json::Value,
}

impl ProtocolMessage {
    #[must_use]
    pub fn new(message_type: &str, content: serde_json::Value) -> Self {
        Self {
            message_type: message_type.to_string(),
            content,
        }
    }

    /// Encodes the message into bytes
    /// # Errors
    /// Returns error if serialization fails
    pub fn encode(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self)?)
    }

    /// Decodes bytes into a message
    /// # Errors
    /// Returns error if deserialization fails
    pub fn decode(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }
}