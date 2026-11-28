use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::data::timeline::{self, Tone};

const STEPS: [(&str, Tone); 12] = [
    ("Launch", Tone::Neutral),
    ("Sign in", Tone::Success),
    ("Open inbox", Tone::Neutral),
    ("Search", Tone::Neutral),
    ("Open message", Tone::Success),
    ("Reply", Tone::Neutral),
    ("Attach file", Tone::Warning),
    ("Send", Tone::Neutral),
    ("Check sent", Tone::Destructive),
    ("Retry", Tone::Neutral),
    ("Check sent", Tone::Success),
    ("Sign out", Tone::Neutral),
];

#[derive(Debug, Clone)]
pub enum Message {
    Step(usize),
}

#[derive(Debug, Default)]
pub struct Example {
    current: usize,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Step(step) = message;
        self.current = step;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let (name, _) = STEPS[self.current];
        column![
            timeline::timeline(STEPS.map(|(_, tone)| tone), self.current).on_select(Message::Step),
            text(format!(
                "Step {} of {}: {name}",
                self.current + 1,
                STEPS.len()
            ))
            .size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(480)
        .into()
    }
}
