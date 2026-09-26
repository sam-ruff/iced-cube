use iced::widget::{container, text};
use iced::{Element, Length};
use iced_cube::layout::accordion::{self, Mode, State, accordion};

#[derive(Debug, Clone)]
pub enum Message {
    Accordion(accordion::Event<&'static str>),
}

#[derive(Debug)]
pub struct Example {
    sections: State<&'static str>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            sections: State::new(Mode::Multiple)
                .with_open("general")
                .with_open("privacy"),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Accordion(event) = message;
        let _ = self.sections.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            accordion(&self.sections)
                .item(
                    "general",
                    "General",
                    text("Language, region and start page.").size(14),
                )
                .item(
                    "privacy",
                    "Privacy",
                    text("Who can see your profile and activity.").size(14),
                )
                .item(
                    "alerts",
                    "Notifications",
                    text("Email and desktop alerts.").size(14),
                )
                .on_event(Message::Accordion),
        )
        .width(Length::Fixed(440.0))
        .into()
    }
}
