//! A searchable list of actions: a search field over grouped results.
//!
//! [`State`] holds the groups of [`Item`]s, the query and the highlighted
//! row. Typing ranks the items with [`score`], a case-insensitive fuzzy
//! match that prefers prefixes and word starts, and checks each item's
//! keywords too. [`State::update`] returns an [`Output`] when an item runs
//! or the query changes.
//!
//! While the search field has focus, the list handles its own keys through
//! a [`Keymap`] of [`Action`]s; see [`default_keymap`].
//!
//! Results can also arrive from background work. [`subscription`] owns a
//! bounded channel and emits [`Event::Ready`] with its [`Sender`] first.
//! When the query changes, `update` returns [`Output::Search`]; hand the
//! query and a sender clone to a producer, which sends [`Results`] tagged
//! with that query. Results for an older query are dropped.
//!
//! ```no_run
//! use iced::{Element, Subscription};
//! use iced_cube::lucide;
//! use iced_cube::navigation::command::{self, Output, command, group, item};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Command(command::Event<&'static str>),
//! }
//!
//! struct App {
//!     command: command::State<&'static str>,
//! }
//!
//! impl App {
//!     fn new() -> Self {
//!         let command = command::State::new([group(
//!             "Suggestions",
//!             [
//!                 item("calendar", "Calendar").icon(lucide!(Calendar)),
//!                 item("settings", "Settings").shortcut("Ctrl+S"),
//!             ],
//!         )]);
//!         Self { command }
//!     }
//!
//!     fn update(&mut self, message: Message) {
//!         let Message::Command(event) = message;
//!         if let Some(Output::Run(id)) = self.command.update(event) {
//!             println!("Run {id}");
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         command(&self.command).on_event(Message::Command).into()
//!     }
//! }
//! ```

use std::fmt;

use iced::futures::channel::mpsc;
use iced::futures::{Stream, StreamExt, stream};
use iced::keyboard::key::Named;
use iced::mouse::ScrollDelta;
use iced::widget::text::LineHeight;
use iced::widget::text_input::{self, Status};
use iced::widget::{self, Column, column, container, mouse_area, row, stack, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Subscription, Theme};

use crate::icon::{Glyph, tinted};
use crate::keys::{self, Chord, Keymap};
use crate::popup::{self, ROW_PADDING, scope};
use crate::theme::{Tokens, fade, space, text_size};

pub use crate::popup::{RowStatus, row_style, surface_style};

/// How many result batches a producer can send before it has to wait.
pub const CHANNEL_CAPACITY: usize = 64;
/// The most result batches delivered in one [`Event::Received`].
pub const BATCH_SIZE: usize = 32;
/// How many rows show at once unless the state sets its own number.
pub const VISIBLE_ROWS: usize = 8;
/// Shown when nothing matches the query, unless the builder sets its own.
pub const DEFAULT_EMPTY: &str = "No results found.";
/// Shown while [`Command::loading`] is set.
pub const SEARCHING: &str = "Searching...";

const SEARCH_HEIGHT: f32 = 44.0;
const ICON_SIZE: f32 = 16.0;
const LINE_HEIGHT: f32 = 1.3;

const EXACT: u32 = 5000;
const PREFIX: u32 = 4000;
const WORD: u32 = 3000;
const CONTAINS: u32 = 2000;
const FUZZY: u32 = 1000;
const WORD_START: u32 = 10;
const CONSECUTIVE: u32 = 5;
/// Keyword matches rank just below the same kind of label match.
const KEYWORD_PENALTY: u32 = 500;

/// How well `query` matches `text`, ignoring case, or `None` when it does
/// not match. Higher is better, and an empty query matches everything
/// with 0.
///
/// An exact match ranks first, then a prefix, then the query at the start
/// of a later word, then anywhere inside a word. Otherwise the letters must
/// appear in order, and runs of letters and word starts score more. Within
/// the first four kinds, shorter texts rank higher.
///
/// ```
/// use iced_cube::navigation::command::score;
///
/// assert!(score("set", "Settings") > score("set", "Reset view"));
/// assert!(score("nf", "New file").is_some());
/// assert!(score("xyz", "Settings").is_none());
/// ```
pub fn score(query: &str, text: &str) -> Option<u32> {
    let query: Vec<char> = query.trim().chars().flat_map(char::to_lowercase).collect();
    if query.is_empty() {
        return Some(0);
    }
    let folded = fold(text);
    let chars: Vec<char> = folded.iter().map(|(c, _)| *c).collect();
    let extra = chars.len().checked_sub(query.len())?;
    let closeness = 999 - extra.min(999) as u32;

    if extra == 0 && chars == query {
        return Some(EXACT);
    }
    if chars.starts_with(&query) {
        return Some(PREFIX + closeness);
    }
    let starts: Vec<usize> = chars
        .windows(query.len())
        .enumerate()
        .filter(|(_, window)| *window == query.as_slice())
        .map(|(start, _)| start)
        .collect();
    if starts.iter().any(|&start| folded[start].1) {
        return Some(WORD + closeness);
    }
    if !starts.is_empty() {
        return Some(CONTAINS + closeness);
    }
    fuzzy(&query, &folded).map(|bonus| FUZZY + bonus.min(999))
}

