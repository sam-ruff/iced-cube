use iced::Element;
use iced_cube::data::calendar::{
    self, Entries, Entry, EntryId, Repeat, Tone, View, calendar, instant,
};
use jiff::Timestamp;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
}

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
}

/// A time in June 2026.
fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
    instant(date(2026, 6, day), time(hour, minute, 0, 0), &TimeZone::UTC)
}

impl Default for Example {
    fn default() -> Self {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Weekly review", at(1, 15, 0), at(1, 16, 0))
                .repeat(Repeat::weekly()),
            Entry::new(EntryId(2), "Dentist", at(4, 8, 30), at(4, 9, 0)).tone(Tone::Neutral),
            Entry::all_day(
                EntryId(3),
                "Conference",
                date(2026, 6, 9),
                date(2026, 6, 10),
            )
            .tone(Tone::Warning),
            Entry::new(EntryId(4), "Design review", at(18, 14, 0), at(18, 15, 0))
                .tone(Tone::Success),
            // An entry with no length, such as a deadline.
            Entry::new(EntryId(5), "Release", at(23, 16, 0), at(23, 16, 0)).tone(Tone::Destructive),
        ]);

        Self {
            calendar: calendar::State::new(date(2026, 6, 18))
                .with_entries(entries)
                .with_view(View::Agenda),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Calendar(event) = message;
        let _ = self.calendar.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        calendar(&self.calendar)
            .hours(8..18)
            .on_event(Message::Calendar)
            .into()
    }
}
