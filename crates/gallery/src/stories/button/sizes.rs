use iced::widget::row;
use iced::{Alignment, Element};
use iced_cube::primitives::button::Size;
use iced_cube::{button, icon_button, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Pressed,
}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        row![
            button("Small").size(Size::Sm).on_press(Message::Pressed),
            button("Medium").size(Size::Md).on_press(Message::Pressed),
            button("Large").size(Size::Lg).on_press(Message::Pressed),
            icon_button(lucide!(ArrowUpRight)).on_press(Message::Pressed),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }
}