/// Lower-cased characters, each marked when it starts a word.
fn fold(text: &str) -> Vec<(char, bool)> {
    let mut folded = Vec::with_capacity(text.len());
    let mut previous: Option<char> = None;
    for c in text.chars() {
        let starts_word = match previous {
            None => true,
            Some(p) => {
                !p.is_alphanumeric() && c.is_alphanumeric() || p.is_lowercase() && c.is_uppercase()
            }
        };
        for (n, lower) in c.to_lowercase().enumerate() {
            folded.push((lower, starts_word && n == 0));
        }
        previous = Some(c);
    }
    folded
}

/// The best in-order match of every query letter, or `None` if there is
/// none. Each matched letter scores 1, or more at a word start, plus a
/// bonus when it follows the previous match directly.
fn fuzzy(query: &[char], text: &[(char, bool)]) -> Option<u32> {
    let mut previous: Vec<Option<u32>> = Vec::new();
    for (i, &wanted) in query.iter().enumerate() {
        let mut current = vec![None; text.len()];
        let mut best_before: Option<u32> = None;
        for (j, &(c, starts_word)) in text.iter().enumerate() {
            if i > 0 && j >= 2 {
                best_before = best_before.max(previous[j - 2]);
            }
            if c != wanted {
                continue;
            }
            let letter = if starts_word { WORD_START } else { 1 };
            let before = if i == 0 {
                Some(0)
            } else {
                let adjacent = j
                    .checked_sub(1)
                    .and_then(|k| previous[k])
                    .map(|score| score + CONSECUTIVE);
                adjacent.max(best_before)
            };
            current[j] = before.map(|score| score + letter);
        }
        previous = current;
    }
    previous.into_iter().flatten().max()
}

/// One action in a [`Group`].
#[derive(Debug, Clone, PartialEq)]
pub struct Item<Id> {
    pub id: Id,
    pub label: String,
    pub icon: Option<Glyph>,
    /// A hint such as `Ctrl+S`, shown at the end of the row.
    pub shortcut: Option<String>,
    /// Other words that find this item.
    pub keywords: Vec<String>,
    pub disabled: bool,
    pub destructive: bool,
}

/// Creates an enabled item.
pub fn item<Id>(id: Id, label: impl Into<String>) -> Item<Id> {
    Item {
        id,
        label: label.into(),
        icon: None,
        shortcut: None,
        keywords: Vec::new(),
        disabled: false,
        destructive: false,
    }
}

impl<Id> Item<Id> {
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn keywords<S: Into<String>>(mut self, keywords: impl IntoIterator<Item = S>) -> Self {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// Shows the item muted, skipped by the keyboard and not clickable.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Colours the label as a destructive action.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    /// How well `query` matches the label or, ranked a little lower, a
    /// keyword. See [`score`].
    pub fn score(&self, query: &str) -> Option<u32> {
        let keyword = self
            .keywords
            .iter()
            .filter_map(|keyword| score(query, keyword))
            .max()
            .map(|score| score.saturating_sub(KEYWORD_PENALTY));
        score(query, &self.label).max(keyword)
    }
}

/// Items under a label. An empty label shows no heading.
#[derive(Debug, Clone, PartialEq)]
pub struct Group<Id> {
    pub label: String,
    pub items: Vec<Item<Id>>,
}

pub fn group<Id>(label: impl Into<String>, items: impl IntoIterator<Item = Item<Id>>) -> Group<Id> {
    Group {
        label: label.into(),
        items: items.into_iter().collect(),
    }
}

/// Items a producer found for a query, shown under `group` after the
/// built-in groups. They are shown in the order they arrive, unranked.
#[derive(Debug, Clone, PartialEq)]
pub struct Results<Id> {
    pub query: String,
    pub group: String,
    pub items: Vec<Item<Id>>,
}

pub fn results<Id>(
    query: impl Into<String>,
    group: impl Into<String>,
    items: impl IntoIterator<Item = Item<Id>>,
) -> Results<Id> {
    Results {
        query: query.into(),
        group: group.into(),
        items: items.into_iter().collect(),
    }
}

/// The sending half of the results channel. Clone it for each producer.
pub type Sender<Id> = mpsc::Sender<Results<Id>>;

/// Everything that changes a command list.
#[derive(Debug, Clone)]
pub enum Event<Id> {
    /// The channel is open. Keep the sender and hand clones to producers.
    Ready(Sender<Id>),
    /// Results from the channel, oldest first.
    Received(Vec<Results<Id>>),
    /// The query was edited.
    Input(String),
    /// Highlights the next enabled row, wrapping round.
    Next,
    /// Highlights the previous enabled row, wrapping round.
    Previous,
    First,
    Last,
    /// Runs the highlighted row.
    Run,
    /// Clears the query, or dismisses the list when it is already empty.
    Clear,
    /// Highlights a row, by its position in [`State::results`].
    Highlight(usize),
    /// Runs a row, by its position in [`State::results`].
    Activate(usize),
    /// Scrolls the list by a number of rows, down when positive.
    Scroll(i32),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone)]
pub enum Output<Id> {
    Ready(Sender<Id>),
    /// The query changed. Start any background search for it here.
    Search(String),
    /// An item was chosen.
    Run(Id),
    /// Escape was pressed with an empty query.
    Dismiss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    remote: bool,
    group: usize,
    item: usize,
}

/// The groups, the query, the ranked results and the highlighted row.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    groups: Vec<Group<Id>>,
    remote: Vec<Group<Id>>,
    query: String,
    entries: Vec<Entry>,
    highlighted: Option<usize>,
    offset: usize,
    rows: usize,
}

