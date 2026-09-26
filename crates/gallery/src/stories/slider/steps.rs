use iced::Element;
use iced::widget::column;
use iced_cube::primitives::slider;

#[derive(Debug, Clone)]
pub enum Message {
    Opacity(f32),
    Quality(f32),
}

#[derive(Debug)]
pub struct Example {
    opacity: f32,
    quality: f32,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            opacity: 0.75,
            quality: 3.0,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Opacity(opacity) => self.opacity = opacity,
            Message::Quality(quality) => self.quality = quality,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            slider(0.0..=1.0, self.opacity)
                .step(0.25)
                .label("Opacity")
                .format_value(|value| format!("{:.0}%", value * 100.0))
                .on_change(Message::Opacity),
            slider(1.0..=5.0, self.quality)
                .step(1.0)
                .label("Quality")
                .format_value(|value| format!("{value:.0} of 5"))
                .on_change(Message::Quality),
        ]
        .spacing(24)
        .width(320)
        .into()
    }
}
