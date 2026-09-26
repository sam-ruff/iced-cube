use iced::widget::row;
use iced::{Alignment, Element};
use iced_cube::badge;
use iced_cube::feedback::badge::Variant;

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        row(Variant::ALL.map(|variant| badge(format!("{variant:?}")).variant(variant).into()))
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap()
            .into()
    }
}
