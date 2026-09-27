//! The window chrome: the header toolbar with its menu and shortcuts
//! popover, the sidebar finder, the command palette and the dialogs.

use iced::widget::{column, container, row, space, text};
use iced::{Alignment, Element, Length};
use iced_cube::dropdown_menu::{
    self, Entry, checkbox_item, dropdown_menu, group_label, item, radio_item, submenu,
};
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::feedback::progress::Size as ProgressSize;
use iced_cube::feedback::spinner::Size as SpinnerSize;
use iced_cube::navigation::command::{self, command, group};
use iced_cube::overlay::dialog::{self, alert_dialog, dialog};
use iced_cube::overlay::popover::{Align, Side, popover};
use iced_cube::overlay::tooltip::Position;
use iced_cube::primitives::button::Variant;
use iced_cube::primitives::icon_button::Variant as IconVariant;
use iced_cube::theme::{Tokens, radius, semibold, text_size};
use iced_cube::{
    badge, button, icon, icon_button, label, lucide, progress, separator, spinner, tooltip,
    vertical_separator,
};

use super::{
    Cmd, Example, MenuItem, Message, Page, Pending, SIDEBAR, caption, count, muted, palette_chord,
};

const FINDER_ID: &str = "demo-finder";
const PALETTE_ID: &str = "demo-palette";
const PALETTE_DIALOG_ID: &str = "demo-palette-dialog";
const NEW_JOB_DIALOG_ID: &str = "demo-new-job";
const DRAWER_ID: &str = "demo-drawer";

/// The toolbar's "More" menu: checkbox items, radio items and a submenu.
pub fn menu_entries() -> Vec<Entry<MenuItem>> {
    vec![
        group_label("View"),
        checkbox_item(MenuItem::Sidebar, "Sidebar", true),
        checkbox_item(MenuItem::Compact, "Compact rows", false),
        dropdown_menu::separator(),
        group_label("Theme"),
        radio_item(MenuItem::System, "Match system", true),
        radio_item(MenuItem::Light, "Light", false),
        radio_item(MenuItem::Dark, "Dark", false),
        dropdown_menu::separator(),
        submenu(
            MenuItem::Environment,
            "Environment",
            [
                radio_item(MenuItem::Production, "Production", true),
                radio_item(MenuItem::Staging, "Staging", false),
                radio_item(MenuItem::Development, "Development", false),
            ],
        )
        .icon(lucide!(Layers)),
        item(MenuItem::Export, "Export jobs")
            .icon(lucide!(Download))
            .shortcut("CSV"),
        dropdown_menu::separator(),
        item(MenuItem::SignOut, "Sign out")
            .icon(lucide!(LogOut))
            .destructive(true),
    ]
}

/// The sidebar's saved views and shortcuts, filtered as you type.
pub fn finder() -> command::State<Cmd> {
    command::State::new([
        group(
            "Pages",
            [
                command::item(Cmd::Go(Page::Overview), "Overview").icon(lucide!(LayoutDashboard)),
                command::item(Cmd::Go(Page::Jobs), "Jobs").icon(lucide!(ListChecks)),
                command::item(Cmd::Go(Page::Activity), "Activity").icon(lucide!(Activity)),
                command::item(Cmd::Go(Page::Settings), "Settings").icon(lucide!(Settings)),
            ],
        ),
        group(
            "Saved views",
            [
                command::item(Cmd::FailedJobs, "Failed jobs").icon(lucide!(CircleX)),
                command::item(Cmd::RunningJobs, "Running now").icon(lucide!(Loader)),
                command::item(Cmd::MyJobs, "My jobs")
                    .icon(lucide!(User))
                    .keywords(["priya", "mine"]),
            ],
        ),
        group(
            "Actions",
            [
                command::item(Cmd::NewJob, "New job").icon(lucide!(Plus)),
                command::item(Cmd::RetryFailed, "Retry failed jobs").icon(lucide!(RotateCw)),
            ],
        ),
    ])
    .with_visible_rows(12)
}

