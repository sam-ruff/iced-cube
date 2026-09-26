use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::forms::combobox::{self, Action, State, combobox};
use iced_cube::keys::{Chord, Keymap};

const FRAMEWORKS: &[&str] = &[
    "Actix", "Axum", "Bevy", "Dioxus", "Iced", "Leptos", "Rocket", "Tauri", "Yew",
];

#[derive(Debug, Clone)]
pub enum Message {
    Framework(combobox::Event),
}

#[derive(Debug)]
pub struct Example {
    framework: State<&'static str>,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        let keymap = combobox::default_keymap()
            .bind(Chord::character('n').ctrl(), Action::Next)
            .bind(Chord::character('p').ctrl(), Action::Previous);

        Self {
            framework: State::new(FRAMEWORKS.iter().copied()).with_visible_rows(5),
            keymap,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Framework(event) = message;
        let _ = self.framework.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let chosen = self.framework.selected().copied().unwrap_or("nothing yet");

        column![
            text("Click the field, then use the arrow keys or Ctrl+N and Ctrl+P. Enter chooses, Escape closes.")
                .size(14),
            text(format!("Chosen: {chosen}")).size(14),
            combobox(&self.framework)
                .placeholder("Search crates...")
                .keymap(self.keymap.clone())
                .on_event(Message::Framework),
        ]
        .spacing(12)
        .width(Length::Fixed(360.0))
        .into()
    }
}
