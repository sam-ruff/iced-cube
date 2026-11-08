use iced::widget::{column, row, text};
use iced::{Element, Length, Subscription};
use iced_cube::checkbox;
use iced_cube::keys::{self, Chord};
use iced_cube::rich_text_editor::{Action, Content, Shortcut, Shortcuts, rich_text_editor};
use iced_cube::theme::{Tokens, text_size};

const FORMATTING: [Shortcut; 8] = [
    Shortcut::Bold,
    Shortcut::Italic,
    Shortcut::Underline,
    Shortcut::Strikethrough,
    Shortcut::BulletedList,
    Shortcut::NumberedList,
    Shortcut::Link,
    Shortcut::ClearFormatting,
];

#[derive(Debug, Clone)]
pub enum Message {
    Edit(Action),
    Formatting(bool),
    History(bool),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    body: Content,
    formatting: bool,
    history: bool,
    last: Option<String>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            body: Content::with_text(
                "Select some text and press Ctrl+B, Ctrl+I or Ctrl+U (Cmd on macOS).",
            ),
            formatting: true,
            history: true,
            last: None,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Edit(action) => self.body.perform(action),
            Message::Formatting(on) => self.formatting = on,
            Message::History(on) => self.history = on,
            Message::Key(key) => {
                self.last = Some(Chord::new(key.key, key.modifiers).to_string());
            }
        }
    }

    fn shortcuts(&self) -> Shortcuts {
        let mut shortcuts = Shortcuts::all();
        if !self.formatting {
            for shortcut in FORMATTING {
                shortcuts = shortcuts.without(shortcut);
            }
        }
        if !self.history {
            shortcuts = shortcuts.without(Shortcut::Undo).without(Shortcut::Redo);
        }
        shortcuts
    }

    pub fn view(&self) -> Element<'_, Message> {
        let last = match &self.last {
            Some(chord) => format!("The app last received {chord}."),
            None => String::from("Keys the editor leaves reach the app."),
        };
        column![
            row![
                checkbox(self.formatting)
                    .label("Formatting shortcuts")
                    .on_toggle(Message::Formatting),
                checkbox(self.history)
                    .label("Undo and redo")
                    .on_toggle(Message::History),
            ]
            .spacing(24)
            .wrap(),
            rich_text_editor(&self.body)
                .shortcuts(self.shortcuts())
                .min_height(80.0)
                .on_action(Message::Edit),
            text(last).size(text_size::SM).style(|theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            }),
        ]
        .spacing(16)
        .max_width(480)
        .width(Length::Fill)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
