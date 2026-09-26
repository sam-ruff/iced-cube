use iced::Element;
use iced_cube::primitives::slider;

#[derive(Debug, Clone)]
pub enum Message {
    Volume(u8),
}

#[derive(Debug)]
pub struct Example {
    volume: u8,
}

impl Default for Example {
    fn default() -> Self {
        Self { volume: 40 }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Volume(volume) = message;
        self.volume = volume;
    }

    pub fn view(&self) -> Element<'_, Message> {
        slider(0..=100, self.volume)
            .label("Volume")
            .show_value()
            .width(320)
            .on_change(Message::Volume)
            .into()
    }
}
