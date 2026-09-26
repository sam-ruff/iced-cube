use iced::widget::column;
use iced::{Element, Length};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, input, lucide, separator};

#[derive(Debug, Clone)]
pub enum Message {
    EmailChanged(String),
    Continue,
    UseKey,
}

#[derive(Debug, Default)]
pub struct Example {
    email: String,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        if let Message::EmailChanged(email) = message {
            self.email = email;
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            input("you@example.com", &self.email)
                .icon(lucide!(Mail))
                .on_input(Message::EmailChanged),
            button("Continue with email")
                .width(Length::Fill)
                .on_press(Message::Continue),
            separator().label("or"),
            button("Use a security key")
                .icon(lucide!(KeyRound))
                .variant(Variant::Outline)
                .width(Length::Fill)
                .on_press(Message::UseKey),
        ]
        .spacing(12)
        .width(Length::Fixed(320.0))
        .into()
    }
}
