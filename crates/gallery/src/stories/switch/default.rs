use iced::Element;
use iced_cube::primitives::switch;

#[derive(Debug, Clone)]
pub enum Message {
    AirplaneMode(bool),
}

#[derive(Debug, Default)]
pub struct Example {
    airplane_mode: bool,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::AirplaneMode(on) = message;
        self.airplane_mode = on;
    }

    pub fn view(&self) -> Element<'_, Message> {
        switch(self.airplane_mode)
            .label("Airplane mode")
            .on_toggle(Message::AirplaneMode)
            .into()
    }
}
