use iced::widget::{column, container, text};
use iced::{Element, Length};
use iced_cube::layout::resizable_panel::{self, Extent, Output, panel, resizable_panel};
use iced_cube::lucide;
use iced_cube::navigation::sidebar::{Event, RAIL_WIDTH, State, group, item, menu_button, sidebar};
use iced_cube::theme::{semibold, text_size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Deployments,
    Logs,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(Event<Page>),
    Panels(resizable_panel::Event),
}

#[derive(Debug)]
pub struct Example {
    sidebar: State<Page>,
    panels: resizable_panel::State,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            sidebar: State::new([group(
                "Project",
                [
                    item(Page::Overview, "Overview").icon(lucide!(LayoutDashboard)),
                    item(Page::Deployments, "Deployments")
                        .icon(lucide!(Rocket))
                        .badge("2"),
                    item(Page::Logs, "Logs").icon(lucide!(Terminal)),
                    item(Page::Settings, "Settings").icon(lucide!(Settings)),
                ],
            )])
            .with_active(Page::Overview),
            // Collapsing the panel to the rail's width shows the rail.
            panels: resizable_panel::State::new([
                panel(Extent::Pixels(220.0))
                    .min(Extent::Pixels(180.0))
                    .max(Extent::Fraction(0.5))
                    .collapsible(true)
                    .collapsed_size(Extent::Pixels(RAIL_WIDTH)),
                panel(Extent::Fraction(1.0)).min(Extent::Pixels(120.0)),
            ]),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Sidebar(Event::Toggle | Event::ToggleRail) => {
                let _ = self.panels.update(resizable_panel::Event::Toggle(0));
                self.sync_rail();
            }
            Message::Sidebar(event) => {
                let _ = self.sidebar.update(event);
            }
            Message::Panels(event) => {
                if let Some(Output::Collapsed(0) | Output::Expanded(0)) = self.panels.update(event)
                {
                    self.sync_rail();
                }
            }
        }
    }

    fn sync_rail(&mut self) {
        let rail = self.panels.is_collapsed(0);
        let _ = self.sidebar.update(Event::SetRail(rail));
    }

    pub fn view(&self) -> Element<'_, Message> {
        let navigation = sidebar(&self.sidebar)
            .header(
                menu_button("Aurora")
                    .subtitle("Production")
                    .icon(lucide!(Cloud))
                    .compact(self.sidebar.is_compact()),
            )
            .width(Length::Fill)
            .on_event(Message::Sidebar);

        let title = self
            .sidebar
            .active()
            .and_then(|page| self.sidebar.item(page))
            .map_or("", |item| item.label.as_str());
        let page = container(
            column![
                text(title).size(text_size::LG).font(semibold()),
                text("Drag the edge of the sidebar to resize it. Drag it past the minimum and it snaps to the rail, and Ctrl+B on the focused sidebar does the same.")
                    .size(text_size::SM),
            ]
            .spacing(8),
        )
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill);

        resizable_panel(&self.panels, [navigation.into(), page.into()])
            .on_event(Message::Panels)
            .into()
    }
}