impl<Id> State<Id> {
    /// Lists every item and highlights the first enabled one.
    pub fn new(groups: impl IntoIterator<Item = Group<Id>>) -> Self {
        let mut state = Self {
            groups: groups.into_iter().collect(),
            remote: Vec::new(),
            query: String::new(),
            entries: Vec::new(),
            highlighted: None,
            offset: 0,
            rows: VISIBLE_ROWS,
        };
        state.rank(false);
        state
    }

    /// Sets how many rows show at once. At least one does.
    pub fn with_visible_rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self.reveal();
        self
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn groups(&self) -> &[Group<Id>] {
        &self.groups
    }

    /// Every result in display order, with its group label.
    pub fn results(&self) -> impl Iterator<Item = (&str, &Item<Id>)> {
        self.entries.iter().filter_map(|&entry| {
            let group = self.group(entry)?;
            Some((group.label.as_str(), group.items.get(entry.item)?))
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The highlighted item, if any result is enabled.
    pub fn highlighted(&self) -> Option<&Item<Id>> {
        self.highlighted.and_then(|position| self.item_at(position))
    }

    /// Position in [`State::results`] of the highlighted row.
    pub fn highlighted_position(&self) -> Option<usize> {
        self.highlighted
    }

    fn group(&self, entry: Entry) -> Option<&Group<Id>> {
        let groups = if entry.remote {
            &self.remote
        } else {
            &self.groups
        };
        groups.get(entry.group)
    }

    fn item_at(&self, position: usize) -> Option<&Item<Id>> {
        let entry = *self.entries.get(position)?;
        self.group(entry)?.items.get(entry.item)
    }

    fn is_enabled(&self, position: usize) -> bool {
        self.item_at(position).is_some_and(|item| !item.disabled)
    }

    fn has_enabled(&self) -> bool {
        (0..self.entries.len()).any(|position| self.is_enabled(position))
    }

    /// Applies an event and returns what the app may need to act on.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>>
    where
        Id: Clone,
    {
        match event {
            Event::Ready(sender) => return Some(Output::Ready(sender)),
            Event::Received(batch) => self.receive(batch),
            Event::Input(query) => return self.search(query),
            Event::Next => self.step(true),
            Event::Previous => self.step(false),
            Event::First => {
                let first = (0..self.entries.len()).find(|&p| self.is_enabled(p));
                self.highlight(first);
            }
            Event::Last => {
                let last = (0..self.entries.len()).rev().find(|&p| self.is_enabled(p));
                self.highlight(last);
            }
            Event::Run => {
                let id = self.highlighted()?.id.clone();
                return Some(Output::Run(id));
            }
            Event::Clear if self.query.is_empty() => return Some(Output::Dismiss),
            Event::Clear => return self.search(String::new()),
            Event::Highlight(position) => {
                if self.is_enabled(position) {
                    self.highlighted = Some(position);
                }
            }
            Event::Activate(position) => {
                if !self.is_enabled(position) {
                    return None;
                }
                self.highlighted = Some(position);
                let id = self.item_at(position)?.id.clone();
                return Some(Output::Run(id));
            }
            Event::Scroll(rows) => {
                let last = self.entries.len().saturating_sub(self.rows);
                let offset = (self.offset as i64).saturating_add(rows.into());
                self.offset = offset.clamp(0, last as i64) as usize;
            }
        }
        None
    }

    fn search(&mut self, query: String) -> Option<Output<Id>> {
        if query == self.query {
            return None;
        }
        self.query = query;
        self.remote.clear();
        self.rank(false);
        Some(Output::Search(self.query.clone()))
    }

    fn receive(&mut self, batch: Vec<Results<Id>>) {
        let mut changed = false;
        for results in batch {
            if results.query != self.query || results.items.is_empty() {
                continue;
            }
            changed = true;
            match self.remote.iter_mut().find(|g| g.label == results.group) {
                Some(group) => group.items.extend(results.items),
                None => self.remote.push(group(results.group, results.items)),
            }
        }
        if changed {
            self.rank(true);
        }
    }

    /// Rebuilds the result list. Built-in groups are filtered and sorted
    /// by score, results from producers are kept as they arrived.
    fn rank(&mut self, keep_highlight: bool) {
        let kept = keep_highlight
            .then(|| self.highlighted.and_then(|p| self.entries.get(p).copied()))
            .flatten();

        let mut entries = Vec::new();
        for (index, group) in self.groups.iter().enumerate() {
            let mut scored: Vec<(u32, usize)> = group
                .items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| item.score(&self.query).map(|score| (score, index)))
                .collect();
            scored.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
            entries.extend(scored.into_iter().map(|(_, item)| Entry {
                remote: false,
                group: index,
                item,
            }));
        }
        for (index, group) in self.remote.iter().enumerate() {
            entries.extend((0..group.items.len()).map(|item| Entry {
                remote: true,
                group: index,
                item,
            }));
        }
        self.entries = entries;

        let kept = kept.and_then(|entry| self.entries.iter().position(|e| *e == entry));
        if kept.is_none() {
            self.offset = 0;
        }
        let first = kept.or_else(|| (0..self.entries.len()).find(|&p| self.is_enabled(p)));
        self.highlight(first);
    }

    /// The nearest enabled row in one direction, wrapping round.
    fn step(&mut self, forward: bool) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let start = self
            .highlighted
            .unwrap_or(if forward { len - 1 } else { 0 });
        let offset = if forward { 1 } else { len - 1 };
        let target = (1..=len)
            .map(|n| (start + offset * n) % len)
            .find(|&position| self.is_enabled(position));
        if target.is_some() {
            self.highlight(target);
        }
    }

    fn highlight(&mut self, position: Option<usize>) {
        self.highlighted = position;
        self.reveal();
    }

    /// Moves the window so the highlighted row is in view.
    fn reveal(&mut self) {
        if let Some(position) = self.highlighted {
            if position < self.offset {
                self.offset = position;
            } else if position >= self.offset + self.rows {
                self.offset = position + 1 - self.rows;
            }
        }
        self.offset = self
            .offset
            .min(self.entries.len().saturating_sub(self.rows));
    }

    /// Positions in [`State::results`] of the rows in view.
    pub fn visible(&self) -> std::ops::Range<usize> {
        let end = (self.offset + self.rows).min(self.entries.len());
        self.offset.min(end)..end
    }
}

/// What a command keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Last,
    Run,
    Clear,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it would
    /// do nothing, so the key can reach the search field instead.
    pub fn event<Id>(self, state: &State<Id>) -> Option<Event<Id>> {
        let movable = state.has_enabled();
        match self {
            Action::Next => movable.then_some(Event::Next),
            Action::Previous => movable.then_some(Event::Previous),
            Action::First => movable.then_some(Event::First),
            Action::Last => movable.then_some(Event::Last),
            Action::Run => state.highlighted.is_some().then_some(Event::Run),
            Action::Clear => Some(Event::Clear),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Next,
        Action::Previous,
        Action::First,
        Action::Last,
        Action::Run,
        Action::Clear,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Next => "Next",
            Action::Previous => "Previous",
            Action::First => "First",
            Action::Last => "Last",
            Action::Run => "Run",
            Action::Clear => "Clear",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next enabled result, wrapping at the end.",
            Action::Previous => "Highlights the previous enabled result, wrapping at the start.",
            Action::First => "Highlights the first enabled result.",
            Action::Last => "Highlights the last enabled result.",
            Action::Run => "Runs the highlighted result.",
            Action::Clear => "Clears the query, or dismisses the list when the query is empty.",
        }
    }
}

