//! The global searchable launcher: a [command list](mod@crate::navigation::command)
//! in a [dialog](mod@crate::overlay::dialog), opened from anywhere with Ctrl+K
//! or Ctrl+Shift+P.
//!
//! [`State`] owns whether the palette is open, its groups of commands, any
//! nested [`Page`]s and the recently run commands, which it lists first
//! while the query is empty. Choosing an item whose id names a page, such
//! as "Change theme...", opens that page's list instead of running it;
//! Backspace on an empty query, Escape or the Back button returns to the
//! page before.
//!
//! The app routes key presses from [`keys::subscription`] through
//! [`State::key_event`] so the toggle chords open the palette anywhere.
//! While it is open, the palette resolves its own keys: the command
//! list's first, then Back and the toggle chords, then the dialog's. Async
//! results arrive through the command list's channel: [`subscription`]
//! wraps [`command::subscription`], and [`Output::Search`] asks the app to
//! search the current page for a query.
//!
//! ```no_run
//! use iced::{Element, Subscription};
//! use iced::widget::text;
//! use iced_cube::command_palette::{self, Output, command_palette, page};
//! use iced_cube::keys;
//! use iced_cube::navigation::command::{group, item};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Palette(command_palette::Event<&'static str>),
//!     Key(keys::Event),
//! }
//!
//! struct App {
//!     palette: command_palette::State<&'static str>,
//! }
//!
//! impl App {
//!     fn new() -> Self {
//!         let palette = command_palette::State::new([group(
//!             "Actions",
//!             [item("new", "New file"), item("theme", "Change theme...")],
//!         )])
//!         .with_page(page("theme", "Theme", [group("", [item("dark", "Dark"), item("light", "Light")])]));
//!         Self { palette }
//!     }
//!
//!     fn update(&mut self, message: Message) {
//!         let event = match message {
//!             Message::Palette(event) => Some(event),
//!             Message::Key(key) => self
//!                 .palette
//!                 .key_event(&command_palette::default_keymap(), &key),
//!         };
//!         if let Some(Output::Activated(id)) = event.and_then(|event| self.palette.update(event)) {
//!             println!("Run {id}");
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         command_palette(&self.palette, text("Press Ctrl+K"))
//!             .on_event(Message::Palette)
//!             .into()
//!     }
//!
//!     fn subscription(&self) -> Subscription<Message> {
//!         keys::subscription().map(Message::Key)
//!     }
//! }
//! ```

use std::rc::Rc;

use iced::keyboard::key::Named;
use iced::widget;
use iced::{Element, Subscription};

use crate::keys::{self, Chord, Keymap};
use crate::navigation::command::{self, Group, Item, command, group};
use crate::overlay::anchored::anchored;
use crate::overlay::dialog::{self, dialog};
use crate::primitives::button::{self, Variant, button};

/// The widget id of the search field, to focus it or find it in tests.
pub const SEARCH_ID: widget::Id = widget::Id::new("iced-cube-command-palette-search");
/// The widget id of the dialog surface, which scopes its focus.
pub const DIALOG_ID: widget::Id = widget::Id::new("iced-cube-command-palette");
/// How many recent commands are kept unless the state sets its own number.
pub const RECENT_LIMIT: usize = 5;
/// The heading over the recent commands.
pub const RECENT_LABEL: &str = "Recent";
/// How many rows show at once unless the state sets its own number.
pub const VISIBLE_ROWS: usize = 8;
/// The dialog title on the first page unless the builder sets its own.
pub const DEFAULT_TITLE: &str = "Command palette";

/// A nested list, opened by choosing the item with the page's id.
#[derive(Debug, Clone, PartialEq)]
pub struct Page<Id> {
    pub id: Id,
    /// Shown as the dialog title while the page is open.
    pub title: String,
    /// Replaces the search field's placeholder while the page is open.
    pub placeholder: Option<String>,
    pub groups: Vec<Group<Id>>,
}

/// A page listing `groups`, opened by the item whose id is `id`.
pub fn page<Id>(
    id: Id,
    title: impl Into<String>,
    groups: impl IntoIterator<Item = Group<Id>>,
) -> Page<Id> {
    Page {
        id,
        title: title.into(),
        placeholder: None,
        groups: groups.into_iter().collect(),
    }
}

