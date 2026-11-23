use std::fmt;

use iced::widget::{column, text};
use iced::{Color, Element, Length, Theme};
use iced_cube::forms::select;
use iced_cube::theme::{self, Tokens};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    OnTrack,
    AtRisk,
    Blocked,
    Done,
    NoStatus,
}

impl Status {
    const ALL: [Status; 5] = [
        Status::OnTrack,
        Status::AtRisk,
        Status::Blocked,
        Status::Done,
        Status::NoStatus,
    ];

    /// The status colour, read from the theme when the swatch is drawn.
    fn colour(self) -> Option<fn(&Theme) -> Color> {
        match self {
            Status::OnTrack => Some(|theme| Tokens::of(theme).success),
            Status::AtRisk => Some(|theme| Tokens::of(theme).warning),
            Status::Blocked => Some(|theme| Tokens::of(theme).destructive),
            Status::Done => Some(|theme| Tokens::of(theme).muted_foreground),
            Status::NoStatus => None,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Status::OnTrack => "On track",
            Status::AtRisk => "At risk",
            Status::Blocked => "Blocked",
            Status::Done => "Done",
            Status::NoStatus => "No status",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Selected(Status),
}

#[derive(Debug)]
pub struct Example {
    status: Status,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            status: Status::AtRisk,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(status) = message;
        self.status = status;
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text("Project status").size(14),
            select(&Status::ALL[..], Some(self.status))
                .leading(|status| status.colour().map(|colour| theme::swatch(colour).into()))
                .width(240)
                .on_select(Message::Selected),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into()
    }
}
