use iced::widget::{column, text};
use iced::{Element, Task};
use iced_cube::data::log_viewer::{self, Action, Event, Level, Output, State, entry, log_viewer};
use iced_cube::keys::{Chord, Keymap};
use iced_cube::theme::text_size;

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Logs(Event),
}

#[derive(Debug)]
pub struct Example {
    logs: State,
    keymap: Keymap<Action>,
    copied: String,
}

impl Default for Example {
    fn default() -> Self {
        let mut logs = State::new();
        logs.extend((0..120).map(|n| {
            let level = Level::ALL[n % Level::ALL.len()];
            entry(level, format!("step {n} of the nightly backup")).timestamp(format!(
                "02:{:02}:{:02}.000",
                n / 60,
                n % 60
            ))
        }));

        // The defaults, plus J and K.
        let keymap = log_viewer::default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .bind(Chord::character('k'), Action::Previous);

        Self {
            logs,
            keymap,
            copied: "Nothing copied yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let Message::Logs(event) = message;
        match self.logs.update(event) {
            Some(Output::Copy(lines)) => {
                self.copied = format!("Copied {} line(s)", lines.lines().count());
                bridge::copy(lines)
            }
            Some(Output::FocusSearch) => log_viewer::focus_search(&self.logs),
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text(
                "Click a line, then use the arrows or J and K. Ctrl+F searches, Enter and F3 \
                 step through matches, Shift+arrows select, Ctrl+C copies, Space pauses, W \
                 wraps, End follows the newest line and Ctrl+L clears."
            )
            .size(text_size::SM),
            log_viewer(&self.logs)
                .keymap(self.keymap.clone())
                .height(320)
                .on_event(Message::Logs),
            text(&self.copied).size(text_size::SM),
        ]
        .spacing(12)
        .into()
    }
}
