use iced::widget::{column, row, text};
use iced::{Alignment, Element};
use iced_cube::lucide;
use iced_cube::primitives::toggle::{Variant, toggle};

#[derive(Debug, Clone)]
pub enum Message {
    Pinned(bool),
}

#[derive(Debug, Default)]
pub struct Example {
    pinned: bool,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Pinned(pinned) = message;
        self.pinned = pinned;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let line = |variant: Variant| {
            row![
                text(format!("{variant:?}")).size(14).width(64),
                toggle("Pin")
                    .icon(lucide!(Pin))
                    .variant(variant)
                    .pressed(self.pinned)
                    .on_toggle(Message::Pinned),
                toggle("Locked")
                    .icon(lucide!(Lock))
                    .variant(variant)
                    .pressed(true),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        };

        column![line(Variant::Default), line(Variant::Outline)]
            .spacing(16)
            .into()
    }
}
