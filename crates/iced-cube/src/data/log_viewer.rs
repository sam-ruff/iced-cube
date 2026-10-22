//! A live, searchable log: entries with a level, an optional timestamp and
//! target, and a message, streamed in from threads or tasks.
//!
//! [`State`] keeps the newest entries up to a limit, dropping the oldest,
//! so memory stays bounded however long the log runs. It filters by
//! level, searches messages with every match marked, follows the newest
//! line, pauses and resumes the stream, and selects lines to copy. Only
//! the lines in view are built, as in a
//! [virtual list](mod@crate::data::virtual_list).
//!
//! [`subscription`] creates a bounded channel and emits [`Event::Ready`]
//! with its [`Sender`] once. Producers send [`Entry`] values through clones
//! of the sender, and entries arrive in batches as [`Event::Received`].
//!
//! Copying returns [`Output::Copy`] with the text, for the app to write to
//! the clipboard, and the search shortcut returns [`Output::FocusSearch`],
//! for the app to answer with [`focus_search`]. Lines can carry a context
//! menu, such as the one from [`menu()`], through
//! [`LogViewer::context_menu`].
//!
//! ```no_run
//! use iced::{Element, Subscription, Task};
//! use iced_cube::data::log_viewer::{self, Event, Level, Output, State, entry, log_viewer};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Logs(Event),
//! }
//!
//! struct App {
//!     logs: State,
//!     sender: Option<log_viewer::Sender>,
//! }
//!
//! impl App {
//!     fn update(&mut self, message: Message) -> Task<Message> {
//!         let Message::Logs(event) = message;
//!         match self.logs.update(event) {
//!             Some(Output::Ready(mut sender)) => {
//!                 let _ = sender.try_send(entry(Level::Info, "Listening on :8080"));
//!                 self.sender = Some(sender);
//!                 Task::none()
//!             }
//!             Some(Output::Copy(text)) => iced::clipboard::write(text),
//!             Some(Output::FocusSearch) => log_viewer::focus_search(&self.logs),
//!             None => Task::none(),
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         log_viewer(&self.logs).height(400).on_event(Message::Logs).into()
//!     }
//!
//!     fn subscription(&self) -> Subscription<Message> {
//!         log_viewer::subscription().map(Message::Logs)
//!     }
//! }
//! ```

use std::collections::{BTreeSet, VecDeque};
use std::fmt;
use std::rc::Rc;

use iced::futures::channel::mpsc;
use iced::futures::{Stream, StreamExt, stream};
use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;
use iced::widget::text::{LineHeight, Wrapping};
use iced::widget::{self, Space, column, container, responsive, row, text};
use iced::{Alignment, Border, Color, Element, Length, Padding, Pixels, Subscription, Task, Theme};

use crate::data::ellipsis::ellipsis;
use crate::data::interaction::{Look, RowMenu, pressable, row_menu, scope, search_keys};
use crate::data::marked::{find, marked};
use crate::data::rows::rows;
use crate::data::virtual_list::{self, Step, target};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::context_menu;
use crate::overlay::dropdown_menu::{item, separator};
use crate::overlay::tooltip::Position;
use crate::primitives::button::{self, button};
use crate::primitives::icon_button::icon_button;
use crate::primitives::input::{self, input};
use crate::primitives::toggle;
use crate::primitives::toggle_group::{self, toggle_group};
use crate::theme::{self, Tokens, fade, mix, radius, space, text_size};

/// Height of a line that does not wrap.
pub const ROW_HEIGHT: f32 = 24.0;
/// Height of a line that does not wrap below the [`BREAKPOINT`], where the
/// message sits under the timestamp, level and target.
pub const STACKED_ROW_HEIGHT: f32 = 42.0;
/// The most entries kept unless the state is given another limit.
pub const DEFAULT_LIMIT: usize = 10_000;
/// How many entries a producer can send before it has to wait.
pub const CHANNEL_CAPACITY: usize = 1024;
/// The most entries delivered in one [`Event::Received`].
pub const BATCH_SIZE: usize = 512;
/// The default widget id of the search field.
pub const SEARCH_ID: widget::Id = widget::Id::new("iced-cube-log-viewer-search");
/// Below this width the toolbar stacks its parts.
pub const BREAKPOINT: f32 = 560.0;
const LINE_HEIGHT: f32 = 18.0;
const TIMESTAMP_WIDTH: f32 = 84.0;
const LEVEL_WIDTH: f32 = 44.0;
const TARGET_WIDTH: f32 = 120.0;
const FRAME_PADDING: f32 = 4.0;
/// Room kept for the match counter, so the search field never moves.
const COUNTER_WIDTH: f32 = 64.0;

/// How severe an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Level {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl Level {
    pub const ALL: [Level; 5] = [
        Level::Trace,
        Level::Debug,
        Level::Info,
        Level::Warn,
        Level::Error,
    ];

    /// The short upper-case tag shown on each line, such as `WARN`.
    pub const fn tag(self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    /// The name shown on the level filter, such as `Warn`.
    pub const fn name(self) -> &'static str {
        match self {
            Level::Trace => "Trace",
            Level::Debug => "Debug",
            Level::Info => "Info",
            Level::Warn => "Warn",
            Level::Error => "Error",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

/// One log line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Entry {
    pub level: Level,
    /// Shown as given, so the app picks the format.
    pub timestamp: Option<String>,
    /// Where the entry came from, such as a module path.
    pub target: Option<String>,
    pub message: String,
}

/// Creates an entry. Add a timestamp and a target with the builder methods.
pub fn entry(level: Level, message: impl Into<String>) -> Entry {
    Entry {
        level,
        timestamp: None,
        target: None,
        message: message.into(),
    }
}

impl Entry {
    pub fn timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// The entry as one line of plain text, as it is copied.
    pub fn line(&self) -> String {
        let mut line = String::new();
        if let Some(timestamp) = &self.timestamp {
            line.push_str(timestamp);
            line.push(' ');
        }
        line.push_str(&format!("{:<5} ", self.level.tag()));
        if let Some(target) = &self.target {
            line.push_str(target);
            line.push_str(": ");
        }
        line.push_str(&self.message);
        line
    }
}

/// Names a line for as long as it is kept: its position counted from the
/// first entry the log ever received.
pub type LineId = u64;

/// The sending half of the channel entries arrive on. Clone it for each
/// producer.
pub type Sender = mpsc::Sender<Entry>;

/// Everything that changes a log.
#[derive(Debug, Clone)]
pub enum Event {
    /// The channel is open. Keep the sender and hand clones to producers.
    Ready(Sender),
    /// Entries from the channel, oldest first.
    Received(Vec<Entry>),
    /// The pointer pressed a line with these modifiers held.
    Press(LineId, Modifiers),
    /// The search text changed.
    Search(String),
    NextMatch,
    PreviousMatch,
    ClearSearch,
    /// Asks the app to focus the search field.
    FocusSearch,
    /// Shows or hides a level.
    ToggleLevel(Level),
    /// The level filters were pressed.
    Levels(toggle_group::Event<Level>),
    /// Shows only the level of this line.
    FilterToLevel(LineId),
    ToggleWrap,
    /// Pauses the stream, holding new entries back, or resumes it.
    TogglePause,
    /// Removes every entry.
    Clear,
    /// Copies the selected lines, or the highlighted one.
    Copy,
    /// Copies one line as plain text, as [`Entry::line`] formats it.
    CopyLine(LineId),
    /// Copies only the message of one line.
    CopyMessage(LineId),
    /// Selects or deselects the highlighted line, keeping the others.
    ToggleHighlighted,
    /// Starts or stops following the newest line. The log sends `false`
    /// when the user scrolls away from the end, and `true` when they
    /// scroll back.
    Follow(bool),
    Next,
    Previous,
    First,
    /// Highlights the newest line and follows it.
    Last,
    NextPage,
    PreviousPage,
    ExtendNext,
    ExtendPrevious,
    SelectAll,
    /// How many lines fit in the view. The log sends this itself.
    Page(usize),
    /// The log gained or lost focus. The log sends this itself.
    Focus(bool),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone)]
pub enum Output {
    Ready(Sender),
    /// Write this text to the clipboard.
    Copy(String),
    /// Focus the search field, with [`focus_search`].
    FocusSearch,
}

/// The kept entries, the filters, the search, the selection and whether the
/// log follows its newest line.
#[derive(Debug, Clone)]
pub struct State {
    entries: VecDeque<Entry>,
    /// The id of the oldest kept entry.
    first: LineId,
    limit: usize,
    counts: [usize; 5],
    shown: [bool; 5],
    /// The level filters as drawn, labelled with their counts.
    levels: toggle_group::State<Level>,
    /// Ids of the entries that pass the level filter, oldest first.
    visible: VecDeque<LineId>,
    /// Visible lines dropped from the front so far, so the lines in view
    /// stay put as old ones go.
    visible_dropped: u64,
    query: String,
    /// Visible lines whose message matches the search, oldest first.
    matches: VecDeque<LineId>,
    current: Option<LineId>,
    selected: BTreeSet<LineId>,
    anchor: Option<LineId>,
    highlighted: Option<LineId>,
    focused: bool,
    following: bool,
    paused: bool,
    /// Entries held back while paused.
    pending: VecDeque<Entry>,
    /// Lines that arrived while not following.
    unseen: usize,
    wrap: bool,
    page: usize,
    reveal: Option<(LineId, u64)>,
    requests: u64,
    search_id: widget::Id,
}

