use iced::widget::column;
use iced::{Element, Length};
use iced_cube::{field, input};

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    EmailChanged(String),
}

#[derive(Debug, Default)]
pub struct Example {
    name: String,
    email: String,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::NameChanged(name) => self.name = name,
            Message::EmailChanged(email) => self.email = email,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            field(
                "Display name",
                input("Ada Lovelace", &self.name).on_input(Message::NameChanged),
            )
            .required(true),
            field(
                "Email",
                input("you@example.com", &self.email).on_input(Message::EmailChanged),
            )
            .description("Receipts are sent to this address."),
        ]
        .spacing(20)
        .width(Length::Fixed(320.0))
        .into()
    }
}