impl<Id> Page<Id> {
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }
}

/// Changes to a command palette.
#[derive(Debug, Clone)]
pub enum Event<Id> {
    /// Opens on the first page with an empty query.
    Open,
    Close,
    Toggle,
    /// Returns to the page before, if a nested page is open.
    Back,
    /// An event for the command list inside.
    Command(command::Event<Id>),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone)]
pub enum Output<Id> {
    /// The results channel is open. Keep the sender for producers.
    Ready(command::Sender<Id>),
    /// The query changed. Search [`State::page_id`] for it in the
    /// background, if anything is searched that way.
    Search(String),
    /// A command was chosen. The palette has closed and noted it as recent.
    Activated(Id),
}

/// Whether the palette is open, its pages, the recent commands and the
/// command list on show.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    open: bool,
    root: Vec<Group<Id>>,
    pages: Vec<Page<Id>>,
    stack: Vec<Id>,
    recent: Vec<Id>,
    recent_limit: usize,
    rows: usize,
    command: command::State<Id>,
}

impl<Id: Clone + PartialEq> State<Id> {
    /// A closed palette listing `groups` on its first page.
    pub fn new(groups: impl IntoIterator<Item = Group<Id>>) -> Self {
        let mut state = Self {
            open: false,
            root: groups.into_iter().collect(),
            pages: Vec::new(),
            stack: Vec::new(),
            recent: Vec::new(),
            recent_limit: RECENT_LIMIT,
            rows: VISIBLE_ROWS,
            command: command::State::new([]),
        };
        state.rebuild(String::new());
        state
    }

    /// Adds a nested page. Choosing the item with the page's id opens it.
    pub fn with_page(mut self, page: Page<Id>) -> Self {
        self.pages.push(page);
        self
    }

    /// Starts with these recent commands, newest first.
    pub fn with_recent(mut self, ids: impl IntoIterator<Item = Id>) -> Self {
        self.recent = ids.into_iter().take(self.recent_limit).collect();
        self.rebuild(String::new());
        self
    }

    /// How many recent commands to keep. Zero turns the recent group off.
    pub fn with_recent_limit(mut self, limit: usize) -> Self {
        self.recent_limit = limit;
        self.recent.truncate(limit);
        self.rebuild(String::new());
        self
    }

    /// How many rows show at once. At least one does.
    pub fn with_visible_rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self.rebuild(String::new());
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The nested page on show, or `None` on the first page.
    pub fn page(&self) -> Option<&Page<Id>> {
        let id = self.stack.last()?;
        self.pages.iter().find(|page| page.id == *id)
    }

    /// The id of the nested page on show.
    pub fn page_id(&self) -> Option<&Id> {
        self.stack.last()
    }

    /// How many nested pages are open.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Recent commands, newest first.
    pub fn recent(&self) -> &[Id] {
        &self.recent
    }

    /// The command list on show, with its query, results and highlight.
    pub fn command(&self) -> &command::State<Id> {
        &self.command
    }

    pub fn query(&self) -> &str {
        self.command.query()
    }

    /// Replaces the first page's groups, such as when a label changes.
    pub fn set_groups(&mut self, groups: impl IntoIterator<Item = Group<Id>>) {
        self.root = groups.into_iter().collect();
        if self.stack.is_empty() {
            self.rebuild(self.command.query().to_owned());
        }
    }

