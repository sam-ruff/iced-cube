use iced::Element;
use iced::widget::{column, row};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide, progress};

const STEPS: u8 = 10;

#[derive(Debug, Clone)]
pub enum Message {
    Step,
    Reset,
}

#[derive(Debug)]
pub struct Example {
    step: u8,
}

impl Default for Example {
    fn default() -> Self {
        Self { step: 3 }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Step => self.step = (self.step + 1).min(STEPS),
            Message::Reset => self.step = 0,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let done = self.step == STEPS;

        column![
            progress(f32::from(self.step) / f32::from(STEPS))
                .label(if done { "Done" } else { "Installing" })
                .show_percentage(true),
            row![
                button("Step")
                    .icon(lucide!(Plus))
                    .on_press_maybe((!done).then_some(Message::Step)),
                button("Reset")
                    .variant(Variant::Outline)
                    .on_press(Message::Reset),
            ]
            .spacing(8),
        ]
        .spacing(16)
        .width(360)
        .into()
    }
}
