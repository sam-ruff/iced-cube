use iced::Element;
use iced::widget::column;
use iced_cube::primitives::slider;

#[derive(Debug, Clone)]
pub enum Message {
    Brightness(u8),
}

#[derive(Debug)]
pub struct Example {
    brightness: u8,
}

impl Default for Example {
    fn default() -> Self {
        Self { brightness: 70 }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Brightness(brightness) = message;
        self.brightness = brightness;
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            slider(0..=100, self.brightness)
                .label("Brightness")
                .show_value()
                .on_change(Message::Brightness),
            slider(0..=100, 30_u8)
                .label("Contrast (locked)")
                .show_value(),
            slider(0..=100, 55_u8),
        ]
        .spacing(24)
        .width(320)
        .into()
    }
}