/// The default command shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
/// | `Enter` | [`Action::Run`] |
/// | `Escape` | [`Action::Clear`] |
///
/// The list handles these itself while its search field has focus, so
/// Home and End move the highlight rather than the text cursor. Pass a
/// changed keymap with [`Command::keymap`]. To use them without focus,
/// resolve presses from [`keys::subscription`] and send
/// [`Action::event`] to the state.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
        .bind(Chord::named(Named::Enter), Action::Run)
        .bind(Chord::named(Named::Escape), Action::Clear)
}

/// Owns the results channel. Emits [`Event::Ready`] first, then batches.
pub fn subscription<Id: Send + 'static>() -> Subscription<Event<Id>> {
    Subscription::run(stream::<Id>)
}

/// The stream behind [`subscription`], for driving it without a runtime.
pub fn stream<Id: Send + 'static>() -> impl Stream<Item = Event<Id>> {
    let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
    stream::once(async move { Event::Ready(sender) }).chain(batches(receiver).map(Event::Received))
}

/// Groups whatever is already waiting in the channel into one batch.
pub fn batches<T>(receiver: impl Stream<Item = T>) -> impl Stream<Item = Vec<T>> {
    receiver.ready_chunks(BATCH_SIZE)
}

/// A command list builder. Convert it into an [`Element`] to render.
///
/// A command list without [`on_event`](Command::on_event) is rendered
/// disabled.
pub struct Command<'a, Id, Message> {
    state: &'a State<Id>,
    placeholder: String,
    empty: String,
    loading: bool,
    width: Length,
    height: Length,
    id: Option<widget::Id>,
    keymap: Keymap<Action>,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

/// Renders the search field and results of `state`.
pub fn command<'a, Id, Message>(state: &'a State<Id>) -> Command<'a, Id, Message> {
    Command {
        state,
        placeholder: "Type a command or search...".to_owned(),
        empty: DEFAULT_EMPTY.to_owned(),
        loading: false,
        width: Length::Fill,
        height: Length::Shrink,
        id: None,
        keymap: default_keymap(),
        on_event: None,
    }
}

impl<Id: fmt::Debug, Message> fmt::Debug for Command<'_, Id, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Command")
            .field("state", self.state)
            .field("placeholder", &self.placeholder)
            .field("empty", &self.empty)
            .field("loading", &self.loading)
            .field("enabled", &self.on_event.is_some())
            .finish_non_exhaustive()
    }
}

impl<'a, Id, Message> Command<'a, Id, Message> {
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Text shown when nothing matches the query.
    pub fn empty(mut self, empty: impl Into<String>) -> Self {
        self.empty = empty.into();
        self
    }

    /// Shows that a background search is still running.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Overrides the width. Fills the available width by default.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Overrides the height. Fits the results by default; a fixed height
    /// stops the list resizing as the results change.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the widget id of the search field, to focus it or find it in
    /// tests.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Replaces the [`default_keymap`].
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_event.is_some()
    }
}

