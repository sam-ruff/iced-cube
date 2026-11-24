use iced::widget::{column, container, row, text};
use iced::{Alignment, Element};
use iced_cube::button;
use iced_cube::overlay::dialog::dialog;
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone)]
pub enum Message {
    Open,
    Close,
    Accept,
}

#[derive(Debug)]
pub struct Example {
    open: bool,
    accepted: bool,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            open: true,
            accepted: false,
        }
    }
}

const CHANGES: [&str; 12] = [
    "Events can repeat every weekday.",
    "Reminders arrive a day, an hour or ten minutes before.",
    "Guests can suggest a new time.",
    "Rooms show whether they are free.",
    "Shared calendars show who changed an event.",
    "Declined events fade instead of disappearing.",
    "Time zones follow each guest.",
    "Week numbers can be turned on.",
    "All-day events stay at the top of each day.",
    "The month view shows up to four events a day.",
    "Search finds events by guest.",
    "Exports include attachments.",
];

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Open => self.open = true,
            Message::Close => self.open = false,
            Message::Accept => {
                self.accepted = true;
                self.open = false;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = if self.accepted {
            "Up to date."
        } else {
            "An update is ready."
        };
        let base = row![
            button("What's new")
                .variant(Variant::Outline)
                .on_press(Message::Open),
            text(status).size(14),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let changes = column(
            CHANGES
                .iter()
                .map(|change| text(format!("- {change}")).size(14).into()),
        )
        .spacing(8);

        dialog(container(base).padding(24))
            .open(self.open)
            .title("What's new in Calendar")
            .description("Twelve changes since your last visit.")
            .body(changes)
            .action(
                button("Later")
                    .variant(Variant::Outline)
                    .on_press(Message::Close),
            )
            .action(button("Got it").on_press(Message::Accept))
            .on_dismiss(Message::Close)
            .into()
    }
}
