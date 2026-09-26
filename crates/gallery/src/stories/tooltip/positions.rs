use iced::Element;
use iced::widget::row;
use iced_cube::button;
use iced_cube::overlay::tooltip::{Position, tooltip};
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone)]
pub enum Message {
    Pressed,
}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, _message: Message) {}

    pub fn view(&self) -> Element<'_, Message> {
        let sides = [
            Position::Top,
            Position::Bottom,
            Position::Left,
            Position::Right,
        ];

        row(sides.map(|position| {
            tooltip(
                button(format!("{position:?}"))
                    .variant(Variant::Outline)
                    .on_press(Message::Pressed),
                format!("Shown on the {position:?}").to_lowercase(),
            )
            .position(position)
            .into()
        }))
        .spacing(8)
        .into()
    }
}
