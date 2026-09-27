use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length, Subscription};
use iced_cube::keys::{self, Keymap};
use iced_cube::lucide;
use iced_cube::navigation::sidebar::{
    self, Action, Event, State, group, item, menu_button, sidebar,
};
use iced_cube::theme::{semibold, text_size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Inbox,
    Calendar,
    Reports,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(Event<Page>),
    Key(keys::Event),
}

#[derive(Debug)]
pub struct Example {
    sidebar: State<Page>,
    keymap: Keymap<Action>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            sidebar: State::new([
                group(
                    "Workspace",
                    [
                        item(Page::Home, "Home").icon(lucide!(House)),
                        item(Page::Inbox, "Inbox").icon(lucide!(Inbox)).badge("3"),
                        item(Page::Calendar, "Calendar").icon(lucide!(Calendar)),
                        item(Page::Reports, "Reports").icon(lucide!(ChartColumn)),
                    ],
                ),
                group(
                    "",
                    [item(Page::Settings, "Settings").icon(lucide!(Settings))],
                ),
            ])
            .with_active(Page::Inbox)
            .with_rail(true),
            keymap: sidebar::default_keymap(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let event = match message {
            Message::Sidebar(event) => Some(event),
            Message::Key(key) => self.sidebar.shortcut(&self.keymap, &key),
        };
        if let Some(event) = event {
            let _ = self.sidebar.update(event);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title = self
            .sidebar
            .active()
            .and_then(|page| self.sidebar.item(page))
            .map_or("", |item| item.label.as_str());

        let page = column![
            row![
                sidebar::trigger(Some(Message::Sidebar(Event::Toggle))),
                text(title).size(text_size::SM).font(semibold()),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text("Ctrl+B or the button toggles the rail. In the rail, hover an icon for its label. Click the sidebar, then Up, Down and Enter move through it, and Escape leaves.")
                .size(text_size::SM),
        ]
        .spacing(12)
        .padding(16);

        sidebar(&self.sidebar)
            .header(
                menu_button("Northwind")
                    .icon(lucide!(Command))
                    .compact(self.sidebar.is_compact()),
            )
            .content(container(page).width(Length::Fill))
            .trigger(false)
            .keymap(self.keymap.clone())
            .on_event(Message::Sidebar)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}
