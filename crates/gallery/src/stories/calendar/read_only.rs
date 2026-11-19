use iced::widget::{column, row};
use iced::{Alignment, Element, Length};
use iced_cube::data::calendar::{self, Entries, Entry, EntryId, Tone, When, calendar, instant};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, input};
use jiff::Timestamp;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
    Title(String),
    Add,
    Remove,
}

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
    title: String,
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
            Entry::all_day(
                EntryId(3),
                "Conference",
                date(2026, 6, 9),
                date(2026, 6, 11),
            )
            .tone(Tone::Warning),
        ]);

        Self {
            // Nothing can be created, dragged or removed from the calendar
            // itself. Selecting, opening and moving about still work.
            calendar: calendar::State::new(date(2026, 6, 18))
                .with_entries(entries)
                .read_only(),
            title: String::new(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Calendar(event) => {
                let _ = self.calendar.update(event);
            }
            Message::Title(title) => self.title = title,
            // The app's own controls change the entries with the same
            // events the calendar uses, whatever the calendar allows.
            Message::Add => {
                let title = self.title.trim();
                if title.is_empty() {
                    return;
                }
                let day = self
                    .calendar
                    .selected_day()
                    .unwrap_or(self.calendar.today());
                let id = self.calendar.entries().next_id();
                let entry = Entry::at(id, title, When::day(day)).tone(Tone::Neutral);
                let _ = self.calendar.update(calendar::Event::Insert(entry));
                self.title.clear();
            }
            Message::Remove => {
                if let Some(id) = self.calendar.selected_entry().map(Entry::id) {
                    let _ = self.calendar.update(calendar::Event::Remove(id));
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let add = (!self.title.trim().is_empty()).then_some(Message::Add);
        let remove = self.calendar.selected_entry().map(|_| Message::Remove);
        let controls = row![
            input("Add to the selected day", &self.title)
                .on_input(Message::Title)
                .on_submit(Message::Add),
            button("Add").on_press_maybe(add),
            button("Remove")
                .variant(Variant::Outline)
                .on_press_maybe(remove),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        column![
            controls,
            calendar(&self.calendar)
                .hours(8..18)
                .height(Length::Fill)
                .on_event(Message::Calendar),
        ]
        .spacing(12)
        .into()
    }
}
