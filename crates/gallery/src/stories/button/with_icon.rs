use iced::Element;
use iced::widget::row;
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

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
            button("New branch")
                .icon(lucide!(GitBranch))
                .on_press(Message::Pressed),
            button("Continue")
                .trailing_icon(lucide!(ArrowRight))
                .variant(Variant::Outline)
                .on_press(Message::Pressed),
            button("Delete")
                .icon(lucide!(Trash))
                .variant(Variant::Destructive)
                .on_press(Message::Pressed),
        ]
        .spacing(8)
        .wrap()
        .into()
    }
}
