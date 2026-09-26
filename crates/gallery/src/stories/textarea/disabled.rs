use iced::widget::row;
use iced::widget::text_editor::{Action, Content};
use iced::{Element, Length};
use iced_cube::textarea;

#[derive(Debug, Clone)]
pub enum Message {
    Edit(Action),
}

#[derive(Debug)]
pub struct Example {
    draft: Content,
    archived: Content,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            draft: Content::with_text("Fixed height, scrolls once the text overflows."),
            archived: Content::with_text("This note is archived and can no longer be edited."),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Edit(action) = message;
        self.draft.perform(action);
    }

    pub fn view(&self) -> Element<'_, Message> {
        row![
            textarea(&self.draft).height(120).on_action(Message::Edit),
            textarea(&self.archived).height(120),
        ]
        .spacing(16)
        .width(Length::Fixed(520.0))
        .into()
    }
}
