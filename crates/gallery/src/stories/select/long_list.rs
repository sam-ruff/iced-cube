use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::forms::select;

#[derive(Debug, Clone)]
pub enum Message {
    Selected(String),
}

#[derive(Debug)]
pub struct Example {
    times: Vec<String>,
    start: String,
}

impl Default for Example {
    fn default() -> Self {
        let times = (0..96)
            .map(|quarter| format!("{:02}:{:02}", quarter / 4, quarter % 4 * 15))
            .collect();
        Self {
            times,
            start: "14:00".to_owned(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(start) = message;
        self.start = start;
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text("Starts").size(14),
            // 96 options: the list shows eight rows at a time and opens
            // around the selected time.
            select(self.times.as_slice(), Some(self.start.clone()))
                .width(120)
                .on_select(Message::Selected),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into()
    }
}
