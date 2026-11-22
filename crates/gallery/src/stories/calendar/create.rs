use iced::widget::container;
use iced::{Element, Padding};
use iced_cube::button;
use iced_cube::data::calendar::editor::{self, editor};
use iced_cube::data::calendar::{self, Entries, Entry, EntryId, Output, View, calendar, instant};
use iced_cube::overlay::dialog::dialog;
use iced_cube::primitives::button::Variant;
use iced_cube::scroll_area;
use jiff::Timestamp;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
    Form(editor::Event),
    Save,
    Delete,
    Close,
}

/// How much of the form the dialog shows at once.
const FORM_HEIGHT: f32 = 280.0;

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
    /// The entry form, while the dialog is open.
    form: Option<editor::State>,
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
            form: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Calendar(event) => match self.calendar.update(event) {
                // A drag on empty space, or a second press on a day or a
                // time: a form for a new entry over that span.
                Some(Output::Requested(draft)) => {
                    let time_zone = self.calendar.time_zone();
                    self.form = Some(editor::State::new(&draft, time_zone));
                }
                // A second press on an entry: a form to change it.
                Some(Output::Opened { entry, .. }) => {
                    let entry = self.calendar.entries().get(entry);
                    let time_zone = self.calendar.time_zone();
                    self.form = entry.map(|entry| editor::State::edit(entry, time_zone));
                }
                _ => {}
            },
            Message::Form(event) => {
                if let Some(form) = &mut self.form {
                    form.update(event);
                }
            }
            Message::Save => {
                let Some(form) = self.form.take() else {
                    return;
                };
                // A new entry takes a fresh id; a changed one keeps its
                // own and replaces the entry it came from.
                let entry = form.entry(self.calendar.entries().next_id());
                let _ = self.calendar.update(calendar::Event::Insert(entry));
            }
            Message::Delete => {
                if let Some(id) = self.form.take().and_then(|form| form.id()) {
                    let _ = self.calendar.update(calendar::Event::Remove(id));
                }
            }
            Message::Close => self.form = None,
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

        let Some(form) = &self.form else {
            return dialog(base).into();
        };
        let heading = if form.is_new() {
            "New entry"
        } else {
            "Edit entry"
        };
        let mut editing = dialog(base)
            .open(true)
            .title(heading)
            // The form grows with its fields, so it scrolls inside the
            // dialog. The padding keeps the fields clear of the scrollbar.
            .body(
                scroll_area(
                    container(
                        editor(form)
                            .on_event(Message::Form)
                            .on_submit(Message::Save),
                    )
                    .padding(Padding::ZERO.right(12)),
                )
                .height(FORM_HEIGHT),
            )
            .on_dismiss(Message::Close);
        if !form.is_new() {
            editing = editing.action(
                button("Delete")
                    .variant(Variant::Destructive)
                    .on_press(Message::Delete),
            );
        }
        editing
            .action(
                button("Cancel")
                    .variant(Variant::Outline)
                    .on_press(Message::Close),
            )
            .action(button("Save").on_press(Message::Save))
            .into()
    }
}
