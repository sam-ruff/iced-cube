use iced::Element;
use iced_cube::primitives::checkbox;

#[derive(Debug, Clone)]
pub enum Message {
    Accept(bool),
}

#[derive(Debug, Default)]
pub struct Example {
    accepted: bool,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Accept(accepted) = message;
        self.accepted = accepted;
    }

    pub fn view(&self) -> Element<'_, Message> {
        checkbox(self.accepted)
            .label("Accept terms and conditions")
            .on_toggle(Message::Accept)
            .into()
    }
}
