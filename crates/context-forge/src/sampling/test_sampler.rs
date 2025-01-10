use std::pin::Pin;
use std::task::{Context, Poll};
use async_trait::async_trait;
use futures::Stream;
use mcp_schema::{SamplingMessage, SamplingContent, TextContent};

use super::{Sampler, ModelInfo, PendingResponse, StopReason};
use crate::Result;

/// A test sampler that returns predefined responses
pub struct TestSampler {
    models: Vec<ModelInfo>,
}

impl TestSampler {
    pub fn new() -> Self {
        Self {
            models: vec![
                ModelInfo {
                    name: "test-fast".to_string(),
                    cost: 0.2,
                    speed: 0.9,
                    intelligence: 0.3,
                    max_tokens: 1000,
                },
                ModelInfo {
                    name: "test-smart".to_string(),
                    cost: 0.8,
                    speed: 0.4,
                    intelligence: 0.9,
                    max_tokens: 4000,
                },
            ],
        }
    }
}

#[async_trait]
impl Sampler for TestSampler {
    fn list_models(&self, _preferences: Option<&ModelPreferences>) -> Vec<ModelInfo> {
        self.models.clone()
    }

    fn select_model(&self, preferences: Option<&ModelPreferences>) -> Result<String> {
        select_model_by_preferences(&self.models, preferences)
    }

    async fn sample<'a>(
        &'a self,
        model: &str,
        _messages: &'a [SamplingMessage],
        _max_tokens: i64,
        _temperature: Option<f64>,
        _stop_sequences: Option<&'a [String]>,
    ) -> Result<Box<dyn Stream<Item = Result<PendingResponse>> + Send + 'a>> {
        Ok(Box::new(TestStream::new(model.to_string())))
    }
}

/// A test stream that yields a few tokens then completes
struct TestStream {
    model: String,
    tokens: Vec<String>,
    current: usize,
}

impl TestStream {
    fn new(model: String) -> Self {
        Self {
            model,
            tokens: vec![
                "Hello".to_string(),
                ", ".to_string(),
                "world".to_string(),
                "!".to_string(),
            ],
            current: 0,
        }
    }
}

impl Stream for TestStream {
    type Item = Result<PendingResponse>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.current >= self.tokens.len() {
            return Poll::Ready(None);
        }

        let is_final = self.current == self.tokens.len() - 1;
        let mut text = String::new();
        for i in 0..=self.current {
            text.push_str(&self.tokens[i]);
        }

        let response = PendingResponse {
            model: self.model.clone(),
            content: SamplingContent::Text(TextContent {
                type_: "text".to_string(),
                text,
                annotated: Default::default(),
            }),
            is_final,
            stop_reason: if is_final {
                Some(StopReason::EndTurn)
            } else {
                None
            },
        };

        self.current += 1;
        Poll::Ready(Some(Ok(response)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use mcp_schema::ModelPreferences;

    #[test]
    fn test_model_selection() {
        let sampler = TestSampler::new();

        // Test selecting fast model
        let preferences = ModelPreferences {
            hints: None,
            cost_priority: Some(0.8),
            speed_priority: Some(0.9),
            intelligence_priority: Some(0.1),
            extra: Default::default(),
        };

        let model = sampler.select_model(Some(&preferences)).unwrap();
        assert_eq!(model, "test-fast");

        // Test selecting smart model
        let preferences = ModelPreferences {
            hints: None,
            cost_priority: Some(0.2),
            speed_priority: Some(0.1),
            intelligence_priority: Some(0.9),
            extra: Default::default(),
        };

        let model = sampler.select_model(Some(&preferences)).unwrap();
        assert_eq!(model, "test-smart");
    }

    #[tokio::test]
    async fn test_sampling() {
        let sampler = TestSampler::new();
        
        let mut stream = sampler.sample(
            "test-model",
            &[],
            100,
            None,
            None,
        ).await.unwrap();

        let mut responses = Vec::new();
        while let Some(response) = stream.next().await {
            responses.push(response.unwrap());
        }

        assert_eq!(responses.len(), 4);
        assert!(responses[3].is_final);
        assert_eq!(responses[3].stop_reason, Some(StopReason::EndTurn));
        
        if let SamplingContent::Text(content) = &responses[3].content {
            assert_eq!(content.text, "Hello, world!");
        } else {
            panic!("Expected text content");
        }
    }
}
