//! Fernhill, the operations console of a fictional data platform, built
//! from every iced-cube component: jobs fed by a simulated worker through
//! channels, a command palette, menus, dialogs, toasts and settings.

mod activity;
mod chrome;
mod data;
mod feed;
mod jobs;
mod new_job;
mod overview;
mod settings;
#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::time::Duration;

use iced::keyboard::key::Named;
use iced::time::Instant;
use iced::widget::{Text, column, container, responsive, row, sensor, space, text};
use iced::{Element, Length, Size, Subscription, Task, Theme};
use iced_cube::context_menu;
use iced_cube::dropdown_menu::{self, Output as MenuOutput};
use iced_cube::feedback::alert::Variant as AlertVariant;
use iced_cube::feedback::spinner::advance;
use iced_cube::forms::combobox;
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::layout::accordion;
use iced_cube::navigation::command;
use iced_cube::navigation::tabs::{self, tab};
use iced_cube::overlay::dialog;
use iced_cube::overlay::popover;
use iced_cube::overlay::toast::{self, Variant as ToastVariant, toast, toasts};
use iced_cube::primitives::checkbox::{self, CheckState};
use iced_cube::primitives::switch;
use iced_cube::theme::{self, Tokens, text_size};
use iced_cube::{alert, separator, vertical_separator};

use crate::app::ThemeChoice;
use crate::bridge;
use data::{
    Environment, Job, JobId, Level, LevelFilter, LogLine, Queue, Range, Runbook, Schedule, Status,
    StatusFilter, TEMPLATES,
};
use feed::{Update, Worker};
use new_job::Draft;
use settings::Settings;

/// Below this width secondary panels collapse and rows stack.
const NARROW: f32 = 640.0;
/// From this width the sidebar shows beside the page.
const WIDE: f32 = 1100.0;
const SIDEBAR: f32 = 248.0;
const LOG_LIMIT: usize = 200;
const JOB_LIMIT: usize = 16;
const QUEUE_LIMIT: usize = 4;
/// Jobs that had finished before the demo started, for the Succeeded total.
const EARLIER_SUCCESSES: usize = 214;
const RUNBOOKS: [Runbook; 3] = [Runbook::Degraded, Runbook::Retry, Runbook::Escalation];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Jobs,
    Activity,
    Settings,
}

impl Page {
    const ALL: [Page; 4] = [Page::Overview, Page::Jobs, Page::Activity, Page::Settings];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    /// Follows the host: the docs site's toggle or the desktop default.
    System,
    Light,
    Dark,
}

/// Items of the toolbar's "More" menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Sidebar,
    Compact,
    System,
    Light,
    Dark,
    Environment,
    Production,
    Staging,
    Development,
    Export,
    SignOut,
}

/// Items of a job row's context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowAction {
    Logs,
    Retry,
    Pause,
    Resume,
    Queue,
    High,
    Normal,
    Low,
    CopyId,
    Delete,
}

/// What the command palette and the sidebar finder can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    Go(Page),
    FailedJobs,
    RunningJobs,
    MyJobs,
    NewJob,
    RetryFailed,
    ToggleScheduler,
    ClearFinished,
    ToggleTheme,
    ToggleSidebar,
    ClearToasts,
    Shortcuts,
    Open(JobId),
}

/// A destructive action waiting for confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    Delete(JobId),
    DeleteSelected,
    ClearFinished,
}

/// A deleted job and its position in the list, so Undo can put it back.
pub type Placed = (usize, Job);

#[derive(Debug, Clone)]
pub enum Message {
    Tabs(tabs::Event<Page>),
    Range(tabs::Event<Range>),
    Menu(dropdown_menu::Event<MenuItem>),
    ToggleHelp,
    CloseHelp,
    OpenPalette,
    ClosePalette,
    Palette(command::Event<Cmd>),
    Finder(command::Event<Cmd>),
    CloseDrawer,
    Run(Cmd),
    OpenNewJob,
    CloseNewJob,
    Draft(new_job::Message),
    CreateJob,
    Search(String),
    Filter(StatusFilter),
    Check(JobId, bool),
    CheckAll(bool),
    Cursor(JobId),
    RowMenu(context_menu::Event<RowAction, JobId>),
    ListHover(bool),
    ShowNew,
    PauseSelected,
    DeleteSelected,
    Confirm,
    Cancel,
    Live(bool),
    Level(LevelFilter),
    LogQuery(String),
    Runbooks(accordion::Event<Runbook>),
    Settings(settings::Message),
    Refresh,
    Feed(feed::Event),
    Submit,
    Toast(toast::Event),
    Restore(Vec<Placed>),
    Tick(Instant),
    Key(keys::Event),
    Resized(Size),
    Host(bridge::Event),
}

/// Every keymap the app routes, one per component, with the demo's changes.
#[derive(Debug, Clone)]
struct Keymaps {
    tabs: Keymap<tabs::Action>,
    accordion: Keymap<accordion::Action>,
    menu: Keymap<dropdown_menu::Action>,
    row_menu: Keymap<context_menu::Action>,
    help: Keymap<popover::Action>,
    toast: Keymap<toast::Action>,
    command: Keymap<command::Action>,
    combobox: Keymap<combobox::Action>,
    dialog: Keymap<dialog::Action>,
    checkbox: Keymap<checkbox::Action>,
    switch: Keymap<switch::Action>,
    settings: settings::Keys,
}

