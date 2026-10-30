use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length};
use iced_cube::{fab, lucide};

#[derive(Debug, Clone)]
pub enum Message {
    Compose,
    Share,
}

#[derive(Debug, Default)]
pub struct Example {
    pressed: Option<Message>,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        self.pressed = Some(message);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let status = match &self.pressed {
            Some(message) => format!("Pressed {message:?}"),
            None => "Press a button.".to_owned(),
        };

        column![
            row![
                fab(lucide!(Pencil))
                    .label("Compose")
                    .on_press(Message::Compose),
                fab(lucide!(Share2))
                    .tooltip("Share")
                    .on_press(Message::Share),
                fab(lucide!(Plus)).label("Disabled"),
            ]
            .spacing(16)
            .align_y(Alignment::Center)
            .wrap(),
            text(status).size(14),
        ]
        .spacing(16)
        .width(Length::Fill)
        .max_width(340)
        .into()
    }
}