impl<'a, Id, Message> From<Command<'a, Id, Message>> for Element<'a, Message>
where
    Id: 'a,
    Message: Clone + 'a,
{
    fn from(command: Command<'a, Id, Message>) -> Self {
        let Command {
            state,
            placeholder,
            empty,
            loading,
            width,
            height,
            id,
            keymap,
            on_event,
        } = command;

        let list = list(state, &empty, loading, on_event.as_deref());
        let list = container(list).height(if height == Length::Shrink {
            Length::Shrink
        } else {
            Length::Fill
        });
        let bindings = on_event
            .as_ref()
            .map(|on_event| popup::bindings(&keymap, |action| action.event(state).map(on_event)));
        let on_input = on_event.map(|on_event| move |query| on_event(Event::Input(query)));

        let surface = container(column![
            search_field(&placeholder, &state.query, id, on_input),
            container(widget::space())
                .width(Length::Fill)
                .height(1)
                .style(|theme| divider_style(&Tokens::of(theme))),
            list,
        ])
        .width(width)
        .height(height)
        .clip(true)
        .style(|theme| surface_style(&Tokens::of(theme)));

        match bindings {
            Some(bindings) => scope(surface).bindings(bindings).into(),
            None => surface.into(),
        }
    }
}

fn search_field<'a, Message: Clone + 'a>(
    placeholder: &str,
    value: &str,
    id: Option<widget::Id>,
    on_input: Option<impl Fn(String) -> Message + 'a>,
) -> Element<'a, Message> {
    let enabled = on_input.is_some();
    let vertical = (SEARCH_HEIGHT - text_size::SM * LINE_HEIGHT) / 2.0;
    let mut input = widget::text_input(placeholder, value)
        .size(text_size::SM)
        .line_height(LineHeight::Relative(LINE_HEIGHT))
        .padding(Padding {
            top: vertical,
            bottom: vertical,
            left: space::MD + ICON_SIZE + space::SM,
            right: space::MD,
        })
        .width(Length::Fill)
        .style(|theme, status| search_style(&Tokens::of(theme), status));
    if let Some(id) = id {
        input = input.id(id);
    }
    if let Some(on_input) = on_input {
        input = input.on_input(on_input);
    }

    let icon = tinted(crate::lucide!(Search), ICON_SIZE, None).style(move |theme: &Theme, _| {
        widget::svg::Style {
            color: Some(icon_colour(&Tokens::of(theme), enabled)),
        }
    });

    stack![
        input,
        container(icon)
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .padding(Padding::ZERO.left(space::MD)),
    ]
    .into()
}

fn list<'a, Id, Message: Clone + 'a>(
    state: &'a State<Id>,
    empty: &str,
    loading: bool,
    on_event: Option<&(dyn Fn(Event<Id>) -> Message + 'a)>,
) -> Element<'a, Message> {
    let mut children: Vec<Element<'a, Message>> = Vec::new();
    let mut previous: Option<(bool, usize)> = None;

    for position in state.visible() {
        let Some(&entry) = state.entries.get(position) else {
            continue;
        };
        let Some(group) = state.group(entry) else {
            continue;
        };
        let Some(item) = group.items.get(entry.item) else {
            continue;
        };

        let key = (entry.remote, entry.group);
        if previous != Some(key) {
            if previous.is_some() {
                children.push(separator());
            }
            if !group.label.is_empty() {
                children.push(inset(group_label(&group.label)));
            }
            previous = Some(key);
        }

        let status = if item.disabled {
            RowStatus::Disabled
        } else if state.highlighted == Some(position) {
            RowStatus::Highlighted
        } else {
            RowStatus::Idle
        };
        let messages = on_event.filter(|_| !item.disabled).map(|on_event| {
            (
                on_event(Event::Highlight(position)),
                on_event(Event::Activate(position)),
            )
        });
        children.push(inset(item_row(item, status, messages)));
    }

    if children.is_empty() {
        let message = if loading { SEARCHING } else { empty };
        return container(muted(message.to_owned(), text_size::SM))
            .padding([space::XL, space::SM])
            .center_x(Length::Fill)
            .into();
    }
    if loading {
        children.push(inset(
            container(muted(SEARCHING.to_owned(), text_size::SM))
                .padding(ROW_PADDING)
                .into(),
        ));
    }

    let rows = Column::from_vec(children)
        .padding([space::XS, 0.0])
        .width(Length::Fill);
    let Some(on_event) = on_event else {
        return rows.into();
    };
    let down = on_event(Event::Scroll(1));
    let up = on_event(Event::Scroll(-1));
    mouse_area(rows)
        .on_scroll(move |delta| match delta {
            ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } if y < 0.0 => down.clone(),
            _ => up.clone(),
        })
        .into()
}

fn item_row<'a, Id, Message: Clone + 'a>(
    item: &'a Item<Id>,
    status: RowStatus,
    messages: Option<(Message, Message)>,
) -> Element<'a, Message> {
    let mut content = row![].spacing(space::SM).align_y(Alignment::Center);
    if let Some(glyph) = item.icon {
        content = content.push(
            tinted(glyph, ICON_SIZE, None).style(move |theme: &Theme, _| widget::svg::Style {
                color: Some(icon_colour(
                    &Tokens::of(theme),
                    status != RowStatus::Disabled,
                )),
            }),
        );
    }
    content = content.push(
        text(item.label.as_str())
            .size(text_size::SM)
            .width(Length::Fill),
    );
    if let Some(shortcut) = &item.shortcut {
        content = content.push(muted(shortcut.clone(), text_size::XS));
    }

    let destructive = item.destructive;
    let body = container(content)
        .padding(ROW_PADDING)
        .width(Length::Fill)
        .style(move |theme| row_style(&Tokens::of(theme), status, destructive));

    let Some((highlight, activate)) = messages else {
        return body.into();
    };
    mouse_area(body)
        .on_enter(highlight)
        .on_press(activate)
        .interaction(iced::mouse::Interaction::Pointer)
        .into()
}

