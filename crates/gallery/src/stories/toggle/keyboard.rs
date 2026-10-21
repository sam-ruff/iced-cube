use iced::widget::{column, row, text};
use iced::{Element, Length, Subscription};
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::lucide;
use iced_cube::primitives::toggle::{self, Action, toggle};

#[derive(Debug, Clone)]
pub enum Message {
    Bold(bool),
    Italic(bool),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    bold: bool,
    italic: bool,
    bold_keys: Keymap<Action>,
    italic_keys: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        // Each toggle gets its own chord in place of the default Space.
        let only = |chord: Chord| toggle::default_keymap().clear().bind(chord, Action::Toggle);
        Self {
            bold: false,
            italic: false,
            bold_keys: only(Chord::character('b').alt()),
            italic_keys: only(Chord::character('i').alt()),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Bold(on) => self.bold = on,
            Message::Italic(on) => self.italic = on,
            Message::Key(key) => {
                if let Some(action) = self.bold_keys.resolve_event(&key) {
                    self.bold = action.apply(self.bold);
                }
                if let Some(action) = self.italic_keys.resolve_event(&key) {
                    self.italic = action.apply(self.italic);
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            row![
                toggle("Bold")
                    .icon(lucide!(Bold))
                    .pressed(self.bold)
                    .on_toggle(Message::Bold),
                toggle("Italic")
                    .icon(lucide!(Italic))
                    .pressed(self.italic)
                    .on_toggle(Message::Italic),
            ]
            .spacing(4),
            text("Alt+B and Alt+I switch the toggles from anywhere.").size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(360)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
