use iced::widget::{column, text};
use iced::{Element, Length, Subscription};
use iced_cube::keys::{self, Keymap};
use iced_cube::navigation::tabs::{self, Action, Event, State, tab, tabs};

#[derive(Debug, Clone)]
pub enum Message {
    Tabs(Event<u8>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    tabs: State<u8>,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            tabs: State::new([
                tab(1, "Step one"),
                tab(2, "Step two").disabled(true),
                tab(3, "Step three"),
                tab(4, "Step four"),
            ]),
            keymap: tabs::default_keymap(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Tabs(event) => Some(event),
            Message::Key(key) => self
                .keymap
                .resolve_event(&key)
                .and_then(|action| action.event(&self.tabs)),
        };
        if let Some(event) = event {
            let _ = self.tabs.update(event);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            tabs(&self.tabs).on_event(Message::Tabs),
            text("Arrow keys or Ctrl+Tab move between tabs. Home and End jump to the ends.")
                .size(14),
        ]
        .spacing(16)
        .width(Length::Fixed(460.0))
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
