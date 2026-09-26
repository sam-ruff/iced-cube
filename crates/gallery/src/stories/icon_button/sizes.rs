use iced::widget::row;
use iced::{Alignment, Element};
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::{icon_button, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Pressed,
}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        row([Size::Sm, Size::Md, Size::Lg].map(|size| {
            icon_button(lucide!(Settings))
                .label(format!("Settings ({size:?})"))
                .variant(Variant::Outline)
                .size(size)
                .on_press(Message::Pressed)
                .into()
        }))
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }
}
