use iced::widget::{column, row, text};
use iced::{Element, Length};
use iced_cube::theme::{Tokens, text_size};
use iced_cube::{separator, vertical_separator};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let muted = |theme: &iced::Theme| text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        };

        column![
            column![
                text("iced-cube").size(text_size::MD),
                text("Components for iced applications.")
                    .size(text_size::SM)
                    .style(muted),
            ]
            .spacing(4),
            separator(),
            row![
                text("Blog").size(text_size::SM),
                vertical_separator(),
                text("Docs").size(text_size::SM),
                vertical_separator(),
                text("Source").size(text_size::SM),
            ]
            .spacing(16)
            .height(20),
        ]
        .spacing(16)
        .width(Length::Fixed(320.0))
        .into()
    }
}
