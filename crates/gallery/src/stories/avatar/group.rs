use iced::widget::{column, row, text};
use iced::{Alignment, Element};
use iced_cube::feedback::avatar::Size;
use iced_cube::{avatar, avatar_group};

const REVIEWERS: [&str; 7] = [
    "Ada Lovelace",
    "Grace Hopper",
    "Alan Turing",
    "Edsger Dijkstra",
    "Barbara Liskov",
    "Ken Thompson",
    "Frances Allen",
];

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let line = |size: Size, max: usize, label: &'static str| {
            row![
                avatar_group(REVIEWERS.map(avatar))
                    .size(size)
                    .max_items(max),
                text(label).size(14),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
        };

        column![
            line(Size::Sm, 5, "Seven reviewers, four shown"),
            line(Size::Md, 4, "Three shown and a count"),
        ]
        .spacing(16)
        .into()
    }
}
