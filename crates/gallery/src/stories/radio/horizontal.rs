use iced::Element;
use iced_cube::primitives::radio::Direction;
use iced_cube::primitives::radio_group;

#[derive(Debug, Clone)]
pub enum Message {
    Selected(&'static str),
}

#[derive(Debug)]
pub struct Example {
    size: &'static str,
}

impl Default for Example {
    fn default() -> Self {
        Self { size: "Medium" }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(size) = message;
        self.size = size;
    }

    pub fn view(&self) -> Element<'_, Message> {
        radio_group(["Small", "Medium", "Large"], Some(self.size))
            .direction(Direction::Horizontal)
            .on_select(Message::Selected)
            .into()
    }
}