impl Example {
    /// Builds the palette afresh on opening, so its labels match the app.
    pub(super) fn palette_state(&self) -> command::State<Cmd> {
        let scheduler = if self.settings.applied().scheduler {
            command::item(Cmd::ToggleScheduler, "Pause scheduler").icon(lucide!(Pause))
        } else {
            command::item(Cmd::ToggleScheduler, "Resume scheduler").icon(lucide!(Play))
        };
        let sidebar = if self.sidebar && self.width >= super::WIDE {
            "Hide sidebar"
        } else {
            "Show sidebar"
        };
        let failed = self
            .jobs
            .iter()
            .any(|job| job.status == super::Status::Failed);
        let finished = self.jobs.iter().any(|job| job.status.is_finished());

        command::State::new([
            group(
                "Go to",
                [
                    command::item(Cmd::Go(Page::Overview), "Overview")
                        .icon(lucide!(LayoutDashboard)),
                    command::item(Cmd::Go(Page::Jobs), "Jobs").icon(lucide!(ListChecks)),
                    command::item(Cmd::Go(Page::Activity), "Activity")
                        .icon(lucide!(Activity))
                        .keywords(["logs"]),
                    command::item(Cmd::Go(Page::Settings), "Settings")
                        .icon(lucide!(Settings))
                        .keywords(["preferences"]),
                ],
            ),
            group(
                "Jobs",
                [
                    command::item(Cmd::NewJob, "New job").icon(lucide!(Plus)),
                    command::item(Cmd::RetryFailed, "Retry failed jobs")
                        .icon(lucide!(RotateCw))
                        .disabled(!failed),
                    scheduler,
                    command::item(Cmd::ClearFinished, "Clear finished jobs")
                        .icon(lucide!(Trash))
                        .disabled(!finished)
                        .destructive(true),
                ],
            ),
            group(
                "Preferences",
                [
                    command::item(Cmd::ToggleTheme, "Toggle theme").icon(lucide!(SunMoon)),
                    command::item(Cmd::ToggleSidebar, sidebar).icon(lucide!(PanelLeft)),
                    command::item(Cmd::ClearToasts, "Clear notifications").icon(lucide!(BellOff)),
                    command::item(Cmd::Shortcuts, "Keyboard shortcuts")
                        .icon(lucide!(Keyboard))
                        .shortcut("Ctrl+/"),
                ],
            ),
            group(
                "Open job",
                // Names can repeat, so each job shows its ID too.
                self.jobs.iter().map(|job| {
                    command::item(Cmd::Open(job.id), job.name.clone())
                        .icon(lucide!(Briefcase))
                        .shortcut(job.id.to_string())
                        .keywords([job.owner.to_owned(), job.id.to_string()])
                }),
            ),
        ])
        .with_visible_rows(8)
    }

    pub(super) fn header(&self, narrow: bool, gutter: f32) -> Element<'_, Message> {
        let logo = container(icon::themed(lucide!(Waypoints), 16.0, 1.0, |theme| {
            Tokens::of(theme).primary_foreground
        }))
        .padding(6)
        .style(|theme| {
            let tokens = Tokens::of(theme);
            container::Style {
                background: Some(tokens.primary.into()),
                border: iced::Border {
                    radius: radius::MD.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            }
        });
        let mut brand = row![
            logo,
            text("Fernhill")
                .size(text_size::MD)
                .font(semibold())
                .wrapping(text::Wrapping::None),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        let environment = if narrow {
            self.environment.short()
        } else {
            self.environment.label()
        };
        brand = brand.push(badge(environment).variant(self.environment.badge()));

        let below = Some(Position::Bottom);
        let search: Element<'_, Message> = if narrow {
            icon_button(lucide!(Search))
                .label("Search")
                .tooltip(below)
                .on_press(Message::OpenPalette)
                .into()
        } else {
            tooltip(
                button("Search...")
                    .icon(lucide!(Search))
                    .variant(Variant::Outline)
                    .width(200)
                    .on_press(Message::OpenPalette),
                "Command palette (Ctrl+K)",
            )
            .position(Position::Bottom)
            .into()
        };
        let new_job: Element<'_, Message> = if narrow {
            icon_button(lucide!(Plus))
                .label("New job")
                .variant(IconVariant::Primary)
                .tooltip(below)
                .on_press(Message::OpenNewJob)
                .into()
        } else {
            button("New job")
                .icon(lucide!(Plus))
                .on_press(Message::OpenNewJob)
                .into()
        };
        let help = popover(
            icon_button(lucide!(Keyboard))
                .label("Keyboard shortcuts (Ctrl+/)")
                .tooltip(below.filter(|_| !self.help))
                .pressed(self.help)
                .on_press(Message::ToggleHelp),
            shortcuts(),
        )
        .open(self.help)
        .side(Side::Bottom)
        .align(Align::End)
        .width(320)
        .keymap(self.keys.help.clone())
        .on_dismiss(Message::CloseHelp);
        let more = dropdown_menu(
            &self.menu,
            icon_button(lucide!(EllipsisVertical))
                .label("More (F10)")
                .tooltip(below.filter(|_| !self.menu.is_open()))
                .pressed(self.menu.is_open())
                .on_press(Message::Menu(dropdown_menu::Event::Toggle)),
        )
        .align(Align::End)
        .keymap(self.keys.menu.clone())
        .on_event(Message::Menu);

