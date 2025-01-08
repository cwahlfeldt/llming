use super::model::{ModelClient, ModelRequestOptions};
// use hyperaxe::HttpClient;
use hyperaxe::HttpClient;
use anyhow::Result;
use hyper::Method;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{debug, error, info};

#[derive(Clone, Default, Debug)]
pub struct DeepSeekClient {
    client: HttpClient,
    api_key: String,
}

#[derive(Serialize, Debug)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    stream: Option<bool>,
}

use super::model::Message;

#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    id: Option<String>,
    choices: Vec<Choice>,
    error: Option<APIError>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct APIError {
    message: String,
    r#type: Option<String>,
    code: Option<String>,
}

impl DeepSeekClient {
    pub fn new() -> Self {
        let api_key = env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set");
        info!("Initializing DeepSeek client");

        let client = HttpClient::new()
            .with_header("Authorization", &format!("Bearer {}", api_key))
            .with_header("Content-Type", "application/json");

        Self { client, api_key }
    }

    pub async fn send_message(&self, content: &str) -> Result<String> {
        debug!("Sending message to DeepSeek API: {}", content);

        let request = ChatRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: content.to_string(),
            }],
            max_tokens: Some(2048),
            temperature: Some(0.7),
            stream: Some(false),
        };

        debug!("Request payload: {:?}", request);

        match self
            .client
            .send_request::<ChatRequest, ChatResponse>(
                Method::POST,
                "https://api.deepseek.com/v1/chat/completions",
                Some(request),
            )
            .await
        {
            Ok(response) => {
                if let Some(error) = response.error {
                    error!("DeepSeek API error: {:?}", error);
                    return Err(anyhow::anyhow!(
                        "API Error: {} (type: {:?}, code: {:?})",
                        error.message,
                        error.r#type,
                        error.code
                    ));
                }

                if response.choices.is_empty() {
                    error!("DeepSeek API returned no choices");
                    return Err(anyhow::anyhow!("No response from model"));
                }

                let content = response.choices[0].message.content.clone();
                debug!("Received response from DeepSeek: {}", content);
                Ok(content)
            }
            Err(e) => {
                error!("Failed to send message to DeepSeek: {}", e);
                Err(anyhow::anyhow!("Failed to send message: {}", e))
            }
        }
    }
}

impl ModelClient for DeepSeekClient {
    type MessageType = Message;
    type ResponseType = ChatResponse;

    fn new() -> Self {
        Self::new()
    }

    fn get_http_client(&self) -> &HttpClient {
        &self.client
    }

    fn get_model_id(&self) -> &str {
        "deepseek-chat"
    }

    async fn send_message_with_options(
        &self,
        content: &str,
        options: ModelRequestOptions,
    ) -> Result<String> {
        let request = ChatRequest {
            model: self.get_model_id().to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: content.to_string(),
            }],
            max_tokens: options.max_tokens,
            temperature: options.temperature,
            stream: Some(false),
        };

        self.send_message(content).await
    }

    async fn send_conversation(&self, messages: Vec<Message>) -> Result<String> {
        let request = ChatRequest {
            model: self.get_model_id().to_string(),
            messages,
            max_tokens: Some(2048),
            temperature: Some(0.7),
            stream: Some(false),
        };

        match self
            .client
            .send_request::<ChatRequest, ChatResponse>(
                Method::POST,
                "https://api.deepseek.com/v1/chat/completions",
                Some(request),
            )
            .await
        {
            Ok(response) => self.extract_content(&response),
            Err(e) => Err(anyhow::anyhow!("Failed to send conversation: {}", e)),
        }
    }

    fn extract_content(&self, response: &ChatResponse) -> Result<String> {
        response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("No content in response"))
    }
}
