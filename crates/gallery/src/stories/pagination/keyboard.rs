use iced::widget::{column, text};
use iced::{Alignment, Element, Subscription};
use iced_cube::keys::{self, Keymap};
use iced_cube::navigation::pagination::{self, Action, Event, State, pagination};

#[derive(Debug, Clone)]
pub enum Message {
    Page(Event),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    pages: State,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            pages: State::new(12).with_page(4),
            keymap: pagination::default_keymap(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Page(event) => Some(event),
            Message::Key(key) => self
                .keymap
                .resolve_event(&key)
                .and_then(|action| action.event(&self.pages)),
        };
        if let Some(event) = event {
            let _ = self.pages.update(event);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            pagination(&self.pages).on_event(Message::Page),
            text("Page Up and Page Down step a page.").size(14),
            text("Ctrl+Home and Ctrl+End jump.").size(14),
        ]
        .spacing(8)
        .align_x(Alignment::Center)
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
