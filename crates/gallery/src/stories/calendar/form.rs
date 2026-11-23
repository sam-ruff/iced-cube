use iced::Element;
use iced::widget::{Column, container};
use iced_cube::button;
use iced_cube::data::calendar::editor::{self, Fields, Part, editor};
use iced_cube::data::calendar::{self, Draft, Entries, View, When, calendar, instant};
use iced_cube::overlay::dialog::{columns, dialog, section};
use iced_cube::primitives::button::Variant;
use iced_cube::primitives::input::input;
use jiff::Timestamp;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
    Form(editor::Event),
    Guests(String),
    Save,
    Close,
}

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
    form: Option<editor::State>,
    /// A field of the app's own, placed among the form's.
    guests: String,
}

/// A time in June 2026.
fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
    instant(date(2026, 6, day), time(hour, minute, 0, 0), &TimeZone::UTC)
}

impl Default for Example {
    fn default() -> Self {
        let utc = TimeZone::UTC;
        // Opens with the form showing, as after asking for a new entry.
        let draft = Draft {
            when: When::timed(at(18, 14, 0), at(18, 15, 0)),
        };
        Self {
            calendar: calendar::State::new(date(2026, 6, 18))
                .with_entries(Entries::utc())
                .with_view(View::Week),
            form: Some(editor::State::new(&draft, &utc)),
            guests: String::new(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Calendar(event) => {
                if let Some(calendar::Output::Requested(draft)) = self.calendar.update(event) {
                    self.form = Some(editor::State::new(&draft, self.calendar.time_zone()));
                }
            }
            Message::Form(event) => {
                if let Some(form) = &mut self.form {
                    form.update(event);
                }
            }
            Message::Guests(guests) => self.guests = guests,
            Message::Save => {
                let Some(form) = self.form.take() else {
                    return;
                };
                let entry = form.entry(self.calendar.entries().next_id());
                let _ = self.calendar.update(calendar::Event::Insert(entry));
            }
            Message::Close => self.form = None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let base = container(
            calendar(&self.calendar)
                .hours(9..17)
                .on_event(Message::Calendar),
        )
        .padding(24);
        let Some(form) = &self.form else {
            return dialog(base).into();
        };

        // The form's own parts, laid out by the app, without the location.
        let Fields {
            title,
            all_day,
            starts,
            ends,
            repeat,
            every,
            stops,
            colour,
            notes,
            ..
        } = editor(form)
            .without(Part::Location)
            .on_event(Message::Form)
            .on_submit(Message::Save)
            .fields();
        let guests = input("Add guests", &self.guests).on_input(Message::Guests);

        let when = Column::with_children(
            [starts, ends, all_day, repeat, every, stops]
                .into_iter()
                .flatten(),
        )
        .spacing(12);
        let details = Column::with_children([title, colour, notes].into_iter().flatten())
            .push(guests)
            .spacing(12);
        let body = columns([
            section("When", when).into(),
            section("Details", details).into(),
        ]);

        dialog(base)
            .open(true)
            .title("New entry")
            .width(760.0)
            .body(body)
            .on_dismiss(Message::Close)
            .action(
                button("Cancel")
                    .variant(Variant::Outline)
                    .on_press(Message::Close),
            )
            .action(button("Save").on_press(Message::Save))
            .into()
    }
}
