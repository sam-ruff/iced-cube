use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::forms::select;

const TIMEZONES: &[&str] = &[
    "Amsterdam (CET)",
    "Athens (EET)",
    "Auckland (NZST)",
    "Bangkok (ICT)",
    "Berlin (CET)",
    "Buenos Aires (ART)",
    "Cairo (EET)",
    "Chicago (CST)",
    "Dubai (GST)",
    "Hong Kong (HKT)",
    "Johannesburg (SAST)",
    "Lisbon (WET)",
    "London (GMT)",
    "Los Angeles (PST)",
    "Madrid (CET)",
    "Mexico City (CST)",
    "Mumbai (IST)",
    "New York (EST)",
    "Paris (CET)",
    "Reykjavik (GMT)",
    "Rome (CET)",
    "Santiago (CLT)",
    "Seoul (KST)",
    "Singapore (SGT)",
    "Sydney (AEST)",
    "Tokyo (JST)",
    "Toronto (EST)",
    "Vancouver (PST)",
];

#[derive(Debug, Clone)]
pub enum Message {
    Selected(&'static str),
}

#[derive(Debug, Default)]
pub struct Example {
    timezone: Option<&'static str>,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(timezone) = message;
        self.timezone = Some(timezone);
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text("Timezone").size(14),
            // Typing in the open list filters it; try "cet" or "new".
            select(TIMEZONES, self.timezone)
                .placeholder("Select a timezone")
                .searchable(true)
                .width(240)
                .on_select(Message::Selected),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into()
    }
}
