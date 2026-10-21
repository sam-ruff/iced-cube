use iced::widget::{self, column};
use iced::{Element, Length, Task};
use iced_cube::data::json_viewer::{self, Event, Output, State, json_viewer};
use serde_json::json;

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Viewer(Event),
}

#[derive(Debug)]
pub struct Example {
    team: State,
}

impl Default for Example {
    fn default() -> Self {
        let mut team = State::new(json!({
            "team": "Platform",
            "members": [
                { "name": "Grace Hopper", "role": "lead", "timezone": "America/New_York" },
                { "name": "Alan Turing", "role": "engineer", "timezone": "Europe/London" },
                { "name": "Katherine Johnson", "role": "engineer", "timezone": "America/New_York" },
                { "name": "Edsger Dijkstra", "role": "reviewer", "timezone": "Europe/Amsterdam" }
            ],
            "oncall": { "primary": "Alan Turing", "secondary": "Grace Hopper" }
        }));
        let _ = team.update(Event::Search("america".into()));

        Self { team }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let Message::Viewer(event) = message;
        match self.team.update(event) {
            Some(Output::CopyValue(text) | Output::CopyPath(text)) => bridge::copy(text),
            Some(Output::FocusSearch) => widget::operation::focus(json_viewer::SEARCH),
            None => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            json_viewer(&self.team)
                .search(true)
                .height(296)
                .on_event(Message::Viewer),
        ]
        .width(Length::Fill)
        .max_width(560)
        .into()
    }
}
