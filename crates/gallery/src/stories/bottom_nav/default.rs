use iced::widget::{column, container, text};
use iced::{Element, Length};
use iced_cube::{bottom_nav, lucide};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Notes,
    Search,
    Shared,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Go(Screen),
}

#[derive(Debug)]
pub struct Example {
    screen: Screen,
    note: String,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            screen: Screen::Notes,
            note: "Choose a destination.".to_owned(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Go(screen) = message;
        // Selecting the current screen again goes back to its top.
        self.note = if screen == self.screen {
            format!("Back to the top of {screen:?}")
        } else {
            format!("Showing {screen:?}")
        };
        self.screen = screen;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let screen = container(text(&self.note).size(14))
            .center(Length::Fill)
            .height(Length::Fill);

        let bar = bottom_nav(self.screen)
            .item(Screen::Notes, lucide!(NotebookText), "Notes")
            .item(Screen::Search, lucide!(Search), "Search")
            .item(Screen::Shared, lucide!(Users), "Shared")
            .item(Screen::Settings, lucide!(Settings), "Settings")
            .on_select(Message::Go);

        column![screen, bar]
            .width(Length::Fill)
            .max_width(360)
            .height(260)
            .into()
    }
}