impl Default for State {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            first: 0,
            limit: DEFAULT_LIMIT,
            counts: [0; 5],
            shown: [true; 5],
            levels: level_filters(&[0; 5], &[true; 5]),
            visible: VecDeque::new(),
            visible_dropped: 0,
            query: String::new(),
            matches: VecDeque::new(),
            current: None,
            selected: BTreeSet::new(),
            anchor: None,
            highlighted: None,
            focused: false,
            following: true,
            paused: false,
            pending: VecDeque::new(),
            unseen: 0,
            wrap: false,
            page: virtual_list::DEFAULT_PAGE,
            reveal: None,
            requests: 0,
            search_id: SEARCH_ID,
        }
    }
}

impl State {
    /// An empty log that keeps [`DEFAULT_LIMIT`] entries, shows every
    /// level and follows the newest line.
    pub fn new() -> Self {
        Self::default()
    }

    /// Keeps at most `limit` entries, dropping the oldest. A limit of zero
    /// is treated as one.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self.trim();
        self.sync_levels();
        self
    }

    /// Shows only the given levels.
    pub fn with_levels(mut self, levels: impl IntoIterator<Item = Level>) -> Self {
        self.shown = [false; 5];
        for level in levels {
            self.shown[level.index()] = true;
        }
        self.refilter();
        self
    }

    /// Starts with long lines wrapped.
    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Replaces [`SEARCH_ID`], so several logs on one screen each focus
    /// their own field.
    pub fn with_search_id(mut self, id: impl Into<widget::Id>) -> Self {
        self.search_id = id.into();
        self
    }

    /// Appends entries straight away, as the channel does.
    pub fn extend(&mut self, entries: impl IntoIterator<Item = Entry>) {
        if self.paused {
            self.pending.extend(entries);
            let over = self.pending.len().saturating_sub(self.limit);
            self.pending.drain(..over);
            return;
        }
        for entry in entries {
            self.append(entry);
        }
        self.trim();
        self.sync_levels();
    }

    /// Every kept entry, oldest first, whether shown or not.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    /// How many entries are kept.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    /// The entry with `id`, if it is still kept.
    pub fn entry(&self, id: LineId) -> Option<&Entry> {
        let index = usize::try_from(id.checked_sub(self.first)?).ok()?;
        self.entries.get(index)
    }

    /// The ids of the lines shown, oldest first.
    pub fn visible(&self) -> impl Iterator<Item = LineId> + '_ {
        self.visible.iter().copied()
    }

    /// How many lines are shown.
    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    /// How many kept entries have this level.
    pub fn count(&self, level: Level) -> usize {
        self.counts[level.index()]
    }

    pub fn is_shown(&self, level: Level) -> bool {
        self.shown[level.index()]
    }

    /// The level filters as a multiple-choice toggle group, each labelled
    /// with its level's count.
    pub fn levels(&self) -> &toggle_group::State<Level> {
        &self.levels
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// The shown lines whose message matches the search, oldest first.
    pub fn matches(&self) -> impl Iterator<Item = LineId> + '_ {
        self.matches.iter().copied()
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// The match the search last moved to, and its position counted from
    /// one, if it is still shown.
    pub fn current_match(&self) -> Option<(LineId, usize)> {
        let id = self.current?;
        let position = self.matches.binary_search(&id).ok()?;
        Some((id, position + 1))
    }

    pub fn is_selected(&self, id: LineId) -> bool {
        self.selected.contains(&id)
    }

    /// The selected lines that are shown, oldest first.
    pub fn selected(&self) -> Vec<LineId> {
        self.selected
            .iter()
            .copied()
            .filter(|id| self.position(*id).is_some())
            .collect()
    }

    /// The line the keyboard acts on.
    pub fn highlighted(&self) -> Option<LineId> {
        self.highlighted.filter(|id| self.position(*id).is_some())
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn is_following(&self) -> bool {
        self.following
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Entries held back while paused.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Lines that arrived since the log stopped following.
    pub fn unseen(&self) -> usize {
        self.unseen
    }

    pub fn is_wrapped(&self) -> bool {
        self.wrap
    }

    pub fn search_id(&self) -> &widget::Id {
        &self.search_id
    }

    /// Whether there is anything to copy: a shown selected line or a
    /// highlight.
    pub fn can_copy(&self) -> bool {
        self.highlighted().is_some() || self.selected.iter().any(|&id| self.position(id).is_some())
    }

    /// The selected lines as text, one per line, or the highlighted line
    /// when nothing is selected.
    pub fn copy_text(&self) -> Option<String> {
        let mut ids = self.selected();
        if ids.is_empty() {
            ids.extend(self.highlighted());
        }
        if ids.is_empty() {
            return None;
        }
        let lines: Vec<String> = ids
            .into_iter()
            .filter_map(|id| self.entry(id).map(Entry::line))
            .collect();
        Some(lines.join("\n"))
    }

    /// Applies an event and returns what the app may need to act on.
    pub fn update(&mut self, event: Event) -> Option<Output> {
        match event {
            Event::Ready(sender) => return Some(Output::Ready(sender)),
            Event::Received(entries) => self.extend(entries),
            Event::Press(id, modifiers) => self.press(id, modifiers),
            Event::Search(query) => {
                self.query = query;
                self.current = None;
                self.rematch();
            }
            Event::NextMatch => self.jump(true),
            Event::PreviousMatch => self.jump(false),
            Event::ClearSearch => {
                self.query.clear();
                self.current = None;
                self.matches.clear();
            }
            Event::FocusSearch => return Some(Output::FocusSearch),
            Event::ToggleLevel(level) => {
                self.shown[level.index()] = !self.shown[level.index()];
                self.refilter();
            }
            Event::Levels(event) => {
                if self.levels.update(event).is_some() {
                    for level in Level::ALL {
                        self.shown[level.index()] = self.levels.is_selected(level);
                    }
                    self.refilter();
                }
            }
            Event::FilterToLevel(id) => {
                let level = self.entry(id)?.level;
                self.shown = [false; 5];
                self.shown[level.index()] = true;
                self.refilter();
            }
            Event::ToggleWrap => self.wrap = !self.wrap,
            Event::TogglePause => self.toggle_pause(),
            Event::Clear => self.clear(),
            Event::Copy => return self.copy_text().map(Output::Copy),
            Event::CopyLine(id) => return self.entry(id).map(|entry| Output::Copy(entry.line())),
            Event::CopyMessage(id) => {
                return self
                    .entry(id)
                    .map(|entry| Output::Copy(entry.message.clone()));
            }
            Event::ToggleHighlighted => {
                let id = self.highlighted()?;
                self.press(id, Modifiers::COMMAND);
            }
            Event::Follow(follow) => self.follow(follow),
            Event::Next => self.step(Step::Next),
            Event::Previous => self.step(Step::Previous),
            Event::First => self.step(Step::First),
            Event::Last => {
                self.step(Step::Last);
                self.follow(true);
            }
            Event::NextPage => self.step(Step::NextPage),
            Event::PreviousPage => self.step(Step::PreviousPage),
            Event::ExtendNext => self.extend_selection(Step::Next),
            Event::ExtendPrevious => self.extend_selection(Step::Previous),
            Event::SelectAll => self.selected = self.visible.iter().copied().collect(),
            Event::Page(lines) => self.page = lines.max(1),
            Event::Focus(focused) => {
                self.focused = focused;
                // Focus lands on the newest line while following, or the
                // first selected. It does not scroll.
                if focused && self.highlighted().is_none() {
                    let start = if self.following {
                        self.visible.back()
                    } else {
                        self.visible.front()
                    };
                    self.highlighted = self.selected().first().or(start).copied();
                }
            }
        }
        None
    }

    /// Turns a key press into an event through `keymap`.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event> {
        keymap.resolve_event(key)?.event(self)
    }

    /// Where a line sits among the shown lines.
    fn position(&self, id: LineId) -> Option<usize> {
        self.visible.binary_search(&id).ok()
    }

    fn matches_query(&self, entry: &Entry) -> bool {
        !self.query.is_empty() && !find(&entry.message, &self.query).is_empty()
    }

    fn append(&mut self, entry: Entry) {
        let id = self.first + self.entries.len() as u64;
        self.counts[entry.level.index()] += 1;
        if self.shown[entry.level.index()] {
            self.visible.push_back(id);
            if self.matches_query(&entry) {
                self.matches.push_back(id);
            }
            if !self.following {
                self.unseen += 1;
            }
        }
        self.entries.push_back(entry);
    }

    /// Drops the oldest entries over the limit, and forgets them.
    fn trim(&mut self) {
        while self.entries.len() > self.limit {
            let Some(entry) = self.entries.pop_front() else {
                break;
            };
            let id = self.first;
            self.first += 1;
            self.counts[entry.level.index()] -= 1;
            if self.visible.front() == Some(&id) {
                let _ = self.visible.pop_front();
                self.visible_dropped += 1;
            }
            if self.matches.front() == Some(&id) {
                let _ = self.matches.pop_front();
            }
        }
        let first = self.first;
        self.selected = self.selected.split_off(&first);
        let gone = |id: &LineId| *id < first;
        if self.highlighted.as_ref().is_some_and(gone) {
            self.highlighted = None;
        }
        if self.anchor.as_ref().is_some_and(gone) {
            self.anchor = None;
        }
        if self.current.as_ref().is_some_and(gone) {
            self.current = None;
        }
        if self.reveal.as_ref().is_some_and(|(id, _)| gone(id)) {
            self.reveal = None;
        }
    }

    /// Rebuilds the shown lines after the level filter changed, dropping
    /// hidden lines from the selection.
    fn refilter(&mut self) {
        let first = self.first;
        self.visible = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| self.shown[entry.level.index()])
            .map(|(index, _)| first + index as u64)
            .collect();
        let visible: BTreeSet<LineId> = self.visible.iter().copied().collect();
        self.selected.retain(|id| visible.contains(id));
        self.sync_levels();
        self.rematch();
        if let Some(id) = self.highlighted() {
            self.reveal_line(id);
        }
    }

    /// Rebuilds the matches after the search or the filter changed.
    fn rematch(&mut self) {
        self.matches = self
            .visible
            .iter()
            .copied()
            .filter(|&id| {
                self.entry(id)
                    .is_some_and(|entry| self.matches_query(entry))
            })
            .collect();
    }

    fn clear(&mut self) {
        self.first += self.entries.len() as u64;
        self.entries.clear();
        self.counts = [0; 5];
        self.visible.clear();
        self.matches.clear();
        self.current = None;
        self.selected.clear();
        self.anchor = None;
        self.highlighted = None;
        self.reveal = None;
        self.pending.clear();
        self.unseen = 0;
        self.following = true;
        self.sync_levels();
    }

    /// Relabels the level filters with the current counts.
    fn sync_levels(&mut self) {
        self.levels = level_filters(&self.counts, &self.shown);
    }

    fn follow(&mut self, follow: bool) {
        self.following = follow;
        if follow {
            self.unseen = 0;
        }
    }

    fn toggle_pause(&mut self) {
        if !self.paused {
            self.paused = true;
            return;
        }
        self.paused = false;
        let pending = std::mem::take(&mut self.pending);
        self.extend(pending);
    }

    fn reveal_line(&mut self, id: LineId) {
        self.requests += 1;
        self.reveal = Some((id, self.requests));
    }

    /// The row to keep in view and its request count, for the view.
    fn reveal_target(&self) -> Option<(usize, u64)> {
        let (id, request) = self.reveal?;
        Some((self.position(id)?, request))
    }

    /// Highlights a line, scrolls it into view and follows only when it is
    /// the newest.
    fn move_to(&mut self, id: LineId) {
        self.highlighted = Some(id);
        self.reveal_line(id);
        self.follow(self.visible.back() == Some(&id));
    }

    fn step(&mut self, step: Step) {
        let current = self.highlighted().and_then(|id| self.position(id));
        let Some(index) = target(step, current, self.visible.len(), self.page) else {
            return;
        };
        if let Some(&id) = self.visible.get(index) {
            self.move_to(id);
        }
    }

    /// Moves to the next or previous match after the current one, or the
    /// highlighted line, wrapping around at the ends.
    fn jump(&mut self, forward: bool) {
        let from = self
            .current_match()
            .map(|(id, _)| id)
            .or(self.highlighted());
        let next = match (forward, from) {
            (true, Some(from)) => {
                let after = self.matches.partition_point(|&id| id <= from);
                self.matches.get(after).or(self.matches.front())
            }
            (true, None) => self.matches.front(),
            (false, Some(from)) => {
                let before = self.matches.partition_point(|&id| id < from);
                before
                    .checked_sub(1)
                    .and_then(|index| self.matches.get(index))
                    .or(self.matches.back())
            }
            (false, None) => self.matches.back(),
        };
        let Some(&id) = next else {
            return;
        };
        self.current = Some(id);
        self.move_to(id);
    }

    fn press(&mut self, id: LineId, modifiers: Modifiers) {
        if self.position(id).is_none() {
            return;
        }
        self.highlighted = Some(id);
        if self.visible.back() != Some(&id) {
            self.follow(false);
        }
        match (modifiers.shift(), modifiers.command(), self.anchor) {
            (true, additive, Some(anchor)) => {
                if !additive {
                    self.selected.clear();
                }
                self.selected.extend(self.range(anchor, id));
            }
            (_, true, _) => {
                if !self.selected.remove(&id) {
                    let _ = self.selected.insert(id);
                }
                self.anchor = Some(id);
            }
            _ => {
                self.selected = BTreeSet::from([id]);
                self.anchor = Some(id);
            }
        }
    }

    /// The shown lines from `a` to `b`, in either order.
    fn range(&self, a: LineId, b: LineId) -> Vec<LineId> {
        let (Some(start), Some(end)) = (self.position(a), self.position(b)) else {
            return vec![b];
        };
        let (start, end) = (start.min(end), start.max(end));
        self.visible.range(start..=end).copied().collect()
    }

    fn extend_selection(&mut self, step: Step) {
        let anchor = self.anchor.or(self.highlighted());
        self.step(step);
        let (Some(anchor), Some(highlighted)) = (anchor, self.highlighted()) else {
            return;
        };
        self.anchor = Some(anchor);
        self.selected = self.range(anchor, highlighted).into_iter().collect();
    }
}