    /// Applies an event and returns what the app may need to act on.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Open => self.open(),
            Event::Close => {
                self.open = false;
                None
            }
            Event::Toggle if self.open => {
                self.open = false;
                None
            }
            Event::Toggle => self.open(),
            Event::Back => self.back(),
            Event::Command(command::Event::Input(query)) => self.input(query),
            Event::Command(event) => match self.command.update(event)? {
                command::Output::Ready(sender) => Some(Output::Ready(sender)),
                command::Output::Search(query) => Some(Output::Search(query)),
                command::Output::Activated(id) => self.activate(id),
                command::Output::Closed if self.stack.is_empty() => {
                    self.open = false;
                    None
                }
                command::Output::Closed => self.back(),
            },
        }
    }

    /// Turns a key press into an event: a toggle chord at any time, and
    /// Back only while a nested page is open with an empty query, so
    /// Backspace still deletes text.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event<Id>> {
        keymap
            .resolve_event(key)
            .and_then(|action| action.event(self))
    }

    fn open(&mut self) -> Option<Output<Id>> {
        let had_query = !self.command.query().is_empty();
        self.open = true;
        self.stack.clear();
        self.rebuild(String::new());
        had_query.then(|| Output::Search(String::new()))
    }

    fn back(&mut self) -> Option<Output<Id>> {
        self.stack.pop()?;
        let had_query = !self.command.query().is_empty();
        self.rebuild(String::new());
        had_query.then(|| Output::Search(String::new()))
    }

    fn input(&mut self, query: String) -> Option<Output<Id>> {
        let shows_recent = self.stack.is_empty() && !self.recent.is_empty();
        let was_empty = self.command.query().is_empty();
        if shows_recent && was_empty != query.is_empty() {
            self.rebuild(query.clone());
            return Some(Output::Search(query));
        }
        match self.command.update(command::Event::Input(query))? {
            command::Output::Search(query) => Some(Output::Search(query)),
            _ => None,
        }
    }

    fn activate(&mut self, id: Id) -> Option<Output<Id>> {
        if self.pages.iter().any(|page| page.id == id) {
            self.stack.push(id);
            self.rebuild(String::new());
            return None;
        }
        self.remember(id.clone());
        self.open = false;
        Some(Output::Activated(id))
    }

    /// Puts `id` first among the recent commands.
    fn remember(&mut self, id: Id) {
        self.recent.retain(|recent| *recent != id);
        self.recent.insert(0, id);
        self.recent.truncate(self.recent_limit);
    }

    /// The groups the list shows for the page on show: with the recent
    /// commands first on the first page while the query is empty.
    fn groups(&self, query: &str) -> Vec<Group<Id>> {
        let Some(id) = self.stack.last() else {
            let mut groups = Vec::with_capacity(self.root.len() + 1);
            let recent: Vec<Item<Id>> = self
                .recent
                .iter()
                .filter_map(|id| self.find(id).cloned())
                .collect();
            if query.is_empty() && !recent.is_empty() {
                groups.push(group(RECENT_LABEL, recent));
            }
            groups.extend(self.root.iter().cloned());
            return groups;
        };
        self.pages
            .iter()
            .find(|page| page.id == *id)
            .map(|page| page.groups.clone())
            .unwrap_or_default()
    }

    /// The first item with `id` on any page.
    fn find(&self, id: &Id) -> Option<&Item<Id>> {
        self.root
            .iter()
            .chain(self.pages.iter().flat_map(|page| &page.groups))
            .flat_map(|group| &group.items)
            .find(|item| item.id == *id)
    }

    /// Replaces the command list with the page on show, keeping focus.
    fn rebuild(&mut self, query: String) {
        let focused = self.command.is_focused();
        self.command = command::State::new(self.groups(&query)).with_visible_rows(self.rows);
        if !query.is_empty() {
            let _ = self.command.update(command::Event::Input(query));
        }
        if focused {
            let _ = self.command.update(command::Event::Focus(true));
        }
    }
}

/// Owns the command list's results channel. Emits [`command::Event::Ready`]
/// first, then batches, wrapped in [`Event::Command`].
pub fn subscription<Id: Send + 'static>() -> Subscription<Event<Id>> {
    command::subscription().map(Event::Command)
}

/// What a command palette keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Opens the palette, or closes it while it is open.
    Toggle,
    /// Returns to the page before.
    Back,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it would
    /// do nothing, so the key reaches the search field instead.
    pub fn event<Id: Clone + PartialEq>(self, state: &State<Id>) -> Option<Event<Id>> {
        match self {
            Action::Toggle => Some(Event::Toggle),
            Action::Back => {
                let back = state.open && !state.stack.is_empty() && state.query().is_empty();
                back.then_some(Event::Back)
            }
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Toggle, Action::Back];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Toggle => "Toggle",
            Action::Back => "Back",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Toggle => "Opens the palette from anywhere, or closes it.",
            Action::Back => "Returns to the page before, while the query is empty.",
        }
    }
}

