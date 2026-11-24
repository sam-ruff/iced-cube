use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::theme::{Tokens, text_size};
use iced_cube::{input, lucide};

const MAX_LENGTH: usize = 24;

#[derive(Debug, Clone)]
pub enum Message {
    Search(String),
    Clear,
}

#[derive(Debug)]
pub struct Example {
    query: String,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            query: "quarterly report".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Search(query) => self.query = query,
            Message::Clear => self.query.clear(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let count = format!("{} / {MAX_LENGTH}", self.query.chars().count());

        column![
            input("Search files", &self.query)
                .icon(lucide!(Search))
                .clearable(Message::Clear)
                .max_length(MAX_LENGTH)
                .on_input(Message::Search),
            text(count).size(text_size::XS).style(|theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            }),
        ]
        .spacing(8)
        .width(Length::Fixed(320.0))
        .into()
    }
}
