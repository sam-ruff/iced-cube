use iced::Element;
use iced::widget::container;
use iced_cube::data::calendar::{
    self, Draft, Entries, Entry, EntryId, Output, View, calendar, instant, when_label,
};
use iced_cube::overlay::dialog::dialog;
use iced_cube::primitives::button::Variant;
use iced_cube::{button, input};
use jiff::Timestamp;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
    Title(String),
    Save,
    Delete,
    Close,
}

/// What the dialog is editing.
#[derive(Debug, Clone, Copy)]
enum Editing {
    New(Draft),
    Existing(EntryId),
}

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
    editing: Option<Editing>,
    title: String,
}

/// A time in June 2026.
fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
    instant(date(2026, 6, day), time(hour, minute, 0, 0), &TimeZone::UTC)
}

impl Default for Example {
    fn default() -> Self {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Stand-up", at(16, 9, 30), at(16, 10, 0)),
            Entry::new(EntryId(2), "Design review", at(18, 14, 0), at(18, 15, 30)),
        ]);

        Self {
            calendar: calendar::State::at(at(18, 10, 20), entries).with_view(View::Week),
            editing: None,
            title: String::new(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Calendar(event) => match self.calendar.update(event) {
                // A drag on empty space, or a second press on a day or a time.
                Some(Output::Requested(draft)) => {
                    self.title.clear();
                    self.editing = Some(Editing::New(draft));
                }
                // A second press on an entry.
                Some(Output::Opened { entry, .. }) => {
                    let Some(opened) = self.calendar.entries().get(entry) else {
                        return;
                    };
                    self.title = opened.title().to_owned();
                    self.editing = Some(Editing::Existing(entry));
                }
                _ => {}
            },
            Message::Title(title) => self.title = title,
            Message::Save => {
                let title = match self.title.trim() {
                    "" => "New entry",
                    title => title,
                };
                let entry = match self.editing.take() {
                    Some(Editing::New(draft)) => {
                        let id = self.calendar.entries().next_id();
                        Some(Entry::at(id, title, draft.when))
                    }
                    Some(Editing::Existing(id)) => {
                        let existing = self.calendar.entries().get(id).cloned();
                        existing.map(|entry| entry.titled(title))
                    }
                    None => None,
                };
                if let Some(entry) = entry {
                    // Nothing is added until the app inserts it.
                    let _ = self.calendar.update(calendar::Event::Insert(entry));
                }
            }
            Message::Delete => {
                if let Some(Editing::Existing(id)) = self.editing.take() {
                    let _ = self.calendar.update(calendar::Event::Remove(id));
                }
            }
            Message::Close => self.editing = None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let base = container(
            calendar(&self.calendar)
                .hours(9..17)
                .hour_height(36.0)
                .on_event(Message::Calendar),
        )
        .padding(24);

        let time_zone = self.calendar.time_zone();
        let (heading, when) = match self.editing {
            Some(Editing::New(draft)) => ("New entry", when_label(&draft.when, time_zone)),
            Some(Editing::Existing(id)) => {
                let entry = self.calendar.entries().get(id);
                let when = entry.map(|entry| when_label(&entry.when(), time_zone));
                ("Edit entry", when.unwrap_or_default())
            }
            None => ("", String::new()),
        };

        let mut editor = dialog(base)
            .open(self.editing.is_some())
            .title(heading)
            .description(when)
            .body(
                input("Title", &self.title)
                    .on_input(Message::Title)
                    .on_submit(Message::Save),
            )
            .on_dismiss(Message::Close);
        if matches!(self.editing, Some(Editing::Existing(_))) {
            editor = editor.action(
                button("Delete")
                    .variant(Variant::Destructive)
                    .on_press(Message::Delete),
            );
        }
        editor
            .action(
                button("Cancel")
                    .variant(Variant::Outline)
                    .on_press(Message::Close),
            )
            .action(button("Save").on_press(Message::Save))
            .into()
    }
}
