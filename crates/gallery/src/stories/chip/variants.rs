use iced::widget::{column, row};
use iced::{Alignment, Element};
use iced_cube::chip;
use iced_cube::feedback::chip::{Size, Variant};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let line = |size: Size| {
            row(Variant::ALL.map(|variant| {
                chip(format!("{variant:?}"))
                    .variant(variant)
                    .size(size)
                    .into()
            }))
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap()
            .vertical_spacing(8)
        };

        column![line(Size::Md), line(Size::Sm)].spacing(16).into()
    }
}
