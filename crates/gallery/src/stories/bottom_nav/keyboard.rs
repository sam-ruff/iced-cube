use iced::widget::{column, container, text};
use iced::{Element, Length, Subscription};
use iced_cube::keys::{self, Keymap};
use iced_cube::lucide;
use iced_cube::navigation::bottom_nav::{Action, bottom_nav, default_keymap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Inbox,
    Profile,
}

const SCREENS: [Screen; 3] = [Screen::Home, Screen::Inbox, Screen::Profile];

#[derive(Debug, Clone)]
pub enum Message {
    Go(Screen),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    screen: Screen,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            screen: Screen::Home,
            keymap: default_keymap(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Go(screen) => self.screen = screen,
            // Alt+1 to Alt+3 pick a destination.
            Message::Key(key) => {
                if let Some(screen) = self
                    .keymap
                    .resolve_event(&key)
                    .and_then(|action| action.apply(&SCREENS))
                {
                    self.screen = screen;
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let screen = container(text(format!("{:?}: Alt+1 to Alt+3 switch", self.screen)).size(14))
            .center(Length::Fill)
            .height(Length::Fill);

        let bar = bottom_nav(self.screen)
            .item(Screen::Home, lucide!(House), "Home")
            .item(Screen::Inbox, lucide!(Inbox), "Inbox")
            .item(Screen::Profile, lucide!(CircleUser), "Profile")
            .on_select(Message::Go);

        column![screen, bar]
            .width(Length::Fill)
            .max_width(360)
            .height(220)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
