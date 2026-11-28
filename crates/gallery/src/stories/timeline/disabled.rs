use iced::widget::container;
use iced::{Element, Length};
use iced_cube::data::timeline::{self, Span, Tone, lane};

#[derive(Debug, Clone)]
pub enum Message {}

#[derive(Debug, Default)]
pub struct Example;

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {}
    }

    pub fn view(&self) -> Element<'_, Message> {
        let steps = [
            Tone::Neutral,
            Tone::Success,
            Tone::Neutral,
            Tone::Destructive,
            Tone::Neutral,
        ];
        // Without on_select the timeline only shows where a run got to.
        container(
            timeline::timeline(steps, 3)
                .lane(lane("Upload", [Span::new(1, 3).tone(Tone::Destructive)])),
        )
        .width(Length::Fill)
        .max_width(480)
        .into()
    }
}
