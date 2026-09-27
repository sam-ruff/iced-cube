//! The "New job" form: an input, a combobox, a select, a radio group, a
//! textarea and a checkbox, laid out for a dialog.

use iced::widget::text_editor::{Action as EditorAction, Content};
use iced::widget::{column, row};
use iced::{Element, Length};
use iced_cube::forms::combobox::{self, combobox};
use iced_cube::keys::Keymap;
use iced_cube::primitives::radio::Direction;
use iced_cube::{checkbox, field, input, lucide, radio_group, select, textarea};

use super::data::{Queue, SOURCES, Schedule};

#[derive(Debug, Clone)]
pub enum Message {
    Name(String),
    Source(combobox::Event),
    Schedule(Schedule),
    Queue(Queue),
    Notes(EditorAction),
    StartNow(bool),
}

/// What the form produces once it is valid.
#[derive(Debug, Clone)]
pub struct Submission {
    pub name: String,
    pub source: String,
    pub schedule: Schedule,
    pub queue: Queue,
    pub notes: String,
    pub start_now: bool,
}

#[derive(Debug)]
pub struct Draft {
    name: String,
    source: combobox::State<&'static str>,
    schedule: Option<Schedule>,
    queue: Queue,
    notes: Content,
    start_now: bool,
    attempted: bool,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            name: String::new(),
            source: combobox::State::new(SOURCES.iter().copied()).with_visible_rows(5),
            schedule: Some(Schedule::Once),
            queue: Queue::Normal,
            notes: Content::new(),
            start_now: true,
            attempted: false,
        }
    }
}

impl Draft {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Name(name) => self.name = name,
            Message::Source(event) => {
                let _ = self.source.update(event);
            }
            Message::Schedule(schedule) => self.schedule = Some(schedule),
            Message::Queue(queue) => self.queue = queue,
            Message::Notes(action) => self.notes.perform(action),
            Message::StartNow(on) => self.start_now = on,
        }
    }

    /// The finished job, or `None` with the errors shown when a required
    /// field is empty.
    pub fn submit(&mut self) -> Option<Submission> {
        self.attempted = true;
        let name = self.name.trim();
        let source = self.source.selected()?;
        let schedule = self.schedule?;
        if name.is_empty() {
            return None;
        }
        Some(Submission {
            name: name.to_owned(),
            source: (*source).to_owned(),
            schedule,
            queue: self.queue,
            notes: self.notes.text().trim().to_owned(),
            start_now: self.start_now,
        })
    }

    pub fn view(&self, keymap: &Keymap<combobox::Action>, narrow: bool) -> Element<'_, Message> {
        let name_error = (self.attempted && self.name.trim().is_empty()).then_some("Name the job.");
        let source_error =
            (self.attempted && self.source.selected().is_none()).then_some("Pick a source.");
        let runs_once = self.schedule == Some(Schedule::Once);

        let schedule = field(
            "Schedule",
            select(&Schedule::ALL[..], self.schedule)
                .width(Length::Fill)
                .on_select(Message::Schedule),
        );
        let queue = field(
            "Queue",
            radio_group(Queue::ALL, Some(self.queue))
                .direction(Direction::Horizontal)
                .on_select(Message::Queue),
        );
        let options: Element<'_, Message> = if narrow {
            column![schedule, queue].spacing(16).into()
        } else {
            row![schedule.width(200), queue].spacing(24).into()
        };

        column![
            field(
                "Name",
                input("nightly-orders-etl", &self.name)
                    .icon(lucide!(Briefcase))
                    .invalid(name_error.is_some())
                    .on_input(Message::Name),
            )
            .required(true)
            .error_maybe(name_error),
            field(
                "Source",
                combobox(&self.source)
                    .placeholder("Search datasets...")
                    .width(Length::Fill)
                    .invalid(source_error.is_some())
                    .keymap(keymap.clone())
                    .on_event(Message::Source),
            )
            .required(true)
            .description("Any table or bucket the platform can read.")
            .error_maybe(source_error),
            options,
            field(
                "Notes",
                textarea(&self.notes)
                    .placeholder("What should the on-call engineer know?")
                    .min_height(64.0)
                    .on_action(Message::Notes),
            )
            .description("Shown beside the job in the list."),
            // A scheduled job waits for its schedule instead.
            checkbox(self.start_now && runs_once)
                .label("Start as soon as a slot is free")
                .on_toggle_maybe(runs_once.then_some(Message::StartNow)),
        ]
        .spacing(16)
        .into()
    }
}
