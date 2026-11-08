use iced::widget::{column, scrollable, text};
use iced::{Element, Length, Padding};
use iced_cube::rich_text_editor::{Action, Content, rich_text_editor};
use iced_cube::theme::{Tokens, semibold, text_size};

#[derive(Debug, Clone)]
pub enum Message {
    Edit(Action),
}

#[derive(Debug)]
pub struct Example {
    body: Content,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            body: Content::with_html(
                "<div>Thanks, <strong>see you</strong> on <em>Friday</em>.</div>\
                 <ol><li>Agenda</li><li><a href=\"https://example.com\">Map</a></li></ol>",
            ),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Edit(action) = message;
        self.body.perform(action);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let heading = |label| text(label).size(text_size::XS).font(semibold());
        let output = |value: String| {
            text(value)
                .size(text_size::XS)
                .width(Length::Fill)
                .style(|theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                })
        };
        column![
            rich_text_editor(&self.body)
                .min_height(72.0)
                .on_action(Message::Edit),
            scrollable(
                column![
                    heading("Plain text"),
                    output(self.body.plain_text()),
                    heading("HTML"),
                    output(self.body.html()),
                ]
                .spacing(6)
                .padding(Padding::ZERO.right(16)),
            )
            .height(150),
        ]
        .spacing(16)
        .max_width(520)
        .width(Length::Fill)
        .into()
    }
}
