// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use conduit::{ClaudeModel, Conduit, ConduitError, StreamEvent};
use cosmic::app::{Core, Task};
use cosmic::cosmic_theme;
use cosmic::iced::advanced::subscription::Recipe;
use cosmic::iced::futures::channel::mpsc;
use cosmic::iced::Color;
use cosmic::iced::{Length, Subscription};
use cosmic::iced_futures::futures::stream::Stream;
use cosmic::iced_futures::subscription::Event as IcedEvent;
use cosmic::theme;
use cosmic::theme::Theme;
use cosmic::widget::container::Style;
use cosmic::widget::{button, column, container, row, text, text_input};
use cosmic::{Apply, Element};
use futures_util::StreamExt;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;

pub struct AppModel {
    core: Core,
    config: Config,
    messages: Vec<ChatMessage>,
    input_value: String,
    conduit: Option<Arc<Conduit>>,
    stream_state: StreamState,
}

#[derive(Debug, Clone)]
struct ChatMessage {
    content: String,
    is_user: bool,
    is_streaming: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    SendMessage,
    UpdateConfig(Config),
    StreamStarted,
    StreamUpdate(String),
    StreamCompleted,
    StreamError(String),
}

#[derive(Debug)]
pub enum StreamState {
    Idle,
    Streaming,
    Error(String),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.waffles.ai-chat.app";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        let config = Config::default();
        let conduit = Conduit::new(config.anthropic.api_key.clone())
            .ok()
            .map(Arc::new);

        let app = AppModel {
            core,
            config,
            messages: Vec::new(),
            input_value: String::new(),
            conduit,
            stream_state: StreamState::Idle,
        };

