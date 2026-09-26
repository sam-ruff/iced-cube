use iced::widget::row;
use iced::{Alignment, Element, Length};
use iced_cube::primitives::button::Variant;
use iced_cube::{icon_button, input, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    PasswordChanged(String),
    ToggleVisibility,
}

#[derive(Debug, Default)]
pub struct Example {
    password: String,
    visible: bool,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::PasswordChanged(password) => self.password = password,
            Message::ToggleVisibility => self.visible = !self.visible,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let toggle = if self.visible {
            lucide!(EyeOff)
        } else {
            lucide!(Eye)
        };

        row![
            input("Password", &self.password)
                .icon(lucide!(LockKeyhole))
                .secure(!self.visible)
                .on_input(Message::PasswordChanged),
            icon_button(toggle)
                .variant(Variant::Outline)
                .on_press(Message::ToggleVisibility),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fixed(320.0))
        .into()
    }
}
