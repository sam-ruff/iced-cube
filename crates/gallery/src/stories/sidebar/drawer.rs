use iced::widget::{column, text};
use iced::{Element, Length};
use iced_cube::lucide;
use iced_cube::navigation::sidebar::{Event, State, group, item, sidebar};
use iced_cube::theme::{semibold, text_size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Orders,
    Customers,
    Products,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(Event<Page>),
}

#[derive(Debug)]
pub struct Example {
    sidebar: State<Page>,
}

impl Default for Example {
    fn default() -> Self {
        let mut sidebar = State::new([
            group(
                "Store",
                [
                    item(Page::Home, "Home").icon(lucide!(House)),
                    item(Page::Orders, "Orders")
                        .icon(lucide!(Package))
                        .badge("8"),
                    item(Page::Customers, "Customers").icon(lucide!(Users)),
                    item(Page::Products, "Products").icon(lucide!(Tag)),
                ],
            ),
            group(
                "",
                [item(Page::Settings, "Settings").icon(lucide!(Settings))],
            ),
        ])
        .with_active(Page::Orders);
        let _ = sidebar.update(Event::Narrow(true));
        let _ = sidebar.update(Event::OpenDrawer);
        Self { sidebar }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Sidebar(event) = message;
        let _ = self.sidebar.update(event);
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title = self
            .sidebar
            .active()
            .and_then(|page| self.sidebar.item(page))
            .map_or("", |item| item.label.as_str());

        let page = column![
            text(title).size(text_size::LG).font(semibold()),
            text("The menu button opens the drawer. Tap the dimmed page, press Escape or choose a page to close it.")
                .size(text_size::SM),
        ]
        .spacing(8)
        .padding(16)
        .width(Length::Fill);

        // Every window counts as narrow here, so the drawer shows on the
        // desktop too. Apps usually keep the default breakpoint.
        sidebar(&self.sidebar)
            .content(page)
            .breakpoint(f32::INFINITY)
            .on_event(Message::Sidebar)
            .into()
    }
}
