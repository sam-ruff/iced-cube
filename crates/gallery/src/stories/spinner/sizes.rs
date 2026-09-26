use iced::time::Instant;
use iced::widget::row;
use iced::{Alignment, Element, Subscription, window};
use iced_cube::feedback::spinner::{Size, advance};
use iced_cube::spinner;

#[derive(Debug, Clone)]
pub enum Message {
    Frame(Instant),
}

#[derive(Debug)]
pub struct Example {
    phase: f32,
    last_frame: Option<Instant>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            phase: 0.15,
            last_frame: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Frame(now) = message;
        if let Some(last) = self.last_frame {
            self.phase = advance(self.phase, now - last);
        }
        self.last_frame = Some(now);
    }

    pub fn view(&self) -> Element<'_, Message> {
        row(Size::ALL.map(|size| spinner(self.phase).size(size).into()))
            .spacing(24)
            .align_y(Alignment::Center)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        window::frames().map(Message::Frame)
    }
}
