use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};
use iced_cube::dropdown_menu::{self, Output as MenuOutput, dropdown_menu};
use iced_cube::navigation::sidebar::{self, Event, State, group, item, menu_button, sidebar};
use iced_cube::overlay::anchored::Side;
use iced_cube::theme::{semibold, text_size};
use iced_cube::{card, lucide, separator, vertical_separator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Dashboard,
    Inbox,
    Projects,
    Website,
    Mobile,
    Calendar,
    General,
    Billing,
    Team,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    Acme,
    Globex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Account {
    Profile,
    SignOut,
}

#[derive(Debug, Clone)]
pub enum Message {
    Sidebar(Event<Page>),
    Workspaces(dropdown_menu::Event<Workspace>),
    Account(dropdown_menu::Event<Account>),
}

#[derive(Debug)]
pub struct Example {
    sidebar: State<Page>,
    workspaces: dropdown_menu::State<Workspace>,
    account: dropdown_menu::State<Account>,
    workspace: Workspace,
}

impl Default for Example {
    fn default() -> Self {
        let navigation = State::new([
            group(
                "Platform",
                [
                    item(Page::Dashboard, "Dashboard").icon(lucide!(LayoutDashboard)),
                    item(Page::Inbox, "Inbox").icon(lucide!(Inbox)).badge("12"),
                    item(Page::Projects, "Projects")
                        .icon(lucide!(FolderOpen))
                        .children([
                            item(Page::Website, "Website"),
                            item(Page::Mobile, "Mobile app"),
                        ]),
                    item(Page::Calendar, "Calendar").icon(lucide!(Calendar)),
                ],
            ),
            group(
                "Settings",
                [
                    item(Page::General, "General").icon(lucide!(Settings)),
                    item(Page::Billing, "Billing").icon(lucide!(CreditCard)),
                    item(Page::Team, "Team").icon(lucide!(Users)).disabled(true),
                ],
            )
            .collapsible(true),
        ])
        .with_active(Page::Website);

        Self {
            sidebar: navigation,
            workspaces: dropdown_menu::State::new([
                dropdown_menu::group_label("Workspaces"),
                dropdown_menu::radio_item(Workspace::Acme, "Acme Inc", true),
                dropdown_menu::radio_item(Workspace::Globex, "Globex", false),
            ]),
            account: dropdown_menu::State::new([
                dropdown_menu::item(Account::Profile, "Profile").icon(lucide!(User)),
                dropdown_menu::separator(),
                dropdown_menu::item(Account::SignOut, "Sign out").icon(lucide!(LogOut)),
            ]),
            workspace: Workspace::Acme,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Sidebar(event) => {
                let _ = self.sidebar.update(event);
            }
            Message::Workspaces(event) => {
                if let Some(MenuOutput::Selected(workspace)) = self.workspaces.update(event) {
                    self.workspace = workspace;
                }
            }
            Message::Account(event) => {
                let _ = self.account.update(event);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let compact = self.sidebar.is_compact();
        let name = match self.workspace {
            Workspace::Acme => "Acme Inc",
            Workspace::Globex => "Globex",
        };
        let switcher = dropdown_menu(
            &self.workspaces,
            menu_button(name)
                .subtitle("Enterprise")
                .icon(lucide!(Command))
                .compact(compact)
                .on_press(Message::Workspaces(dropdown_menu::Event::Toggle)),
        )
        .side(if compact { Side::Right } else { Side::Bottom })
        .on_event(Message::Workspaces);
        let user = dropdown_menu(
            &self.account,
            menu_button("Sam Taylor")
                .subtitle("sam@example.com")
                .initials("ST")
                .compact(compact)
                .on_press(Message::Account(dropdown_menu::Event::Toggle)),
        )
        .side(if compact { Side::Right } else { Side::Top })
        .on_event(Message::Account);

        sidebar(&self.sidebar)
            .header(switcher)
            .footer(user)
            .content(self.page())
            .trigger(false)
            .on_event(Message::Sidebar)
            .into()
    }

    fn page(&self) -> Element<'_, Message> {
        let title = self
            .sidebar
            .active()
            .and_then(|page| self.sidebar.item(page))
            .map_or("Dashboard", |item| item.label.as_str());

        let bar = row![
            sidebar::trigger(Some(Message::Sidebar(Event::Toggle))),
            container(vertical_separator()).height(16),
            text(title)
                .size(text_size::SM)
                .font(semibold())
                .wrapping(text::Wrapping::None),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding([6, 12]);

        let body = card()
            .title(title)
            .description("Everything in this project, in one place.")
            .body(
                text("Choose a page on the left. The button above collapses the sidebar to icons.")
                    .size(text_size::SM),
            )
            .width(Length::Fill);

        column![bar, separator(), container(body).padding(16)]
            .width(Length::Fill)
            .into()
    }
}