fn group_label<'a, Message: 'a>(label: &str) -> Element<'a, Message> {
    container(
        text(label.to_owned())
            .size(text_size::XS)
            .font(crate::theme::semibold())
            .style(|theme: &Theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            }),
    )
    .padding(ROW_PADDING)
    .into()
}

fn muted<'a, Message: 'a>(content: String, size: f32) -> Element<'a, Message> {
    text(content)
        .size(size)
        .style(|theme: &Theme| text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        })
        .into()
}

/// Indents a row from the surface edge.
fn inset<'a, Message: 'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content).padding([0.0, space::XS]).into()
}

fn separator<'a, Message: 'a>() -> Element<'a, Message> {
    container(
        container(widget::space())
            .width(Length::Fill)
            .height(1)
            .style(|theme| divider_style(&Tokens::of(theme))),
    )
    .padding([space::XS, 0.0])
    .into()
}

/// The search field: borderless and transparent, inside the surface.
pub fn search_style(tokens: &Tokens, status: Status) -> text_input::Style {
    let style = text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: tokens.muted_foreground,
        placeholder: tokens.muted_foreground,
        value: tokens.foreground,
        selection: fade(tokens.primary, 0.25),
    };
    if status != Status::Disabled {
        return style;
    }
    text_input::Style {
        placeholder: fade(style.placeholder, 0.5),
        value: fade(style.value, 0.5),
        icon: fade(style.icon, 0.5),
        selection: Color::TRANSPARENT,
        ..style
    }
}

/// The line under the search field and between groups.
pub fn divider_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.border)),
        ..container::Style::default()
    }
}