/// The level filters for these counts, with the shown levels on.
fn level_filters(counts: &[usize; 5], shown: &[bool; 5]) -> toggle_group::State<Level> {
    let items = Level::ALL.map(|level| {
        let label = format!("{} {}", level.name(), counts[level.index()]);
        toggle_group::item(level, label)
    });
    toggle_group::State::multiple(items)
        .with_selected(Level::ALL.into_iter().filter(|level| shown[level.index()]))
}

/// What a log keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Latest,
    NextPage,
    PreviousPage,
    ExtendNext,
    ExtendPrevious,
    Select,
    SelectAll,
    Copy,
    FocusSearch,
    NextMatch,
    PreviousMatch,
    ClearSearch,
    TogglePause,
    ToggleWrap,
    Clear,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing, so the key is left for others.
    pub fn event(self, state: &State) -> Option<Event> {
        let lines = state.visible_len() > 0;
        match self {
            Action::Select => state.highlighted().map(|_| Event::ToggleHighlighted),
            Action::Next => lines.then_some(Event::Next),
            Action::Previous => lines.then_some(Event::Previous),
            Action::First => lines.then_some(Event::First),
            Action::Latest => Some(Event::Last),
            Action::NextPage => lines.then_some(Event::NextPage),
            Action::PreviousPage => lines.then_some(Event::PreviousPage),
            Action::ExtendNext => lines.then_some(Event::ExtendNext),
            Action::ExtendPrevious => lines.then_some(Event::ExtendPrevious),
            Action::SelectAll => lines.then_some(Event::SelectAll),
            Action::Copy => state.can_copy().then_some(Event::Copy),
            Action::FocusSearch => Some(Event::FocusSearch),
            Action::NextMatch => (state.match_count() > 0).then_some(Event::NextMatch),
            Action::PreviousMatch => (state.match_count() > 0).then_some(Event::PreviousMatch),
            Action::ClearSearch => (!state.query().is_empty()).then_some(Event::ClearSearch),
            Action::TogglePause => Some(Event::TogglePause),
            Action::ToggleWrap => Some(Event::ToggleWrap),
            Action::Clear => (!state.is_empty()).then_some(Event::Clear),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Next,
        Action::Previous,
        Action::First,
        Action::Latest,
        Action::NextPage,
        Action::PreviousPage,
        Action::ExtendNext,
        Action::ExtendPrevious,
        Action::Select,
        Action::SelectAll,
        Action::Copy,
        Action::FocusSearch,
        Action::NextMatch,
        Action::PreviousMatch,
        Action::ClearSearch,
        Action::TogglePause,
        Action::ToggleWrap,
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
            Action::Latest => "Latest",
            Action::NextPage => "NextPage",
            Action::PreviousPage => "PreviousPage",
            Action::ExtendNext => "ExtendNext",
            Action::ExtendPrevious => "ExtendPrevious",
            Action::Select => "Select",
            Action::SelectAll => "SelectAll",
            Action::Copy => "Copy",
            Action::FocusSearch => "FocusSearch",
            Action::NextMatch => "NextMatch",
            Action::PreviousMatch => "PreviousMatch",
            Action::ClearSearch => "ClearSearch",
            Action::TogglePause => "TogglePause",
            Action::ToggleWrap => "ToggleWrap",
            Action::Clear => "Clear",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next line.",
            Action::Previous => "Highlights the previous line.",
            Action::First => "Highlights the oldest line.",
            Action::Latest => "Jumps to the newest line and follows new ones.",
            Action::NextPage => "Moves the highlight down by a page of lines.",
            Action::PreviousPage => "Moves the highlight up by a page of lines.",
            Action::ExtendNext => "Moves down and extends the selection.",
            Action::ExtendPrevious => "Moves up and extends the selection.",
            Action::Select => "Selects or deselects the highlighted line.",
            Action::SelectAll => "Selects every shown line.",
            Action::Copy => "Copies the selected lines, or the highlighted one.",
            Action::FocusSearch => "Focuses the search field.",
            Action::NextMatch => "Moves to the next line matching the search.",
            Action::PreviousMatch => "Moves to the previous line matching the search.",
            Action::ClearSearch => "Clears the search.",
            Action::TogglePause => "Pauses the stream, or resumes it with the held lines.",
            Action::ToggleWrap => "Wraps long lines, or ends them in an ellipsis.",
            Action::Clear => "Removes every line.",
        }
    }
}

