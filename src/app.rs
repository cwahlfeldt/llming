// SPDX-License-Identifier: MPL-2.0

use crate::config::Config;
use crate::llm::LLM;
use cosmic::app::{Core, Task};
use cosmic::cosmic_theme;
use cosmic::iced::theme::Palette;
use cosmic::iced::{Length, Subscription};
use cosmic::theme;
use cosmic::widget::markdown;
use cosmic::widget::{button, card, column, container, icon, row, text, text_input};
use cosmic::Apply;
use cosmic::Element;
use cosmic::Theme;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Messages {
    value: Vec<ChatMessage>,
}

pub struct AppModel {
    core: Core,
    config: Config,
    messages: Messages,
    input_value: String,
    llm_fs: Option<LLM>,
    parsed_messages: Vec<Vec<markdown::Item>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    content: String,
    is_user: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    SendMessage,
    UpdateConfig(Config),
    LLMFSInitialized(LLM),
    ReceivedResponse(String),
    Error(String),
    LinkClicked(markdown::Url),
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
            messages: Messages::default(),
            input_value: String::new(),
            llm_fs: None,
            parsed_messages: Vec::new(),
        };

        let init_task = Task::future(async move {
            let addr: SocketAddr = "[::1]:3456".parse().unwrap();
            let allowed_paths = vec![
                "/home/waffles".to_string(),
                "/home/waffles/code".to_string(),
            ];

            match LLM::new(addr, allowed_paths).await {
                Ok(llm_fs) => cosmic::app::Message::App(Message::LLMFSInitialized(llm_fs)),
                Err(e) => cosmic::app::Message::App(Message::Error(e.to_string())),
            }
        });

        (app, init_task)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(value) => {
                self.input_value = value;
                Task::none()
            }
            Message::SendMessage => {
                if !self.input_value.trim().is_empty() {
                    let message = ChatMessage {
                        content: self.input_value.to_owned(),
                        is_user: true,
                    };
                    self.parsed_messages
                        .push(markdown::parse(&message.content).collect());

                    self.messages.value.push(message);

                    if let Some(llm_fs) = self.llm_fs.to_owned() {
                        let input = serde_json::to_string(&self.messages.value).unwrap();
                        self.input_value.clear();

                        return Task::future(async move {
                            match llm_fs.chat(&input).await {
                                Ok(response) => {
                                    cosmic::app::Message::App(Message::ReceivedResponse(response))
                                }
                                Err(e) => cosmic::app::Message::App(Message::Error(e.to_string())),
                            }
                        });
                    }
                }
                Task::none()
            }
            Message::LLMFSInitialized(llm_fs) => {
                self.llm_fs = Some(llm_fs);
                Task::none()
            }
            Message::ReceivedResponse(response) => {
                self.parsed_messages
                    .push(markdown::parse(&response).collect());

                self.messages.value.push(ChatMessage {
                    content: response,
                    is_user: false,
                });
                Task::none()
            }
            Message::Error(error) => {
                self.messages.value.push(ChatMessage {
                    content: format!("Error: {}", error),
                    is_user: false,
                });
                Task::none()
            }
            Message::UpdateConfig(config) => {
                self.config = config;
                Task::none()
            }
            Message::LinkClicked(url) => {
                println!("Link clicked: {}", url);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let cosmic_theme::Spacing {
            space_xxs,
            space_m,
            space_l,
            ..
        } = theme::active().cosmic().spacing;

        let messages = self.messages.value.iter().enumerate().fold(
            column::with_capacity(self.messages.value.len())
                .spacing(space_l)
                .padding(space_m),
            |column, (index, message)| {
                let message_content = container(
                    markdown::view(
                        &self.parsed_messages[index],
                        markdown::Settings::default(),
                        markdown::Style::from_palette(Palette::CATPPUCCIN_MOCHA),
                    )
                    .map(Message::LinkClicked),
                )
                .padding(10)
                .width(Length::Fill);

                column.push(
                    container::Container::new(message_content)
                        .width(Length::Fill)
                        .align_x(if message.is_user {
                            cosmic::iced::alignment::Alignment::End
                        } else {
                            cosmic::iced::alignment::Alignment::Start
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
            .push(button::standard("Send").on_press(Message::SendMessage));

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
