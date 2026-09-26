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
use iced::widget::{Text, column, container, responsive, row, text};
use iced::{Element, Length, Size, Subscription, Theme};
use iced_cube::context_menu;
use iced_cube::dropdown_menu::{self, Output as MenuOutput};
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
use iced_cube::{separator, vertical_separator};

use data::{
    Environment, Job, JobId, LevelFilter, LogLine, Queue, Range, Runbook, Status, StatusFilter,
    TEMPLATES,
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
const RUNBOOKS: [Runbook; 3] = [Runbook::Degraded, Runbook::Retry, Runbook::Escalation];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Jobs,
    Activity,
    Settings,
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
    Toast(toast::Event),
    Restore(Vec<Placed>),
    Tick(Instant),
    Key(keys::Event),
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
    sidebar: bool,
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
    search: String,
    filter: StatusFilter,
    cursor: Option<JobId>,
    row_menu: context_menu::State<RowAction, JobId>,
    logs: VecDeque<(u32, LogLine)>,
    held: Vec<LogLine>,
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
            sidebar: true,
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
            search: String::new(),
            filter: StatusFilter::All,
            cursor: None,
            row_menu: context_menu::State::new(jobs::row_menu_entries()),
            logs: data::logs().into(),
            held: Vec::new(),
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
    pub fn update(&mut self, message: Message) {
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
                    self.run(cmd);
                }
            }
            Message::Run(cmd) => self.run(cmd),
            Message::OpenNewJob => {
                self.draft = Draft::default();
                self.new_job_open = true;
            }
            Message::CloseNewJob => self.new_job_open = false,
            Message::Draft(message) => self.draft.update(message),
            Message::CreateJob => self.create_job(),
            Message::Search(search) => self.search = search,
            Message::Filter(filter) => self.filter = filter,
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
            Message::RowMenu(event) => self.row_menu_event(event),
            Message::PauseSelected => {
                let paused = self
                    .jobs
                    .iter_mut()
                    .filter(|job| job.checked)
                    .fold(0, |n, job| {
                        job.checked = false;
                        if job.status != Status::Running {
                            return n;
                        }
                        job.status = Status::Paused;
                        n + 1
                    });
                self.notify(toast(format!("Paused {paused} running jobs")));
                self.schedule();
            }
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
                    self.notify(
                        toast("Settings saved")
                            .description("Everyone in the workspace sees the new settings.")
                            .variant(ToastVariant::Success),
                    );
                }
                self.schedule();
            }
            Message::Refresh => {
                self.log(LogLine::new(
                    data::Level::Info,
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
            Message::Toast(event) => self.toast_event(event),
            Message::Restore(jobs) => self.restore(jobs),
            Message::Tick(now) => {
                if let Some(last) = self.last_tick {
                    self.phase = advance(self.phase, now - last);
                }
                self.last_tick = Some(now);
            }
            Message::Key(key) => self.key(&key),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let app = responsive(move |size| {
            let narrow = size.width < NARROW;
            self.dialogs(self.shell(size), narrow)
        });

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
            feed::subscription().map(Message::Feed),
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

    /// The header, the sidebar on wide windows, the tabs and the open page.
    fn shell(&self, size: Size) -> Element<'_, Message> {
        let narrow = size.width < NARROW;
        let sidebar = self.sidebar && size.width >= WIDE;
        let gutter = if narrow { 16.0 } else { 24.0 };
        let content = size.width - 2.0 * gutter - if sidebar { SIDEBAR + 1.0 } else { 0.0 };

        let page = match self.page.selected().unwrap_or(Page::Overview) {
            Page::Overview => self.overview(content),
            Page::Jobs => self.jobs_page(narrow),
            Page::Activity => self.activity(narrow),
            Page::Settings => self.settings_page(narrow, gutter),
        };
        let main = column![
            container(
                tabs::tabs(&self.page)
                    .width(Length::Fill)
                    .on_event(Message::Tabs)
            )
            .padding([12.0, gutter]),
            container(page)
                .padding(iced::Padding {
                    top: 4.0,
                    right: gutter,
                    bottom: 0.0,
                    left: gutter,
                })
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill);

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
                    bottom: gutter,
                    right: if narrow { 0.0 } else { 12.0 },
                    ..iced::Padding::ZERO
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
        if self.palette_open || self.new_job_open || self.pending.is_some() {
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
            self.toast_events(action.events(&self.toasts));
            return;
        }

        let handled = match self.page.selected() {
            Some(Page::Overview) => self.overview_key(key),
            Some(Page::Jobs) => self.jobs_key(key),
            Some(Page::Activity) => self.activity_key(key),
            Some(Page::Settings) => match self.settings.key(&self.keys.settings, key) {
                Some(message) => {
                    self.update(Message::Settings(message));
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
        self.update(Message::Live(action.apply(self.live)));
        true
    }

    fn jobs_key(&mut self, key: &keys::Event) -> bool {
        use iced::keyboard::Key;

        if let Some(id) = self.cursor {
            if let Some(event) = self.row_menu.key_event(&self.keys.row_menu, key, id) {
                self.row_menu_event(event);
                return true;
            }
            if let Some(action) = self.keys.checkbox.resolve_event(key) {
                let checked = self.job(id).is_some_and(|job| job.checked);
                let checked = action.apply(CheckState::from(checked));
                self.update(Message::Check(id, checked));
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
            Some(MenuOutput::Toggled(MenuItem::Sidebar, on)) => self.sidebar = on,
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

    fn set_theme(&mut self, mode: ThemeMode) {
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
            Cmd::NewJob => self.update(Message::OpenNewJob),
            Cmd::RetryFailed => {
                let retried = self
                    .jobs
                    .iter_mut()
                    .filter(|job| job.status == Status::Failed)
                    .fold(0, |n, job| {
                        retry(job);
                        n + 1
                    });
                self.notify(toast(format!("Retrying {retried} failed jobs")));
                self.schedule();
            }
            Cmd::ToggleScheduler => {
                self.settings.scheduler = !self.settings.scheduler;
                let state = if self.settings.scheduler {
                    "resumed"
                } else {
                    "paused"
                };
                self.notify(toast(format!("Scheduler {state}")));
                self.schedule();
            }
            Cmd::ClearFinished => self.pending = Some(Pending::ClearFinished),
            Cmd::ToggleTheme => self.set_theme(match self.theme {
                ThemeMode::Dark => ThemeMode::Light,
                ThemeMode::System | ThemeMode::Light => ThemeMode::Dark,
            }),
            Cmd::ToggleSidebar => {
                self.sidebar = !self.sidebar;
                self.menu.set_checked(MenuItem::Sidebar, self.sidebar);
            }
            Cmd::ClearToasts => {
                self.toast_events(toast::Action::DismissAll.events(&self.toasts));
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
        job.status = if submission.start_now {
            Status::Queued
        } else {
            Status::Paused
        };
        self.notify(
            toast(format!("Created {}", job.name))
                .description(format!("{id}, {}", submission.schedule))
                .variant(ToastVariant::Success),
        );
        self.jobs.insert(0, job);
        self.new_job_open = false;
        self.cursor = Some(id);
        self.schedule();
    }

    fn row_menu_event(&mut self, event: context_menu::Event<RowAction, JobId>) {
        if let context_menu::Event::Open(id, _) | context_menu::Event::OpenFromKeyboard(id) = event
        {
            self.cursor = Some(id);
            self.prepare_row_menu(id);
        }
        let output = self.row_menu.update(event);
        let Some(&target) = self.row_menu.target() else {
            return;
        };
        match output {
            Some(MenuOutput::Activated(action)) => self.row_action(target, action),
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

    fn row_action(&mut self, id: JobId, action: RowAction) {
        let Some(job) = self.job_mut(id) else {
            return;
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
            }
            RowAction::Delete => self.pending = Some(Pending::Delete(id)),
            RowAction::Queue | RowAction::High | RowAction::Normal | RowAction::Low => {}
        }
    }

    fn confirm(&mut self) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        // Each removed job keeps its place in the list so Undo can put it back.
        let (removed, kept): (Vec<Placed>, Vec<Placed>) = std::mem::take(&mut self.jobs)
            .into_iter()
            .enumerate()
            .partition(|(_, job)| match pending {
                Pending::Delete(id) => job.id == id,
                Pending::DeleteSelected => job.checked,
                Pending::ClearFinished => job.status.is_finished(),
            });
        self.jobs = kept.into_iter().map(|(_, job)| job).collect();
        if self
            .cursor
            .is_some_and(|id| removed.iter().any(|(_, job)| job.id == id))
        {
            self.cursor = None;
        }

        let title = match removed.as_slice() {
            [(_, job)] => format!("Deleted {}", job.name),
            jobs => format!("Deleted {} jobs", jobs.len()),
        };
        let _ = self.toasts.push_with(
            toast(title)
                .description("Their logs are kept for the retention period.")
                .action("Undo"),
            Message::Restore(removed),
        );
        self.schedule();
    }

    /// Puts deleted jobs back where they were, earliest position first.
    fn restore(&mut self, jobs: Vec<Placed>) {
        let title = match jobs.as_slice() {
            [(_, job)] => format!("Restored {}", job.name),
            jobs => format!("Restored {} jobs", jobs.len()),
        };
        for (index, job) in jobs {
            let index = index.min(self.jobs.len());
            self.jobs.insert(index, job);
        }
        self.notify(toast(title));
        self.schedule();
    }

    fn toast_event(&mut self, event: toast::Event) {
        let event = match event {
            toast::Event::Received(batch) => toast::Event::Received(self.allowed(batch)),
            event => event,
        };
        match self.toasts.update(event) {
            Some(toast::Output::Ready(sender)) => self.toast_sender = Some(sender),
            Some(toast::Output::Payload(message)) => self.update(message),
            Some(toast::Output::Action(..)) | None => {}
        }
    }

    fn toast_events(&mut self, events: Vec<toast::Event>) {
        for event in events {
            self.toast_event(event);
        }
    }

    /// Worker toasts that the notification settings let through.
    fn allowed(&self, batch: Vec<toast::Toast>) -> Vec<toast::Toast> {
        if !self.settings.toasts {
            return Vec::new();
        }
        batch
            .into_iter()
            .filter(|toast| {
                !self.settings.failures_only || toast.variant == ToastVariant::Destructive
            })
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
                if let Some(job) = self.job_mut(id) {
                    job.status = Status::Succeeded;
                    job.progress = 1.0;
                }
            }
            Update::Failed(id, progress) => {
                if let Some(job) = self.job_mut(id) {
                    job.status = Status::Failed;
                    job.progress = progress;
                }
            }
            Update::Log(line) => self.log(line),
            Update::Submitted(template) => {
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
                    data::Level::Info,
                    "scheduler",
                    format!("{name} ({id}) submitted by {owner}"),
                ));
                self.trim_jobs();
            }
        }
    }

    fn log(&mut self, line: LogLine) {
        if !self.live {
            self.held.push(line);
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

    /// Starts queued jobs, high priority first, while slots are free.
    fn schedule(&mut self) {
        if !self.settings.scheduler {
            return;
        }
        let slots = usize::from(self.settings.concurrency);
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
        let jobs = self
            .jobs
            .iter()
            .filter(|job| self.settings.scheduler && job.status == Status::Running)
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

    /// Jobs that pass the search and the status filter, in list order.
    fn visible_jobs(&self) -> impl Iterator<Item = &Job> {
        let query = self.search.trim().to_lowercase();
        self.jobs.iter().filter(move |job| {
            self.filter.allows(job.status)
                && (query.is_empty()
                    || job.name.to_lowercase().contains(&query)
                    || job.owner.to_lowercase().contains(&query)
                    || job.id.to_string().to_lowercase().contains(&query))
        })
    }

    fn count(&self, status: Status) -> usize {
        self.jobs.iter().filter(|job| job.status == status).count()
    }
}

fn retry(job: &mut Job) {
    job.status = Status::Queued;
    job.progress = 0.0;
    job.fails_at = None;
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
