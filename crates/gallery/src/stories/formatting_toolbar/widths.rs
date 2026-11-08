use iced::widget::{self, column};
use iced::{Element, Length, Subscription, Task};
use iced_cube::formatting_toolbar::{self, LINK_FIELD, Output, formatting_toolbar};
use iced_cube::keys::{self, Keymap};
use iced_cube::primitives::slider;
use iced_cube::rich_text_editor::{Action, Content, rich_text_editor};
use iced_cube::toolbar;

const BODY: widget::Id = widget::Id::new("widths-body");

#[derive(Debug, Clone)]
pub enum Message {
    Edit(Action),
    Format(formatting_toolbar::Event),
    Width(f32),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    body: Content,
    formatting: formatting_toolbar::State,
    keymap: Keymap<toolbar::Action>,
    width: f32,
}

impl Default for Example {
    fn default() -> Self {
        let body = Content::with_html(
            "<div>Drag the slider: buttons that do not fit move into <strong>More</strong>.</div>",
        );
        let mut formatting = formatting_toolbar::State::new();
        formatting.sync(&body);
        Self {
            body,
            formatting,
            keymap: toolbar::default_keymap(),
            width: 360.0,
        }
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
            Message::Width(width) => {
                self.width = width;
                None
            }
            Message::Key(key) => self
                .formatting
                .key_event(&self.keymap, &key)
                .and_then(|event| self.formatting.update(event, &self.body)),
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
            column![
                formatting_toolbar(&self.formatting).on_event(Message::Format),
                rich_text_editor(&self.body)
                    .id(BODY)
                    .min_height(72.0)
                    .on_action(Message::Edit),
            ]
            .spacing(4)
            .max_width(self.width),
            slider(240.0..=640.0, self.width)
                .step(10.0)
                .label("Width")
                .format_value(|width| format!("{width:.0}px"))
                .width(Length::Fixed(280.0))
                .on_change(Message::Width),
        ]
        .spacing(24)
        .width(Length::Fill)
        .align_x(iced::Alignment::Center)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
