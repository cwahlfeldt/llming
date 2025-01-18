use crate::app::Message;
use conduit::{ClaudeModel, Conduit, StreamEvent};
use cosmic::iced::advanced::subscription::Recipe;
use cosmic::iced_futures::futures::stream::Stream;
use cosmic::iced_futures::subscription::Event as IcedEvent;
use futures_util::StreamExt;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone)]
pub struct StreamSubscription {
    pub conduit: Arc<Conduit>,
    pub prompt: String,
}

impl Recipe for StreamSubscription {
    type Output = Message;

    fn hash(&self, state: &mut cosmic::iced::advanced::graphics::futures::subscription::Hasher) {
        use std::hash::Hasher;
        self.prompt.hash(state);
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: Pin<Box<dyn Stream<Item = IcedEvent> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Message> + Send>> {
        let prompt = self.prompt.clone();
        let conduit = self.conduit.clone();

        Box::pin(async_stream::stream! {
            yield Message::StreamStarted;

            match conduit.stream_message(&prompt, ClaudeModel::Claude35Sonnet, 1024).await {
                Ok(mut stream) => {
                    let mut buffer = String::new();

                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(event) => {
                                eprintln!("Processing event: {:?}", event);
                                match event {
                                    StreamEvent::MessageStart { .. } => {
                                        eprintln!("Message started");
                                    }
                                    StreamEvent::ContentBlockStart(_) => {
                                        eprintln!("Content block started");
                                    }
                                    StreamEvent::ContentBlockDelta(delta) => {
                                        if !delta.delta.text.is_empty() {
                                            buffer.push_str(&delta.delta.text);
                                            yield Message::StreamUpdate(delta.delta.text);
                                        }
                                    }
                                    StreamEvent::ContentBlockStop(_) => {
                                        eprintln!("Content block complete: {}", buffer);
                                        buffer.clear();
                                    }
                                    StreamEvent::MessageStop => {
                                        eprintln!("Message complete");
                                        yield Message::StreamCompleted;
                                        break;
                                    }
                                    StreamEvent::Ping => {
                                        // Ignore ping events
                                    }
                                    _ => {
                                        eprintln!("Unhandled event: {:?}", event);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Stream error: {:?}", e);
                                yield Message::StreamError(e.to_string());
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create stream: {}", e);
                    yield Message::StreamError(e.to_string());
                }
            }
        })
    }
}
