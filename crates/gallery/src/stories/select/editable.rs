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
        let times = (0..48)
            .map(|half| format!("{:02}:{:02}", half / 2, half % 2 * 30))
            .collect();
        Self {
            times,
            start: "09:00".to_owned(),
        }
    }
}

/// Reads "9", "0940", "9:40" or "9.40" as a time of day.
fn parse(typed: &str) -> Option<String> {
    let typed = typed.replace('.', ":");
    let (hours, minutes) = match typed.split_once(':') {
        Some((hours, minutes)) if minutes.len() == 2 => (hours, minutes),
        Some(_) => return None,
        None if typed.len() <= 2 => (typed.as_str(), "00"),
        None if typed.len() == 4 => typed.split_at(2),
        None => return None,
    };
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
            // Type straight into the field: "9" filters the half hours,
            // and Enter takes it as 09:00. A time off the list, such as
            // 16:41, shows as selected.
            select(self.times.as_slice(), Some(self.start.clone()))
                .editable(true)
                .searchable(true)
                .on_custom(|typed| parse(&typed).map(Message::Selected))
                .prefer_custom(true)
                .commit_on_leave(true)
                .not_valid("Use hh:mm")
                .width(120)
                .on_select(Message::Selected),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into()
    }
}
