use iced::widget::{column, container, row, text};
use iced::{Alignment, Element};
use iced_cube::overlay::dialog::{Size, columns, dialog, section};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, field, input};

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    Title(String),
    Starts(String),
    Ends(String),
    Location(String),
    Guests(String),
    Save,
}

#[derive(Debug)]
pub struct Example {
    open: bool,
    title: String,
    starts: String,
    ends: String,
    location: String,
    guests: String,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            open: true,
            title: String::from("Design review"),
            starts: String::from("14:00"),
            ends: String::from("15:00"),
            location: String::from("Room 2"),
            guests: String::from("ada, grace"),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            Message::Close | Message::Save => self.open = false,
            Message::Title(title) => self.title = title,
            Message::Starts(starts) => self.starts = starts,
            Message::Ends(ends) => self.ends = ends,
            Message::Location(location) => self.location = location,
            Message::Guests(guests) => self.guests = guests,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let base = row![
            button("New event")
                .variant(Variant::Outline)
                .on_press(Message::Open),
            text(&self.title).size(14),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let details = section(
            "Details",
            column![
                field(
                    "Title",
                    input("Event title", &self.title).on_input(Message::Title)
                ),
                field(
                    "Location",
                    input("Where it happens", &self.location).on_input(Message::Location),
                ),
            ]
            .spacing(12),
        );
        let time = section(
            "Time",
            column![
                field(
                    "Starts",
                    input("09:00", &self.starts).on_input(Message::Starts)
                ),
                field("Ends", input("10:00", &self.ends).on_input(Message::Ends)),
            ]
            .spacing(12),
        );
        let people = section(
            "People",
            input("Names, separated by commas", &self.guests).on_input(Message::Guests),
        )
        .description("Guests get an invitation by email.");

        let body = column![columns([details.into(), time.into()]), people].spacing(24);

        dialog(container(base).padding(24))
            .open(self.open)
            .title("New event")
            .description("Sections sit side by side, and stack in one column on a phone.")
            .body(body)
            .action(
                button("Cancel")
                    .variant(Variant::Outline)
                    .on_press(Message::Close),
            )
            .action(button("Save event").on_press(Message::Save))
            .on_dismiss(Message::Close)
            .size(Size::Lg)
            .into()
    }
}
