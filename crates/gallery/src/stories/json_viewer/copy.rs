use iced::widget::{column, container, text};
use iced::{Element, Length, Task};
use iced_cube::context_menu;
use iced_cube::data::json_viewer::{self, Event, MenuItem, NodeId, Output, State, json_viewer};
use iced_cube::dropdown_menu;
use iced_cube::layout::card;
use iced_cube::theme::{Tokens, text_size};
use serde_json::json;

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Viewer(Event),
    Menu(context_menu::Event<MenuItem, NodeId>),
}

#[derive(Debug)]
pub struct Example {
    config: State,
    menu: context_menu::State<MenuItem, NodeId>,
    copied: String,
}

impl Default for Example {
    fn default() -> Self {
        let mut config = State::new(json!({
            "service": "billing-api",
            "replicas": 3,
            "env": { "RUST_LOG": "info", "DATABASE_URL": "postgres://billing@db:5432/billing" },
            "ports": [8080, 9090]
        }))
        .with_depth(1);
        if let Some(&env) = config.roots().get(2) {
            let _ = config.update(Event::Press(env));
        }

        Self {
            config,
            menu: json_viewer::menu(),
            copied: "Nothing copied yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Viewer(event) => self.viewer(event),
            Message::Menu(event) => {
                let chosen = self.menu.update(event);
                match (chosen, self.menu.target()) {
                    (Some(dropdown_menu::Output::Activated(item)), Some(&id)) => {
                        self.viewer(item.event(id))
                    }
                    _ => Task::none(),
                }
            }
        }
    }

    fn viewer(&mut self, event: Event) -> Task<Message> {
        match self.config.update(event) {
            Some(Output::CopyValue(text) | Output::CopyPath(text)) => {
                self.copied = text.clone();
                bridge::copy(text)
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let clipboard =
            container(
                text(&self.copied)
                    .size(text_size::XS)
                    .style(|theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
            )
            .padding(12)
            .width(Length::Fill)
            .height(120)
            .clip(true)
            .style(|theme| card::style(&Tokens::of(theme)));

        column![
            json_viewer(&self.config)
                .height(224)
                .context_menu(&self.menu, Message::Menu)
                .on_event(Message::Viewer),
            text("Clipboard").size(text_size::SM),
            clipboard,
        ]
        .spacing(8)
        .width(Length::Fill)
        .max_width(560)
        .into()
    }
}
