use iced::Element;
use iced_cube::data::calendar::{self, Emphasis, Entries, Entry, EntryId, Tone, calendar, instant};
use iced_cube::lucide;
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
            // An invitation not yet answered: outlined, with a mail icon.
            Entry::new(EntryId(2), "Design review", at(17, 14, 0), at(17, 15, 0))
                .tone(Tone::Success)
                .emphasis(Emphasis::Outlined)
                .icon(lucide!(Mail))
                .icon_label("Invitation"),
            // Declined: struck through.
            Entry::new(EntryId(3), "Vendor call", at(18, 11, 0), at(18, 12, 0))
                .emphasis(Emphasis::Struck),
            // Someone else's private time: hatched and locked, so it can
            // be opened but not dragged or deleted.
            Entry::all_day(EntryId(4), "Busy", date(2026, 6, 23), date(2026, 6, 24))
                .tone(Tone::Neutral)
                .emphasis(Emphasis::Hatched)
                .icon(lucide!(Lock))
                .icon_label("Private")
                .locked(true),
            // Cancelled: faded.
            Entry::new(EntryId(5), "Offsite planning", at(25, 10, 0), at(25, 11, 0))
                .tone(Tone::Warning)
                .emphasis(Emphasis::Faded),
            Entry::all_day(EntryId(6), "Team week", date(2026, 6, 8), date(2026, 6, 10))
                .tone(Tone::Warning)
                .icon(lucide!(Users))
                .icon_label("Team entry"),
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
