use iced::widget::{column, row, text};
use iced::{Element, Length, Task};
use iced_cube::button;
use iced_cube::data::json_viewer::{Event, Output, State, json_viewer};
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::theme::text_size;
use serde_json::{Value, json};

use crate::bridge;

#[derive(Debug, Clone)]
pub enum Message {
    Viewer(Event),
}

#[derive(Debug)]
pub struct Example {
    orders: State,
}

impl Default for Example {
    fn default() -> Self {
        let orders: Vec<Value> = (1..=1_500)
            .map(|number| {
                let country = ["GB", "DE", "US"][number % 3];
                json!({
                    "id": format!("ORD-{number:05}"),
                    "total": (number * 37 % 900) as f64 + 0.99,
                    "paid": number % 3 != 0,
                    "customer": { "id": number % 120, "country": country },
                    "items": [number % 7 + 1, number % 5 + 1]
                })
            })
            .collect();

        Self {
            orders: State::new(json!({ "count": orders.len(), "orders": orders })).with_depth(2),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let Message::Viewer(event) = message;
        match self.orders.update(event) {
            Some(Output::CopyValue(text) | Output::CopyPath(text)) => bridge::copy(text),
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let control = |label, event| {
            button(label)
                .variant(Variant::Outline)
                .size(Size::Sm)
                .on_press(Message::Viewer(event))
        };

        column![
            row![
                control("Expand all", Event::ExpandAll),
                control("Two levels", Event::ExpandToDepth(2)),
                control("Collapse all", Event::CollapseAll),
            ]
            .spacing(8)
            .wrap(),
            json_viewer(&self.orders)
                .height(312)
                .on_event(Message::Viewer),
            text(format!(
                "{} nodes, {} rows shown",
                self.orders.len(),
                self.orders.visible().len()
            ))
            .size(text_size::SM),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(560)
        .into()
    }
}
