use iced::widget::{column, row, text};
use iced::{Alignment, Element};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Increment,
    Reset,
}

#[derive(Debug, Default)]
pub struct Example {
    count: u32,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Increment => self.count += 1,
            Message::Reset => self.count = 0,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let reset = (self.count > 0).then_some(Message::Reset);

        column![
            text(format!("Pressed {} times", self.count)).size(14),
            row![
                button("Press me")
                    .icon(lucide!(Plus))
                    .on_press(Message::Increment),
                button("Reset")
                    .variant(Variant::Secondary)
                    .on_press_maybe(reset),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .align_x(Alignment::Center)
        .into()
    }
}