/// The default command palette shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Ctrl+K`, `Ctrl+Shift+P` | [`Action::Toggle`] |
/// | `Backspace` | [`Action::Back`] |
///
/// Ctrl is Cmd on macOS. Route presses from [`keys::subscription`] through
/// [`State::key_event`] so the toggle works anywhere; the open dialog lets
/// the toggle chords through to the app. Back only claims Backspace on a
/// nested page with an empty query. The command list inside keeps its own
/// keys (arrows, Enter and Escape) through [`CommandPalette::command_keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::character('k').command(), Action::Toggle)
        .bind(Chord::character('p').command().shift(), Action::Toggle)
        .bind(Chord::named(Named::Backspace), Action::Back)
}

type OnEvent<'a, Id, Message> = Rc<dyn Fn(Event<Id>) -> Message + 'a>;

/// A command palette builder. Convert it into an [`Element`] to render.
pub struct CommandPalette<'a, Id, Message> {
    state: &'a State<Id>,
    base: Element<'a, Message>,
    title: String,
    placeholder: String,
    empty: Option<String>,
    loading: bool,
    max_height: f32,
    size: dialog::Size,
    keymap: Keymap<Action>,
    command_keymap: Keymap<command::Action>,
    dialog_keymap: Keymap<dialog::Action>,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for CommandPalette<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandPalette")
            .field("state", self.state)
            .field("title", &self.title)
            .field("placeholder", &self.placeholder)
            .field("loading", &self.loading)
            .field("max_height", &self.max_height)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// Wraps `base`, the app the palette opens over. Without
/// [`on_event`](CommandPalette::on_event) the list renders disabled.
pub fn command_palette<'a, Id, Message>(
    state: &'a State<Id>,
    base: impl Into<Element<'a, Message>>,
) -> CommandPalette<'a, Id, Message> {
    CommandPalette {
        state,
        base: base.into(),
        title: DEFAULT_TITLE.to_owned(),
        placeholder: "Type a command or search...".to_owned(),
        empty: None,
        loading: false,
        max_height: 400.0,
        size: dialog::Size::Md,
        keymap: default_keymap(),
        command_keymap: command::default_keymap(),
        dialog_keymap: dialog::default_keymap(),
        on_event: None,
    }
}

impl<'a, Id, Message> CommandPalette<'a, Id, Message> {
    /// The dialog title on the first page. Nested pages show their own.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// The search field's placeholder, unless the page sets its own.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Text shown when nothing matches the query.
    pub fn empty(mut self, empty: impl Into<String>) -> Self {
        self.empty = Some(empty.into());
        self
    }

    /// Shows that a background search is still running.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Caps the list's height in pixels. It shrinks around a few results.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }

    /// The dialog's maximum width. Defaults to [`dialog::Size::Md`].
    pub fn size(mut self, size: dialog::Size) -> Self {
        self.size = size;
        self
    }

    /// Replaces the [`default_keymap`]: the toggle chords and Back.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    /// Replaces the [command list keys](command::default_keymap).
    pub fn command_keymap(mut self, keymap: Keymap<command::Action>) -> Self {
        self.command_keymap = keymap;
        self
    }