/// The default log shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Latest`] |
/// | `PageDown` | [`Action::NextPage`] |
/// | `PageUp` | [`Action::PreviousPage`] |
/// | `Shift+ArrowDown` | [`Action::ExtendNext`] |
/// | `Shift+ArrowUp` | [`Action::ExtendPrevious`] |
/// | `Space` | [`Action::Select`] |
/// | `Mod+A` (Ctrl, or Cmd on macOS) | [`Action::SelectAll`] |
/// | `Mod+C` | [`Action::Copy`] |
/// | `Mod+F` | [`Action::FocusSearch`] |
/// | `F3` | [`Action::NextMatch`] |
/// | `Shift+F3` | [`Action::PreviousMatch`] |
/// | `Escape` | [`Action::ClearSearch`] |
/// | `Ctrl+L` | [`Action::Clear`] |
///
/// The log resolves these itself, only while it has focus, which it takes
/// when any part of it is pressed. Keys the search field uses, such as
/// typing, Home and End, stay with the field. While the field has focus,
/// Enter and Shift+Enter step through the matches, and Escape clears the
/// text, then a second Escape leaves the field. [`Action::TogglePause`]
/// and [`Action::ToggleWrap`] are on the toolbar and have no chord until
/// one is bound. Pass a changed keymap with [`LogViewer::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Latest)
        .bind(Chord::named(Named::PageDown), Action::NextPage)
        .bind(Chord::named(Named::PageUp), Action::PreviousPage)
        .bind(Chord::named(Named::ArrowDown).shift(), Action::ExtendNext)
        .bind(Chord::named(Named::ArrowUp).shift(), Action::ExtendPrevious)
        .bind(Chord::named(Named::Space), Action::Select)
        .bind(Chord::character('a').command(), Action::SelectAll)
        .bind(Chord::character('c').command(), Action::Copy)
        .bind(Chord::character('f').command(), Action::FocusSearch)
        .bind(Chord::named(Named::F3), Action::NextMatch)
        .bind(Chord::named(Named::F3).shift(), Action::PreviousMatch)
        .bind(Chord::named(Named::Escape), Action::ClearSearch)
        .bind(Chord::character('l').ctrl(), Action::Clear)
}

/// Owns the channel entries arrive on. Emits [`Event::Ready`] first, then
/// batches of up to [`BATCH_SIZE`] entries.
pub fn subscription() -> Subscription<Event> {
    Subscription::run(stream)
}

/// The stream behind [`subscription`], for driving it without a runtime.
pub fn stream() -> impl Stream<Item = Event> {
    let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
    stream::once(async move { Event::Ready(sender) })
        .chain(receiver.ready_chunks(BATCH_SIZE).map(Event::Received))
}

/// Focuses the search field of `state`'s log.
pub fn focus_search<T>(state: &State) -> Task<T> {
    widget::operation::focus(state.search_id.clone())
}

/// The text beside the search field: which match is current, of how many.
/// Empty while there is no search.
pub fn match_label(state: &State) -> String {
    if state.query().is_empty() {
        return String::new();
    }
    let total = state.match_count();
    match (total, state.current_match()) {
        (0, _) => "No matches".to_owned(),
        (_, Some((_, position))) => format!("{position} of {total}"),
        (_, None) => format!("{total} found"),
    }
}

/// An entry of the context menu from [`menu`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuItem {
    CopyLine,
    CopyMessage,
    FilterToLevel,
}

impl MenuItem {
    pub const ALL: [MenuItem; 3] = [
        MenuItem::CopyLine,
        MenuItem::CopyMessage,
        MenuItem::FilterToLevel,
    ];

    /// The event the entry sends for the line the menu opened on.
    pub fn event(self, id: LineId) -> Event {
        match self {
            MenuItem::CopyLine => Event::CopyLine(id),
            MenuItem::CopyMessage => Event::CopyMessage(id),
            MenuItem::FilterToLevel => Event::FilterToLevel(id),
        }
    }
}

/// A context menu to pass to [`LogViewer::context_menu`]: copy the line or
/// only its message, or show only lines of its level. Turn the chosen entry
/// into an event with [`MenuItem::event`].
pub fn menu() -> context_menu::State<MenuItem, LineId> {
    context_menu::State::new([
        item(MenuItem::CopyLine, "Copy line").icon(crate::lucide!(Copy)),
        item(MenuItem::CopyMessage, "Copy message").icon(crate::lucide!(TextQuote)),
        separator(),
        item(MenuItem::FilterToLevel, "Show only this level").icon(crate::lucide!(ListFilter)),
    ])
}

/// The colour of a level's tag.
pub fn level_colour(tokens: &Tokens, level: Level) -> Color {
    // Bright status colours are darkened on light backgrounds so the small
    // tag stays readable.
    let legible = |colour: Color| {
        if tokens.is_dark {
            colour
        } else {
            mix(colour, tokens.foreground, 0.25)
        }
    };
    match level {
        Level::Trace => fade(tokens.muted_foreground, 0.7),
        Level::Debug => tokens.muted_foreground,
        Level::Info => legible(tokens.success),
        Level::Warn => legible(tokens.warning),
        Level::Error => legible(tokens.destructive),
    }
}

/// How a line is shown.
pub type Status = virtual_list::Status;

/// The colours of a line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineStyle {
    pub background: Option<Color>,
    /// The ring around the line carrying the keyboard highlight.
    pub ring: Option<Color>,
    pub text: Color,
    /// Behind the parts of the message that match the search.
    pub mark: Color,
}

/// Resolves a line's colours. Lines take the list rows' colours, with the
/// accent's text on a selected line, and an error line is tinted when
/// nothing else colours it. The current match is marked more strongly than
/// the others.
pub fn line_style(tokens: &Tokens, level: Level, current: bool, status: Status) -> LineStyle {
    let row = virtual_list::row_style(tokens, status);
    let tint = (level == Level::Error)
        .then(|| fade(tokens.destructive, if tokens.is_dark { 0.14 } else { 0.07 }));
    let selected = status.selected && !status.disabled;
    LineStyle {
        background: row.background.or(tint),
        ring: row.ring,
        text: if selected {
            tokens.accent_foreground
        } else {
            tokens.foreground
        },
        mark: fade(tokens.warning, if current { 0.7 } else { 0.35 }),
    }
}

type OnEvent<'a, Message> = dyn Fn(Event) -> Message + 'a;
type BuildMenu<'a, Message> =
    dyn FnOnce(Keymap<context_menu::Action>) -> RowMenu<'a, LineId, Message> + 'a;

/// A log viewer builder. Convert it into an [`Element`] to render.
pub struct LogViewer<'a, Message> {
    state: &'a State,
    width: Length,
    height: Length,
    toolbar: bool,
    placeholder: String,
    empty: String,
    keymap: Keymap<Action>,
    menu: Option<Box<BuildMenu<'a, Message>>>,
    menu_keymap: Keymap<context_menu::Action>,
    on_event: Option<Box<OnEvent<'a, Message>>>,
}

/// Renders `state`: a toolbar with the search, level filters and actions,
/// above the lines. Without [`on_event`](LogViewer::on_event) everything
/// renders disabled.
pub fn log_viewer<'a, Message>(state: &'a State) -> LogViewer<'a, Message> {
    LogViewer {
        state,
        width: Length::Fill,
        height: Length::Fill,
        toolbar: true,
        placeholder: "Search messages".to_owned(),
        empty: "No log lines yet".to_owned(),
        keymap: default_keymap(),
        menu: None,
        menu_keymap: context_menu::default_keymap(),
        on_event: None,
    }
}

impl<Message> fmt::Debug for LogViewer<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogViewer")
            .field("lines", &self.state.visible_len())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("toolbar", &self.toolbar)
            .field("placeholder", &self.placeholder)
            .field("empty", &self.empty)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> LogViewer<'a, Message> {
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The height of the whole log, toolbar included. Fills by default.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Shows or hides the toolbar. Its actions stay on the keyboard.
    pub fn toolbar(mut self, toolbar: bool) -> Self {
        self.toolbar = toolbar;
        self
    }

    /// The search field's placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// What shows while no lines are shown.
    pub fn empty(mut self, message: impl Into<String>) -> Self {
        self.empty = message.into();
        self
    }

    /// Replaces the [`default_keymap`] the focused log resolves keys with.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// Gives every line the context menu of `menu`, keyed by line, such as
    /// the one from [`menu()`]. It opens on a right-click or a long press,
    /// and on the highlighted line with its opening chord while the log has
    /// focus.
    pub fn context_menu<MenuId>(
        mut self,
        menu: &'a context_menu::State<MenuId, LineId>,
        on_event: impl Fn(context_menu::Event<MenuId, LineId>) -> Message + 'a,
    ) -> Self
    where
        MenuId: Copy + PartialEq + 'a,
        Message: Clone + 'a,
    {
        let on_event: Rc<dyn Fn(context_menu::Event<MenuId, LineId>) -> Message + 'a> =
            Rc::new(on_event);
        self.menu = Some(Box::new(move |keymap| row_menu(menu, keymap, on_event)));
        self
    }

    /// Replaces the context menu's default keymap.
    pub fn context_menu_keymap(mut self, keymap: Keymap<context_menu::Action>) -> Self {
        self.menu_keymap = keymap;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_event.is_some()
    }
}

