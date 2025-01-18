// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use conduit::{Conduit, StreamEvent};
use cosmic::app::{Core, Task};
use cosmic::cosmic_theme;
use cosmic::iced::Color;
use cosmic::iced::{Length, Subscription};
use cosmic::theme;
use cosmic::theme::Theme;
use cosmic::widget::container::Style;
use cosmic::widget::{button, column, container, row, text, text_input};
use cosmic::{Apply, Element};
use std::sync::Arc;

mod stream_subscription;
use stream_subscription::StreamSubscription;

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

pub struct AppModel {
    core: Core,
    config: Config,
    messages: Vec<ChatMessage>,
    input_value: String,
    conduit: Option<Arc<Conduit>>,
    stream_state: StreamState,
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
                    // Use the last user message as the prompt
                    let prompt = self.messages
                        .iter()
                        .rev()
                        .find(|msg| msg.is_user)
                        .map(|msg| msg.content.clone())
                        .unwrap_or_default();

                    if prompt.is_empty() {
                        return Subscription::none();
                    }

                    let subscription = StreamSubscription {
                        conduit: Arc::clone(conduit),
                        prompt,
                    };

                    cosmic::iced::advanced::graphics::futures::subscription::from_recipe(subscription)
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
                } else if self.conduit.is_some() {
                    // Add user message
                    self.messages.push(ChatMessage {
                        content: self.input_value.clone(),
                        is_user: true,
                        is_streaming: false,
                    });

                    // Add placeholder for assistant response
                    self.messages.push(ChatMessage {
                        content: String::new(),
                        is_user: false,
                        is_streaming: true,
                    });

                    // Set streaming state and clear input immediately
                    self.stream_state = StreamState::Streaming;
                    self.input_value.clear();
                } else {
                    self.stream_state = StreamState::Error("API connection not available".to_string());
                }
            }
            Message::StreamStarted => {
                if let Some(last) = self.messages.last_mut() {
                    if !last.is_user {
                        last.is_streaming = true;
                        last.content.clear(); // Ensure we start with empty content
                    }
                }
            }
            Message::StreamUpdate(content) => {
                if let Some(last) = self.messages.last_mut() {
                    if !last.is_user && last.is_streaming {
                        last.content.push_str(&content);
                        // Force a redraw by cloning the message
                        let updated_message = last.clone();
                        *last = updated_message;
                    }
                }
            }
            Message::StreamCompleted => {
                // Stream completed
                if let Some(last) = self.messages.last_mut() {
                    if !last.is_user {
                        last.is_streaming = false;
                    }
                }
                self.stream_state = StreamState::Idle;
            }
            Message::StreamError(error) => {
                // Handle stream error
                eprintln!("Stream error: {}", error);
                self.stream_state = StreamState::Error(error);
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
            .push(
                button::custom("Send")
                    .class(theme::Button::Text)
                    .on_press(Message::SendMessage),
            );

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