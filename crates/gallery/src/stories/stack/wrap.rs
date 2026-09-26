use iced::Element;
use iced_cube::feedback::badge::Variant;
use iced_cube::layout::stack::Gap;
use iced_cube::{badge, hstack};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

const TOPICS: [&str; 14] = [
    "Rust",
    "GUI",
    "Desktop",
    "Wasm",
    "Themes",
    "Icons",
    "Layout",
    "Forms",
    "Tables",
    "Charts",
    "Dialogs",
    "Menus",
    "Testing",
    "Accessibility",
];

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        hstack(TOPICS.map(|topic| badge(topic).variant(Variant::Outline).into()))
            .gap(Gap::Sm)
            .wrap()
            .width(360)
            .into()
    }
}