impl Default for Keymaps {
    fn default() -> Self {
        Self {
            tabs: tabs::default_keymap(),
            accordion: accordion::default_keymap(),
            // F10 opens the menu, as it opens a menu bar on the desktop.
            menu: dropdown_menu::default_keymap()
                .bind(Chord::named(Named::F10), dropdown_menu::Action::Open),
            row_menu: context_menu::default_keymap(),
            help: popover::default_keymap()
                .bind(Chord::character('/').command(), popover::Action::Toggle),
            toast: toast::default_keymap(),
            command: command::default_keymap(),
            combobox: combobox::default_keymap(),
            dialog: dialog::default_keymap(),
            checkbox: checkbox::default_keymap(),
            switch: switch::default_keymap(),
            settings: settings::Keys::default(),
        }
    }
}

/// Opens the command palette from anywhere, and closes it again.
fn palette_chord() -> Chord {
    Chord::character('k').command()
}

#[derive(Debug)]
pub struct Example {
    page: tabs::State<Page>,
    range: tabs::State<Range>,
    menu: dropdown_menu::State<MenuItem>,
    theme: ThemeMode,
    environment: Environment,
    /// The window width, for choices made in `update`.
    width: f32,
    sidebar: bool,
    /// The sidebar as a dialog, on windows too narrow to show it beside the page.
    drawer: bool,
    compact: bool,
    help: bool,
    palette_open: bool,
    palette: command::State<Cmd>,
    finder: command::State<Cmd>,
    new_job_open: bool,
    draft: Draft,
    pending: Option<Pending>,
    jobs: Vec<Job>,
    next_id: u32,
    next_template: usize,
    succeeded: usize,
    search: String,
    filter: StatusFilter,
    cursor: Option<JobId>,
    row_menu: context_menu::State<RowAction, JobId>,
    list_hovered: bool,
    /// Submitted jobs held back behind the "new jobs" pill.
    fresh: Vec<JobId>,
    /// The rows on show, kept while something points at the list so no row
    /// moves under the pointer or an open menu.
    frozen: Option<Vec<JobId>>,
    logs: VecDeque<(u32, LogLine)>,
    held: VecDeque<LogLine>,
    clock: u32,
    live: bool,
    level: LevelFilter,
    log_query: String,
    runbooks: accordion::State<Runbook>,
    settings: Settings,
    // Undo toasts carry the message that restores what they report.
    toasts: toast::State<Message>,
    toast_sender: Option<toast::Sender>,
    feed: Option<feed::Sender>,
    phase: f32,
    last_tick: Option<Instant>,
    keys: Keymaps,
}

