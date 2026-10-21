use iced::widget::{column, text};
use iced::{Element, Length, Subscription};
use iced_cube::keys::{self, Keymap};
use iced_cube::navigation::breadcrumb::{self, Action};
use iced_cube::{crumb, lucide};

const PATH: [&str; 5] = ["Library", "Music", "Albums", "1997", "Disc one"];

#[derive(Debug, Clone)]
pub enum Message {
    Open(usize),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    depth: usize,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            depth: PATH.len(),
            keymap: breadcrumb::default_keymap(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let target = match message {
            Message::Open(index) => Some(index),
            Message::Key(key) => self
                .keymap
                .resolve_event(&key)
                .and_then(|action| action.target(self.depth)),
        };
        if let Some(index) = target {
            self.depth = index + 1;
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let crumbs = PATH[..self.depth].iter().enumerate().map(|(index, name)| {
            let crumb = crumb(*name).on_press(Message::Open(index));
            if index == 0 {
                crumb.icon(lucide!(Library))
            } else {
                crumb
            }
        });

        column![
            breadcrumb::breadcrumb(crumbs),
            text("Alt+Up goes up a level and Alt+Shift+Up to the top.").size(14),
        ]
        .spacing(12)
        .width(Length::Fill)
        .max_width(380)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
