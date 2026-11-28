use iced::widget::{column, text};
use iced::{Element, Length, Subscription};
use iced_cube::data::timeline::{self, Action, Tone};
use iced_cube::keys::{self, Chord, Keymap};

const STEPS: usize = 30;

#[derive(Debug, Clone)]
pub enum Message {
    Step(usize),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    current: usize,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            current: 0,
            keymap: timeline::default_keymap()
                .bind(Chord::character('j'), Action::Next)
                .bind(Chord::character('k'), Action::Previous),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Step(step) => self.current = step,
            Message::Key(key) => {
                if let Some(action) = self.keymap.resolve_event(&key) {
                    self.current = action.apply(self.current, STEPS);
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let steps = (0..STEPS).map(|step| {
            if step % 7 == 6 {
                Tone::Warning
            } else {
                Tone::Neutral
            }
        });
        column![
            timeline::timeline(steps, self.current).on_select(Message::Step),
            text(format!("Step {} of {STEPS}", self.current + 1)).size(14),
            text("Arrow keys step, Shift or Page keys jump ten, Home and End go to the ends, and J and K step too.")
                .size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(480)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
