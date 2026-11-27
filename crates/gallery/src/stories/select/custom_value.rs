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
            start: "09:00".to_owned(),
        }
    }
}

/// Reads "9:40" or "09:40" as a time of day, or `None` for anything else.
fn parse(typed: &str) -> Option<String> {
    let (hours, minutes) = typed.split_once(':')?;
    if minutes.len() != 2 {
        return None;
    }
    let (hours, minutes): (u8, u8) = (hours.parse().ok()?, minutes.parse().ok()?);
    (hours < 24 && minutes < 60).then(|| format!("{hours:02}:{minutes:02}"))
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(start) = message;
        self.start = start;
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text("Starts").size(14),
            // Type a time off the quarter hours, such as 9:40, and press
            // Enter. A time that cannot be read stays in red.
            select(self.times.as_slice(), Some(self.start.clone()))
                .searchable(true)
                .search_placeholder("hh:mm")
                .on_custom(|typed| parse(&typed).map(Message::Selected))
                .width(120)
                .on_select(Message::Selected),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into()
    }
}
