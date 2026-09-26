use iced::Element;
use iced_cube::forms::combobox::{self, Event, State, combobox};

const LANGUAGES: &[&str] = &["English", "French", "German", "Italian", "Spanish", "Welsh"];

#[derive(Debug, Clone)]
pub enum Message {
    Language(combobox::Event),
}

#[derive(Debug)]
pub struct Example {
    language: State<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        let mut language = State::new(LANGUAGES.iter().copied());
        let _ = language.update(Event::Input("Klingon".to_owned()));
        Self { language }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Language(event) = message;
        let _ = self.language.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        combobox(&self.language)
            .placeholder("Search languages...")
            .empty("No language found.")
            .width(280)
            .on_event(Message::Language)
            .into()
    }
}