    /// Replaces the [dialog keys](dialog::default_keymap).
    pub fn dialog_keymap(mut self, keymap: Keymap<dialog::Action>) -> Self {
        self.dialog_keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Id, Message> From<CommandPalette<'a, Id, Message>> for Element<'a, Message>
where
    Id: Clone + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(palette: CommandPalette<'a, Id, Message>) -> Self {
        let CommandPalette {
            state,
            base,
            title,
            placeholder,
            empty,
            loading,
            max_height,
            size,
            keymap,
            command_keymap,
            dialog_keymap,
            on_event,
        } = palette;
        let on_event: Option<OnEvent<'a, Id, Message>> = on_event.map(Rc::from);
        let page = state.page();
        let title = page.map_or(title, |page| page.title.clone());
        let placeholder = page
            .and_then(|page| page.placeholder.clone())
            .unwrap_or(placeholder);
        let toggles: Vec<Chord> = keymap
            .chords(&Action::Toggle)
            .into_iter()
            .cloned()
            .collect();

        let mut list = command(&state.command)
            .placeholder(placeholder)
            .loading(loading)
            .max_height(max_height)
            .id(SEARCH_ID)
            .keymap(command_keymap);
        if let Some(empty) = empty {
            list = list.empty(empty);
        }

        let mut palette = dialog(base)
            .open(state.open)
            .title(title)
            .size(size)
            .close_button(false)
            .id(DIALOG_ID)
            .keymap(dialog_keymap)
            .pass_through(toggles);
        let Some(on_event) = on_event else {
            return palette.body(list).into();
        };

        if !state.stack.is_empty() {
            palette = palette.action(
                button("Back")
                    .icon(crate::lucide!(ChevronLeft))
                    .variant(Variant::Ghost)
                    .size(button::Size::Sm)
                    .on_press(on_event(Event::Back)),
            );
        }
        let on_list = on_event.clone();
        let on_key = on_event.clone();
        let body = anchored(list.on_event(move |event| on_list(Event::Command(event))))
            .dismiss_keys([])
            .on_key(move |key| state.key_event(&keymap, key).map(&*on_key));
        palette.body(body).on_dismiss(on_event(Event::Close)).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::navigation::command::item;
    use iced::keyboard::{Key, Modifiers};

    fn state() -> State<&'static str> {
        State::new([
            group(
                "Files",
                [item("new", "New file"), item("open", "Open file")],
            ),
            group(
                "Preferences",
                [
                    item("theme", "Change theme..."),
                    item("settings", "Settings"),
                ],
            ),
        ])
        .with_page(
            page(
                "theme",
                "Theme",
                [group("", [item("light", "Light"), item("dark", "Dark")])],
            )
            .placeholder("Pick a theme..."),
        )
    }

    fn opened() -> State<&'static str> {
        let mut state = state();
        let _ = state.update(Event::Open);
        state
    }

    fn labels(state: &State<&'static str>) -> Vec<(String, String)> {
        state
            .command()
            .results()
            .map(|(group, item)| (group.to_owned(), item.label.clone()))
            .collect()
    }

