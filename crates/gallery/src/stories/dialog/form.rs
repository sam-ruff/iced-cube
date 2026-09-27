use iced::widget::{column, container, row, text};
use iced::{Alignment, Element};
use iced_cube::overlay::dialog::{Size, dialog};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, field, input};

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    NameChanged(String),
    UsernameChanged(String),
    Save,
}

#[derive(Debug)]
pub struct Example {
    open: bool,
    name: String,
    username: String,
    saved: String,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            open: true,
            name: String::from("Ada Lovelace"),
            username: String::from("ada"),
            saved: String::from("Ada Lovelace"),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            Message::Close => {
                self.name = self.saved.clone();
                self.open = false;
            }
            Message::NameChanged(name) => self.name = name,
            Message::UsernameChanged(username) => self.username = username,
            Message::Save => {
                self.saved = self.name.clone();
                self.open = false;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let base = row![
            button("Edit profile")
                .variant(Variant::Outline)
                .on_press(Message::Open),
            text(format!("Signed in as {}", self.saved)).size(14),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let form = column![
            field(
                "Name",
                input("Your name", &self.name).on_input(Message::NameChanged),
            ),
            field(
                "Username",
                input("username", &self.username).on_input(Message::UsernameChanged),
            ),
        ]
        .spacing(16);

        dialog(container(base).padding(24))
            .open(self.open)
            .title("Edit profile")
            .description("Tab and Shift+Tab move between the fields. Enter saves, Escape cancels.")
            .body(form)
            .action(button("Save changes").on_press(Message::Save))
            .on_confirm(Message::Save)
            .on_dismiss(Message::Close)
            .size(Size::Sm)
            .into()
    }
}
