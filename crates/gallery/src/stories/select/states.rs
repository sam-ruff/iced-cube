use std::fmt;

use iced::widget::{container, row};
use iced::{Element, Length};
use iced_cube::forms::select;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Viewer,
    Editor,
    Owner,
}

impl Role {
    const ALL: [Role; 3] = [Role::Viewer, Role::Editor, Role::Owner];
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Role::Viewer => "Viewer",
            Role::Editor => "Editor",
            Role::Owner => "Owner",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Selected(Role),
}

#[derive(Debug)]
pub struct Example {
    role: Role,
}

impl Default for Example {
    fn default() -> Self {
        Self { role: Role::Editor }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(role) = message;
        self.role = role;
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            row![
                select(&Role::ALL[..], Some(self.role))
                    .width(160)
                    .on_select(Message::Selected),
                select(&Role::ALL[..], None)
                    .placeholder("Disabled")
                    .width(160),
                select(&Role::ALL[..], Some(Role::Owner)).width(160),
            ]
            .spacing(12)
            .wrap(),
        )
        .height(Length::Fill)
        .into()
    }
}
