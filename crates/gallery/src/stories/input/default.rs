use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::input;
use iced_cube::theme::{Tokens, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    Submit,
}

#[derive(Debug, Default)]
pub struct Example {
    name: String,
    greeting: Option<String>,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::NameChanged(name) => self.name = name,
            Message::Submit => self.greeting = Some(format!("Hello, {}!", self.name.trim())),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let hint = self.greeting.as_deref().unwrap_or("Press Enter to submit.");

        column![
            input("Your name", &self.name)
                .on_input(Message::NameChanged)
                .on_submit(Message::Submit),
            text(hint).size(text_size::SM).style(|theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            }),
        ]
        .spacing(12)
        .width(Length::Fixed(320.0))
        .into()
    }
}
