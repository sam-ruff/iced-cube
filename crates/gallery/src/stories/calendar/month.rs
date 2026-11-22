use iced::Element;
use iced_cube::data::calendar::{self, Entries, Entry, EntryId, Tone, calendar, instant};
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
            Entry::new(EntryId(1), "Stand-up", at(16, 9, 0), at(16, 9, 15)),
            Entry::new(EntryId(2), "Design review", at(18, 14, 0), at(18, 15, 0))
                .tone(Tone::Success),
            Entry::new(EntryId(3), "Lunch with Ada", at(18, 12, 30), at(18, 13, 30))
                .tone(Tone::Neutral),
            Entry::new(EntryId(4), "Release", at(23, 16, 0), at(23, 17, 0)).tone(Tone::Destructive),
            Entry::all_day(
                EntryId(5),
                "Conference",
                date(2026, 6, 9),
                date(2026, 6, 11),
            )
            .tone(Tone::Warning),
            Entry::all_day(EntryId(6), "Holiday", date(2026, 6, 26), date(2026, 6, 30))
                .tone(Tone::Success),
        ]);

        Self {
            // The calendar never reads the clock, so today is passed in.
            calendar: calendar::State::new(date(2026, 6, 18)).with_entries(entries),
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
            // The hours the day, week and four-day views show.
            .hours(8..18)
            .on_event(Message::Calendar)
            .into()
    }
}
