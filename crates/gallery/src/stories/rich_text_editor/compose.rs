use iced::widget::{self, column};
use iced::{Element, Length, Task};
use iced_cube::formatting_toolbar::{self, LINK_FIELD, Output, formatting_toolbar};
use iced_cube::rich_text_editor::{Action, Content, rich_text_editor};

const BODY: widget::Id = widget::Id::new("compose-body");

const DRAFT: &str = "<div>Hi Sam,</div>\
    <div><br></div>\
    <div>The <strong>launch</strong> moves to <em>Thursday</em>. \
    Please <span style=\"font-size:large\">bring</span>:</div>\
    <ul><li>the slides, <u>final</u> version</li><li>the <s>old</s> new budget</li></ul>\
    <div>Notes are <a href=\"https://example.com/notes\">here</a>.</div>";

#[derive(Debug, Clone)]
pub enum Message {
    Edit(Action),
    Format(formatting_toolbar::Event),
}

#[derive(Debug)]
pub struct Example {
    body: Content,
    formatting: formatting_toolbar::State,
}

impl Default for Example {
    fn default() -> Self {
        let body = Content::with_html(DRAFT);
        let mut formatting = formatting_toolbar::State::new();
        formatting.sync(&body);
        Self { body, formatting }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let output = match message {
            Message::Edit(action) => {
                self.body.perform(action);
                self.formatting.sync(&self.body)
            }
            Message::Format(event) => self.formatting.update(event, &self.body),
        };
        match output {
            Some(Output::Format(action)) => {
                self.body.perform(action);
                self.formatting.sync(&self.body);
                widget::operation::focus(BODY)
            }
            Some(Output::EditLink) => widget::operation::focus(LINK_FIELD),
            Some(Output::Closed) => widget::operation::focus(BODY),
            Some(Output::Unfocus) => formatting_toolbar::unfocus(),
            None => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            formatting_toolbar(&self.formatting).on_event(Message::Format),
            rich_text_editor(&self.body)
                .id(BODY)
                .placeholder("Write your message")
                .height(220)
                .on_action(Message::Edit),
        ]
        .spacing(4)
        .max_width(560)
        .width(Length::Fill)
        .into()
    }
}
