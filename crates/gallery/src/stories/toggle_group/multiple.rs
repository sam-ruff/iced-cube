use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::lucide;
use iced_cube::primitives::toggle_group::{self, State, item, toggle_group};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Bold,
    Italic,
    Underline,
    Code,
}

#[derive(Debug, Clone)]
pub enum Message {
    Format(toggle_group::Event<Format>),
}

#[derive(Debug)]
pub struct Example {
    format: State<Format>,
}

impl Default for Example {
    fn default() -> Self {
        let format = State::multiple([
            item(Format::Bold, "Bold").icon(lucide!(Bold)),
            item(Format::Italic, "Italic").icon(lucide!(Italic)),
            item(Format::Underline, "Underline").icon(lucide!(Underline)),
            item(Format::Code, "Code")
                .icon(lucide!(Code))
                .disabled(true),
        ]);
        Self {
            format: format.with_selected([Format::Bold]),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Format(event) = message;
        let _ = self.format.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let on: Vec<String> = self
            .format
            .selected()
            .iter()
            .map(|format| format!("{format:?}").to_lowercase())
            .collect();
        let status = match on.as_slice() {
            [] => String::from("Plain text"),
            names => format!("Formatting: {}", names.join(", ")),
        };

        column![
            toggle_group(&self.format).on_event(Message::Format),
            text(status).size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(420)
        .into()
    }
}
