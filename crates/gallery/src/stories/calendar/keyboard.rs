use iced::widget::{column, text};
use iced::{Element, Length, Subscription};
use iced_cube::data::calendar::{
    self, Action, Entries, Entry, EntryId, Selection, Tone, View, calendar, instant,
};
use iced_cube::keys::{self, Keymap};
use iced_cube::theme::{Tokens, text_size};
use jiff::Timestamp;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
    keymap: Keymap<Action>,
}

/// A time in June 2026.
fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
    instant(date(2026, 6, day), time(hour, minute, 0, 0), &TimeZone::UTC)
}

impl Default for Example {
    fn default() -> Self {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Stand-up", at(18, 9, 30), at(18, 10, 0)),
            Entry::new(EntryId(2), "Design review", at(18, 14, 0), at(18, 15, 0))
                .tone(Tone::Success),
            Entry::new(EntryId(3), "Release", at(19, 11, 0), at(19, 12, 0)).tone(Tone::Destructive),
        ]);
        let today = date(2026, 6, 18);

        Self {
            calendar: calendar::State::new(today)
                .with_entries(entries)
                .with_view(View::Week)
                .with_selection(Selection::Day(today)),
            keymap: calendar::default_keymap(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Calendar(event) => Some(event),
            // A key becomes an action, and the action an event.
            Message::Key(key) => self
                .keymap
                .resolve_event(&key)
                .and_then(|action| action.event(&self.calendar)),
        };
        if let Some(event) = event {
            let _ = self.calendar.update(event);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let hint = text("Arrows move the day, Alt+Down picks an entry, D W M Y A X switch view.")
            .size(text_size::SM)
            .style(|theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            });

        column![
            calendar(&self.calendar)
                .hours(9..16)
                .hour_height(36.0)
                .height(Length::Fill)
                .on_event(Message::Calendar),
            hint,
        ]
        .spacing(8)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
