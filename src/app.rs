// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use cosmic::app::{Core, Task};
use cosmic::cosmic_theme;
use cosmic::iced::{Length, Subscription};
use cosmic::theme;
use cosmic::widget::{button, column, container, row, text, text_input};
use cosmic::Apply;
use cosmic::Element;

pub struct AppModel {
    core: Core,
    config: Config,
    messages: Vec<ChatMessage>,
    input_value: String,
}

#[derive(Debug, Clone)]
struct ChatMessage {
    content: String,
    is_user: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    SendMessage,
    UpdateConfig(Config),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.waffles.llming.app";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        let app = AppModel {
            core,
            config: Config::default(),
            messages: Vec::new(),
            input_value: String::new(),
        };

        (app, Task::none())
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(value) => {
                self.input_value = value;
            }
            Message::SendMessage => {
                if !self.input_value.trim().is_empty() {
                    self.messages.push(ChatMessage {
                        content: self.input_value.clone(),
                        is_user: true,
                    });
                    // Simple echo response
                    self.messages.push(ChatMessage {
                        content: format!("You said: {}", self.input_value),
                        is_user: false,
                    });
                    self.input_value.clear();
                }
            }
            Message::UpdateConfig(config) => {
                self.config = config;
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
                let message_text = text::body(&message.content);

                let message_container = container::Container::new(message_text);

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
