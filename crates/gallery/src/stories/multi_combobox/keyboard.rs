use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::forms::multi_combobox::{self, Action, State, multi_combobox};
use iced_cube::keys::{Chord, Keymap};

const CRATES: &[&str] = &[
    "Actix", "Axum", "Bevy", "Dioxus", "Iced", "Leptos", "Rocket", "Tauri", "Yew",
];

#[derive(Debug, Clone)]
pub enum Message {
    Crates(multi_combobox::Event),
}

#[derive(Debug)]
pub struct Example {
    crates: State<&'static str>,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        let keymap = multi_combobox::default_keymap()
            .bind(Chord::character('n').ctrl(), Action::Next)
            .bind(Chord::character('p').ctrl(), Action::Previous);

        Self {
            crates: State::new(CRATES.iter().copied())
                .with_chosen(&["Iced", "Axum"])
                .with_visible_rows(5),
            keymap,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Crates(event) = message;
        let _ = self.crates.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text(
                "Click the field, then use the arrow keys or Ctrl+N and Ctrl+P. Enter adds or removes, Backspace in the empty field removes the last chip, Escape closes."
            )
            .size(14),
            multi_combobox(&self.crates)
                .placeholder("Search crates...")
                .width(Length::Fill)
                .keymap(self.keymap.clone())
                .on_event(Message::Crates),
        ]
        .spacing(12)
        .width(Length::Fixed(360.0))
        .height(Length::Fill)
        .into()
    }
}
