use iced::widget::column;
use iced::{Element, Length, Task};
use iced_cube::context_menu;
use iced_cube::data::json_viewer::{self, Event, MenuItem, NodeId, Output, State, json_viewer};
use iced_cube::dropdown_menu;
use serde_json::json;

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Viewer(Event),
    Menu(context_menu::Event<MenuItem, NodeId>),
}

#[derive(Debug)]
pub struct Example {
    response: State,
    menu: context_menu::State<MenuItem, NodeId>,
}

impl Default for Example {
    fn default() -> Self {
        let response = State::new(json!({
            "status": 200,
            "ok": true,
            "requestId": "req_8f2c41d7a9",
            "data": {
                "user": {
                    "id": 4821,
                    "name": "Ada Lovelace",
                    "email": "ada@example.com",
                    "roles": ["admin", "billing"],
                    "lastLogin": null
                },
                "plan": { "name": "Team", "seats": 12, "price": 49.5 }
            },
            "links": {
                "self": "https://api.example.com/v2/users/4821?include=plan,roles,preferences",
                "next": null
            }
        }))
        .with_depth(2);

        Self {
            response,
            menu: json_viewer::menu(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Viewer(event) => self.viewer(event),
            Message::Menu(event) => {
                let chosen = self.menu.update(event);
                match (chosen, self.menu.target()) {
                    (Some(dropdown_menu::Output::Activated(item)), Some(&id)) => {
                        self.viewer(item.event(id))
                    }
                    _ => Task::none(),
                }
            }
        }
    }

    fn viewer(&mut self, event: Event) -> Task<Message> {
        match self.response.update(event) {
            Some(Output::CopyValue(text) | Output::CopyPath(text)) => bridge::copy(text),
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            json_viewer(&self.response)
                .height(352)
                .context_menu(&self.menu, Message::Menu)
                .on_event(Message::Viewer),
        ]
        .width(Length::Fill)
        .max_width(560)
        .into()
    }
}
