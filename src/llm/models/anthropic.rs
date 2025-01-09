use super::model::{APIError, Message, ModelClient, ModelRequestOptions, TextContent};
use anyhow::Result;
use hyper::Method;
use hyperax::HttpClient;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{debug, error, info};

#[derive(Clone, Debug)]
pub struct AnthropicClient {
    client: HttpClient,
    api_key: String,
    model_id: String,
}

#[derive(Serialize, Debug, Clone)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: Option<u32>,
    system: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    id: String,
    content: Vec<Content>,
    model: String,
    role: String,
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    error: Option<APIError>,
}

#[derive(Deserialize, Debug)]
pub struct Content {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Deserialize, Debug)]
pub struct Choice {
    index: u32,
    message: Message,
    finish_reason: Option<String>,
}

impl ModelClient for AnthropicClient {
    type MessageType = TextContent;
    type ResponseType = ChatResponse;

    fn new() -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
        info!("Initializing Anthropic client");

        let client = HttpClient::new()
            .with_header("x-api-key", &api_key)
            .with_header("anthropic-version", "2023-06-01");

        Self {
            client,
            api_key,
            model_id: "claude-3-5-sonnet-latest".to_string(),
        }
    }

    fn get_http_client(&self) -> &HttpClient {
        &self.client
    }

    fn get_model_id(&self) -> &str {
        &self.model_id
    }

    async fn send_message(&self, content: &str) -> Result<String> {
        let response: ChatResponse = self
            .client
            .send_request(
                Method::POST,
                "https://api.anthropic.com/v1/messages",
                Some(ChatRequest {
                    model: self.model_id.clone(),
                    messages: vec![Message {
                        role: "user".to_string(),
                        content: content.to_string(),
                    }],
                    max_tokens: Some(2048),
                    system: Some("You are a helpful AI assistant.".to_string()),
                }),
            )
            .await?;

        debug!("Raw API response: {:?}", response);

        // Try content first, fall back to choices
        if !response.content.is_empty() {
            Ok(response.content[0].text.clone())
        } else if !response.choices.is_empty() {
            Ok(response.choices[0].message.content.clone())
        } else {
            Err(anyhow::anyhow!("No content in response"))
        }
    }

    async fn send_message_with_options(
        &self,
        content: &str,
        options: ModelRequestOptions,
    ) -> Result<String> {
        debug!("Sending message to Anthropic: {}", content);

        let request = ChatRequest {
            model: self.model_id.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: content.to_string(),
            }],
            max_tokens: options.max_tokens,
            system: Some("You are a helpful AI assistant.".to_string()),
        };

        debug!("Request payload: {:?}", request);

        let response: ChatResponse = self
            .client
            .send_request(
                Method::POST,
                "https://api.anthropic.com/v1/messages",
                Some(request),
            )
            .await?;

        debug!("Raw response: {:?}", response);

        if let Some(error) = response.error {
            error!("Anthropic API error: {:?}", error);
            return Err(anyhow::anyhow!(
                "API Error: {} (type: {:?}, code: {:?})",
                error.message,
                error.r#type,
                error.code
            ));
        }

        if response.choices.is_empty() {
            error!("Anthropic API returned no choices");
            return Err(anyhow::anyhow!("No response from model"));
        }

        let content = response.choices[0].message.content.clone();
        let text = &content;
        debug!("Extracted content: {}", content);
        Ok(text.to_owned())
    }

    async fn send_conversation(&self, messages: Vec<Message>) -> Result<String> {
        let request = ChatRequest {
            model: self.model_id.clone(),
            messages,
            max_tokens: Some(2048),
            system: Some("You are a helpful AI assistant.".to_string()),
        };

        let response: ChatResponse = self
            .client
            .send_request(
                Method::POST,
                "https://api.anthropic.com/v1/messages",
                Some(request),
            )
            .await?;

        if response.choices.is_empty() {
            return Err(anyhow::anyhow!("No response from model"));
        }

        let text = response.choices[0].message.content.clone();

        Ok(text)
    }

    fn extract_content(&self, response: &ChatResponse) -> Result<String> {
        if !response.content.is_empty() {
            Ok(response.content[0].text.clone())
        } else {
            Err(anyhow::anyhow!("No content in response"))
        }
    }
}
