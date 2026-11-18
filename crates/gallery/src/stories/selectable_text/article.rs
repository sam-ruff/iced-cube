use iced::widget::{column, span, text};
use iced::{Element, Length, Task, Theme};
use iced_cube::data::selectable_text::{selectable_text, selection_area};
use iced_cube::theme::{self, Tokens};

use crate::bridge;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Guide,
    Changelog,
}

#[derive(Debug, Clone)]
pub enum Message {
    Copy(String),
    Open(Link),
}

#[derive(Debug, Default)]
pub struct Example {
    status: Option<String>,
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Copy(text) => {
                self.status = Some(format!("Copied {} characters", text.chars().count()));
                bridge::copy(text)
            }
            Message::Open(link) => {
                self.status = Some(format!("Opened the {link:?} link"));
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let link = |label: &'static str, target: Link| {
            span(label)
                .link(target)
                .font(theme::semibold())
                .underline(true)
        };

        // One area around every paragraph, so a drag can run from the
        // heading into the text below it.
        let article = selection_area(
            column![
                selectable_text::<Link, _>(vec![span("Getting started")])
                    .size(20)
                    .font(theme::semibold()),
                selectable_text(vec![
                    span("Drag across this text to select it, double click a word or "),
                    span("triple click a paragraph, then press Ctrl+C. Read the "),
                    link("guide", Link::Guide),
                    span(" for more."),
                ])
                .size(14)
                .font(theme::font())
                .on_link_click(Message::Open),
                selectable_text(vec![
                    span("A click on a link still opens it. See the "),
                    link("changelog", Link::Changelog),
                    span(" for what is new."),
                ])
                .size(14)
                .font(theme::font())
                .on_link_click(Message::Open),
            ]
            .spacing(12),
        )
        .on_copy(Message::Copy);

        let status = self
            .status
            .clone()
            .unwrap_or_else(|| "Nothing copied yet.".to_owned());

        column![
            article,
            text(status).size(14).style(|theme: &Theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            }),
        ]
        .spacing(16)
        .width(Length::Fill)
        .max_width(360)
        .into()
    }
}