impl Default for Example {
    fn default() -> Self {
        let jobs = data::jobs();
        let next_id = jobs.iter().map(|job| job.id.0).max().unwrap_or(1000) + 1;
        let succeeded = EARLIER_SUCCESSES
            + jobs
                .iter()
                .filter(|job| job.status == Status::Succeeded)
                .count();
        Self {
            page: tabs::State::new([
                tab(Page::Overview, "Overview"),
                tab(Page::Jobs, "Jobs"),
                tab(Page::Activity, "Activity"),
                tab(Page::Settings, "Settings"),
            ]),
            range: tabs::State::new([
                tab(Range::Day, "24h"),
                tab(Range::Week, "7d"),
                tab(Range::Month, "30d"),
            ]),
            menu: dropdown_menu::State::new(chrome::menu_entries()),
            theme: ThemeMode::System,
            environment: Environment::Production,
            width: WIDE,
            sidebar: true,
            drawer: false,
            compact: false,
            help: false,
            palette_open: false,
            palette: command::State::new([]),
            finder: chrome::finder(),
            new_job_open: false,
            draft: Draft::default(),
            pending: None,
            jobs,
            next_id,
            next_template: 0,
            succeeded,
            search: String::new(),
            filter: StatusFilter::All,
            cursor: None,
            row_menu: context_menu::State::new(jobs::row_menu_entries()),
            list_hovered: false,
            fresh: Vec::new(),
            frozen: None,
            logs: data::logs().into(),
            held: VecDeque::new(),
            clock: 9 * 3600 + 41 * 60,
            live: true,
            level: LevelFilter::All,
            log_query: String::new(),
            runbooks: accordion::State::new(accordion::Mode::Single).with_open(Runbook::Degraded),
            settings: Settings::default(),
            toasts: toast::State::default(),
            toast_sender: None,
            feed: None,
            phase: 0.15,
            last_tick: None,
            keys: Keymaps::default(),
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.handle(message);
        self.settle_list();
        task
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tabs(event) => {
                let _ = self.page.update(event);
            }
            Message::Range(event) => {
                let _ = self.range.update(event);
            }
            Message::Menu(event) => self.menu_event(event),
            Message::ToggleHelp => self.help = !self.help,
            Message::CloseHelp => self.help = false,
            Message::OpenPalette => self.open_palette(),
            Message::ClosePalette => self.palette_open = false,
            Message::Palette(event) => match self.palette.update(event) {
                Some(command::Output::Activated(cmd)) => {
                    self.palette_open = false;
                    self.run(cmd);
                }
                Some(command::Output::Closed) => self.palette_open = false,
                _ => {}
            },
            Message::Finder(event) => {
                if let Some(command::Output::Activated(cmd)) = self.finder.update(event) {
                    self.drawer = false;
                    self.run(cmd);
                }
            }
            Message::CloseDrawer => self.drawer = false,
            Message::Run(cmd) => self.run(cmd),
            Message::OpenNewJob => {
                self.draft = Draft::default();
                self.new_job_open = true;
            }
            Message::CloseNewJob => self.new_job_open = false,
            Message::Draft(message) => self.draft.update(message),
            Message::CreateJob => self.create_job(),
            Message::Search(search) => {
                self.search = search;
                self.untick_hidden();
            }
            Message::Filter(filter) => {
                self.filter = filter;
                self.untick_hidden();
            }
            Message::Check(id, checked) => {
                if let Some(job) = self.job_mut(id) {
                    job.checked = checked;
                }
            }
            Message::CheckAll(checked) => {
                let visible: Vec<JobId> = self.visible_jobs().map(|job| job.id).collect();
                for job in &mut self.jobs {
                    if visible.contains(&job.id) {
                        job.checked = checked;
                    }
                }
            }
            Message::Cursor(id) => self.cursor = Some(id),
            Message::RowMenu(event) => return self.row_menu_event(event),
            Message::ListHover(hovered) => self.list_hovered = hovered,
            Message::ShowNew => self.show_new(),
            Message::PauseSelected => self.pause_selected(),
            Message::DeleteSelected => self.pending = Some(Pending::DeleteSelected),
            Message::Confirm => self.confirm(),
            Message::Cancel => self.pending = None,
            Message::Live(live) => {
                self.live = live;
                if live {
                    for line in std::mem::take(&mut self.held) {
                        self.log(line);
                    }
                }
            }
            Message::Level(level) => self.level = level,
            Message::LogQuery(query) => self.log_query = query,
            Message::Runbooks(event) => {
                let _ = self.runbooks.update(event);
            }
            Message::Settings(settings::Message::ClearFinished) => {
                self.pending = Some(Pending::ClearFinished);
            }
            Message::Settings(message) => {
                if self.settings.update(message) {
                    let workspace = &self.settings.applied().workspace;
                    let description = format!("Everyone in {workspace} sees the new settings.");
                    self.notify(
                        toast("Settings saved")
                            .description(description)
                            .variant(ToastVariant::Success),
                    );
                    self.schedule();
                }
            }
            Message::Refresh => {
                self.log(LogLine::new(
                    Level::Info,
                    "scheduler",
                    "Manual refresh requested",
                ));
                self.notify(toast("Refreshed").description("Job states are up to date."));
            }
            Message::Feed(feed::Event::Ready(sender)) => self.feed = Some(sender),
            Message::Feed(feed::Event::Received(updates)) => {
                for update in updates {
                    self.apply(update);
                }
                self.schedule();
            }
            Message::Submit => {
                let template = self.next_template;
                self.next_template = (template + 1) % TEMPLATES.len();
                self.apply(Update::Submitted(template));
                self.schedule();
            }
            Message::Toast(event) => return self.toast_event(event),
            Message::Restore(jobs) => self.restore(jobs),
            Message::Tick(now) => {
                if let Some(last) = self.last_tick {
                    self.phase = advance(self.phase, now - last);
                }
                self.last_tick = Some(now);
            }
            Message::Key(key) => self.key(&key),
            Message::Resized(size) => {
                self.width = size.width;
                if self.width >= WIDE {
                    self.drawer = false;
                }
            }
            // The page's own toggle wins: the app follows the host again.
            Message::Host(bridge::Event::Theme(_)) => self.apply_theme(ThemeMode::System),
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let app = responsive(move |size| {
            let narrow = size.width < NARROW;
            self.dialogs(self.shell(size), narrow)
        });
        let app = sensor(app)
            .on_show(Message::Resized)
            .on_resize(Message::Resized);

        toasts(&self.toasts, app).on_event(Message::Toast).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let worker = match self.worker() {
            Some(worker) => Subscription::run_with(worker, Worker::run).map(|never| match never {}),
            None => Subscription::none(),
        };
        // Spinners only animate while a job is running.
        let ticks = if self.jobs.iter().any(|job| job.status == Status::Running) || self.live {
            feed::ticks(Duration::from_millis(40)).map(Message::Tick)
        } else {
            Subscription::none()
        };

        Subscription::batch([
            keys::subscription().map(Message::Key),
            bridge::subscription().map(Message::Host),
            feed::subscription().map(Message::Feed),
            feed::ticks(feed::SUBMIT_EVERY).map(|_| Message::Submit),
            toast::subscription().map(Message::Toast),
            toast::timer(&self.toasts).map(Message::Toast),
            worker,
            ticks,
        ])
    }

    /// The demo's own theme, or `None` to follow the host.
    pub fn theme(&self) -> Option<Theme> {
        match self.theme {
            ThemeMode::System => None,
            ThemeMode::Light => Some(theme::light()),
            ThemeMode::Dark => Some(theme::dark()),
        }
    }

    /// The header, the sidebar on wide windows, the banner, the tabs and the
    /// open page.
    fn shell(&self, size: Size) -> Element<'_, Message> {
        let narrow = size.width < NARROW;
        let sidebar = self.sidebar && size.width >= WIDE;
        let gutter = if narrow { 16.0 } else { 24.0 };
        let content = size.width - 2.0 * gutter - if sidebar { SIDEBAR + 1.0 } else { 0.0 };

        let selected = self.page.selected().unwrap_or(Page::Overview);
        // Card pages keep their gutters inside the scroll area for their shadows.
        let card_page = matches!(selected, Page::Overview | Page::Settings);
        let page = match selected {
            Page::Overview => self.overview(content, gutter),
            Page::Jobs => self.jobs_page(narrow),
            Page::Activity => self.activity(narrow),
            Page::Settings => self.settings_page(narrow, gutter),
        };
        // Each page has a slot of its own, and the others hold empty space, so
        // no page inherits another's widget state, such as a scroll position.
        let mut page = Some(page);
        let page = column(
            Page::ALL.map(|slot| match page.take_if(|_| slot == selected) {
                Some(page) => page,
                None => space().into(),
            }),
        )
        .height(Length::Fill);

        let mut main = column![].width(Length::Fill).height(Length::Fill);
        let banner = self.settings.applied().banner.trim();
        if !banner.is_empty() {
            main = main.push(
                container(
                    alert(banner)
                        .variant(AlertVariant::Warning)
                        .width(Length::Fill),
                )
                .padding(iced::Padding {
                    top: 12.0,
                    right: gutter,
                    bottom: 0.0,
                    left: gutter,
                }),
            );
        }
        let main = main
            .push(
                container(
                    tabs::tabs(&self.page)
                        .width(Length::Fill)
                        .on_event(Message::Tabs),
                )
                .padding(iced::Padding {
                    top: 12.0,
                    right: gutter,
                    bottom: if card_page { 0.0 } else { 12.0 },
                    left: gutter,
                }),
            )
            .push(
                container(page)
                    .padding(iced::Padding {
                        top: if card_page { 0.0 } else { 4.0 },
                        right: if card_page { 0.0 } else { gutter },
                        bottom: 0.0,
                        left: if card_page { 0.0 } else { gutter },
                    })
                    .width(Length::Fill)
                    .height(Length::Fill),
            );

        let body: Element<'_, Message> = if sidebar {
            row![self.sidebar_view(), vertical_separator(), main].into()
        } else {
            main.into()
        };

