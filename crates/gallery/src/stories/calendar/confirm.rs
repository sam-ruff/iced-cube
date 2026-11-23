use iced::Element;
use iced::widget::container;
use iced_cube::data::calendar::{
    self, Change, Changed, Entries, Entry, EntryId, Output, View, calendar, instant, when_label,
};
use iced_cube::icon_button;
use iced_cube::lucide;
use iced_cube::overlay::dialog::alert_dialog;
use iced_cube::primitives::icon_button::Variant;
use jiff::Timestamp;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
    New,
    Confirm,
    Cancel,
}

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
    /// The change the calendar proposed, while the app asks about it.
    asking: Option<Changed>,
    /// What happened last, shown under the calendar.
    note: String,
}

/// A time in June 2026.
fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
    instant(date(2026, 6, day), time(hour, minute, 0, 0), &TimeZone::UTC)
}

impl Default for Example {
    fn default() -> Self {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Stand-up", at(16, 9, 0), at(16, 9, 30)),
            Entry::new(EntryId(2), "Design review", at(18, 14, 0), at(18, 15, 0)),
            Entry::all_day(EntryId(3), "Offsite", date(2026, 6, 19), date(2026, 6, 19)),
        ]);
        let calendar = calendar::State::new(date(2026, 6, 18))
            .with_entries(entries)
            .with_view(View::Month)
            // Drags and the Delete key ask first.
            .with_proposals(true)
            // Only a double click opens; a single one selects.
            .with_activate_on_second_press(false);
        Self {
            calendar,
            asking: None,
            note: String::from("Drag an entry to another day, or double click one."),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Calendar(event) => match self.calendar.update(event) {
                Some(Output::Proposed(changed)) => self.asking = Some(*changed),
                Some(Output::Opened { entry, .. }) => {
                    let title = self.calendar.entries().get(entry).map(Entry::title);
                    self.note = format!("Opened {}", title.unwrap_or_default());
                }
                Some(Output::Requested(draft)) => {
                    self.note = format!("Asked for an entry on {}", self.when(&draft.when));
                }
                _ => {}
            },
            Message::New => {
                let id = self.calendar.entries().next_id();
                let today = self.calendar.today();
                let entry = Entry::all_day(id, "New entry", today, today);
                let _ = self.calendar.update(calendar::Event::Insert(entry));
            }
            Message::Confirm => {
                self.asking = None;
                let _ = self.calendar.update(calendar::Event::Confirm);
            }
            Message::Cancel => {
                self.asking = None;
                let _ = self.calendar.update(calendar::Event::Discard);
            }
        }
    }

    fn when(&self, when: &calendar::When) -> String {
        when_label(when, self.calendar.time_zone())
    }

    pub fn view(&self) -> Element<'_, Message> {
        let new = icon_button(lucide!(Plus))
            .label("New entry")
            .variant(Variant::Primary)
            .on_press(Message::New);
        let base = container(
            iced::widget::column![
                calendar(&self.calendar)
                    .header_leading(new)
                    .hours(8..18)
                    .on_event(Message::Calendar),
                iced::widget::text(&self.note).size(13),
            ]
            .spacing(8),
        )
        .padding(24);

        let (title, description, confirm) = match &self.asking {
            Some(changed) if changed.change == Change::Removed => (
                format!("Delete {}?", changed.entry.title()),
                String::from("It will be removed from the calendar."),
                "Delete",
            ),
            Some(changed) => (
                format!("Move {}?", changed.entry.title()),
                format!("To {}.", self.when(&changed.entry.when())),
                "Move",
            ),
            None => (String::new(), String::new(), "Move"),
        };
        alert_dialog(base, title, description)
            .open(self.asking.is_some())
            .confirm(confirm)
            .on_confirm(Message::Confirm)
            .on_cancel(Message::Cancel)
            // Putting it back is the safe choice, so a click outside does.
            .dismiss_on_scrim(true)
            .into()
    }
}
