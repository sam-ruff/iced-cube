use iced::widget::column;
use iced::{Element, Length};
use iced_cube::primitives::input::Size;
use iced_cube::{input, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Search(String),
    Email(String),
    Website(String),
}

#[derive(Debug, Default)]
pub struct Example {
    search: String,
    email: String,
    website: String,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Search(value) => self.search = value,
            Message::Email(value) => self.email = value,
            Message::Website(value) => self.website = value,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            input("Search", &self.search)
                .icon(lucide!(Search))
                .size(Size::Sm)
                .on_input(Message::Search),
            input("Email", &self.email)
                .icon(lucide!(Mail))
                .on_input(Message::Email),
            input("Website", &self.website)
                .icon(lucide!(Globe))
                .size(Size::Lg)
                .on_input(Message::Website),
        ]
        .spacing(12)
        .width(Length::Fixed(320.0))
        .into()
    }
}
