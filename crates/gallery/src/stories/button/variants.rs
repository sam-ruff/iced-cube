use iced::Element;
use iced::widget::row;
use iced_cube::button;
use iced_cube::primitives::button::Variant;

#[derive(Debug, Clone)]
pub enum Message {
    Pressed(Variant),
}

#[derive(Debug, Default)]
pub struct Example {
    last: Option<Variant>,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Pressed(variant) = message;
        self.last = Some(variant);
    }

    pub fn view(&self) -> Element<'_, Message> {
        row(Variant::ALL.map(|variant| {
            button(format!("{variant:?}"))
                .variant(variant)
                .on_press(Message::Pressed(variant))
                .into()
        }))
        .spacing(8)
        .wrap()
        .into()
    }
}
