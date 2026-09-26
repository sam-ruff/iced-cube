use iced::widget::text_editor::{Action, Content};
use iced::widget::{column, text};
use iced::{Alignment, Element, Length};
use iced_cube::textarea;
use iced_cube::theme::{Tokens, text_size};

const LIMIT: usize = 280;

#[derive(Debug, Clone)]
pub enum Message {
    Edit(Action),
}

#[derive(Debug, Default)]
pub struct Example {
    content: Content,
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Edit(action) = message;
        self.content.perform(action);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let count = self.content.text().trim_end_matches('\n').chars().count();

        column![
            textarea(&self.content)
                .placeholder("Type your message here.")
                .invalid(count > LIMIT)
                .on_action(Message::Edit),
            text(format!("{count} / {LIMIT}"))
                .size(text_size::XS)
                .style(|theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                }),
        ]
        .spacing(8)
        .align_x(Alignment::End)
        .width(Length::Fixed(360.0))
        .into()
    }
}
