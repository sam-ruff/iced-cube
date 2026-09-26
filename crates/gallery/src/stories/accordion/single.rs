use iced::widget::{container, text};
use iced::{Element, Length};
use iced_cube::layout::accordion::{self, Mode, State, accordion};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    Shipping,
    Returns,
    Support,
}

#[derive(Debug, Clone)]
pub enum Message {
    Accordion(accordion::Event<Question>),
}

#[derive(Debug)]
pub struct Example {
    sections: State<Question>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            sections: State::new(Mode::Single).with_open(Question::Shipping),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Accordion(event) = message;
        let _ = self.sections.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let answer = |body| text(body).size(14);

        container(
            accordion(&self.sections)
                .item(
                    Question::Shipping,
                    "How long does shipping take?",
                    answer("Orders leave the warehouse within two working days."),
                )
                .item(
                    Question::Returns,
                    "Can I return an item?",
                    answer("Yes, within 30 days, as long as it is unused."),
                )
                .item(
                    Question::Support,
                    "How do I contact support?",
                    answer("Email us any time and we reply within a day."),
                )
                .on_event(Message::Accordion),
        )
        .width(Length::Fixed(440.0))
        .into()
    }
}
