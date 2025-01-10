use std::sync::Arc;
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use mcp_schema::{ServerResult, CreateMessageResult};

use crate::{Error, Result};
use crate::handler::RequestHandler;
use super::{Sampler, StopReason};

/// Handler for sampling-related requests
pub struct SamplingHandler<S: Sampler> {
    sampler: Arc<S>,
}

impl<S: Sampler> SamplingHandler<S> {
    /// Create a new sampling handler with the given sampler
    pub fn new(sampler: S) -> Self {
        Self {
            sampler: Arc::new(sampler),
        }
    }
}

#[async_trait]
impl<S: Sampler> RequestHandler for SamplingHandler<S> {
    async fn handle(&self, params: Value) -> Result<ServerResult> {
        // Parse parameters
        let params: CreateMessageParams = serde_json::from_value(params)
            .map_err(|e| Error::InvalidParams(e.to_string()))?;

        // Select model based on preferences
        let model = self.sampler.select_model(params.model_preferences.as_ref())?;

        // Start sampling
        let mut stream = self.sampler.sample(
            &model,
            &params.messages,
            params.max_tokens,
            params.temperature,
            params.stop_sequences.as_deref(),
        ).await?;

        // Process stream until we get final response
        let mut final_response = None;
        while let Some(response) = stream.next().await {
            let response = response?;
            if response.is_final {
                final_response = Some(response);
                break;
            }
        }

        // Convert final response to result
        let response = final_response.ok_or_else(|| Error::Internal("no final response".into()))?;

        Ok(ServerResult::CreateMessage(CreateMessageResult {
            meta: None,
            role: Role::Assistant,
            content: response.content,
            model: response.model,
            stop_reason: Some(response.stop_reason.unwrap_or(StopReason::EndTurn).into()),
            extra: Default::default(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::TestSampler;
    use mcp_schema::{Role, SamplingMessage, TextContent};

    #[tokio::test]
    async fn test_sampling_handler() {
        let sampler = TestSampler::new();
        let handler = SamplingHandler::new(sampler);

        let params = serde_json::json!({
            "messages": [{
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Hello"
                }
            }],
            "maxTokens": 100
        });

        let result = handler.handle(params).await.unwrap();
        
        match result {
            ServerResult::CreateMessage(result) => {
                assert_eq!(result.role, Role::Assistant);
                if let SamplingContent::Text(content) = result.content {
                    assert_eq!(content.text, "Hello, world!");
                } else {
                    panic!("Expected text content");
                }
            }
            _ => panic!("Expected CreateMessage result"),
        }
    }
}