impl<'a, Message> From<LogViewer<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(viewer: LogViewer<'a, Message>) -> Self {
        let LogViewer {
            state,
            width,
            height,
            toolbar,
            placeholder,
            empty,
            keymap,
            menu,
            menu_keymap,
            on_event,
        } = viewer;
        let on_event: Option<Rc<OnEvent<'a, Message>>> = on_event.map(Rc::from);
        let menu = menu
            .filter(|_| on_event.is_some())
            .map(|build| Rc::new(build(menu_keymap)));
        let view = Rc::new(View {
            state,
            toolbar,
            placeholder,
            empty,
            menu: menu.clone(),
            on_event: on_event.clone(),
        });
        let body = responsive(move |size| view.body(size.width < BREAKPOINT))
            .width(width)
            .height(height);

        let Some(on_event) = on_event else {
            return body.into();
        };
        let on_focus = on_event.clone();
        scope(body)
            .on_focus(move |focused| on_focus(Event::Focus(focused)))
            .on_key(move |key| {
                let opened = menu
                    .as_ref()
                    .zip(state.highlighted())
                    .and_then(|(menu, target)| menu.key(key, target));
                opened.or_else(|| state.key_event(&keymap, key).map(&*on_event))
            })
            .into()
    }
}

/// Everything the log's parts are built from.
struct View<'a, Message> {
    state: &'a State,
    toolbar: bool,
    placeholder: String,
    empty: String,
    menu: Option<Rc<RowMenu<'a, LineId, Message>>>,
    on_event: Option<Rc<OnEvent<'a, Message>>>,
}

fn muted(theme: &Theme) -> Color {
    Tokens::of(theme).muted_foreground
}

impl<'a, Message: Clone + 'a> View<'a, Message> {
    fn send(&self, event: Event) -> Option<Message> {
        self.on_event.as_ref().map(|on_event| on_event(event))
    }

    fn body(&self, narrow: bool) -> Element<'a, Message> {
        let mut parts = column![].spacing(space::SM).height(Length::Fill);
        if self.toolbar {
            parts = parts.push(self.toolbar(narrow));
        }
        parts.push(self.lines(narrow)).into()
    }

    fn toolbar(&self, narrow: bool) -> Element<'a, Message> {
        let state = self.state;
        let has_matches = state.match_count() > 0;
        let field = input(&self.placeholder, state.query())
            .id(state.search_id.clone())
            .icon(crate::lucide!(Search))
            .size(input::Size::Sm)
            .width(if narrow {
                Length::Fill
            } else {
                Length::Fixed(240.0)
            })
            .on_input_maybe(
                self.on_event
                    .clone()
                    .map(|on_event| move |query| on_event(Event::Search(query))),
            )
            .on_submit_maybe(self.send(Event::NextMatch).filter(|_| has_matches));
        let search = search_keys(field, state.search_id.clone())
            .on_clear(
                self.send(Event::ClearSearch)
                    .filter(|_| !state.query().is_empty()),
            )
            .on_previous(self.send(Event::PreviousMatch).filter(|_| has_matches));
        let found = container(
            text(match_label(state))
                .size(text_size::XS)
                .wrapping(Wrapping::None)
                .style(|theme| text::Style {
                    color: Some(muted(theme)),
                }),
        )
        .center_x(COUNTER_WIDTH);
        let previous = icon_button(crate::lucide!(ChevronUp))
            .label("Previous match")
            .tooltip(Some(Position::Bottom))
            .size(button::Size::Sm)
            .on_press_maybe(self.send(Event::PreviousMatch).filter(|_| has_matches));
        let next = icon_button(crate::lucide!(ChevronDown))
            .label("Next match")
            .tooltip(Some(Position::Bottom))
            .size(button::Size::Sm)
            .on_press_maybe(self.send(Event::NextMatch).filter(|_| has_matches));
        let stepper = row![found, previous, next]
            .spacing(space::XS)
            .align_y(Alignment::Center);

        let mut levels = toggle_group(&state.levels).size(toggle::Size::Sm);
        if let Some(on_event) = self.on_event.clone() {
            levels = levels.on_event(move |event| on_event(Event::Levels(event)));
        }

        let can_copy = state.can_copy();
        let actions: [Element<'a, Message>; 4] = [
            icon_button(crate::lucide!(TextWrap))
                .label("Wrap lines")
                .tooltip(Some(Position::Bottom))
                .size(button::Size::Sm)
                .pressed(state.is_wrapped())
                .on_press_maybe(self.send(Event::ToggleWrap))
                .into(),
            icon_button(if state.is_paused() {
                crate::lucide!(Play)
            } else {
                crate::lucide!(Pause)
            })
            .label(if state.is_paused() { "Resume" } else { "Pause" })
            .tooltip(Some(Position::Bottom))
            .size(button::Size::Sm)
            .pressed(state.is_paused())
            .on_press_maybe(self.send(Event::TogglePause))
            .into(),
            icon_button(crate::lucide!(Copy))
                .label("Copy selected lines")
                .tooltip(Some(Position::Bottom))
                .size(button::Size::Sm)
                .on_press_maybe(self.send(Event::Copy).filter(|_| can_copy))
                .into(),
            icon_button(crate::lucide!(Trash))
                .label("Clear")
                .tooltip(Some(Position::Bottom))
                .size(button::Size::Sm)
                .on_press_maybe(self.send(Event::Clear).filter(|_| !state.is_empty()))
                .into(),
        ];

        // Narrow, the search field, the filters and the actions each take a
        // row, the filters wrap, and the match stepper joins the actions so
        // the field keeps the full width.
        if narrow {
            let tools = row![
                row(actions).spacing(space::XS),
                Space::new().width(Length::Fill),
                stepper,
            ]
            .align_y(Alignment::Center);
            return column![search, levels, tools].spacing(space::SM).into();
        }
        column![
            row![
                search,
                stepper,
                Space::new().width(Length::Fill),
                row(actions).spacing(space::XS),
            ]
            .spacing(space::SM)
            .align_y(Alignment::Center),
            levels,
        ]
        .spacing(space::SM)
        .into()
    }

    fn lines(&self, narrow: bool) -> Element<'a, Message> {
        let state = self.state;
        let frame = |content: Element<'a, Message>| {
            container(content)
                .padding(FRAME_PADDING)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|theme| virtual_list::frame_style(&Tokens::of(theme)))
        };
        if state.visible_len() == 0 {
            let message = text(self.empty.clone())
                .size(text_size::SM)
                .style(|theme| text::Style {
                    color: Some(muted(theme)),
                });
            return frame(container(message).center(Length::Fill).into()).into();
        }

        let on_event = self.on_event.clone();
        let menu = self.menu.clone();
        let wrap = state.is_wrapped();
        let build = move |range: std::ops::Range<usize>| -> Element<'a, Message> {
            column(range.filter_map(|index| {
                let id = *state.visible.get(index)?;
                let entry = state.entry(id)?;
                let line = line_view(state, id, entry, narrow, on_event.as_ref());
                Some(match &menu {
                    Some(menu) => menu.wrap(id, line),
                    None => line,
                })
            }))
            .into()
        };
        let row_height = if narrow {
            STACKED_ROW_HEIGHT
        } else {
            ROW_HEIGHT
        };
        let mut list = rows(state.visible_len(), row_height, build)
            .reveal(state.reveal_target())
            .follow(state.is_following())
            .origin(state.visible_dropped)
            .height(Length::Fill);
        if wrap {
            list =
                list.measured(move |index| state.visible.get(index).copied().unwrap_or_default());
        }
        if let Some(on_event) = self.on_event.clone() {
            let on_page = on_event.clone();
            list = list
                .on_page(move |lines| on_page(Event::Page(lines)))
                .on_follow(move |follow| on_event(Event::Follow(follow)));
        }

        let Some(pill) = self.pill() else {
            return frame(list.into()).into();
        };
        // Below the lines, inside the frame, so the newest line stays in
        // view above it.
        let pill = container(pill)
            .align_right(Length::Fill)
            .padding(Padding::ZERO.top(FRAME_PADDING).right(space::XS));
        frame(column![list, pill].into()).into()
    }

    /// A button under the lines: resume while paused, or jump to the newest
    /// line while not following.
    fn pill(&self) -> Option<Element<'a, Message>> {
        let state = self.state;
        if state.is_paused() {
            let label = match state.pending() {
                0 => "Resume".to_owned(),
                waiting => format!("Resume ({waiting} new)"),
            };
            let resume = button(label)
                .icon(crate::lucide!(Play))
                .size(button::Size::Sm)
                .variant(button::Variant::Secondary)
                .on_press_maybe(self.send(Event::TogglePause));
            return Some(resume.into());
        }
        if state.is_following() {
            return None;
        }
        let label = match state.unseen() {
            0 => "Jump to latest".to_owned(),
            unseen => format!("Jump to latest ({unseen} new)"),
        };
        let jump = button(label)
            .icon(crate::lucide!(ArrowDown))
            .size(button::Size::Sm)
            .variant(button::Variant::Secondary)
            .on_press_maybe(self.send(Event::Last));
        Some(jump.into())
    }
}

