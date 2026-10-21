use iced::widget::{column, row};
use iced::{Alignment, Element};
use iced_cube::avatar;
use iced_cube::feedback::avatar::{Shape, Size};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let line = |shape: Shape, name: &str| {
            row(Size::ALL.map(|size| avatar(name).size(size).shape(shape).into()))
                .spacing(12)
                .align_y(Alignment::Center)
        };

        column![
            line(Shape::Circle, "Ada Lovelace"),
            line(Shape::Square, "Platform team"),
        ]
        .spacing(16)
        .into()
    }
}
