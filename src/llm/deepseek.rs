use crate::http::HttpClient;
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

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
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
