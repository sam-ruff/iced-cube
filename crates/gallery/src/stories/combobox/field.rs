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

const OFFICES: &[&str] = &["Edinburgh", "Leeds", "London", "Manchester"];

#[derive(Debug, Clone)]
pub enum Message {
    Timezone(combobox::Event),
    Office(combobox::Event),
}

#[derive(Debug)]
pub struct Example {
    timezone: State<&'static str>,
    office: State<&'static str>,
    region: State<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            timezone: State::new(TIMEZONES.iter().copied()).with_selected(&"London (GMT)"),
            office: State::new(OFFICES.iter().copied()),
            region: State::new(["Europe"]).with_selected(&"Europe"),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Timezone(event) => {
                let _ = self.timezone.update(event);
            }
            Message::Office(event) => {
                let _ = self.office.update(event);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // A required value that is still missing shows as invalid.
        let office_error = self
            .office
            .selected()
            .is_none()
            .then_some("Pick the office you work from.");

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
            field(
                "Office",
                combobox(&self.office)
                    .placeholder("Search offices...")
                    .width(Length::Fill)
                    .invalid(office_error.is_some())
                    .on_event(Message::Office),
            )
            .required(true)
            .error_maybe(office_error),
            field("Region", combobox(&self.region).width(Length::Fill))
                .description("Set by your organisation.")
                .disabled(true),
        ]
        .spacing(20)
        .width(Length::Fixed(320.0))
        .height(Length::Fill)
        .into()
    }
}