    fn run(state: &mut State<&'static str>, id: &'static str) -> Option<Output<&'static str>> {
        let position = state
            .command()
            .results()
            .position(|(_, item)| item.id == id)
            .expect("the item is listed");
        state.update(Event::Command(command::Event::Activate(position)))
    }

    #[test]
    fn starts_closed_on_the_first_page_with_nothing_recent() {
        let state = state();
        assert!(!state.is_open());
        assert_eq!(state.depth(), 0);
        assert!(state.page().is_none() && state.page_id().is_none());
        assert!(state.recent().is_empty());
        assert_eq!(state.command().len(), 4);
        assert_eq!(state.query(), "");
    }

    #[test]
    fn open_close_and_toggle() {
        let mut state = state();
        assert!(state.update(Event::Open).is_none());
        assert!(state.is_open());
        let _ = state.update(Event::Close);
        assert!(!state.is_open());
        let _ = state.update(Event::Toggle);
        assert!(state.is_open());
        let _ = state.update(Event::Toggle);
        assert!(!state.is_open());
    }

    #[test]
    fn running_a_command_closes_and_remembers_it() {
        let mut state = opened();
        assert!(matches!(
            run(&mut state, "settings"),
            Some(Output::Activated("settings"))
        ));
        assert!(!state.is_open());
        assert_eq!(state.recent(), ["settings"]);

        let _ = state.update(Event::Open);
        let first = labels(&state).into_iter().next();
        assert_eq!(first, Some(("Recent".into(), "Settings".into())));
    }

    #[test]
    fn recent_commands_are_newest_first_unique_and_capped() {
        let mut state = state().with_recent_limit(2);
        for id in ["new", "open", "new", "settings"] {
            let _ = state.update(Event::Open);
            let _ = run(&mut state, id);
        }
        assert_eq!(state.recent(), ["settings", "new"]);

        let off = self::state().with_recent_limit(0).with_recent(["new"]);
        assert!(off.recent().is_empty());
    }

    #[test]
    fn recent_items_from_nested_pages_are_found() {
        let state = state().with_recent(["dark", "missing"]);
        let recent: Vec<_> = labels(&state)
            .into_iter()
            .filter(|(group, _)| group == RECENT_LABEL)
            .map(|(_, label)| label)
            .collect();
        assert_eq!(recent, ["Dark"], "unknown ids are skipped");
    }

    #[test]
    fn typing_hides_the_recent_group_and_clearing_brings_it_back() {
        let mut state = state().with_recent(["open"]);
        let _ = state.update(Event::Open);
        let output = state.update(Event::Command(command::Event::Input("fi".into())));
        assert!(matches!(output, Some(Output::Search(query)) if query == "fi"));
        assert!(
            labels(&state)
                .iter()
                .all(|(group, _)| group != RECENT_LABEL)
        );
        assert_eq!(state.query(), "fi");

        let output = state.update(Event::Command(command::Event::Input("fil".into())));
        assert!(matches!(output, Some(Output::Search(query)) if query == "fil"));

        let output = state.update(Event::Command(command::Event::Input(String::new())));
        assert!(matches!(output, Some(Output::Search(query)) if query.is_empty()));
        assert_eq!(labels(&state)[0].0, RECENT_LABEL);
    }

    #[test]
    fn a_page_item_opens_its_page_instead_of_running() {
        let mut state = opened();
        assert!(run(&mut state, "theme").is_none());
        assert!(state.is_open());
        assert_eq!(state.depth(), 1);
        assert_eq!(state.page_id(), Some(&"theme"));
        assert_eq!(state.page().map(|page| page.title.as_str()), Some("Theme"));
        let items: Vec<_> = labels(&state).into_iter().map(|(_, label)| label).collect();
        assert_eq!(items, ["Light", "Dark"]);

        assert!(matches!(
            run(&mut state, "dark"),
            Some(Output::Activated("dark"))
        ));
        assert!(!state.is_open());
        assert_eq!(state.recent(), ["dark"], "the page itself is not recent");
    }

    #[test]
    fn back_returns_to_the_page_before() {
        let mut state = opened();
        let _ = run(&mut state, "theme");
        assert!(state.update(Event::Back).is_none());
        assert_eq!(state.depth(), 0);
        assert_eq!(state.command().len(), 4);
        assert!(state.update(Event::Back).is_none(), "nothing to go back to");
        assert!(state.is_open());
    }

    #[test]
    fn going_back_with_a_query_clears_it() {
        let mut state = opened();
        let _ = run(&mut state, "theme");
        let _ = state.update(Event::Command(command::Event::Input("da".into())));
        let output = state.update(Event::Back);
        assert!(matches!(output, Some(Output::Search(query)) if query.is_empty()));
        assert_eq!(state.query(), "");
    }

    #[test]
    fn escape_on_an_empty_query_goes_back_then_closes() {
        let mut state = opened();
        let _ = run(&mut state, "theme");
        let _ = state.update(Event::Command(command::Event::Close));
        assert_eq!(state.depth(), 0);
        assert!(state.is_open());
        let _ = state.update(Event::Command(command::Event::Close));
        assert!(!state.is_open());
    }

    #[test]
    fn opening_again_starts_on_the_first_page() {
        let mut state = opened();
        let _ = run(&mut state, "theme");
        let _ = state.update(Event::Command(command::Event::Input("li".into())));
        let _ = state.update(Event::Close);
        let output = state.update(Event::Open);
        assert!(matches!(output, Some(Output::Search(query)) if query.is_empty()));
        assert_eq!(state.depth(), 0);
        assert_eq!(state.query(), "");
    }

    #[test]
    fn focus_survives_a_page_change() {
        let mut state = opened();
        let _ = state.update(Event::Command(command::Event::Focus(true)));
        let _ = run(&mut state, "theme");
        assert!(state.command().is_focused());
        assert!(state.command().shows_highlight());
    }

    #[test]
    fn set_groups_replaces_the_first_page() {
        let mut state = opened();
        state.set_groups([group("Only", [item("one", "One")])]);
        assert_eq!(labels(&state), [("Only".into(), "One".into())]);

        let _ = state.update(Event::Open);
        state.set_groups([group("Only", [item("theme", "Change theme...")])]);
        let _ = run(&mut state, "theme");
        state.set_groups([group("Other", [item("two", "Two")])]);
        assert_eq!(state.depth(), 1, "a nested page stays on show");
    }

    #[test]
    fn channel_events_pass_through() {
        let mut state = opened();
        let (sender, _receiver) = iced::futures::channel::mpsc::channel(1);
        let output = state.update(Event::Command(command::Event::Ready(sender)));
        assert!(matches!(output, Some(Output::Ready(_))));

        let _ = state.update(Event::Command(command::Event::Input("xyz".into())));
        let _ = state.update(Event::Command(command::Event::Received(vec![
            command::results("xyz", "Files", [item("remote", "xyz.txt")]),
        ])));
        assert!(labels(&state).contains(&("Files".into(), "xyz.txt".into())));
    }

    fn key(key: Key, modifiers: Modifiers) -> keys::Event {
        keys::Event { key, modifiers }
    }

    #[test]
    fn toggle_chords_work_anywhere_and_back_only_on_a_nested_empty_page() {
        let keymap = default_keymap();
        let mut state = state();
        let ctrl_k = key(Key::Character("k".into()), Modifiers::COMMAND);
        let ctrl_shift_p = key(
            Key::Character("P".into()),
            Modifiers::COMMAND | Modifiers::SHIFT,
        );
        let backspace = key(Key::Named(Named::Backspace), Modifiers::empty());
        assert!(matches!(
            state.key_event(&keymap, &ctrl_k),
            Some(Event::Toggle)
        ));
        assert!(matches!(
            state.key_event(&keymap, &ctrl_shift_p),
            Some(Event::Toggle)
        ));
        assert!(state.key_event(&keymap, &backspace).is_none());

        let _ = state.update(Event::Open);
        assert!(state.key_event(&keymap, &backspace).is_none(), "first page");
        let _ = run(&mut state, "theme");
        assert!(matches!(
            state.key_event(&keymap, &backspace),
            Some(Event::Back)
        ));
        let _ = state.update(Event::Command(command::Event::Input("d".into())));
        assert!(
            state.key_event(&keymap, &backspace).is_none(),
            "Backspace deletes text"
        );
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(keymap.chords(&Action::Toggle).len(), 2);
        assert_eq!(
            keymap.chords(&Action::Back),
            [&Chord::named(Named::Backspace)]
        );
        assert_eq!(Keymap::<Action>::defaults(), keymap);
        for &action in <Action as keys::Action>::ALL {
            assert!(!keys::Action::name(action).is_empty());
            assert!(keys::Action::description(action).ends_with('.'));
        }
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind_action(&Action::Back)
            .bind(Chord::named(Named::F1), Action::Toggle);
        let mut state = opened();
        let _ = run(&mut state, "theme");
        let backspace = key(Key::Named(Named::Backspace), Modifiers::empty());
        let f1 = key(Key::Named(Named::F1), Modifiers::empty());
        assert!(state.key_event(&keymap, &backspace).is_none());
        assert!(matches!(state.key_event(&keymap, &f1), Some(Event::Toggle)));
    }

    #[test]
    fn builder_defaults_and_overrides() {
        let state = state();
        let palette: CommandPalette<'_, &str, ()> = command_palette(&state, widget::text("App"));
        assert_eq!(palette.title, DEFAULT_TITLE);
        assert!(palette.empty.is_none());
        assert!(!palette.loading);
        assert_eq!(palette.size, dialog::Size::Md);
        assert_eq!(palette.keymap, default_keymap());
        assert_eq!(palette.command_keymap, command::default_keymap());
        assert_eq!(palette.dialog_keymap, dialog::default_keymap());
        assert!(palette.on_event.is_none());

        let palette: CommandPalette<'_, &str, ()> = command_palette(&state, widget::text("App"))
            .title("Go to")
            .placeholder("Search...")
            .empty("Nothing here.")
            .loading(true)
            .max_height(200.0)
            .size(dialog::Size::Lg)
            .keymap(Keymap::new())
            .command_keymap(Keymap::new())
            .dialog_keymap(Keymap::new())
            .on_event(|_| ());
        assert_eq!(palette.title, "Go to");
        assert_eq!(palette.placeholder, "Search...");
        assert_eq!(palette.empty.as_deref(), Some("Nothing here."));
        assert!(palette.loading);
        assert_eq!(palette.max_height, 200.0);
        assert_eq!(palette.size, dialog::Size::Lg);
        assert!(palette.keymap.is_empty() && palette.command_keymap.is_empty());
        assert!(palette.dialog_keymap.is_empty() && palette.on_event.is_some());
    }
}
