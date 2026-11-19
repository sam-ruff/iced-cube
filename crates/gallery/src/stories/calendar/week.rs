use iced::Element;
use iced_cube::data::calendar::{self, Entries, Entry, EntryId, Tone, View, calendar, instant};
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
            Entry::new(EntryId(1), "Stand-up", at(16, 9, 0), at(16, 9, 30)),
            Entry::new(EntryId(2), "Planning", at(16, 10, 0), at(16, 12, 0)).tone(Tone::Success),
            Entry::new(EntryId(3), "Design review", at(18, 14, 0), at(18, 15, 30))
                .tone(Tone::Success),
            Entry::new(EntryId(4), "Lunch with Ada", at(18, 12, 30), at(18, 13, 30))
                .tone(Tone::Neutral),
            Entry::new(EntryId(5), "Interview", at(18, 14, 30), at(18, 15, 30)).tone(Tone::Warning),
            Entry::new(EntryId(6), "Release", at(19, 15, 0), at(19, 16, 0)).tone(Tone::Destructive),
            Entry::all_day(
                EntryId(7),
                "Conference",
                date(2026, 6, 15),
                date(2026, 6, 17),
            )
            .tone(Tone::Warning),
        ]);

        Self {
            // Thursday at 10:20. `at` also draws the line for the time.
            calendar: calendar::State::at(at(18, 10, 20), entries).with_view(View::Week),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Calendar(event) = message;
        // A drag on an entry moves it; the state applies that itself.
        let _ = self.calendar.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        calendar(&self.calendar)
            // The working day, with the hours sharing the height.
            .hours(9..17)
            .hour_height(36.0)
            .on_event(Message::Calendar)
            .into()
    }
}
