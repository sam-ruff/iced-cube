use iced::widget::{column, space, text};
use iced::{Element, Length};
use iced_cube::forms::select;

const COUNTRIES: &[&str] = &[
    "Australia",
    "Austria",
    "Belgium",
    "Brazil",
    "Canada",
    "Denmark",
    "Finland",
    "France",
    "Germany",
    "Ireland",
    "Italy",
    "Japan",
    "Netherlands",
    "New Zealand",
    "Norway",
    "Portugal",
    "Spain",
    "Sweden",
    "Switzerland",
    "United Kingdom",
];

#[derive(Debug, Clone)]
pub enum Message {
    Selected(&'static str),
}

#[derive(Debug)]
pub struct Example {
    country: &'static str,
}

impl Default for Example {
    fn default() -> Self {
        Self { country: "Norway" }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(country) = message;
        self.country = country;
    }

    pub fn view(&self) -> Element<'_, Message> {
        // With no room below, the list opens above the field, and is never
        // taller than the space there.
        column![
            space().height(Length::Fill),
            text("Country").size(14),
            select(COUNTRIES, Some(self.country))
                .width(240)
                .on_select(Message::Selected),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into()
    }
}