        column![self.header(narrow, gutter), separator(), body]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn settings_page(&self, narrow: bool, gutter: f32) -> Element<'_, Message> {
        iced_cube::scroll_area(
            container(self.settings.view(narrow).map(Message::Settings))
                .padding(iced::Padding {
                    top: 16.0,
                    bottom: gutter,
                    right: gutter + if narrow { 0.0 } else { 12.0 },
                    left: gutter,
                })
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn key(&mut self, key: &keys::Event) {
        let toggles_palette = palette_chord().matches(&key.key, key.modifiers);
        // An open dialog keeps every key except the ones it passes through.
        if self.palette_open || self.new_job_open || self.pending.is_some() || self.drawer {
            if toggles_palette && self.palette_open {
                self.palette_open = false;
            }
            return;
        }
        if toggles_palette {
            self.open_palette();
            return;
        }
        // Open menus and popovers have already taken their keys, so only
        // the chords that open them arrive here.
        if let Some(event) = self.menu.key_event(&self.keys.menu, key) {
            self.menu_event(event);
            return;
        }
        if let Some(open) = self
            .keys
            .help
            .resolve_event(key)
            .and_then(|action| action.apply(self.help))
        {
            self.help = open;
            return;
        }
        if !self.toasts.is_empty()
            && let Some(action) = self.keys.toast.resolve_event(key)
        {
            let _ = self.toast_events(action.events(&self.toasts));
            return;
        }

        let handled = match self.page.selected() {
            Some(Page::Overview) => self.overview_key(key),
            Some(Page::Jobs) => self.jobs_key(key),
            Some(Page::Activity) => self.activity_key(key),
            Some(Page::Settings) => match self.settings.key(&self.keys.settings, key) {
                Some(message) => {
                    let _ = self.handle(Message::Settings(message));
                    true
                }
                None => false,
            },
            None => false,
        };
        if handled {
            return;
        }
        if let Some(event) = self
            .keys
            .tabs
            .resolve_event(key)
            .and_then(|action| action.event(&self.page))
        {
            let _ = self.page.update(event);
        }
    }

    fn overview_key(&mut self, key: &keys::Event) -> bool {
        let Some(action) = self.keys.accordion.resolve_event(key) else {
            return false;
        };
        for event in action.events(&self.runbooks, &RUNBOOKS) {
            let _ = self.runbooks.update(event);
        }
        true
    }

    fn activity_key(&mut self, key: &keys::Event) -> bool {
        let Some(action) = self.keys.switch.resolve_event(key) else {
            return false;
        };
        let _ = self.handle(Message::Live(action.apply(self.live)));
        true
    }

    fn jobs_key(&mut self, key: &keys::Event) -> bool {
        use iced::keyboard::Key;

        if let Some(id) = self.cursor {
            if let Some(event) = self.row_menu.key_event(&self.keys.row_menu, key, id) {
                let _ = self.row_menu_event(event);
                return true;
            }
            if let Some(action) = self.keys.checkbox.resolve_event(key) {
                let checked = self.job(id).is_some_and(|job| job.checked);
                let checked = action.apply(CheckState::from(checked));
                let _ = self.handle(Message::Check(id, checked));
                return true;
            }
            if Chord::named(Named::Delete).matches(&key.key, key.modifiers) {
                self.pending = Some(Pending::Delete(id));
                return true;
            }
        }

        if !key.modifiers.is_empty() {
            return false;
        }
        let visible: Vec<JobId> = self.visible_jobs().map(|job| job.id).collect();
        let current = self
            .cursor
            .and_then(|id| visible.iter().position(|visible| *visible == id));
        let last = visible.len().saturating_sub(1);
        let target = match (&key.key, current) {
            (Key::Named(Named::ArrowDown), Some(index)) => (index + 1).min(last),
            (Key::Named(Named::ArrowUp), Some(index)) => index.saturating_sub(1),
            (Key::Named(Named::ArrowDown | Named::Home), None) | (Key::Named(Named::Home), _) => 0,
            (Key::Named(Named::ArrowUp | Named::End), None) | (Key::Named(Named::End), _) => last,
            _ => return false,
        };
        self.cursor = visible.get(target).copied();
        true
    }

    fn menu_event(&mut self, event: dropdown_menu::Event<MenuItem>) {
        match self.menu.update(event) {
            Some(MenuOutput::Toggled(MenuItem::Sidebar, on)) => self.toggle_sidebar(on),
            Some(MenuOutput::Toggled(MenuItem::Compact, on)) => self.compact = on,
            Some(MenuOutput::Selected(item)) => self.menu_selected(item),
            Some(MenuOutput::Activated(MenuItem::Export)) => self.notify(
                toast("Export ready")
                    .description(format!("fernhill-jobs.csv has {} rows.", self.jobs.len())),
            ),
            Some(MenuOutput::Activated(MenuItem::SignOut)) => self.notify(
                toast("You are still signed in")
                    .description("Signing out is switched off in the demo."),
            ),
            _ => {}
        }
    }

    /// Shows or hides the sidebar, or opens it as a dialog when the window
    /// is too narrow to show it beside the page.
    fn toggle_sidebar(&mut self, on: bool) {
        if self.width < WIDE {
            self.drawer = true;
        } else {
            self.sidebar = on;
        }
        self.menu.set_checked(MenuItem::Sidebar, self.sidebar);
    }

    fn menu_selected(&mut self, item: MenuItem) {
        let environment = match item {
            MenuItem::System => return self.set_theme(ThemeMode::System),
            MenuItem::Light => return self.set_theme(ThemeMode::Light),
            MenuItem::Dark => return self.set_theme(ThemeMode::Dark),
            MenuItem::Production => Environment::Production,
            MenuItem::Staging => Environment::Staging,
            MenuItem::Development => Environment::Development,
            _ => return,
        };
        self.environment = environment;
        self.notify(toast(format!("Switched to {}", environment.label())));
    }

    /// A theme picked in the app, which the hosting page follows.
    fn set_theme(&mut self, mode: ThemeMode) {
        self.apply_theme(mode);
        bridge::report_theme(match mode {
            ThemeMode::System => None,
            ThemeMode::Light => Some(ThemeChoice::Light),
            ThemeMode::Dark => Some(ThemeChoice::Dark),
        });
    }

    fn apply_theme(&mut self, mode: ThemeMode) {
        self.theme = mode;
        for (item, candidate) in [
            (MenuItem::System, ThemeMode::System),
            (MenuItem::Light, ThemeMode::Light),
            (MenuItem::Dark, ThemeMode::Dark),
        ] {
            self.menu.set_checked(item, candidate == mode);
        }
    }

    fn open_palette(&mut self) {
        self.palette = self.palette_state();
        self.palette_open = true;
        self.help = false;
    }

    /// Runs a palette or finder command. Most of them say what they did with a toast.
    fn run(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Go(page) => self.go(page),
            Cmd::FailedJobs => self.show_jobs(StatusFilter::Only(Status::Failed), ""),
            Cmd::RunningJobs => self.show_jobs(StatusFilter::Only(Status::Running), ""),
            Cmd::MyJobs => self.show_jobs(StatusFilter::All, data::ME),
            Cmd::NewJob => {
                let _ = self.handle(Message::OpenNewJob);
            }
            Cmd::RetryFailed => {
                let retried = self
                    .jobs
                    .iter_mut()
                    .filter(|job| job.status == Status::Failed)
                    .fold(0, |n, job| {
                        retry(job);
                        n + 1
                    });
                self.notify(toast(format!("Retrying {}", count(retried, "failed job"))));
                self.schedule();
            }
            Cmd::ToggleScheduler => {
                let on = !self.settings.applied().scheduler;
                self.settings.set_scheduler(on);
                let state = if on { "resumed" } else { "paused" };
                self.notify(toast(format!("Scheduler {state}")));
                self.schedule();
            }
            Cmd::ClearFinished => self.pending = Some(Pending::ClearFinished),
            Cmd::ToggleTheme => self.set_theme(match self.theme {
                ThemeMode::Dark => ThemeMode::Light,
                ThemeMode::System | ThemeMode::Light => ThemeMode::Dark,
            }),
            Cmd::ToggleSidebar => self.toggle_sidebar(!self.sidebar),
            Cmd::ClearToasts => {
                let _ = self.toast_events(toast::Action::DismissAll.events(&self.toasts));
            }
            Cmd::Shortcuts => self.help = true,
            Cmd::Open(id) => {
                self.show_jobs(StatusFilter::All, "");
                self.cursor = Some(id);
            }
        }
    }

