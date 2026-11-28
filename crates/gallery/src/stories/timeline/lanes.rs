use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::data::timeline::{self, Span, Tone, lane};

const STEPS: usize = 16;

#[derive(Debug, Clone)]
pub enum Message {
    Step(usize),
}

#[derive(Debug)]
pub struct Example {
    current: usize,
}

impl Default for Example {
    fn default() -> Self {
        Self { current: 6 }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Step(step) = message;
        self.current = step;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut steps = [Tone::Neutral; STEPS];
        steps[9] = Tone::Destructive;
        steps[13] = Tone::Success;

        column![
            timeline::timeline(steps, self.current)
                .lane(lane("Load notes", [Span::new(1, 5).tone(Tone::Success)]))
                .lane(lane(
                    "Sync",
                    [
                        Span::new(3, 4),
                        Span::new(8, 9).tone(Tone::Destructive),
                        Span::new(11, 13).tone(Tone::Success),
                    ],
                ))
                .lane(lane("Thumbnails", [Span::new(6, 15).tone(Tone::Warning)]))
                .on_select(Message::Step),
            text(format!("Step {} of {STEPS}", self.current + 1)).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(520)
        .into()
    }
}
