use super::types::{ChatMessage, ChatResponse, Message};
use crate::error::{AssistantError, Result};
use crate::models::Model;
use async_trait::async_trait;
use hyperax::client::Client;
use hyperax::common::Body;

#[allow(clippy::module_name_repetitions)]
pub struct AnthropicClient {
    client: Client,
    model: String,
}

impl AnthropicClient {
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        let client = Client::builder()
            .base_url("https://api.anthropic.com/v1/")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .build();

        Self { client, model }
    }

    pub async fn send_message(&self, message: &str) -> Result<String> {
        let chat_message = ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
        };

        let payload = Message {
            model: self.model.clone(),
            messages: vec![chat_message],
            system: None,
            max_tokens: None,
            temperature: Some(0.7),
        };

        let body = Body::new(serde_json::to_vec(&payload)?);
        let response = self.client.post("messages", body).await?;

        if response.status() != 200 {
            return Err(AssistantError::Provider(format!(
                "Anthropic API error: {}",
                response.status()
            )));
        }

        let bytes = response.into_body();
        let chat_response: ChatResponse = serde_json::from_slice(&bytes)?;

        if chat_response.content.is_empty() {
            return Err(AssistantError::Provider(
                "Empty response from Anthropic API".to_string(),
            ));
        }

        Ok(chat_response.content[0].text.clone())
    }
}

#[async_trait]
impl Model for AnthropicClient {
    async fn initialize(&self) -> Result<()> {
        let test_msg = Message {
            model: self.model.clone(),
            messages: vec![],
            system: None,
            max_tokens: Some(1),
            temperature: Some(0.0),
        };

        let body = Body::new(serde_json::to_vec(&test_msg)?);
        let response = self.client.post("messages", body).await?;

        if response.status() != 200 {
            return Err(AssistantError::Auth(
                "Failed to authenticate with Anthropic API".to_string(),
            ));
        }

        Ok(())
    }

    async fn send_message(&self, message: &str) -> Result<String> {
        self.send_message(message).await
    }

    fn id(&self) -> &str {
        "anthropic"
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}
