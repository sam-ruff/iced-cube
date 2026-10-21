use iced::keyboard::Modifiers;
use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::data::virtual_list::{self, Action, Event, Mode, Output, State, virtual_list};
use iced_cube::keys::{Chord, Keymap};
use iced_cube::theme::text_size;

#[derive(Debug, Clone)]
pub enum Message {
    Chapters(Event<String>),
}

#[derive(Debug)]
pub struct Example {
    chapters: State<String>,
    keymap: Keymap<Action>,
    last: String,
}

impl Default for Example {
    fn default() -> Self {
        let chapters = (1..=500).map(|n| format!("Chapter {n}"));

        // The defaults, plus J and K.
        let keymap = virtual_list::default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .bind(Chord::character('k'), Action::Previous);

        let mut chapters = State::new(chapters).with_mode(Mode::Multiple);
        let _ = chapters.update(Event::Press(2, Modifiers::empty()));

        Self {
            chapters,
            keymap,
            last: "Nothing opened yet".into(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Chapters(event) = message;
        if let Some(Output::Activated(index)) = self.chapters.update(event) {
            let title = self.chapters.get(index).map_or("", String::as_str);
            self.last = format!("Opened {title}");
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let selected = self.chapters.selected().len();

        column![
            text(
                "Click the list, then use the arrow keys or J and K. Page Up and Page Down \
                 move a screenful, Home and End jump to the ends, Shift+arrows extend the \
                 selection and Enter opens."
            )
            .size(text_size::SM),
            virtual_list(&self.chapters, |_, title: &String| {
                text(title.as_str()).size(text_size::SM).into()
            })
            .keymap(self.keymap.clone())
            .height(200)
            .on_event(Message::Chapters),
            text(format!("{selected} selected. {}", self.last)).size(text_size::SM),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(420)
        .into()
    }
}
