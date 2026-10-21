use iced::widget::column;
use iced::{Element, Length, Task};
use iced_cube::data::json_viewer::{Event, Output, State, json_viewer};
use iced_cube::input;

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Edit(String),
    Viewer(Event),
}

#[derive(Debug)]
pub struct Example {
    text: String,
    parsed: State,
}

impl Default for Example {
    fn default() -> Self {
        let text = r#"{"name": "Ada", "roles": ["admin" "billing"]}"#.to_owned();
        Self {
            parsed: State::parse(&text),
            text,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(text) => {
                self.parsed = State::parse(&text).with_depth(1);
                self.text = text;
                Task::none()
            }
            Message::Viewer(event) => match self.parsed.update(event) {
                Some(Output::CopyValue(text) | Output::CopyPath(text)) => bridge::copy(text),
                _ => Task::none(),
            },
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            input("Paste JSON", &self.text).on_input(Message::Edit),
            json_viewer(&self.parsed).on_event(Message::Viewer),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(560)
        .into()
    }
}