        (app, Task::none())
    }
    fn subscription(&self) -> Subscription<Message> {
        match &self.stream_state {
            StreamState::Streaming => {
                if let Some(conduit) = &self.conduit {
                    // Only use the last user message
                    if let Some(last_user_msg) = self.messages.iter().rev().find(|msg| msg.is_user)
                    {
                        let prompt = last_user_msg.content.clone();
                        eprintln!("Creating subscription for message: '{}'", prompt);

                        struct StreamSubscription {
                            conduit: Arc<Conduit>,
                            prompt: String,
                        }

                        impl Recipe for StreamSubscription {
                            type Output = Message;

                            fn hash(
                                &self,
                                state: &mut cosmic::iced::advanced::graphics::futures::subscription::Hasher,
                            ) {
                                use std::hash::Hash;
                                // Include timestamp in hash to ensure unique subscriptions
                                (
                                    self.prompt.clone(),
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                )
                                    .hash(state);
                            }

                            fn stream(
                                self: Box<Self>,
                                _input: Pin<Box<dyn Stream<Item = IcedEvent> + Send>>,
                            ) -> Pin<Box<dyn Stream<Item = Message> + Send>>
                            {
                                Box::pin(async_stream::stream! {
                                    eprintln!("Starting stream for message: '{}'", self.prompt);
                                    match self.conduit.stream_message(&self.prompt, ClaudeModel::Claude35Sonnet, 1024).await {
                                        Ok(stream) => {
                                            let mut pinned = Box::pin(stream);
                                            let mut content_started = false;

                                            yield Message::StreamStarted;

                                            while let Some(event) = pinned.next().await {
                                                match event {
                                                    Ok(StreamEvent::ContentBlockDelta(content)) => {
                                                        if !content.delta.text.is_empty() {
                                                            content_started = true;
                                                            eprintln!("Got content: {}", content.delta.text);
                                                            yield Message::StreamUpdate(content.delta.text);
                                                        }
                                                    }
                                                    Ok(StreamEvent::MessageStop) => {
                                                        eprintln!("Stream complete");
                                                        if content_started {
                                                            yield Message::StreamCompleted;
                                                        } else {
                                                            // If no content was received, treat as an error
                                                            yield Message::StreamError("No content received".to_string());
                                                        }
                                                        break;
                                                    }
                                                    Ok(StreamEvent::MessageStart { .. }) => {
                                                        eprintln!("Message start received");
                                                    }
                                                    Ok(StreamEvent::ContentBlockStart(_)) => {
                                                        eprintln!("Content block start received");
                                                    }
                                                    Ok(StreamEvent::ContentBlockStop(_)) => {
                                                        eprintln!("Content block stop received");
                                                    }
                                                    Ok(_) => {
                                                        eprintln!("Other event type received");
                                                    }
                                                    Err(e) => {
                                                        eprintln!("Stream error: {}", e);
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

                        cosmic::iced::advanced::graphics::futures::subscription::from_recipe(
                            StreamSubscription {
                                conduit: Arc::clone(conduit),
                                prompt,
                            },
                        )
                    } else {
                        Subscription::none()
                    }
                } else {
                    Subscription::none()
                }
            }
            _ => Subscription::none(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(value) => {
                self.input_value = value;
            }
            Message::SendMessage => {
                let prompt = self.input_value.trim();
                if prompt.is_empty() {
                    self.stream_state = StreamState::Error("Cannot send empty message".to_string());
                } else if self.conduit.is_some() && matches!(self.stream_state, StreamState::Idle) {
                    // Only allow sending if we're in Idle state
                    eprintln!("Sending message: {}", prompt);

                    // Add user message
                    self.messages.push(ChatMessage {
                        content: prompt.to_string(),
                        is_user: true,
                        is_streaming: false,
                    });

                    // Add placeholder for assistant response
                    self.messages.push(ChatMessage {
                        content: String::new(),
                        is_user: false,
                        is_streaming: true,
                    });

                    // Set streaming state and clear input
                    self.stream_state = StreamState::Streaming;
                    self.input_value.clear();
                }
            }

            Message::StreamStarted => {
                if let Some(last) = self.messages.last_mut() {
                    if !last.is_user {
                        last.is_streaming = true;
                    }
                }
            }
            Message::StreamUpdate(content) => {
                if let Some(last) = self.messages.last_mut() {
                    if !last.is_user && last.is_streaming {
                        last.content.push_str(&content);
                    }
                }
            }
            Message::StreamCompleted => {
                eprintln!("Stream completed, resetting state");
                if let Some(last) = self.messages.last_mut() {
                    if !last.is_user {
                        last.is_streaming = false;
                    }
                }
                self.stream_state = StreamState::Idle;
            }

            Message::StreamError(error) => {
                eprintln!("Stream error: {}", error);
                if let Some(last) = self.messages.last_mut() {
                    if !last.is_user {
                        last.is_streaming = false;
                        last.content = format!("[Error: {}]", error);
                    }
                }
                self.stream_state = StreamState::Idle;
            }
            Message::UpdateConfig(config) => {
                self.config = config;
                // Recreate conduit with new config
                self.conduit = Conduit::new(self.config.anthropic.api_key.clone())
                    .ok()
                    .map(Arc::new);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let cosmic_theme::Spacing {
            space_xxs,
            space_m,
            space_l,
            ..
        } = theme::active().cosmic().spacing;

        // Build message list
        let messages = self.messages.iter().fold(
            column::with_capacity(self.messages.len())
                .spacing(space_l)
                .padding(space_m),
            |column, message| {
                let message_text = if message.is_streaming {
                    let mut content = message.content.clone();
                    content.push('▋'); // Add cursor for streaming messages
                    text::body(content)
                } else {
                    text::body(&message.content)
                };

                let message_container =
                    container::Container::new(message_text).style(|_theme: &Theme| Style {
                        text_color: if message.is_streaming {
                            Some(Color::new(0.5, 0.5, 0.5, 1.0))
                        } else {
                            None
                        },
                        ..Style::default()
                    });

                column.push(
                    container::Container::new(message_container)
                        .width(Length::Fill)
                        .align_x(if message.is_user {
                            cosmic::iced::alignment::Horizontal::Right
                        } else {
                            cosmic::iced::alignment::Horizontal::Left
                        }),
                )
            },
        );

        // Disable the send button while streaming
        let send_button = if matches!(self.stream_state, StreamState::Streaming) {
            button::custom("Send").class(theme::Button::Text)
        } else {
            button::custom("Send")
                .class(theme::Button::Text)
                .on_press(Message::SendMessage)
        };

        // Input row with text input and send button
        let input = row::with_capacity(2)
            .spacing(space_xxs)
            .push(
                text_input::text_input("Type a message...", &self.input_value)
                    .on_input(Message::InputChanged)
                    .on_submit(Message::SendMessage)
                    .padding(space_m)
                    .width(Length::Fill),
            )
            .push(send_button);

        // Input row with text input and send button
        // let input = row::with_capacity(2)
        //     .spacing(space_xxs)
        //     .push(
        //         text_input::text_input("Type a message...", &self.input_value)
        //             .on_input(Message::InputChanged)
        //             .on_submit(Message::SendMessage)
        //             .padding(space_m)
        //             .width(Length::Fill),
        //     )
        //     .push(
        //         button::custom("Send")
        //             .class(theme::Button::Text)
        //             .on_press(Message::SendMessage),
        //     );

        // Main layout
        let content = column::with_capacity(2)
            .push(cosmic::iced_widget::Scrollable::new(messages).height(Length::Fill))
            .push(
                container::Container::new(input)
                    .padding(space_m)
                    .width(Length::Fill),
            )
            .spacing(space_xxs)
            .width(Length::Fill)
            .height(Length::Fill);

        container::Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .apply(Element::from)
    }
}
