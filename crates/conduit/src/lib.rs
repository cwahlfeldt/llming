// Re-export types from mesh that we use publicly
use futures_util::StreamExt;
pub use mesh::anthropic::{
    client::Client,
    completion::{
        message::{Content, ContentType, Message, MessageRequest, Role},
        stream::StreamEvent,
    },
    config::Config,
    error::AnthropicError,
    models::claude::ClaudeModel,
};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ConduitError {
    ApiError(AnthropicError),
    EmptyResponse,
}

impl std::fmt::Display for ConduitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConduitError::ApiError(e) => write!(f, "API error: {}", e),
            ConduitError::EmptyResponse => write!(f, "Empty response from API"),
        }
    }
}

impl Error for ConduitError {}

impl From<AnthropicError> for ConduitError {
    fn from(error: AnthropicError) -> Self {
        ConduitError::ApiError(error)
    }
}

pub struct Conduit {
    client: Client,
    config: Config,
}

impl Clone for Conduit {
    fn clone(&self) -> Self {
        // Create a new instance with the same config
        Self::new(self.config.api_key.to_string()).unwrap()
    }
}

impl fmt::Debug for Conduit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Conduit").finish()
    }
}

impl Conduit {
    /// Creates a new Conduit instance with the provided API key
    pub fn new(api_key: impl Into<String>) -> Result<Self, ConduitError> {
        let config = Config::new(api_key.into());
        let client = Client::new(config.clone()).map_err(ConduitError::from)?;
        Ok(Self { client, config })
    }

    /// Sends a message to Claude and returns the response
    pub async fn send_message(
        &self,
        prompt: impl Into<String>,
        model: ClaudeModel,
        max_tokens: u32,
    ) -> Result<String, ConduitError> {
        unsafe {
            // Create a safe message structure
            let content = Content {
                content_type: ContentType::Text,
                text: prompt.into(),
            };

            let message = Message {
                role: Role::User,
                content: vec![content],
            };

            // Create a safe request structure
            let mut request = MessageRequest::default();
            request.model = model;
            request.max_tokens = max_tokens;
            request.messages = vec![message];

            // Send the request and handle the response
            let response = self.client.create_message(request).await?;

            // Safely extract the response text
            response
                .content
                .get(0)
                .ok_or(ConduitError::EmptyResponse)
                .map(|c| c.text.clone())
        }
    }

    /// Streams a message from Claude and returns a stream of response chunks
    pub async fn stream_message(
        &self,
        prompt: impl Into<String>,
        model: ClaudeModel,
        max_tokens: u32,
    ) -> Result<impl StreamExt<Item = Result<StreamEvent, AnthropicError>>, ConduitError> {
        unsafe {
            // Create message structure
            let content = Content {
                content_type: ContentType::Text,
                text: prompt.into(),
            };

            let message = Message {
                role: Role::User,
                content: vec![content],
            };

            // Create request structure
            let mut request = MessageRequest::default();
            request.model = model;
            request.max_tokens = max_tokens;
            request.stream = true;
            request.messages = vec![message];

            eprintln!("Conduit - Sending stream request with prompt: {:?}", request);

            // Send the streaming request
            eprintln!("Conduit - About to send request to API");
            let raw_stream = self.client.stream_message(request).await?;
            eprintln!("Conduit - Got raw stream from API");

            // Convert the raw stream into a mapped stream with explicit event handling
            use futures_util::{Stream, StreamExt, TryStreamExt};
            use std::pin::Pin;
            use std::task::{Context, Poll};

            struct TrackedStream<S> {
                inner: S,
            }

            impl<S: Stream + Unpin> Stream for TrackedStream<S> {
                type Item = S::Item;

                fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                    eprintln!("Conduit - Polling stream");
                    match self.inner.poll_next_unpin(cx) {
                        Poll::Ready(Some(item)) => {
                            eprintln!("Conduit - Got stream item");
                            Poll::Ready(Some(item))
                        }
                        Poll::Ready(None) => {
                            eprintln!("Conduit - Stream ended");
                            Poll::Ready(None)
                        }
                        Poll::Pending => {
                            eprintln!("Conduit - Stream pending");
                            Poll::Pending
                        }
                    }
                }
            }

            let tracked = TrackedStream { inner: raw_stream };
            Ok(Box::pin(tracked))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_message() {
        let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
        let conduit = Conduit::new(api_key).expect("Failed to create Conduit instance");

        let prompt = "Say hello";
        let result = conduit
            .send_message(prompt, ClaudeModel::Claude35Sonnet, 1024)
            .await
            .expect("Failed to send message");

        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_stream_message() {
        let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY must be set");
        let conduit = Conduit::new(api_key).expect("Failed to create Conduit instance");

        let mut total_text = String::new();
        let prompt = "Count from 1 to 5";
        let mut stream = conduit
            .stream_message(prompt, ClaudeModel::Claude35Sonnet, 1024)
            .await
            .expect("Failed to create stream");

        while let Some(event) = stream.next().await {
            match event {
                Ok(StreamEvent::ContentBlockDelta(content)) => {
                    total_text.push_str(&content.delta.text);
                }
                Ok(StreamEvent::MessageStop) => break,
                Ok(_) => {} // Handle other successful events gracefully
                Err(e) => {
                    println!("Stream event error: {}", e);
                    continue; // Skip invalid events and continue streaming
                }
            }
        }

        assert!(!total_text.is_empty());
        assert!(total_text.contains("1"));
        assert!(total_text.contains("5"));
    }
}
