use iced::Element;
use iced::widget::column;
use iced_cube::primitives::radio;

#[derive(Debug, Clone)]
pub enum Message {
    Selected(u8),
}

#[derive(Debug)]
pub struct Example {
    plan: u8,
}

impl Default for Example {
    fn default() -> Self {
        Self { plan: 1 }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(plan) = message;
        self.plan = plan;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let selected = Some(self.plan);

        column![
            radio("Starter", 1, selected).on_select(Message::Selected),
            radio("Professional", 2, selected).on_select(Message::Selected),
            radio("Enterprise (contact sales)", 3, selected),
        ]
        .spacing(12)
        .into()
    }
}
