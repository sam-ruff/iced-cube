use std::fmt;

use iced::Element;
use iced::widget::{column, text};
use iced_cube::primitives::radio_group;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    Comfortable,
    Default,
    Compact,
}

impl fmt::Display for Density {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Density::Comfortable => "Comfortable",
            Density::Default => "Default",
            Density::Compact => "Compact",
        })
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Selected(Density),
}

#[derive(Debug)]
pub struct Example {
    density: Density,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            density: Density::Default,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(density) = message;
        self.density = density;
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            radio_group(
                [Density::Comfortable, Density::Default, Density::Compact],
                Some(self.density),
            )
            .on_select(Message::Selected),
            text(format!("Selected: {}", self.density)).size(14),
        ]
        .spacing(20)
        .into()
    }
}
