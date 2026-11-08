use iced::widget::column;
use iced::{Element, Length};
use iced_cube::rich_text_editor::{Action, Content, rich_text_editor};

#[derive(Debug, Clone)]
pub enum Message {
    Edit(Action),
}

#[derive(Debug)]
pub struct Example {
    reply: Content,
    sent: Content,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            reply: Content::new(),
            sent: Content::with_html(
                "<div>This message was <strong>sent</strong> and can no longer be edited.</div>",
            ),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Edit(action) = message;
        self.reply.perform(action);
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            rich_text_editor(&self.reply)
                .placeholder("Write a reply")
                .min_height(72.0)
                .on_action(Message::Edit),
            rich_text_editor(&self.sent).min_height(48.0),
        ]
        .spacing(16)
        .max_width(480)
        .width(Length::Fill)
        .into()
    }
}
