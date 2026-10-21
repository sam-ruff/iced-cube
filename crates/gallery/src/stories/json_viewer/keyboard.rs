use iced::widget::{self, column, text};
use iced::{Element, Length, Task};
use iced_cube::data::json_viewer::{self, Action, Event, Output, State, json_viewer};
use iced_cube::keys::{Chord, Keymap};
use iced_cube::theme::text_size;
use serde_json::json;

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Viewer(Event),
}

#[derive(Debug)]
pub struct Example {
    package: State,
    keymap: Keymap<Action>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let package = State::new(json!({
            "name": "iced-cube",
            "version": "0.0.3",
            "private": false,
            "scripts": { "build": "cargo build", "test": "cargo test --workspace" },
            "keywords": ["iced", "gui", "components"],
            "engines": { "rust": ">=1.88" }
        }))
        .with_depth(1);

        // The defaults, plus J and K.
        let keymap = json_viewer::default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .bind(Chord::character('k'), Action::Previous);

        Self {
            package,
            keymap,
            last: "Nothing copied yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let Message::Viewer(event) = message;
        match self.package.update(event) {
            Some(Output::CopyValue(value)) => {
                self.last = "Copied the value".into();
                bridge::copy(value)
            }
            Some(Output::CopyPath(path)) => {
                self.last = format!("Copied {path}");
                bridge::copy(path)
            }
            Some(Output::FocusSearch) => widget::operation::focus(json_viewer::SEARCH),
            None => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text(
                "Click a row, then use the arrows or J and K. Shift+Right expands \
                 everything, Ctrl+C copies the value, Ctrl+Shift+C its path and Ctrl+F \
                 searches."
            )
            .size(text_size::SM),
            json_viewer(&self.package)
                .search(true)
                .keymap(self.keymap.clone())
                .height(256)
                .on_event(Message::Viewer),
            text(&self.last).size(text_size::SM),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(560)
        .into()
    }
}
