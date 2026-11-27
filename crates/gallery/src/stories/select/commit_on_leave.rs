use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length};
use iced_cube::button;
use iced_cube::forms::select;

#[derive(Debug, Clone)]
pub enum Message {
    Selected(String),
    Save,
}

#[derive(Debug)]
pub struct Example {
    times: Vec<String>,
    start: String,
    saved: String,
}

impl Default for Example {
    fn default() -> Self {
        let times = (0..48)
            .map(|half| format!("{:02}:{:02}", half / 2, half % 2 * 30))
            .collect();
        Self {
            times,
            start: "09:00".to_owned(),
            saved: "09:00".to_owned(),
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
        match message {
            Message::Selected(start) => self.start = start,
            Message::Save => self.saved = self.start.clone(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text("Starts").size(14),
            row![
                // Open the list, type 9:40 and press Save: the time is
                // entered as the list closes, and the same click saves.
                // A time that cannot be read keeps the list open.
                select(self.times.as_slice(), Some(self.start.clone()))
                    .on_custom(|typed| parse(&typed).map(Message::Selected))
                    .commit_on_leave(true)
                    .pass_through(true)
                    .not_valid("Use hh:mm")
                    .width(160)
                    .on_select(Message::Selected),
                button("Save").on_press(Message::Save),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(format!("Saved {}", self.saved)).size(14),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into()
    }
}
