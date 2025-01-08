use hyperaxe::HttpClient;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Common message structure used across different LLM implementations
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelContent {
    pub content: String,
}

pub trait ModelMessage {
    fn get_content(&self) -> String;
}

impl ModelMessage for Message {
    fn get_content(&self) -> String {
        self.content.clone()
    }
}

impl ModelMessage for TextContent {
    fn get_content(&self) -> String {
        self.text.clone()
    }
}

/// Common error structure for API responses
#[derive(Deserialize, Debug)]
pub struct APIError {
    pub message: String,
    pub r#type: Option<String>,
    pub code: Option<String>,
}

/// Configuration options for model requests
#[derive(Serialize, Debug, Clone)]
pub struct ModelRequestOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

impl Default for ModelRequestOptions {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            max_tokens: Some(2048),
            stream: Some(false),
        }
    }
}

pub trait ModelClient: Send + Sync + Debug {
    type MessageType: ModelMessage;
    type ResponseType: Debug;

    /// Initialize a new instance of the model client
    fn new() -> Self
    where
        Self: Sized;

    /// Get the client's HTTP instance
    fn get_http_client(&self) -> &HttpClient;

    /// Get the model identifier
    fn get_model_id(&self) -> &str;

    /// Send a message to the model and get a response
    async fn send_message(&self, content: &str) -> Result<String> {
        self.send_message_with_options(content, ModelRequestOptions::default())
            .await
    }

    /// Send a message with custom options
    async fn send_message_with_options(
        &self,
        content: &str,
        options: ModelRequestOptions,
    ) -> Result<String>;

    fn extract_content(&self, response: &Self::ResponseType) -> Result<String>;

    /// Send a conversation (multiple messages) to the model
    async fn send_conversation(&self, messages: Vec<Message>) -> Result<String>;
}

/// Base implementation for model responses
#[derive(Deserialize, Debug)]
pub struct ModelResponse<T> {
    pub id: Option<String>,
    pub choices: Vec<T>,
    pub error: Option<APIError>,
}

/// Helper function to create a user message
pub fn create_user_message(content: &str) -> Message {
    Message {
        role: "user".to_string(),
        content: content.to_string(),
    }
}

/// Helper function to create an assistant message
pub fn create_assistant_message(content: &str) -> Message {
    Message {
        role: "assistant".to_string(),
        content: content.to_string(),
    }
}

/// Helper function to create a system message
pub fn create_system_message(content: &str) -> Message {
    Message {
        role: "system".to_string(),
        content: content.to_string(),
    }
}
