use iced::Element;
use iced_cube::data::calendar::{self, Entries, Entry, EntryId, Repeat, Tone, calendar, instant};
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
            // Every day for two weeks.
            Entry::new(EntryId(1), "Stand-up", at(1, 9, 0), at(1, 9, 15))
                .repeat(Repeat::daily().until(date(2026, 6, 12))),
            // Every Friday, with no end.
            Entry::new(EntryId(2), "Review", at(5, 15, 0), at(5, 16, 0))
                .tone(Tone::Success)
                .repeat(Repeat::weekly()),
            // Three days every other week, twice.
            Entry::all_day(EntryId(3), "On call", date(2026, 6, 8), date(2026, 6, 10))
                .tone(Tone::Warning)
                .repeat(Repeat::weekly().interval(2).count(2)),
            // The 31st of each month, or its last day when it is shorter.
            Entry::all_day(EntryId(4), "Invoices", date(2026, 5, 31), date(2026, 5, 31))
                .tone(Tone::Destructive)
                .repeat(Repeat::monthly()),
        ]);

        Self {
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
            .hours(8..18)
            .on_event(Message::Calendar)
            .into()
    }
}