    fn go(&mut self, page: Page) {
        let _ = self.page.update(tabs::Event::Select(page));
    }

    fn show_jobs(&mut self, filter: StatusFilter, search: &str) {
        self.filter = filter;
        self.search = search.to_owned();
        self.untick_hidden();
        self.go(Page::Jobs);
    }

    fn create_job(&mut self) {
        let Some(submission) = self.draft.submit() else {
            return;
        };
        let id = JobId(self.next_id);
        self.next_id += 1;
        let mut job = Job::new(id, submission.name, data::ME, submission.source);
        job.queue = submission.queue;
        job.schedule = submission.schedule;
        job.notes = submission.notes;
        job.status = if submission.schedule != Schedule::Once {
            Status::Scheduled
        } else if submission.start_now {
            Status::Queued
        } else {
            Status::Paused
        };
        let region = self.settings.applied().region;
        self.notify(
            toast(format!("Created {}", job.name))
                .description(format!("{id}, {}, runs in {region}", submission.schedule))
                .variant(ToastVariant::Success),
        );
        self.jobs.insert(0, job);
        if let Some(frozen) = &mut self.frozen {
            frozen.push(id);
        }
        self.new_job_open = false;
        self.cursor = Some(id);
        self.schedule();
    }

    fn row_menu_event(&mut self, event: context_menu::Event<RowAction, JobId>) -> Task<Message> {
        if let context_menu::Event::Open(id, _) | context_menu::Event::OpenFromKeyboard(id) = event
        {
            self.cursor = Some(id);
            self.prepare_row_menu(id);
        }
        let output = self.row_menu.update(event);
        let Some(&target) = self.row_menu.target() else {
            return Task::none();
        };
        match output {
            Some(MenuOutput::Activated(action)) => return self.row_action(target, action),
            Some(MenuOutput::Selected(action)) => {
                let queue = match action {
                    RowAction::High => Queue::High,
                    RowAction::Low => Queue::Low,
                    _ => Queue::Normal,
                };
                if let Some(job) = self.job_mut(target) {
                    job.queue = queue;
                }
            }
            _ => {}
        }
        Task::none()
    }

