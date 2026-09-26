use iced::widget::{column, text};
use iced::{Element, Subscription};
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::primitives::slider::{self, Action};

const RANGE: std::ops::RangeInclusive<u8> = 0..=100;
const STEP: u8 = 5;

#[derive(Debug, Clone)]
pub enum Message {
    Volume(u8),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    volume: u8,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            volume: 40,
            keymap: slider::default_keymap().bind(Chord::character('m'), Action::Min),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Volume(volume) => self.volume = volume,
            Message::Key(key) => {
                if let Some(action) = self.keymap.resolve_event(&key) {
                    self.volume = action.apply(self.volume, RANGE, Some(STEP));
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            slider::slider(RANGE, self.volume)
                .step(STEP)
                .label("Volume")
                .show_value()
                .width(320)
                .on_change(Message::Volume),
            text("Arrow keys step by 5, Home and End jump to the ends, and M mutes.").size(14),
        ]
        .spacing(16)
        .width(320)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