/// The colour of the search icon and item icons.
pub fn icon_colour(tokens: &Tokens, enabled: bool) -> Color {
    if enabled {
        tokens.muted_foreground
    } else {
        fade(tokens.muted_foreground, 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use futures::SinkExt;
    use futures::executor::block_on;

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    fn sample() -> State<&'static str> {
        State::new([
            group(
                "Suggestions",
                [
                    item("calendar", "Calendar"),
                    item("emoji", "Search emoji").keywords(["smiley"]),
                    item("calculator", "Calculator").disabled(true),
                ],
            ),
            group(
                "Settings",
                [
                    item("profile", "Profile").shortcut("Ctrl+P"),
                    item("billing", "Billing"),
                    item("settings", "Settings"),
                ],
            ),
        ])
    }

    fn ids(state: &State<&'static str>) -> Vec<&'static str> {
        state.results().map(|(_, item)| item.id).collect()
    }

    fn highlighted(state: &State<&'static str>) -> Option<&'static str> {
        state.highlighted().map(|item| item.id)
    }

    #[test]
    fn empty_query_matches_everything_equally() {
        assert_eq!(score("", "Anything"), Some(0));
        assert_eq!(score("   ", "Anything"), Some(0));
    }

    #[test]
    fn matching_ignores_case() {
        assert_eq!(score("SET", "settings"), score("set", "Settings"));
        assert!(score("cAl", "CALENDAR").is_some());
    }

    #[test]
    fn match_kinds_rank_in_order() {
        let exact = score("set", "Set");
        let prefix = score("set", "Settings");
        let word = score("set", "Reset settings");
        let contains = score("set", "Asset");
        let fuzzy = score("set", "Sheet editor");
        assert!(exact > prefix, "{exact:?} {prefix:?}");
        assert!(prefix > word, "{prefix:?} {word:?}");
        assert!(word > contains, "{word:?} {contains:?}");
        assert!(contains > fuzzy, "{contains:?} {fuzzy:?}");
        assert!(fuzzy.is_some());
    }

    #[test]
    fn letters_must_appear_in_order() {
        assert!(score("tes", "Settings").is_none());
        assert!(score("xyz", "Settings").is_none());
        assert!(score("settings!", "Settings").is_none());
    }

    #[test]
    fn shorter_texts_win_within_a_kind() {
        assert!(score("set", "Settings") > score("set", "Settings and privacy"));
    }

    #[test]
    fn fuzzy_prefers_word_starts_and_runs() {
        assert!(score("ts", "Toggle sidebar") > score("ts", "Tabs list"));
        assert!(score("nf", "newFile") > score("nf", "notify"));
        assert!(score("sba", "Show bar") > score("sba", "Toggle sidebar"));
        assert!(score("ogl", "toggle") > score("ogl", "tobgaly"));
    }

    #[test]
    fn keywords_find_items_but_rank_below_labels() {
        let emoji = item("emoji", "Search emoji").keywords(["smiley"]);
        assert!(emoji.score("smil").is_some());
        let smiley = item("smiley", "Smiley");
        assert!(smiley.score("smil") > emoji.score("smil"));
        assert!(item("x", "Plain").score("smil").is_none());
    }

    #[test]
    fn new_state_lists_every_item_and_highlights_the_first() {
        let state = sample();
        assert_eq!(state.len(), 6);
        assert_eq!(highlighted(&state), Some("calendar"));
        assert_eq!(state.query(), "");
        assert!(State::<u8>::new([]).is_empty());
        assert!(State::<u8>::new([]).highlighted().is_none());
    }

    #[test]
    fn typing_filters_ranks_and_asks_for_a_search() {
        let mut state = sample();
        let output = state.update(Event::Input("cal".into()));
        assert!(matches!(output, Some(Output::Search(query)) if query == "cal"));
        assert_eq!(ids(&state), ["calendar", "calculator"]);
        assert_eq!(highlighted(&state), Some("calendar"));

        // Calculator is the closer match now, but it is disabled.
        let _ = state.update(Event::Input("calc".into()));
        assert_eq!(ids(&state), ["calculator"]);
        assert!(state.highlighted().is_none());

        let _ = state.update(Event::Input("set".into()));
        assert_eq!(ids(&state), ["settings"]);
        let groups: Vec<_> = state.results().map(|(group, _)| group).collect();
        assert_eq!(groups, ["Settings"]);

        assert!(state.update(Event::Input("set".into())).is_none());
    }

    #[test]
    fn nothing_matching_leaves_nothing_highlighted() {
        let mut state = sample();
        let _ = state.update(Event::Input("zzz".into()));
        assert!(state.is_empty());
        assert!(state.highlighted().is_none());
        assert!(state.update(Event::Run).is_none());
        let _ = state.update(Event::Next);
        assert!(state.highlighted().is_none());
    }

    #[test]
    fn arrows_wrap_and_skip_disabled_items() {
        let mut state = sample();
        let _ = state.update(Event::Next);
        assert_eq!(highlighted(&state), Some("emoji"));
        let _ = state.update(Event::Next);
        assert_eq!(highlighted(&state), Some("profile"));
        let _ = state.update(Event::Previous);
        let _ = state.update(Event::Previous);
        let _ = state.update(Event::Previous);
        assert_eq!(highlighted(&state), Some("settings"));
        let _ = state.update(Event::Next);
        assert_eq!(highlighted(&state), Some("calendar"));
    }

    #[test]
    fn home_and_end_jump_to_enabled_ends() {
        let mut state = State::new([group(
            "",
            [
                item(1, "One").disabled(true),
                item(2, "Two"),
                item(3, "Three"),
                item(4, "Four").disabled(true),
            ],
        )]);
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted().map(|i| i.id), Some(3));
        let _ = state.update(Event::First);
        assert_eq!(state.highlighted().map(|i| i.id), Some(2));
    }

    #[test]
    fn run_and_activate_return_the_id() {
        let mut state = sample();
        let _ = state.update(Event::Next);
        assert!(matches!(
            state.update(Event::Run),
            Some(Output::Run("emoji"))
        ));

        assert!(matches!(
            state.update(Event::Activate(4)),
            Some(Output::Run("billing"))
        ));
        assert_eq!(highlighted(&state), Some("billing"));
        assert!(state.update(Event::Activate(2)).is_none());
        assert!(state.update(Event::Activate(99)).is_none());
    }

    #[test]
    fn hovering_a_disabled_row_keeps_the_highlight() {
        let mut state = sample();
        let _ = state.update(Event::Highlight(2));
        assert_eq!(highlighted(&state), Some("calendar"));
        let _ = state.update(Event::Highlight(3));
        assert_eq!(highlighted(&state), Some("profile"));
    }

    #[test]
    fn clear_empties_the_query_then_dismisses() {
        let mut state = sample();
        let _ = state.update(Event::Input("bill".into()));
        let output = state.update(Event::Clear);
        assert!(matches!(output, Some(Output::Search(query)) if query.is_empty()));
        assert_eq!(state.len(), 6);
        assert!(matches!(state.update(Event::Clear), Some(Output::Dismiss)));
    }

    #[test]
    fn results_for_the_current_query_join_a_remote_group() {
        let mut state = sample();
        let _ = state.update(Event::Input("rep".into()));
        assert!(state.is_empty());

        let _ = state.update(Event::Received(vec![
            results("rep", "Files", [item("a", "report.pdf")]),
            results("rep", "Files", [item("b", "reply.txt")]),
        ]));
        assert_eq!(ids(&state), ["a", "b"]);
        assert!(state.results().all(|(group, _)| group == "Files"));
        assert_eq!(highlighted(&state), Some("a"));
    }

    #[test]
    fn stale_results_are_dropped() {
        let mut state = sample();
        let _ = state.update(Event::Input("re".into()));
        let _ = state.update(Event::Input("rep".into()));
        let _ = state.update(Event::Received(vec![results(
            "re",
            "Files",
            [item("old", "readme.md")],
        )]));
        assert!(state.is_empty());
    }

    #[test]
    fn a_new_query_clears_remote_results() {
        let mut state = sample();
        let files = |state: &State<&'static str>| {
            state
                .results()
                .filter(|(group, _)| *group == "Files")
                .count()
        };
        let _ = state.update(Event::Input("re".into()));
        let _ = state.update(Event::Received(vec![results(
            "re",
            "Files",
            [item("old", "readme.md")],
        )]));
        assert_eq!(files(&state), 1);
        let _ = state.update(Event::Input("rea".into()));
        assert_eq!(files(&state), 0);
    }

    #[test]
    fn arriving_results_keep_the_highlight() {
        let mut state = sample();
        let _ = state.update(Event::Input("s".into()));
        let _ = state.update(Event::Next);
        let before = highlighted(&state);
        let _ = state.update(Event::Received(vec![results(
            "s",
            "Files",
            [item("f", "styles.css")],
        )]));
        assert_eq!(highlighted(&state), before);
        assert_eq!(state.results().last().map(|(_, item)| item.id), Some("f"));
    }

    #[test]
    fn remote_results_are_not_filtered_again() {
        let mut state = sample();
        let _ = state.update(Event::Input("q".into()));
        let _ = state.update(Event::Received(vec![results(
            "q",
            "Files",
            [item("x", "unrelated.txt")],
        )]));
        assert_eq!(ids(&state), ["x"]);
    }

    #[test]
    fn the_window_follows_the_highlight_and_scrolls() {
        let mut state = sample().with_visible_rows(3);
        assert_eq!(state.visible(), 0..3);
        let _ = state.update(Event::Last);
        assert_eq!(state.visible(), 3..6);
        let _ = state.update(Event::Next);
        assert_eq!(state.visible(), 0..3);
        let _ = state.update(Event::Scroll(10));
        assert_eq!(state.visible(), 3..6);
        let _ = state.update(Event::Scroll(-1));
        assert_eq!(state.visible(), 2..5);
        assert_eq!(State::<u8>::new([]).with_visible_rows(0).rows, 1);
        assert_eq!(State::<u8>::new([]).visible(), 0..0);
    }

    #[test]
    fn ready_hands_the_sender_back() {
        let (sender, _receiver) = mpsc::channel(1);
        let output = sample().update(Event::Ready(sender));
        assert!(matches!(output, Some(Output::Ready(_))));
    }

    #[test]
    fn default_keymap_and_overrides() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowUp"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Home"), Some(Action::First));
        assert_eq!(press(&keymap, "End"), Some(Action::Last));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Run));
        assert_eq!(press(&keymap, "Escape"), Some(Action::Clear));

        let custom = keymap
            .bind("Ctrl+N".parse().unwrap(), Action::Next)
            .unbind(&Chord::named(Named::Home));
        assert_eq!(press(&custom, "ctrl+n"), Some(Action::Next));
        assert_eq!(press(&custom, "Home"), None);
        assert!(default_keymap().clear().is_empty());
    }

    #[test]
    fn actions_map_to_events_only_when_they_apply() {
        let mut state = sample();
        for action in [Action::Next, Action::Previous, Action::First, Action::Last] {
            assert!(action.event(&state).is_some(), "{action:?}");
        }
        assert!(matches!(Action::Run.event(&state), Some(Event::Run)));
        assert!(matches!(Action::Clear.event(&state), Some(Event::Clear)));

        let _ = state.update(Event::Input("zzz".into()));
        for action in [
            Action::Next,
            Action::Previous,
            Action::First,
            Action::Last,
            Action::Run,
        ] {
            assert!(action.event(&state).is_none(), "{action:?}");
        }
        assert!(matches!(Action::Clear.event(&state), Some(Event::Clear)));
    }

    #[test]
    fn stream_emits_ready_first_then_batches() {
        let mut events = Box::pin(stream::<&'static str>());
        let Some(Event::Ready(mut sender)) = block_on(events.next()) else {
            panic!("first event must be Ready");
        };
        block_on(async {
            for name in ["a", "b", "c"] {
                sender
                    .send(results("q", "Files", [item(name, name)]))
                    .await
                    .ok();
            }
        });
        let Some(Event::Received(batch)) = block_on(events.next()) else {
            panic!("expected a batch");
        };
        let names: Vec<_> = batch.iter().map(|r| r.items[0].id).collect();
        assert_eq!(names, ["a", "b", "c"]);
    }

    #[test]
    fn batches_split_bursts_at_batch_size() {
        let items = stream::iter(0..BATCH_SIZE + 3);
        let sizes: Vec<_> = block_on(batches(items).map(|batch| batch.len()).collect());
        assert_eq!(sizes, [BATCH_SIZE, 3]);
    }

    #[test]
    fn default_builder_is_disabled() {
        let state = sample();
        let c: Command<'_, &str, ()> = command(&state);
        assert!(!c.is_enabled());
        assert_eq!(c.empty, DEFAULT_EMPTY);
        assert!(!c.loading);
        assert_eq!((c.width, c.height), (Length::Fill, Length::Shrink));
        let c = c.loading(true).height(320).on_event(|_| ());
        assert!(c.is_enabled() && c.loading);
        assert_eq!(c.height, Length::Fixed(320.0));
    }

    const STATES: [Status; 5] = [
        Status::Active,
        Status::Hovered,
        Status::Focused { is_hovered: false },
        Status::Focused { is_hovered: true },
        Status::Disabled,
    ];

    #[test]
    fn search_field_is_borderless_and_fades_when_disabled() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in STATES {
                let style = search_style(&tokens, status);
                assert_eq!(style.border.width, 0.0);
                assert_eq!(style.background, Background::Color(Color::TRANSPARENT));
                let expected = if status == Status::Disabled {
                    fade(tokens.foreground, 0.5)
                } else {
                    tokens.foreground
                };
                assert_eq!(style.value, expected, "{status:?}");
            }
        }
    }

    #[test]
    fn dividers_and_icons_use_tokens() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(
                divider_style(&tokens).background,
                Some(Background::Color(tokens.border))
            );
            assert_eq!(icon_colour(&tokens, true), tokens.muted_foreground);
            assert!(icon_colour(&tokens, false).a < tokens.muted_foreground.a);
        }
    }
}
