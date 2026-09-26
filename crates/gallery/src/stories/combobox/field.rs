use iced::widget::column;
use iced::{Element, Length};
use iced_cube::field;
use iced_cube::forms::combobox::{self, State, combobox};

const TIMEZONES: &[&str] = &[
    "Auckland (NZST)",
    "Berlin (CET)",
    "Chicago (CST)",
    "Dubai (GST)",
    "Honolulu (HST)",
    "London (GMT)",
    "Los Angeles (PST)",
    "Mumbai (IST)",
    "New York (EST)",
    "Paris (CET)",
    "Singapore (SGT)",
    "Sydney (AEST)",
    "Tokyo (JST)",
];

#[derive(Debug, Clone)]
pub enum Message {
    Timezone(combobox::Event),
}

#[derive(Debug)]
pub struct Example {
    timezone: State<&'static str>,
    region: State<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            timezone: State::new(TIMEZONES.iter().copied()).with_selected(&"London (GMT)"),
            region: State::new(["Europe"]).with_selected(&"Europe"),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Timezone(event) = message;
        let _ = self.timezone.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            field(
                "Timezone",
                combobox(&self.timezone)
                    .placeholder("Search timezones...")
                    .empty("No timezone found.")
                    .width(Length::Fill)
                    .on_event(Message::Timezone),
            )
            .description("Meeting times are shown in this timezone."),
            field("Region", combobox(&self.region).width(Length::Fill))
                .description("Set by your organisation.")
                .disabled(true),
        ]
        .spacing(20)
        .width(Length::Fixed(320.0))
        .into()
    }
}
