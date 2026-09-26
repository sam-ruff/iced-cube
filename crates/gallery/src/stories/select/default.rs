use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::forms::select;

const TIMEZONES: &[&str] = &[
    "London (GMT)",
    "Paris (CET)",
    "New York (EST)",
    "Tokyo (JST)",
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
            select(TIMEZONES, self.timezone)
                .placeholder("Select a timezone")
                .width(240)
                .on_select(Message::Selected),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into()
    }
}