        let mut tools = row![search, new_job].spacing(6).align_y(Alignment::Center);
        // Phones have no keyboard to list shortcuts for.
        if !narrow {
            tools = tools
                .push(
                    icon_button(lucide!(RefreshCw))
                        .label("Refresh")
                        .tooltip(below)
                        .on_press(Message::Refresh),
                )
                .push(help);
        }
        let tools = tools
            .push(container(vertical_separator()).height(20))
            .push(more);

        row![brand, space::horizontal(), tools]
            .spacing(12)
            .padding([10.0, gutter])
            .align_y(Alignment::Center)
            .into()
    }

    pub(super) fn sidebar_view(&self) -> Element<'_, Message> {
        container(self.sidebar_body(Length::Fill))
            .padding(16)
            .width(SIDEBAR)
            .height(Length::Fill)
            .into()
    }

    /// The workspace, the finder and the scheduler, beside the page or in
    /// the drawer dialog on narrow windows.
    fn sidebar_body(&self, finder_height: Length) -> Element<'_, Message> {
        let settings = self.settings.applied();
        let running = self.count(super::Status::Running);
        let slots = settings.concurrency;
        let status: Element<'_, Message> = if !settings.scheduler {
            badge("Paused").variant(BadgeVariant::Warning).into()
        } else if running > 0 {
            spinner(self.phase).size(SpinnerSize::Sm).into()
        } else {
            badge("Idle").variant(BadgeVariant::Secondary).into()
        };

        column![
            column![
                caption("Workspace"),
                text(settings.workspace.as_str())
                    .size(text_size::SM)
                    .font(semibold()),
            ]
            .spacing(2),
            label("Jump to"),
            command(&self.finder)
                .placeholder("Filter views...")
                .height(finder_height)
                .id(FINDER_ID)
                .keymap(self.keys.command.clone())
                .on_event(Message::Finder),
            separator(),
            row![
                text("Scheduler").size(text_size::SM).font(semibold()),
                space::horizontal(),
                status,
            ]
            .align_y(Alignment::Center),
            progress(running as f32 / f32::from(slots.max(1)))
                .size(ProgressSize::Sm)
                .label(format!("{running} of {slots} slots busy")),
        ]
        .spacing(12)
        .into()
    }

    /// Wraps the app in its dialogs. At most one of them is open at a time.
    pub(super) fn dialogs<'a>(
        &'a self,
        base: Element<'a, Message>,
        narrow: bool,
    ) -> Element<'a, Message> {
        let (title, description, confirm) = self.pending_text();
        let confirmation = alert_dialog(base, title, description)
            .open(self.pending.is_some())
            .confirm(confirm)
            .keymap(self.keys.dialog.clone())
            .on_cancel(Message::Cancel)
            .on_confirm(Message::Confirm);

        let settings = self.settings.applied();
        let new_job = dialog(confirmation)
            .open(self.new_job_open)
            .title("New job")
            .description(format!(
                "Jobs in {} run in {} unless their source lives elsewhere.",
                settings.workspace, settings.region
            ))
            .body(
                self.draft
                    .view(&self.keys.combobox, narrow)
                    .map(Message::Draft),
            )
            .action(
                button("Cancel")
                    .variant(Variant::Outline)
                    .on_press(Message::CloseNewJob),
            )
            .action(
                button("Create job")
                    .icon(lucide!(Plus))
                    .on_press(Message::CreateJob),
            )
            .size(dialog::Size::Md)
            .id(NEW_JOB_DIALOG_ID)
            .keymap(self.keys.dialog.clone())
            .on_confirm(Message::CreateJob)
            .on_dismiss(Message::CloseNewJob);

        let drawer = dialog(new_job)
            .open(self.drawer)
            .title("Sidebar")
            .body(self.sidebar_body(Length::Fixed(280.0)))
            .size(dialog::Size::Sm)
            .id(DRAWER_ID)
            .keymap(self.keys.dialog.clone())
            .on_dismiss(Message::CloseDrawer);

        dialog(drawer)
            .open(self.palette_open)
            .title("Command palette")
            .body(
                command(&self.palette)
                    .placeholder("Search pages, actions and jobs...")
                    .max_height(400.0)
                    .id(PALETTE_ID)
                    .keymap(self.keys.command.clone())
                    .on_event(Message::Palette),
            )
            .size(dialog::Size::Md)
            .close_button(false)
            .id(PALETTE_DIALOG_ID)
            .keymap(self.keys.dialog.clone())
            .pass_through([palette_chord()])
            .on_dismiss(Message::ClosePalette)
            .into()
    }

    pub(super) fn pending_text(&self) -> (String, String, &'static str) {
        let undo = "Undo it from the notification, or with Alt+Z, for at least ten seconds.";
        match self.pending {
            Some(Pending::Delete(id)) => {
                let name = self.job(id).map_or_else(
                    || String::from("this job"),
                    |job| format!("{} ({id})", job.name),
                );
                (
                    format!("Delete {name}?"),
                    format!(
                        "This stops the job if it is running and removes it from the list. {undo}"
                    ),
                    "Delete job",
                )
            }
            Some(Pending::DeleteSelected) => {
                let selected = self.selected().len();
                (
                    format!("Delete {}?", count(selected, "selected job")),
                    format!("Running jobs stop straight away. {undo}"),
                    "Delete jobs",
                )
            }
            Some(Pending::ClearFinished) => {
                let finished = self
                    .jobs
                    .iter()
                    .filter(|job| job.status.is_finished())
                    .count();
                (
                    format!("Clear {}?", count(finished, "finished job")),
                    format!("Succeeded and failed jobs leave the list. {undo}"),
                    "Clear jobs",
                )
            }
            None => (String::new(), String::new(), "Continue"),
        }
    }
}

/// The shortcuts popover: every chord the demo routes.
fn shortcuts<'a>() -> Element<'a, Message> {
    const KEYS: &[(&str, &str)] = &[
        ("Ctrl+K", "Command palette"),
        ("Ctrl+/", "This list"),
        ("F10", "The More menu"),
        ("Left, Right", "Switch pages"),
        ("Up, Down", "Move through jobs, runbooks or settings"),
        ("Space", "Tick the job, or change the setting"),
        ("Shift+F10", "Actions for the current job"),
        ("Delete", "Delete the current job"),
        ("Alt+Z", "Undo from the newest notification"),
        ("Escape", "Close the newest notification"),
    ];

    column![
        text("Keyboard shortcuts")
            .size(text_size::SM)
            .font(semibold()),
        column(KEYS.iter().map(|(keys, action)| {
            row![
                container(badge(*keys).variant(BadgeVariant::Outline)).width(96),
                muted(*action).width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        }))
        .spacing(8),
        caption("Menus, lists and dialogs handle their own keys while open."),
    ]
    .spacing(12)
    .into()
}
