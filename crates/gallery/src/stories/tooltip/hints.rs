use iced::keyboard::key::Named;
use iced::widget::{self, column, row, text};
use iced::{Alignment, Element, Length, Subscription, Task};
use iced_cube::keys::{self, Chord};
use iced_cube::overlay::tooltip::{Position, tooltip};
use iced_cube::{icon_button, input, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Name(String),
    Help,
    Key(keys::Event),
}

#[derive(Debug, Default)]
pub struct Example {
    name: String,
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Name(name) => self.name = name,
            Message::Help => {}
            Message::Key(key) if Chord::named(Named::Tab).matches(&key.key, key.modifiers) => {
                return widget::operation::focus_next();
            }
            Message::Key(_) => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let name = tooltip(
            input("Display name", &self.name).on_input(Message::Name),
            "Shown to guests on every invitation you send.",
        )
        .position(Position::Bottom)
        .max_width(240.0);

        let help = tooltip(
            icon_button(lucide!(Info)).on_press(Message::Help),
            "Guests see your display name and email address, but never your calendar.",
        )
        .position(Position::Bottom)
        .max_width(240.0);

        column![
            row![name, help].spacing(8).align_y(Alignment::Center),
            text("Hover the info button, or press Tab to focus the field.").size(14),
        ]
        .spacing(12)
        .width(Length::Fixed(280.0))
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