    /// Enables the actions that make sense for the job's status.
    fn prepare_row_menu(&mut self, id: JobId) {
        let Some(job) = self.job(id) else {
            return;
        };
        let (status, queue) = (job.status, job.queue);
        self.row_menu
            .set_disabled(RowAction::Retry, status != Status::Failed);
        self.row_menu
            .set_disabled(RowAction::Pause, status != Status::Running);
        self.row_menu
            .set_disabled(RowAction::Resume, status != Status::Paused);
        for (action, candidate) in [
            (RowAction::High, Queue::High),
            (RowAction::Normal, Queue::Normal),
            (RowAction::Low, Queue::Low),
        ] {
            self.row_menu.set_checked(action, candidate == queue);
        }
    }

    fn row_action(&mut self, id: JobId, action: RowAction) -> Task<Message> {
        let Some(job) = self.job_mut(id) else {
            return Task::none();
        };
        match action {
            RowAction::Logs => {
                self.log_query = job.name.clone();
                self.go(Page::Activity);
            }
            RowAction::Retry => {
                retry(job);
                self.schedule();
            }
            RowAction::Pause => job.status = Status::Paused,
            RowAction::Resume => {
                job.status = Status::Queued;
                self.schedule();
            }
            RowAction::CopyId => {
                let message = format!("Copied {id}");
                self.notify(toast(message).description("Paste it into a ticket or a query."));
                return bridge::copy(id.to_string());
            }
            RowAction::Delete => self.pending = Some(Pending::Delete(id)),
            RowAction::Queue | RowAction::High | RowAction::Normal | RowAction::Low => {}
        }
        Task::none()
    }

    fn pause_selected(&mut self) {
        let selected = self.selected();
        let mut paused = 0;
        for job in self
            .jobs
            .iter_mut()
            .filter(|job| selected.contains(&job.id))
        {
            job.checked = false;
            if job.status == Status::Running {
                job.status = Status::Paused;
                paused += 1;
            }
        }
        self.notify(toast(format!("Paused {}", count(paused, "job"))));
        self.schedule();
    }

    fn confirm(&mut self) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        let selected = self.selected();
        // Each removed job keeps its place in the list so Undo can put it back.
        let (removed, kept): (Vec<Placed>, Vec<Placed>) = std::mem::take(&mut self.jobs)
            .into_iter()
            .enumerate()
            .partition(|(_, job)| match pending {
                Pending::Delete(id) => job.id == id,
                Pending::DeleteSelected => selected.contains(&job.id),
                Pending::ClearFinished => job.status.is_finished(),
            });
        self.jobs = kept.into_iter().map(|(_, job)| job).collect();
        if removed.is_empty() {
            return;
        }
        if self
            .cursor
            .is_some_and(|id| removed.iter().any(|(_, job)| job.id == id))
        {
            self.cursor = None;
        }

