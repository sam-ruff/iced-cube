use iced::{Element, Theme, color};
use iced_cube::data::calendar::{self, Entries, Entry, EntryId, Tone, calendar, instant};
use iced_cube::theme::Config;
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
            // Each tone is one of the theme's colours.
            Entry::new(EntryId(1), "Stand-up", at(16, 9, 0), at(16, 9, 15)).tone(Tone::Primary),
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
            Entry::all_day(EntryId(6), "Sprint", date(2026, 6, 1), date(2026, 6, 5))
                .tone(Tone::Primary),
        ]);

        Self {
            calendar: calendar::State::at(at(18, 10, 20), entries),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Calendar(event) = message;
        let _ = self.calendar.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Nothing here mentions a colour. The calendar reads them all
        // from whatever theme the app returns.
        calendar(&self.calendar)
            .hours(8..18)
            .on_event(Message::Calendar)
            .into()
    }

    pub fn theme(&self) -> Option<Theme> {
        Some(
            Config::light()
                .name("Brand")
                .background(color!(0xfdfcff))
                .foreground(color!(0x1e1b2e))
                .primary(color!(0x6d28d9))
                .secondary(color!(0xede9fe))
                .accent(color!(0xede9fe))
                .border(color!(0xddd6fe))
                .success(color!(0x0d9488))
                .warning(color!(0xea580c))
                .destructive(color!(0xe11d48))
                .build(),
        )
    }
}