/// One line: timestamp, level, target and the message with its matches
/// marked.
/// Below the breakpoint the message sits under the other parts, so it keeps
/// the full width.
fn line_view<'a, Message: Clone + 'a>(
    state: &'a State,
    id: LineId,
    entry: &'a Entry,
    stacked: bool,
    on_event: Option<&Rc<OnEvent<'a, Message>>>,
) -> Element<'a, Message> {
    let status = Status {
        selected: state.selected.contains(&id),
        hovered: false,
        highlighted: state.focused && state.highlighted == Some(id),
        disabled: on_event.is_none(),
    };
    let level = entry.level;
    let current = state.current == Some(id);
    let small = |content: &'a str, width: f32| {
        container(
            ellipsis(content)
                .size(text_size::XS, LINE_HEIGHT)
                .width(Length::Shrink)
                .colour(muted),
        )
        .max_width(width)
    };

    let mut meta = row![].spacing(space::SM).align_y(Alignment::Start);
    if let Some(timestamp) = &entry.timestamp {
        meta = meta.push(small(timestamp, TIMESTAMP_WIDTH).width(TIMESTAMP_WIDTH));
    }
    meta = meta.push(
        text(level.tag())
            .size(text_size::XS)
            .line_height(LineHeight::Absolute(Pixels(LINE_HEIGHT)))
            .font(theme::semibold())
            .wrapping(Wrapping::None)
            .width(LEVEL_WIDTH)
            .style(move |theme| text::Style {
                color: Some(level_colour(&Tokens::of(theme), level)),
            }),
    );
    if let Some(target) = &entry.target {
        let width = if stacked { f32::INFINITY } else { TARGET_WIDTH };
        meta = meta.push(small(target, width));
    }
    let wrapping = if state.wrap {
        Wrapping::WordOrGlyph
    } else {
        Wrapping::None
    };
    let message = marked(
        &entry.message,
        find(&entry.message, &state.query),
        move |theme| line_style(&Tokens::of(theme), level, current, status).text,
        move |theme| line_style(&Tokens::of(theme), level, current, status).mark,
    )
    .size(text_size::XS)
    .line_height(LineHeight::Absolute(Pixels(LINE_HEIGHT)))
    .wrapping(wrapping);
    let (line, height): (Element<'a, Message>, f32) = if stacked {
        (column![meta, message].into(), STACKED_ROW_HEIGHT)
    } else {
        (meta.push(message).into(), ROW_HEIGHT)
    };

    let padding = (ROW_HEIGHT - LINE_HEIGHT) / 2.0;
    let body = container(line)
        .padding(Padding::from([padding, space::SM]))
        .width(Length::Fill)
        .height(if state.wrap {
            Length::Shrink
        } else {
            Length::Fixed(height)
        })
        .clip(true);
    let row = pressable(body, move |theme, hovered| {
        let style = line_style(
            &Tokens::of(theme),
            level,
            current,
            Status { hovered, ..status },
        );
        Look {
            background: style.background,
            border: Border {
                color: style.ring.unwrap_or(Color::TRANSPARENT),
                width: if style.ring.is_some() { 1.0 } else { 0.0 },
                radius: radius::SM.into(),
            },
            divider: None,
        }
    });
    let Some(on_event) = on_event.cloned() else {
        return row.into();
    };
    row.on_press(move |modifiers| on_event(Event::Press(id, modifiers)))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Action as _;
    use crate::theme::{dark, light};
    use iced::futures::SinkExt;
    use iced::futures::executor::block_on;
    use iced::keyboard::Key;

    fn log(lines: &[(Level, &str)]) -> State {
        let mut state = State::new();
        state.extend(lines.iter().map(|&(level, message)| entry(level, message)));
        state
    }

    fn sample() -> State {
        log(&[
            (Level::Info, "server started"),
            (Level::Debug, "loaded config"),
            (Level::Warn, "disk almost full"),
            (Level::Error, "request failed: timeout"),
            (Level::Info, "request served"),
        ])
    }

    fn key(key: Key, modifiers: Modifiers) -> keys::Event {
        keys::Event { key, modifiers }
    }

    #[test]
    fn entries_format_as_one_line() {
        let full = entry(Level::Warn, "slow query")
            .timestamp("12:00:01.250")
            .target("db::pool");
        assert_eq!(full.line(), "12:00:01.250 WARN  db::pool: slow query");
        assert_eq!(entry(Level::Error, "boom").line(), "ERROR boom");
        assert_eq!(Entry::default().level, Level::Info);
    }

    #[test]
    fn levels_have_tags_names_and_an_order() {
        assert_eq!(Level::ALL.len(), 5);
        assert_eq!(Level::Warn.tag(), "WARN");
        assert_eq!(Level::Warn.name(), "Warn");
        assert_eq!(Level::Error.to_string(), "ERROR");
        assert!(Level::Trace < Level::Error);
        for (index, level) in Level::ALL.iter().enumerate() {
            assert_eq!(level.index(), index);
        }
    }

    #[test]
    fn a_new_log_follows_and_shows_every_level() {
        let state = State::new();
        assert!(state.is_empty());
        assert!(state.is_following());
        assert!(!state.is_paused());
        assert!(!state.is_wrapped());
        assert_eq!(state.limit(), DEFAULT_LIMIT);
        assert_eq!(state.search_id(), &SEARCH_ID);
        assert!(Level::ALL.iter().all(|&level| state.is_shown(level)));
        assert!(State::new().with_wrap(true).is_wrapped());
        let custom = State::new().with_search_id(widget::Id::new("logs"));
        assert_eq!(custom.search_id(), &widget::Id::new("logs"));
    }

    #[test]
    fn received_entries_are_counted_by_level() {
        let mut state = sample();
        assert_eq!(state.len(), 5);
        assert_eq!(state.visible_len(), 5);
        assert_eq!(state.count(Level::Info), 2);
        assert_eq!(state.count(Level::Trace), 0);
        let _ = state.update(Event::Received(vec![entry(Level::Error, "again")]));
        assert_eq!(state.count(Level::Error), 2);
        assert_eq!(
            state.entry(5).map(|entry| entry.message.as_str()),
            Some("again")
        );
        assert_eq!(state.entries().count(), 6);
    }

    #[test]
    fn the_limit_keeps_the_newest_entries() {
        let mut state = sample().with_limit(3);
        assert_eq!(state.len(), 3);
        assert_eq!(state.entry(0), None, "dropped");
        assert_eq!(state.visible().collect::<Vec<_>>(), [2, 3, 4]);
        assert_eq!(state.count(Level::Info), 1);
        assert_eq!(state.count(Level::Debug), 0);
        assert_eq!(state.visible_dropped, 2);
        let _ = state.update(Event::Press(2, Modifiers::empty()));
        let _ = state.update(Event::Received(vec![
            entry(Level::Info, "a"),
            entry(Level::Info, "b"),
        ]));
        assert_eq!(state.visible().collect::<Vec<_>>(), [4, 5, 6]);
        assert!(state.selected().is_empty(), "the selected line was dropped");
        assert_eq!(state.highlighted(), None);
        assert_eq!(State::new().with_limit(0).limit(), 1);
    }

    #[test]
    fn hiding_a_level_filters_lines_and_their_selection() {
        let mut state = sample();
        let _ = state.update(Event::Press(1, Modifiers::empty()));
        let _ = state.update(Event::ToggleLevel(Level::Debug));
        assert!(!state.is_shown(Level::Debug));
        assert_eq!(state.visible().collect::<Vec<_>>(), [0, 2, 3, 4]);
        assert!(state.selected().is_empty());
        let _ = state.update(Event::Received(vec![entry(Level::Debug, "hidden")]));
        assert_eq!(state.visible_len(), 4, "new hidden lines stay hidden");
        assert_eq!(state.count(Level::Debug), 2, "but are still counted");
        let _ = state.update(Event::ToggleLevel(Level::Debug));
        assert_eq!(state.visible_len(), 6);

        let errors = sample().with_levels([Level::Warn, Level::Error]);
        assert_eq!(errors.visible().collect::<Vec<_>>(), [2, 3]);
    }

    #[test]
    fn search_finds_messages_and_moves_between_matches() {
        let mut state = sample();
        let _ = state.update(Event::Search("REQUEST".into()));
        assert_eq!(state.query(), "REQUEST");
        assert_eq!(state.matches().collect::<Vec<_>>(), [3, 4]);
        assert_eq!(state.current_match(), None);
        let _ = state.update(Event::NextMatch);
        assert_eq!(state.current_match(), Some((3, 1)));
        assert_eq!(state.highlighted(), Some(3));
        assert!(
            !state.is_following(),
            "a match above the end stops following"
        );
        let _ = state.update(Event::NextMatch);
        assert_eq!(state.current_match(), Some((4, 2)));
        assert!(state.is_following(), "the newest line follows again");
        let _ = state.update(Event::NextMatch);
        assert_eq!(state.current_match(), Some((3, 1)), "wraps to the first");
        let _ = state.update(Event::PreviousMatch);
        assert_eq!(state.current_match(), Some((4, 2)), "wraps to the last");
        let _ = state.update(Event::Received(vec![entry(Level::Info, "request queued")]));
        assert_eq!(
            state.match_count(),
            3,
            "new lines are searched as they arrive"
        );
        let _ = state.update(Event::ClearSearch);
        assert_eq!(state.query(), "");
        assert_eq!(state.match_count(), 0);
        assert_eq!(state.current_match(), None);
    }

    #[test]
    fn previous_match_from_nothing_starts_at_the_newest() {
        let mut state = sample();
        let _ = state.update(Event::Search("request".into()));
        let _ = state.update(Event::PreviousMatch);
        assert_eq!(state.current_match(), Some((4, 2)));
        let mut none = sample();
        let _ = none.update(Event::Search("nothing like this".into()));
        let _ = none.update(Event::NextMatch);
        assert_eq!(none.current_match(), None);
        assert_eq!(none.highlighted(), None);
    }

    #[test]
    fn matches_follow_the_level_filter() {
        let mut state = sample();
        let _ = state.update(Event::Search("request".into()));
        let _ = state.update(Event::ToggleLevel(Level::Error));
        assert_eq!(state.matches().collect::<Vec<_>>(), [4]);
    }

    #[test]
    fn pausing_holds_entries_back_until_resumed() {
        let mut state = sample().with_limit(6);
        let _ = state.update(Event::TogglePause);
        assert!(state.is_paused());
        let _ = state.update(Event::Received(
            (0..10)
                .map(|n| entry(Level::Info, format!("held {n}")))
                .collect(),
        ));
        assert_eq!(state.len(), 5, "nothing shown while paused");
        assert_eq!(state.pending(), 6, "held entries are bounded by the limit");
        let _ = state.update(Event::TogglePause);
        assert!(!state.is_paused());
        assert_eq!(state.pending(), 0);
        assert_eq!(state.len(), 6);
        let last = state.entries().last().map(|entry| entry.message.clone());
        assert_eq!(last.as_deref(), Some("held 9"));
    }

    #[test]
    fn clearing_removes_everything_and_follows() {
        let mut state = sample();
        let _ = state.update(Event::Press(1, Modifiers::empty()));
        let _ = state.update(Event::Clear);
        assert!(state.is_empty());
        assert_eq!(state.visible_len(), 0);
        assert_eq!(state.count(Level::Info), 0);
        assert!(state.selected().is_empty());
        assert!(state.is_following());
        let _ = state.update(Event::Received(vec![entry(Level::Info, "fresh")]));
        assert_eq!(state.visible().collect::<Vec<_>>(), [5], "ids carry on");
    }

    #[test]
    fn copying_returns_the_selected_lines_in_order() {
        let mut state = sample();
        assert!(state.update(Event::Copy).is_none(), "nothing to copy");
        assert!(!state.can_copy());
        let _ = state.update(Event::Press(3, Modifiers::empty()));
        assert!(state.can_copy());
        let _ = state.update(Event::Press(0, Modifiers::COMMAND));
        let Some(Output::Copy(text)) = state.update(Event::Copy) else {
            panic!("copies the selection");
        };
        assert_eq!(text, "INFO  server started\nERROR request failed: timeout");
        let _ = state.update(Event::Press(0, Modifiers::COMMAND));
        let _ = state.update(Event::Press(3, Modifiers::COMMAND));
        assert!(state.selected().is_empty());
        assert_eq!(
            state.copy_text().as_deref(),
            Some("ERROR request failed: timeout"),
            "falls back to the highlighted line"
        );
    }

    #[test]
    fn shift_presses_and_keys_select_ranges_of_shown_lines() {
        let mut state = sample();
        let _ = state.update(Event::ToggleLevel(Level::Debug));
        let _ = state.update(Event::Press(0, Modifiers::empty()));
        let _ = state.update(Event::Press(3, Modifiers::SHIFT));
        assert_eq!(state.selected(), [0, 2, 3], "the hidden line is skipped");
        let _ = state.update(Event::ExtendNext);
        assert_eq!(state.selected(), [0, 2, 3, 4]);
        let _ = state.update(Event::Press(2, Modifiers::empty()));
        let _ = state.update(Event::ExtendPrevious);
        assert_eq!(state.selected(), [0, 2]);
        let _ = state.update(Event::SelectAll);
        assert_eq!(state.selected(), [0, 2, 3, 4]);
        let _ = state.update(Event::Press(1, Modifiers::empty()));
        assert_eq!(
            state.selected(),
            [0, 2, 3, 4],
            "a hidden line cannot be pressed"
        );
    }

    #[test]
    fn navigation_follows_only_at_the_newest_line() {
        let mut state = sample();
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(4));
        assert!(state.is_following());
        let _ = state.update(Event::Previous);
        assert!(!state.is_following());
        let _ = state.update(Event::Received(vec![entry(Level::Info, "new")]));
        assert_eq!(state.unseen(), 1);
        let _ = state.update(Event::First);
        assert_eq!(state.highlighted(), Some(0));
        let _ = state.update(Event::Page(2));
        let _ = state.update(Event::NextPage);
        assert_eq!(state.highlighted(), Some(2));
        let _ = state.update(Event::PreviousPage);
        assert_eq!(state.highlighted(), Some(0));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(1));
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(5));
        assert!(state.is_following());
        assert_eq!(state.unseen(), 0);
        assert_eq!(state.reveal_target().map(|(index, _)| index), Some(5));
    }

    #[test]
    fn scrolling_away_stops_following_and_counts_new_lines() {
        let mut state = sample();
        let _ = state.update(Event::Follow(false));
        assert!(!state.is_following());
        let _ = state.update(Event::Received(vec![
            entry(Level::Info, "one"),
            entry(Level::Info, "two"),
        ]));
        assert_eq!(state.unseen(), 2);
        let _ = state.update(Event::Follow(true));
        assert!(state.is_following());
        assert_eq!(state.unseen(), 0);
    }

    #[test]
    fn focus_lands_on_the_newest_line_without_scrolling() {
        let mut state = sample();
        let _ = state.update(Event::Focus(true));
        assert!(state.is_focused());
        assert_eq!(state.highlighted(), Some(4));
        assert_eq!(state.reveal_target(), None);
        let mut paused = sample();
        let _ = paused.update(Event::Follow(false));
        let _ = paused.update(Event::Focus(true));
        assert_eq!(paused.highlighted(), Some(0));
        let _ = paused.update(Event::Focus(false));
        assert!(!paused.is_focused());
    }

    #[test]
    fn wrap_and_page_events_update_the_state() {
        let mut state = sample();
        let _ = state.update(Event::ToggleWrap);
        assert!(state.is_wrapped());
        let _ = state.update(Event::Page(0));
        assert_eq!(state.page, 1);
        assert!(matches!(
            state.update(Event::FocusSearch),
            Some(Output::FocusSearch)
        ));
        let (sender, _receiver) = mpsc::channel(1);
        assert!(matches!(
            state.update(Event::Ready(sender)),
            Some(Output::Ready(_))
        ));
    }

    #[test]
    fn the_default_keymap_maps_to_events() {
        let mut state = sample();
        let keymap = default_keymap();
        let resolve = |state: &State, pressed: keys::Event| state.key_event(&keymap, &pressed);
        let named = |name: Named, modifiers| key(Key::Named(name), modifiers);
        let character = |c: &str, modifiers| key(Key::Character(c.into()), modifiers);

        assert!(matches!(
            resolve(&state, character("f", Modifiers::COMMAND)),
            Some(Event::FocusSearch)
        ));
        assert!(matches!(
            resolve(&state, named(Named::End, Modifiers::empty())),
            Some(Event::Last)
        ));
        assert!(
            resolve(&state, named(Named::Space, Modifiers::empty())).is_none(),
            "nothing highlighted to select"
        );
        assert!(
            resolve(&state, character("w", Modifiers::empty())).is_none(),
            "no bare letters"
        );
        assert!(
            resolve(&state, named(Named::Enter, Modifiers::empty())).is_none(),
            "lines have no activation"
        );
        assert!(matches!(
            resolve(&state, character("l", Modifiers::CTRL)),
            Some(Event::Clear)
        ));
        assert!(
            resolve(&state, named(Named::F3, Modifiers::empty())).is_none(),
            "no search, so F3 is left for others"
        );
        assert!(resolve(&state, named(Named::Escape, Modifiers::empty())).is_none());
        assert!(resolve(&state, character("c", Modifiers::COMMAND)).is_none());

        let _ = state.update(Event::Search("request".into()));
        assert!(matches!(
            resolve(&state, named(Named::F3, Modifiers::empty())),
            Some(Event::NextMatch)
        ));
        assert!(
            resolve(&state, named(Named::Enter, Modifiers::SHIFT)).is_none(),
            "Enter steps through matches only in the search field"
        );
        assert!(matches!(
            resolve(&state, named(Named::F3, Modifiers::SHIFT)),
            Some(Event::PreviousMatch)
        ));
        assert!(matches!(
            resolve(&state, named(Named::Escape, Modifiers::empty())),
            Some(Event::ClearSearch)
        ));
        let _ = state.update(Event::Press(0, Modifiers::empty()));
        assert!(matches!(
            resolve(&state, character("c", Modifiers::COMMAND)),
            Some(Event::Copy)
        ));
        assert!(matches!(
            resolve(&state, named(Named::ArrowDown, Modifiers::SHIFT)),
            Some(Event::ExtendNext)
        ));
        assert!(matches!(
            resolve(&state, named(Named::Space, Modifiers::empty())),
            Some(Event::ToggleHighlighted)
        ));

        let empty = State::new();
        assert!(resolve(&empty, named(Named::ArrowDown, Modifiers::empty())).is_none());
        assert!(resolve(&empty, character("l", Modifiers::CTRL)).is_none());
        assert!(
            matches!(
                resolve(&empty, named(Named::End, Modifiers::empty())),
                Some(Event::Last)
            ),
            "End always resumes following"
        );
    }

    #[test]
    fn keymaps_can_be_overridden_and_unbound() {
        let keymap = default_keymap()
            .bind(Chord::character('p'), Action::TogglePause)
            .unbind(&Chord::named(Named::Space));
        let state = sample();
        let p = key(Key::Character("p".into()), Modifiers::empty());
        assert!(matches!(
            state.key_event(&keymap, &p),
            Some(Event::TogglePause)
        ));
        let space = key(Key::Named(Named::Space), Modifiers::empty());
        assert!(state.key_event(&keymap, &space).is_none());
    }

    #[test]
    fn every_action_is_named_described_and_bound_but_the_toolbar_toggles() {
        let toolbar_only = [Action::TogglePause, Action::ToggleWrap];
        for action in Action::ALL {
            assert!(!action.name().is_empty());
            assert!(action.description().ends_with('.'));
            let bound = !default_keymap().chords(action).is_empty();
            assert_eq!(bound, !toolbar_only.contains(action), "{action:?}");
        }
        let state = sample();
        assert!(matches!(
            Action::TogglePause.event(&state),
            Some(Event::TogglePause)
        ));
        assert!(matches!(
            Action::ToggleWrap.event(&state),
            Some(Event::ToggleWrap)
        ));
    }

    #[test]
    fn no_default_chord_is_a_bare_letter_or_alt() {
        for (chord, _) in default_keymap().bindings() {
            let label = chord.to_string();
            assert!(!label.contains("Alt"), "{label}");
            let bare = label.chars().count() == 1;
            assert!(!bare, "{label}");
        }
    }

    #[test]
    fn space_selects_the_highlighted_line_and_keeps_the_others() {
        let mut state = sample();
        let _ = state.update(Event::Press(1, Modifiers::empty()));
        let _ = state.update(Event::Next);
        let _ = state.update(Event::ToggleHighlighted);
        assert_eq!(state.selected(), [1, 2]);
        let _ = state.update(Event::ToggleHighlighted);
        assert_eq!(state.selected(), [1]);
        assert!(State::new().update(Event::ToggleHighlighted).is_none());
    }

    #[test]
    fn the_level_filters_are_a_multiple_choice_group_with_counts() {
        let mut state = sample();
        let levels = state.levels();
        assert_eq!(levels.mode(), toggle_group::Mode::Multiple);
        assert_eq!(levels.selected(), Level::ALL);
        let labels: Vec<&str> = levels
            .items()
            .iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(
            labels,
            ["Trace 0", "Debug 1", "Info 2", "Warn 1", "Error 1"]
        );

        let _ = state.update(Event::Levels(toggle_group::Event::Toggle(Level::Info)));
        assert!(!state.is_shown(Level::Info));
        assert!(!state.levels().is_selected(Level::Info));
        assert_eq!(state.visible().collect::<Vec<_>>(), [1, 2, 3]);

        let _ = state.update(Event::Received(vec![entry(Level::Info, "more")]));
        assert_eq!(state.levels().items()[2].label, "Info 3", "counts follow");
        assert!(!state.levels().is_selected(Level::Info), "and stay off");
        let _ = state.update(Event::ToggleLevel(Level::Info));
        assert!(state.levels().is_selected(Level::Info));
        let _ = state.update(Event::Clear);
        assert_eq!(state.levels().items()[2].label, "Info 0");
    }

    #[test]
    fn the_menu_copies_and_filters_one_line() {
        let mut state = sample();
        let Some(Output::Copy(line)) = state.update(MenuItem::CopyLine.event(2)) else {
            panic!("copies the line");
        };
        assert_eq!(line, "WARN  disk almost full");
        let Some(Output::Copy(message)) = state.update(MenuItem::CopyMessage.event(3)) else {
            panic!("copies the message");
        };
        assert_eq!(message, "request failed: timeout");
        assert!(state.update(Event::CopyLine(99)).is_none());

        let _ = state.update(MenuItem::FilterToLevel.event(0));
        assert_eq!(state.visible().collect::<Vec<_>>(), [0, 4]);
        assert_eq!(state.levels().selected(), [Level::Info]);
        let _ = state.update(Event::FilterToLevel(99));
        assert_eq!(state.visible_len(), 2, "a missing line changes nothing");

        let menu = menu();
        assert_eq!(MenuItem::ALL.len(), 3);
        for item in MenuItem::ALL {
            assert!(menu.menu().item(item).is_some(), "{item:?}");
        }
    }

    #[test]
    fn the_match_label_counts_matches() {
        let mut state = sample();
        assert_eq!(match_label(&state), "");
        let _ = state.update(Event::Search("request".into()));
        assert_eq!(match_label(&state), "2 found");
        let _ = state.update(Event::NextMatch);
        assert_eq!(match_label(&state), "1 of 2");
        let _ = state.update(Event::Search("nothing like this".into()));
        assert_eq!(match_label(&state), "No matches");
    }

    #[test]
    fn level_colours_come_from_the_tokens() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(level_colour(&tokens, Level::Debug), tokens.muted_foreground);
            let colours: Vec<Color> = Level::ALL
                .iter()
                .map(|&level| level_colour(&tokens, level))
                .collect();
            for (index, colour) in colours.iter().enumerate() {
                assert!(
                    !colours[index + 1..].contains(colour),
                    "levels look different: {colours:?}"
                );
            }
        }
        let dark = Tokens::of(&dark());
        assert_eq!(level_colour(&dark, Level::Error), dark.destructive);
    }

    #[test]
    fn line_styles_cover_levels_matches_and_statuses() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = line_style(&tokens, Level::Info, false, Status::default());
            assert_eq!(idle.background, None);
            assert_eq!(idle.ring, None);
            assert_eq!(idle.text, tokens.foreground);
            let error = line_style(&tokens, Level::Error, false, Status::default());
            assert!(error.background.is_some(), "error lines are tinted");
            let selected = line_style(
                &tokens,
                Level::Error,
                false,
                Status {
                    selected: true,
                    ..Status::default()
                },
            );
            assert_eq!(selected.background, Some(tokens.accent), "selection wins");
            assert_eq!(selected.text, tokens.accent_foreground);
            let hovered = line_style(
                &tokens,
                Level::Warn,
                false,
                Status {
                    hovered: true,
                    ..Status::default()
                },
            );
            assert!(hovered.background.is_some());
            let highlighted = line_style(
                &tokens,
                Level::Info,
                false,
                Status {
                    highlighted: true,
                    ..Status::default()
                },
            );
            assert_eq!(highlighted.ring, Some(tokens.ring));
            let disabled = line_style(
                &tokens,
                Level::Info,
                false,
                Status {
                    disabled: true,
                    highlighted: true,
                    ..Status::default()
                },
            );
            assert_eq!(disabled.ring, None);
            let current = line_style(&tokens, Level::Info, true, Status::default());
            assert!(current.mark.a > idle.mark.a, "the current match stands out");
        }
    }

    #[test]
    fn the_builder_has_sensible_defaults() {
        let state = sample();
        let viewer: LogViewer<'_, ()> = log_viewer(&state);
        assert!(!viewer.is_enabled());
        assert!(viewer.toolbar);
        assert_eq!(viewer.height, Length::Fill);
        assert!(viewer.menu.is_none());
        assert_eq!(viewer.menu_keymap, context_menu::default_keymap());
        let menu = menu();
        let viewer = viewer
            .toolbar(false)
            .height(300)
            .width(400)
            .placeholder("Filter")
            .empty("Quiet")
            .keymap(Keymap::new())
            .context_menu(&menu, |_| ())
            .context_menu_keymap(Keymap::new())
            .on_event(|_| ());
        assert!(viewer.menu.is_some());
        assert!(viewer.menu_keymap.is_empty());
        assert!(viewer.is_enabled());
        assert!(!viewer.toolbar);
        assert_eq!(viewer.height, Length::Fixed(300.0));
        assert_eq!(viewer.placeholder, "Filter");
        assert_eq!(viewer.empty, "Quiet");
        assert!(viewer.keymap.is_empty());
        assert!(format!("{viewer:?}").contains("LogViewer"));
    }

    #[test]
    fn stream_emits_ready_first_then_batches() {
        let mut events = Box::pin(stream());
        let Some(Event::Ready(mut sender)) = block_on(events.next()) else {
            panic!("first event must be Ready");
        };
        block_on(async {
            for n in 0..3 {
                sender
                    .send(entry(Level::Info, format!("line {n}")))
                    .await
                    .expect("channel open");
            }
        });
        let Some(Event::Received(batch)) = block_on(events.next()) else {
            panic!("then a batch");
        };
        assert_eq!(batch.len(), 3);
        assert_eq!(batch[2].message, "line 2");
    }

    #[test]
    fn bursts_arrive_in_batches_of_the_batch_size() {
        let items = stream::iter(0..BATCH_SIZE * 2 + 1);
        let sizes: Vec<usize> = block_on(
            items
                .ready_chunks(BATCH_SIZE)
                .map(|batch| batch.len())
                .collect(),
        );
        assert_eq!(sizes, [BATCH_SIZE, BATCH_SIZE, 1]);
    }
}