        let title = match removed.as_slice() {
            [(_, job)] => format!("Deleted {} ({})", job.name, job.id),
            jobs => format!("Deleted {}", count(jobs.len(), "job")),
        };
        let retention = self.settings.applied().retention;
        let _ = self.toasts.push_with(
            toast(title)
                .description(format!("Logs are kept for {retention} days."))
                .action("Undo"),
            Message::Restore(removed),
        );
        self.schedule();
    }

    /// Puts deleted jobs back where they were, earliest position first.
    fn restore(&mut self, jobs: Vec<Placed>) {
        let title = match jobs.as_slice() {
            [(_, job)] => format!("Restored {}", job.name),
            jobs => format!("Restored {}", count(jobs.len(), "job")),
        };
        for (index, job) in jobs {
            if let Some(frozen) = &mut self.frozen {
                frozen.push(job.id);
            }
            let index = index.min(self.jobs.len());
            self.jobs.insert(index, job);
        }
        self.notify(toast(title));
        self.schedule();
    }

    fn toast_event(&mut self, event: toast::Event) -> Task<Message> {
        let event = match event {
            toast::Event::Received(batch) => toast::Event::Received(self.allowed(batch)),
            event => event,
        };
        match self.toasts.update(event) {
            Some(toast::Output::Ready(sender)) => self.toast_sender = Some(sender),
            Some(toast::Output::Payload(message)) => return self.handle(message),
            Some(toast::Output::Action(..)) | None => {}
        }
        Task::none()
    }

    fn toast_events(&mut self, events: Vec<toast::Event>) -> Task<Message> {
        Task::batch(events.into_iter().map(|event| self.toast_event(event)))
    }

    /// Worker toasts that the notification settings let through.
    fn allowed(&self, batch: Vec<toast::Toast>) -> Vec<toast::Toast> {
        let settings = self.settings.applied();
        if !settings.toasts {
            return Vec::new();
        }
        batch
            .into_iter()
            .filter(|toast| !settings.failures_only || toast.variant == ToastVariant::Destructive)
            .collect()
    }

    fn notify(&mut self, toast: toast::Toast) {
        let _ = self.toasts.push(toast);
    }

    fn apply(&mut self, update: Update) {
        match update {
            Update::Progress(id, progress) => {
                if let Some(job) = self.job_mut(id).filter(|job| job.status == Status::Running) {
                    job.progress = progress;
                }
            }
            Update::Succeeded(id) => {
                let Some(job) = self.job_mut(id) else {
                    return;
                };
                if job.status == Status::Succeeded {
                    return;
                }
                job.status = Status::Succeeded;
                job.progress = 1.0;
                self.succeeded += 1;
            }
            Update::Failed(id, progress) => self.failed(id, progress),
            Update::Log(line) => self.log(line),
            Update::Submitted(template) => self.submitted(template),
        }
    }

    /// Starts a failed job again while it has retries left, and otherwise
    /// marks it failed and tells the owner.
    fn failed(&mut self, id: JobId, progress: f32) {
        let settings = self.settings.applied();
        let (retries, channel) = (settings.retries, settings.channel);
        let Some(job) = self.job_mut(id) else {
            return;
        };
        if job.attempts < retries {
            job.attempts += 1;
            job.status = Status::Queued;
            job.progress = 0.0;
            let line = format!(
                "{} ({id}) retrying, attempt {} of {retries}",
                job.name, job.attempts
            );
            self.log(LogLine::new(Level::Warning, "scheduler", line));
            return;
        }
        job.status = Status::Failed;
        job.progress = progress;
        let runs = usize::from(job.attempts) + 1;
        let title = format!("{} failed", job.name);
        let description = format!(
            "It ran out of memory after {}. The owner was alerted by {}.",
            count(runs, "attempt"),
            channel.to_string().to_lowercase()
        );
        let failure = toast(title)
            .description(description)
            .variant(ToastVariant::Destructive);
        for toast in self.allowed(vec![failure]) {
            self.notify(toast);
        }
    }

    fn submitted(&mut self, template: usize) {
        let Some((name, owner, source)) = TEMPLATES.get(template) else {
            return;
        };
        // A long queue turns further submissions away.
        if self.count(Status::Queued) >= QUEUE_LIMIT {
            return;
        }
        let id = JobId(self.next_id);
        self.next_id += 1;
        let mut job = Job::new(id, *name, owner, *source);
        job.queue = Queue::ALL[template % Queue::ALL.len()];
        self.jobs.insert(0, job);
        self.log(LogLine::new(
            Level::Info,
            "scheduler",
            format!("{name} ({id}) submitted by {owner}"),
        ));
        if self.holding() {
            self.fresh.push(id);
        } else {
            self.trim_jobs();
        }
    }

    fn log(&mut self, line: LogLine) {
        if !self.live {
            self.held.push_back(line);
            if self.held.len() > LOG_LIMIT {
                let _ = self.held.pop_front();
            }
            return;
        }
        self.clock += 3;
        self.logs.push_front((self.clock, line));
        self.logs.truncate(LOG_LIMIT);
    }

    /// Drops the oldest finished jobs once the list is long.
    fn trim_jobs(&mut self) {
        while self.jobs.len() > JOB_LIMIT {
            let Some(index) = self.jobs.iter().rposition(|job| job.status.is_finished()) else {
                return;
            };
            self.jobs.remove(index);
        }
    }

    /// Whether something points at the job list: a menu, a dialog or the
    /// pointer itself. New jobs wait and rows stay put until it stops.
    fn holding(&self) -> bool {
        self.row_menu.is_open()
            || self.menu.is_open()
            || self.list_hovered
            || self.pending.is_some()
            || self.new_job_open
            || self.palette_open
            || self.drawer
    }

    fn settle_list(&mut self) {
        if self.holding() {
            if self.frozen.is_none() {
                self.frozen = Some(self.filtered().map(|job| job.id).collect());
            }
            return;
        }
        self.frozen = None;
        if !self.fresh.is_empty() {
            self.fresh.clear();
            self.trim_jobs();
        }
    }

    /// Lets the held jobs into the list, from the "new jobs" pill.
    fn show_new(&mut self) {
        self.fresh.clear();
        self.frozen = None;
        self.trim_jobs();
    }

    /// Unticks jobs that the search or filter has hidden, so bulk actions
    /// only ever act on rows people can see.
    fn untick_hidden(&mut self) {
        self.frozen = None;
        let shown: Vec<JobId> = self.filtered().map(|job| job.id).collect();
        for job in &mut self.jobs {
            if !shown.contains(&job.id) {
                job.checked = false;
            }
        }
    }

    /// Starts queued jobs, high priority first, while slots are free.
    fn schedule(&mut self) {
        let settings = self.settings.applied();
        if !settings.scheduler {
            return;
        }
        let slots = usize::from(settings.concurrency);
        for queue in Queue::ALL {
            loop {
                if self.count(Status::Running) >= slots {
                    return;
                }
                let Some(job) = self
                    .jobs
                    .iter_mut()
                    .rev()
                    .find(|job| job.status == Status::Queued && job.queue == queue)
                else {
                    break;
                };
                job.status = Status::Running;
                job.started = data::clock(self.clock)[..5].to_owned();
            }
        }
    }

    /// The simulated worker for the jobs running now, once both channels are open.
    fn worker(&self) -> Option<Worker> {
        let feed = self.feed.clone()?;
        let toasts = self.toast_sender.clone()?;
        let scheduler = self.settings.applied().scheduler;
        let jobs = self
            .jobs
            .iter()
            .filter(|job| scheduler && job.status == Status::Running)
            .map(|job| feed::Running {
                id: job.id,
                name: job.name.clone(),
                progress: job.progress,
                speed: job.speed,
                fails_at: job.fails_at,
            })
            .collect();
        Some(Worker { jobs, feed, toasts })
    }

    fn job(&self, id: JobId) -> Option<&Job> {
        self.jobs.iter().find(|job| job.id == id)
    }

    fn job_mut(&mut self, id: JobId) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|job| job.id == id)
    }

    /// Jobs that pass the search and the status filter, leaving out new
    /// jobs held behind the pill, in list order.
    fn filtered(&self) -> impl Iterator<Item = &Job> {
        self.jobs
            .iter()
            .filter(move |job| !self.fresh.contains(&job.id) && self.matches(job))
    }

    /// Whether a job passes the search and the status filter.
    fn matches(&self, job: &Job) -> bool {
        let query = self.search.trim().to_lowercase();
        self.filter.allows(job.status)
            && (query.is_empty()
                || job.name.to_lowercase().contains(&query)
                || job.owner.to_lowercase().contains(&query)
                || job.id.to_string().to_lowercase().contains(&query))
    }

    /// How many held jobs the list would show once they are let in.
    fn fresh_shown(&self) -> usize {
        self.fresh
            .iter()
            .filter(|id| self.job(**id).is_some_and(|job| self.matches(job)))
            .count()
    }

    /// The rows on show: the filtered jobs, or the rows that were on show
    /// when something started pointing at the list.
    fn visible_jobs(&self) -> impl Iterator<Item = &Job> {
        self.jobs.iter().filter(move |job| match &self.frozen {
            Some(frozen) => frozen.contains(&job.id),
            None => self.filtered().any(|shown| shown.id == job.id),
        })
    }

    /// The ticked jobs among the rows on show.
    fn selected(&self) -> Vec<JobId> {
        self.visible_jobs()
            .filter(|job| job.checked)
            .map(|job| job.id)
            .collect()
    }

    fn count(&self, status: Status) -> usize {
        self.jobs.iter().filter(|job| job.status == status).count()
    }
}

fn retry(job: &mut Job) {
    job.status = Status::Queued;
    job.progress = 0.0;
    job.fails_at = None;
    job.attempts = 0;
}

/// "1 job", "3 jobs": a number and a noun that agrees with it.
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// Small secondary text.
fn muted<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content)
        .size(text_size::SM)
        .style(|theme| text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        })
}

/// Extra small secondary text, for captions and column headings.
fn caption<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    muted(content).size(text_size::XS)
}
