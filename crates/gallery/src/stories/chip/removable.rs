use iced::widget::{column, row, text};
use iced::{Element, Length};
use iced_cube::button;
use iced_cube::chip;
use iced_cube::primitives::button::{Size, Variant};

const TAGS: &[&str] = &["design", "frontend", "accessibility", "release", "docs"];

#[derive(Debug, Clone)]
pub enum Message {
    Remove(&'static str),
    Reset,
}

#[derive(Debug)]
pub struct Example {
    tags: Vec<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            tags: TAGS.to_vec(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Remove(tag) => self.tags.retain(|t| *t != tag),
            Message::Reset => self.tags = TAGS.to_vec(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let tags: Element<'_, Message> = if self.tags.is_empty() {
            text("No tags.").size(14).into()
        } else {
            row(self
                .tags
                .iter()
                .map(|&tag| chip(tag).id(tag).on_remove(Message::Remove(tag)).into()))
            .spacing(6)
            .wrap()
            .vertical_spacing(6)
            .into()
        };

        column![
            tags,
            button("Reset")
                .variant(Variant::Outline)
                .size(Size::Sm)
                .on_press(Message::Reset),
        ]
        .spacing(16)
        .width(Length::Fixed(320.0))
        .into()
    }
}
