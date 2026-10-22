//! A long list of equally tall rows that only builds the rows in view, so
//! a hundred thousand items scroll as smoothly as ten.
//!
//! [`State`] holds the items, the selection in the chosen [`Mode`] and the
//! keyboard highlight. Its [`update`](State::update) handles presses,
//! keyboard navigation and items arriving from the channel that
//! [`subscription`] owns. The app draws each row with a closure that gets
//! the item's index and the item.
//!
//! A list made [`with_follow`](State::with_follow) sticks to its end as
//! items arrive, like a feed or a log. Scrolling up stops following, and
//! scrolling back to the end, or pressing End, starts again. A list made
//! [`with_limit`](State::with_limit) drops its oldest items once it holds
//! that many, so a stream never grows without bound.
//!
//! The list resolves its keys through a [`Keymap`] of [`Action`]s while it
//! has focus, which it takes when pressed; see [`default_keymap`].
//!
//! ```no_run
//! use iced::widget::text;
//! use iced::{Element, Subscription};
//! use iced_cube::data::virtual_list::{self, Event, Output, State, virtual_list};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Feed(Event<String>),
//! }
//!
//! struct App {
//!     feed: State<String>,
//!     sender: Option<virtual_list::Sender<String>>,
//! }
//!
//! impl App {
//!     fn update(&mut self, message: Message) {
//!         let Message::Feed(event) = message;
//!         if let Some(Output::Ready(sender)) = self.feed.update(event) {
//!             // Hand clones of the sender to producers.
//!             self.sender = Some(sender);
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         virtual_list(&self.feed, |_, line: &String| text(line.as_str()).into())
//!             .height(320)
//!             .on_event(Message::Feed)
//!             .into()
//!     }
//!
//!     fn subscription(&self) -> Subscription<Message> {
//!         virtual_list::subscription().map(Message::Feed)
//!     }
//! }
//! ```

use std::collections::{BTreeSet, VecDeque};
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

use iced::futures::channel::mpsc;
use iced::futures::{Stream, StreamExt, stream};
use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;
use iced::widget::{column, container};
use iced::{Alignment, Border, Color, Element, Length, Padding, Subscription};

use crate::data::interaction::{Look, RowMenu, pressable, row_menu, scope};
use crate::data::rows::rows;
use crate::keys::{self, Chord, Keymap};
use crate::overlay::context_menu;
use crate::theme::{Tokens, fade, radius, space};

/// The default height of every row.
pub const ROW_HEIGHT: f32 = 32.0;
/// How many items a producer can send before it has to wait.
pub const CHANNEL_CAPACITY: usize = 256;
/// The most items delivered in one [`Event::Received`].
pub const BATCH_SIZE: usize = 256;
/// Rows a page key moves until the list reports how many fit.
pub const DEFAULT_PAGE: usize = 10;
/// Space between the frame and the rows.
const FRAME_PADDING: f32 = 4.0;

/// Names an item for as long as it is in the list: its position counted
/// from the first item the list ever held, so it stays the same when
/// older items are dropped. Without a limit it equals the index.
pub type RowId = u64;

/// How items are selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// Items can be highlighted and activated, but not selected.
    None,
    /// One item at a time.
    #[default]
    Single,
    /// Ctrl or Cmd click toggles an item, Shift click selects a range.
    Multiple,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::None, Mode::Single, Mode::Multiple];
}

/// The sending half of the channel items arrive on. Clone it for each
/// producer.
pub type Sender<T> = mpsc::Sender<T>;

/// Everything that changes a list.
#[derive(Debug, Clone)]
pub enum Event<T> {
    /// The channel is open. Keep the sender and hand clones to producers.
    Ready(Sender<T>),
    /// Items from the channel, oldest first. They are appended.
    Received(Vec<T>),
    /// The pointer pressed a row with these modifiers held.
    Press(RowId, Modifiers),
    /// A row was double clicked.
    Activate(RowId),
    Next,
    Previous,
    First,
    Last,
    NextPage,
    PreviousPage,
    /// Moves the highlight and extends the selection to it, in
    /// [`Mode::Multiple`].
    ExtendNext,
    ExtendPrevious,
    /// Selects or toggles the highlighted item, by [`Mode`].
    ToggleHighlighted,
    ActivateHighlighted,
    /// Selects every item, in [`Mode::Multiple`].
    SelectAll,
    /// Starts or stops following the end. The list sends `false` when the
    /// user scrolls away from the end, and `true` when they scroll back.
    Follow(bool),
    /// How many rows fit in the view. The list sends this itself, and the
    /// page keys move by it.
    Page(usize),
    /// The list gained or lost focus. The list sends this itself.
    Focus(bool),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone)]
pub enum Output<T> {
    Ready(Sender<T>),
    /// An item was activated with Enter or a double click. Holds its index.
    Activated(usize),
    /// The selection changed. Holds every selected index, in order.
    Selected(Vec<usize>),
}

/// The items, the selection, the highlight and whether the list follows
/// its end.
#[derive(Debug, Clone)]
pub struct State<T> {
    items: VecDeque<T>,
    limit: Option<usize>,
    /// Items dropped from the front so far, which is the first item's id.
    dropped: u64,
    mode: Mode,
    selected: BTreeSet<RowId>,
    /// Where a Shift click or Shift arrow range starts.
    anchor: Option<RowId>,
    highlighted: Option<RowId>,
    focused: bool,
    /// Whether the list can follow its end at all.
    tail: bool,
    following: bool,
    page: usize,
    /// The item to scroll into view, and a count that changes with every
    /// request so the same item can be revealed again.
    reveal: Option<(RowId, u64)>,
    requests: u64,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        Self {
            items: VecDeque::new(),
            limit: None,
            dropped: 0,
            mode: Mode::default(),
            selected: BTreeSet::new(),
            anchor: None,
            highlighted: None,
            focused: false,
            tail: false,
            following: false,
            page: DEFAULT_PAGE,
            reveal: None,
            requests: 0,
        }
    }
}

impl<T> State<T> {
    /// A list in [`Mode::Single`] holding `items`.
    pub fn new(items: impl IntoIterator<Item = T>) -> Self {
        Self {
            items: items.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Keeps at most `limit` items, dropping the oldest as new ones
    /// arrive. A limit of zero is treated as one.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit.max(1));
        self.trim();
        self
    }

    /// Follows the end as items arrive, starting now. Scrolling away stops
    /// following, and scrolling back to the end or pressing End starts it
    /// again.
    pub fn with_follow(mut self, follow: bool) -> Self {
        self.tail = follow;
        self.following = follow;
        self
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    /// The items, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    /// The most items kept, if there is a limit.
    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    /// The id of the item at `index`.
    pub fn id(&self, index: usize) -> RowId {
        self.dropped + index as u64
    }

    /// The index of the item with `id`, if it is still in the list.
    pub fn index(&self, id: RowId) -> Option<usize> {
        let index = usize::try_from(id.checked_sub(self.dropped)?).ok()?;
        (index < self.items.len()).then_some(index)
    }

    /// How many items have been dropped from the front so far.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selected.contains(&self.id(index))
    }

    /// Every selected index, in order.
    pub fn selected(&self) -> Vec<usize> {
        self.selected
            .iter()
            .filter_map(|&id| self.index(id))
            .collect()
    }

    /// The index the keyboard acts on.
    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted.and_then(|id| self.index(id))
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Whether the list is following its end.
    pub fn is_following(&self) -> bool {
        self.tail && self.following
    }

    /// How many rows the page keys move.
    pub fn page_size(&self) -> usize {
        self.page
    }

    /// Appends an item, dropping the oldest when over the limit.
    pub fn push(&mut self, item: T) {
        self.items.push_back(item);
        self.trim();
    }

    /// Appends items, dropping the oldest when over the limit.
    pub fn extend(&mut self, items: impl IntoIterator<Item = T>) {
        self.items.extend(items);
        self.trim();
    }

    /// Replaces every item. The selection and highlight are cleared, and
    /// ids carry on from where the old items ended.
    pub fn set_items(&mut self, items: impl IntoIterator<Item = T>) {
        self.dropped += self.items.len() as u64;
        self.items = items.into_iter().collect();
        self.selected.clear();
        self.anchor = None;
        self.highlighted = None;
        self.reveal = None;
        self.trim();
    }

    /// Removes every item.
    pub fn clear(&mut self) {
        self.set_items(std::iter::empty());
    }

    /// Scrolls the item at `index` into view without highlighting it. A
    /// following list stops following unless it is the last item.
    pub fn scroll_to(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        self.requests += 1;
        self.reveal = Some((self.id(index), self.requests));
        self.following = index + 1 == self.items.len();
    }

    /// The row to keep in view and its request count, for the view.
    fn reveal_target(&self) -> Option<(usize, u64)> {
        let (id, request) = self.reveal?;
        Some((self.index(id)?, request))
    }

    /// Applies an event and returns what the app may need to act on.
    pub fn update(&mut self, event: Event<T>) -> Option<Output<T>> {
        match event {
            Event::Ready(sender) => return Some(Output::Ready(sender)),
            Event::Received(items) => self.extend(items),
            Event::Press(id, modifiers) => return self.press(id, modifiers),
            Event::Activate(id) => {
                let index = self.index(id)?;
                self.highlighted = Some(id);
                return Some(Output::Activated(index));
            }
            Event::Next => self.step(Step::Next),
            Event::Previous => self.step(Step::Previous),
            Event::First => self.step(Step::First),
            Event::Last => self.step(Step::Last),
            Event::NextPage => self.step(Step::NextPage),
            Event::PreviousPage => self.step(Step::PreviousPage),
            Event::ExtendNext => return self.extend_selection(Step::Next),
            Event::ExtendPrevious => return self.extend_selection(Step::Previous),
            Event::ToggleHighlighted => return self.toggle_highlighted(),
            Event::ActivateHighlighted => return self.highlighted().map(Output::Activated),
            Event::SelectAll => return self.select_all(),
            Event::Follow(follow) => self.following = follow,
            Event::Page(rows) => self.page = rows.max(1),
            Event::Focus(focused) => {
                self.focused = focused;
                // Focus lands on the first selected item, or the item at
                // the end being followed, or the first. It does not scroll.
                if focused && self.highlighted().is_none() && !self.items.is_empty() {
                    let last = self.id(self.items.len() - 1);
                    let start = if self.is_following() {
                        last
                    } else {
                        self.dropped
                    };
                    self.highlighted = self.selected.first().copied().or(Some(start));
                }
            }
        }
        None
    }

    /// Turns a key press into an event through `keymap`.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event<T>> {
        keymap.resolve_event(key)?.event(self)
    }

    /// Drops the oldest items over the limit, and forgets them.
    fn trim(&mut self) {
        let Some(limit) = self.limit else {
            return;
        };
        let over = self.items.len().saturating_sub(limit);
        if over == 0 {
            return;
        }
        self.items.drain(..over);
        self.dropped += over as u64;
        let first = self.dropped;
        self.selected = self.selected.split_off(&first);
        let gone = |id: &RowId| *id < first;
        if self.highlighted.as_ref().is_some_and(gone) {
            self.highlighted = None;
        }
        if self.anchor.as_ref().is_some_and(gone) {
            self.anchor = None;
        }
        if self.reveal.as_ref().is_some_and(|(id, _)| gone(id)) {
            self.reveal = None;
        }
    }

    /// Moves the highlight and scrolls it into view. In a following list,
    /// landing on the last item follows the end, and anywhere else stops.
    fn step(&mut self, step: Step) {
        let Some(index) = target(step, self.highlighted(), self.items.len(), self.page) else {
            return;
        };
        let id = self.id(index);
        self.highlighted = Some(id);
        self.requests += 1;
        self.reveal = Some((id, self.requests));
        self.following = index + 1 == self.items.len();
    }

    fn selection_output(&self, before: &BTreeSet<RowId>) -> Option<Output<T>> {
        (self.selected != *before).then(|| Output::Selected(self.selected()))
    }

    fn press(&mut self, id: RowId, modifiers: Modifiers) -> Option<Output<T>> {
        let index = self.index(id)?;
        self.highlighted = Some(id);
        if index + 1 != self.items.len() {
            self.following = false;
        }
        match self.mode {
            Mode::None => None,
            Mode::Single => {
                let before = std::mem::replace(&mut self.selected, BTreeSet::from([id]));
                self.anchor = Some(id);
                self.selection_output(&before)
            }
            Mode::Multiple => {
                let before = self.selected.clone();
                match (modifiers.shift(), modifiers.command(), self.anchor) {
                    (true, additive, Some(anchor)) => {
                        if !additive {
                            self.selected.clear();
                        }
                        self.selected.extend(span(anchor, id));
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
                self.selection_output(&before)
            }
        }
    }

    fn extend_selection(&mut self, step: Step) -> Option<Output<T>> {
        if self.mode != Mode::Multiple {
            self.step(step);
            return None;
        }
        let anchor = self.anchor.or(self.highlighted);
        self.step(step);
        let (Some(anchor), Some(highlighted)) = (anchor, self.highlighted) else {
            return None;
        };
        self.anchor = Some(anchor);
        let before = self.selected.clone();
        self.selected = span(anchor, highlighted).collect();
        self.selection_output(&before)
    }

    fn toggle_highlighted(&mut self) -> Option<Output<T>> {
        let id = self.highlighted?;
        match self.mode {
            Mode::None => None,
            Mode::Single => self.press(id, Modifiers::empty()),
            Mode::Multiple => self.press(id, Modifiers::COMMAND),
        }
    }

    fn select_all(&mut self) -> Option<Output<T>> {
        if self.mode != Mode::Multiple || self.items.is_empty() {
            return None;
        }
        let before = self.selected.clone();
        self.selected = (self.dropped..self.id(self.items.len())).collect();
        self.selection_output(&before)
    }
}

/// The ids from `a` to `b`, in either order.
fn span(a: RowId, b: RowId) -> impl Iterator<Item = RowId> {
    a.min(b)..=a.max(b)
}

/// A move of the keyboard highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Next,
    Previous,
    First,
    Last,
    NextPage,
    PreviousPage,
}

/// The index a step from `current` lands on in a list of `len` rows, or
/// `None` when the list is empty. Down from nothing starts at the top, and
/// up from nothing at the bottom.
pub(crate) fn target(step: Step, current: Option<usize>, len: usize, page: usize) -> Option<usize> {
    let last = len.checked_sub(1)?;
    let index = match (step, current) {
        (Step::First, _) => 0,
        (Step::Last, _) => last,
        (Step::Next, None) => 0,
        (Step::Previous | Step::PreviousPage | Step::NextPage, None) => last,
        (Step::Next, Some(index)) => (index + 1).min(last),
        (Step::Previous, Some(index)) => index.saturating_sub(1),
        (Step::NextPage, Some(index)) => index.saturating_add(page).min(last),
        (Step::PreviousPage, Some(index)) => index.saturating_sub(page),
    };
    Some(index)
}

/// What a list keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Last,
    NextPage,
    PreviousPage,
    ExtendNext,
    ExtendPrevious,
    Activate,
    Select,
    SelectAll,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing, so the key is left for others.
    pub fn event<T>(self, state: &State<T>) -> Option<Event<T>> {
        if state.is_empty() {
            return None;
        }
        let highlighted = state.highlighted().is_some();
        let multiple = state.mode() == Mode::Multiple;
        match self {
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::First => Some(Event::First),
            Action::Last => Some(Event::Last),
            Action::NextPage => Some(Event::NextPage),
            Action::PreviousPage => Some(Event::PreviousPage),
            Action::ExtendNext if multiple => Some(Event::ExtendNext),
            Action::ExtendNext => Some(Event::Next),
            Action::ExtendPrevious if multiple => Some(Event::ExtendPrevious),
            Action::ExtendPrevious => Some(Event::Previous),
            Action::Activate => highlighted.then_some(Event::ActivateHighlighted),
            Action::Select => {
                (highlighted && state.mode() != Mode::None).then_some(Event::ToggleHighlighted)
            }
            Action::SelectAll => multiple.then_some(Event::SelectAll),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Next,
        Action::Previous,
        Action::First,
        Action::Last,
        Action::NextPage,
        Action::PreviousPage,
        Action::ExtendNext,
        Action::ExtendPrevious,
        Action::Activate,
        Action::Select,
        Action::SelectAll,
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
            Action::NextPage => "NextPage",
            Action::PreviousPage => "PreviousPage",
            Action::ExtendNext => "ExtendNext",
            Action::ExtendPrevious => "ExtendPrevious",
            Action::Activate => "Activate",
            Action::Select => "Select",
            Action::SelectAll => "SelectAll",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next item.",
            Action::Previous => "Highlights the previous item.",
            Action::First => "Highlights the first item.",
            Action::Last => "Highlights the last item, and follows the end in a list that follows.",
            Action::NextPage => "Moves the highlight down by a page of rows.",
            Action::PreviousPage => "Moves the highlight up by a page of rows.",
            Action::ExtendNext => "Moves down and extends the selection, in a multiple selection.",
            Action::ExtendPrevious => {
                "Moves up and extends the selection, in a multiple selection."
            }
            Action::Activate => "Activates the highlighted item.",
            Action::Select => "Selects or toggles the highlighted item.",
            Action::SelectAll => "Selects every item, in a multiple selection.",
        }
    }
}

/// The default list shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
/// | `PageDown` | [`Action::NextPage`] |
/// | `PageUp` | [`Action::PreviousPage`] |
/// | `Shift+ArrowDown` | [`Action::ExtendNext`] |
/// | `Shift+ArrowUp` | [`Action::ExtendPrevious`] |
/// | `Enter` | [`Action::Activate`] |
/// | `Space` | [`Action::Select`] |
/// | `Mod+A` (Ctrl, or Cmd on macOS) | [`Action::SelectAll`] |
///
/// The list resolves these itself, only while it has focus, so they never
/// clash with other components. Pass a changed keymap with
/// [`VirtualList::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
        .bind(Chord::named(Named::PageDown), Action::NextPage)
        .bind(Chord::named(Named::PageUp), Action::PreviousPage)
        .bind(Chord::named(Named::ArrowDown).shift(), Action::ExtendNext)
        .bind(Chord::named(Named::ArrowUp).shift(), Action::ExtendPrevious)
        .bind(Chord::named(Named::Enter), Action::Activate)
        .bind(Chord::named(Named::Space), Action::Select)
        .bind(Chord::character('a').command(), Action::SelectAll)
}

/// Owns the channel items arrive on. Emits [`Event::Ready`] first, then
/// batches of up to [`BATCH_SIZE`] items.
pub fn subscription<T: Send + 'static>() -> Subscription<Event<T>> {
    Subscription::run(stream::<T>)
}

/// The stream behind [`subscription`], for driving it without a runtime.
pub fn stream<T: Send + 'static>() -> impl Stream<Item = Event<T>> {
    let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
    stream::once(async move { Event::Ready(sender) })
        .chain(receiver.ready_chunks(BATCH_SIZE).map(Event::Received))
}

/// How a row is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Status {
    pub selected: bool,
    pub hovered: bool,
    /// Carries the keyboard highlight while the list has focus.
    pub highlighted: bool,
    pub disabled: bool,
}

/// The colours of a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStyle {
    pub background: Option<Color>,
    /// The ring around the row carrying the keyboard highlight.
    pub ring: Option<Color>,
}

/// Resolves a row's colours as a tree's rows do: the accent behind a
/// selected row, a lighter tint under the pointer and a ring around the
/// keyboard highlight. Disabled rows are plain.
pub fn row_style(tokens: &Tokens, status: Status) -> RowStyle {
    if status.disabled {
        return RowStyle {
            background: None,
            ring: None,
        };
    }
    let background = if status.selected {
        Some(tokens.accent)
    } else if status.hovered {
        Some(fade(tokens.accent, 0.6))
    } else {
        None
    };
    RowStyle {
        background,
        ring: status.highlighted.then_some(tokens.ring),
    }
}

/// The frame around the rows: a subtle border with rounded corners.
pub fn frame_style(tokens: &Tokens) -> container::Style {
    container::Style {
        border: Border {
            color: tokens.border,
            width: 1.0,
            radius: radius::MD.into(),
        },
        ..container::Style::default()
    }
}

type OnEvent<'a, T, Message> = dyn Fn(Event<T>) -> Message + 'a;
type RowView<'a, T, Message> = dyn Fn(usize, &'a T) -> Element<'a, Message> + 'a;
type BuildMenu<'a, Message> =
    dyn FnOnce(Keymap<context_menu::Action>) -> RowMenu<'a, RowId, Message> + 'a;

/// A virtual list builder. Convert it into an [`Element`] to render.
pub struct VirtualList<'a, T, Message> {
    state: &'a State<T>,
    row: Rc<RowView<'a, T, Message>>,
    row_height: f32,
    width: Length,
    height: Length,
    keymap: Keymap<Action>,
    menu: Option<Box<BuildMenu<'a, Message>>>,
    menu_keymap: Keymap<context_menu::Action>,
    on_event: Option<Box<OnEvent<'a, T, Message>>>,
}

/// Renders `state`, drawing each row with `row`, which gets the item's
/// index and the item. Without [`on_event`](VirtualList::on_event) the
/// rows render disabled.
pub fn virtual_list<'a, T, Message>(
    state: &'a State<T>,
    row: impl Fn(usize, &'a T) -> Element<'a, Message> + 'a,
) -> VirtualList<'a, T, Message> {
    VirtualList {
        state,
        row: Rc::new(row),
        row_height: ROW_HEIGHT,
        width: Length::Fill,
        height: Length::Shrink,
        keymap: default_keymap(),
        menu: None,
        menu_keymap: context_menu::default_keymap(),
        on_event: None,
    }
}

impl<T, Message> fmt::Debug for VirtualList<'_, T, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VirtualList")
            .field("len", &self.state.len())
            .field("row_height", &self.row_height)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

impl<'a, T, Message> VirtualList<'a, T, Message> {
    /// The height of every row, [`ROW_HEIGHT`] by default.
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height.max(1.0);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// A fixed or filling height scrolls the rows. Defaults to shrinking to
    /// fit them, which builds every row, so give a long list a height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Replaces the [`default_keymap`] the focused list resolves keys with.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<T>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// Gives every row the context menu of `menu`, keyed by the row's
    /// [`RowId`]; turn it into an index with [`State::index`]. It opens on a
    /// right-click or a long press, and on the highlighted row with its
    /// opening chord while the list has focus.
    pub fn context_menu<MenuId>(
        mut self,
        menu: &'a context_menu::State<MenuId, RowId>,
        on_event: impl Fn(context_menu::Event<MenuId, RowId>) -> Message + 'a,
    ) -> Self
    where
        MenuId: Copy + PartialEq + 'a,
        Message: Clone + 'a,
    {
        let on_event: Rc<dyn Fn(context_menu::Event<MenuId, RowId>) -> Message + 'a> =
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

impl<'a, T, Message> From<VirtualList<'a, T, Message>> for Element<'a, Message>
where
    T: 'a,
    Message: Clone + 'a,
{
    fn from(list: VirtualList<'a, T, Message>) -> Self {
        let VirtualList {
            state,
            row,
            row_height,
            width,
            height,
            keymap,
            menu,
            menu_keymap,
            on_event,
        } = list;
        let on_event: Option<Rc<OnEvent<'a, T, Message>>> = on_event.map(Rc::from);
        let menu = menu
            .filter(|_| on_event.is_some())
            .map(|build| Rc::new(build(menu_keymap)));

        let build = {
            let on_event = on_event.clone();
            let menu = menu.clone();
            move |range: Range<usize>| -> Element<'a, Message> {
                column(range.filter_map(|index| {
                    let item = state.get(index)?;
                    let content = row(index, item);
                    let framed = row_frame(state, index, content, row_height, on_event.as_ref());
                    Some(match &menu {
                        Some(menu) => menu.wrap(state.id(index), framed),
                        None => framed,
                    })
                }))
                .into()
            }
        };
        let inner = if height == Length::Shrink {
            Length::Shrink
        } else {
            Length::Fill
        };
        let mut list = rows(state.len(), row_height, build)
            .reveal(state.reveal_target())
            .follow(state.is_following())
            .origin(state.dropped())
            .height(inner);
        if let Some(on_event) = on_event.clone() {
            let on_page = on_event.clone();
            list = list.on_page(move |rows| on_page(Event::Page(rows)));
            if state.tail {
                let on_follow = on_event.clone();
                list = list.on_follow(move |follow| on_follow(Event::Follow(follow)));
            }
        }
        let framed = container(list)
            .padding(FRAME_PADDING)
            .width(width)
            .height(height)
            .style(|theme| frame_style(&Tokens::of(theme)));

        let Some(on_event) = on_event else {
            return framed.into();
        };
        let on_focus = on_event.clone();
        scope(framed)
            .on_focus(move |focused| on_focus(Event::Focus(focused)))
            .on_key(move |key| {
                let target = state.highlighted().map(|index| state.id(index));
                let opened = menu
                    .as_ref()
                    .zip(target)
                    .and_then(|(menu, target)| menu.key(key, target));
                opened.or_else(|| state.key_event(&keymap, key).map(&*on_event))
            })
            .into()
    }
}

/// Wraps a row's content so it paints its status and reports presses.
fn row_frame<'a, T, Message>(
    state: &'a State<T>,
    index: usize,
    content: Element<'a, Message>,
    row_height: f32,
    on_event: Option<&Rc<OnEvent<'a, T, Message>>>,
) -> Element<'a, Message>
where
    T: 'a,
    Message: Clone + 'a,
{
    let id = state.id(index);
    let status = Status {
        selected: state.mode != Mode::None && state.selected.contains(&id),
        hovered: false,
        highlighted: state.focused && state.highlighted == Some(id),
        disabled: on_event.is_none(),
    };
    let body = container(content)
        .padding(Padding::ZERO.left(space::SM).right(space::SM))
        .width(Length::Fill)
        .height(row_height)
        .align_y(Alignment::Center);
    let row = pressable(body, move |theme, hovered| {
        let style = row_style(&Tokens::of(theme), Status { hovered, ..status });
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
    let activate = on_event(Event::Activate(id));
    row.on_press(move |modifiers| on_event(Event::Press(id, modifiers)))
        .on_double_click(Some(activate))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::Action as _;
    use crate::theme::{dark, light};
    use iced::futures::executor::block_on;
    use iced::futures::{SinkExt, StreamExt};
    use iced::keyboard::Key;

    fn numbers(count: usize) -> State<usize> {
        State::new(0..count)
    }

    fn selected(output: Option<Output<usize>>) -> Option<Vec<usize>> {
        match output {
            Some(Output::Selected(indices)) => Some(indices),
            _ => None,
        }
    }

    fn press(key: Key, modifiers: Modifiers) -> keys::Event {
        keys::Event { key, modifiers }
    }

    #[test]
    fn a_new_list_holds_its_items_and_selects_one_at_a_time() {
        let state = numbers(5);
        assert_eq!(state.len(), 5);
        assert!(!state.is_empty());
        assert_eq!(state.mode(), Mode::Single);
        assert_eq!(state.get(2), Some(&2));
        assert_eq!(state.get(9), None);
        assert_eq!(state.highlighted(), None);
        assert!(!state.is_following());
        assert_eq!(state.page_size(), DEFAULT_PAGE);
        assert_eq!(state.limit(), None);
        assert!(State::<u8>::default().is_empty());
        assert_eq!(Mode::ALL.len(), 3);
    }

    #[test]
    fn ids_equal_indices_until_items_are_dropped() {
        let mut state = numbers(3).with_limit(3);
        assert_eq!(state.id(1), 1);
        state.extend([3, 4]);
        assert_eq!(state.dropped(), 2);
        assert_eq!(state.id(0), 2);
        assert_eq!(state.index(2), Some(0));
        assert_eq!(state.index(1), None, "dropped");
        assert_eq!(state.index(9), None, "not yet arrived");
        assert_eq!(state.iter().copied().collect::<Vec<_>>(), [2, 3, 4]);
    }

    #[test]
    fn a_press_selects_one_item_in_single_mode() {
        let mut state = numbers(5);
        let output = state.update(Event::Press(2, Modifiers::empty()));
        assert_eq!(selected(output), Some(vec![2]));
        assert_eq!(state.highlighted(), Some(2));
        let again = state.update(Event::Press(2, Modifiers::empty()));
        assert!(again.is_none(), "no change, no output");
        assert_eq!(
            selected(state.update(Event::Press(4, Modifiers::empty()))),
            Some(vec![4])
        );
        assert!(state.update(Event::Press(99, Modifiers::empty())).is_none());
    }

    #[test]
    fn modifier_presses_toggle_and_extend_in_multiple_mode() {
        let mut state = numbers(10).with_mode(Mode::Multiple);
        let _ = state.update(Event::Press(2, Modifiers::empty()));
        let output = state.update(Event::Press(5, Modifiers::SHIFT));
        assert_eq!(selected(output), Some(vec![2, 3, 4, 5]));
        let output = state.update(Event::Press(8, Modifiers::COMMAND));
        assert_eq!(selected(output), Some(vec![2, 3, 4, 5, 8]));
        let output = state.update(Event::Press(3, Modifiers::COMMAND));
        assert_eq!(selected(output), Some(vec![2, 4, 5, 8]));
        let output = state.update(Event::Press(1, Modifiers::SHIFT | Modifiers::COMMAND));
        assert_eq!(
            selected(output),
            Some(vec![1, 2, 3, 4, 5, 8]),
            "added range"
        );
        let output = state.update(Event::Press(6, Modifiers::empty()));
        assert_eq!(selected(output), Some(vec![6]));
    }

    #[test]
    fn mode_none_highlights_without_selecting() {
        let mut state = numbers(5).with_mode(Mode::None);
        assert!(state.update(Event::Press(2, Modifiers::empty())).is_none());
        assert_eq!(state.highlighted(), Some(2));
        assert!(state.selected().is_empty());
        assert!(Action::Select.event(&state).is_none());
    }

    #[test]
    fn steps_move_the_highlight_within_bounds() {
        let mut state = numbers(30);
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(0), "starts at the top");
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(0), "stays at the top");
        let _ = state.update(Event::Page(8));
        let _ = state.update(Event::NextPage);
        assert_eq!(state.highlighted(), Some(8));
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(29));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(29), "stays at the end");
        let _ = state.update(Event::NextPage);
        assert_eq!(state.highlighted(), Some(29));
        let _ = state.update(Event::PreviousPage);
        assert_eq!(state.highlighted(), Some(21));
        let _ = state.update(Event::First);
        assert_eq!(state.highlighted(), Some(0));
        let _ = state.update(Event::PreviousPage);
        assert_eq!(state.highlighted(), Some(0));
    }

    #[test]
    fn up_from_nothing_starts_at_the_end() {
        let mut state = numbers(5);
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(4));
        let mut empty = State::<u8>::default();
        let _ = empty.update(Event::Next);
        assert_eq!(empty.highlighted(), None);
    }

    #[test]
    fn a_page_is_at_least_one_row() {
        let mut state = numbers(5);
        let _ = state.update(Event::Page(0));
        assert_eq!(state.page_size(), 1);
    }

    #[test]
    fn steps_reveal_the_highlight_but_focus_does_not_scroll() {
        let mut state = numbers(100);
        let _ = state.update(Event::Focus(true));
        assert_eq!(state.highlighted(), Some(0));
        assert_eq!(state.reveal_target(), None, "focus never scrolls");
        let _ = state.update(Event::Last);
        assert_eq!(state.reveal_target(), Some((99, 1)));
        let _ = state.update(Event::Last);
        assert_eq!(
            state.reveal_target(),
            Some((99, 2)),
            "the same row is revealed again"
        );
    }

    #[test]
    fn focus_starts_at_the_selection() {
        let mut state = numbers(10);
        let _ = state.update(Event::Press(6, Modifiers::empty()));
        state.highlighted = None;
        let _ = state.update(Event::Focus(true));
        assert!(state.is_focused());
        assert_eq!(state.highlighted(), Some(6));
        let _ = state.update(Event::Focus(false));
        assert!(!state.is_focused());
    }

    #[test]
    fn scroll_to_reveals_without_highlighting() {
        let mut state = numbers(100).with_follow(true);
        state.scroll_to(40);
        assert_eq!(state.reveal_target(), Some((40, 1)));
        assert_eq!(state.highlighted(), None);
        assert!(!state.is_following());
        state.scroll_to(99);
        assert!(state.is_following(), "the last row is the end");
        state.scroll_to(500);
        assert_eq!(
            state.reveal_target(),
            Some((99, 2)),
            "out of range is ignored"
        );
    }

    #[test]
    fn extending_selects_from_the_anchor() {
        let mut state = numbers(10).with_mode(Mode::Multiple);
        let _ = state.update(Event::Press(3, Modifiers::empty()));
        let _ = state.update(Event::ExtendNext);
        let output = state.update(Event::ExtendNext);
        assert_eq!(selected(output), Some(vec![3, 4, 5]));
        let output = state.update(Event::ExtendPrevious);
        assert_eq!(selected(output), Some(vec![3, 4]));

        let mut single = numbers(10);
        assert!(single.update(Event::ExtendNext).is_none());
        assert_eq!(single.highlighted(), Some(0), "moves without selecting");
    }

    #[test]
    fn select_all_and_toggle_follow_the_mode() {
        let mut state = numbers(4).with_mode(Mode::Multiple);
        assert_eq!(
            selected(state.update(Event::SelectAll)),
            Some(vec![0, 1, 2, 3])
        );
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(3));
        assert_eq!(
            selected(state.update(Event::ToggleHighlighted)),
            Some(vec![0, 1, 2])
        );

        let mut single = numbers(4);
        assert!(single.update(Event::SelectAll).is_none());
        let _ = single.update(Event::Next);
        assert_eq!(
            selected(single.update(Event::ToggleHighlighted)),
            Some(vec![0])
        );
    }

    #[test]
    fn activating_reports_the_index() {
        let mut state = numbers(4);
        assert!(matches!(
            state.update(Event::Activate(2)),
            Some(Output::Activated(2))
        ));
        assert_eq!(state.highlighted(), Some(2));
        assert!(matches!(
            state.update(Event::ActivateHighlighted),
            Some(Output::Activated(2))
        ));
        assert!(state.update(Event::Activate(40)).is_none());
    }

    #[test]
    fn the_limit_drops_the_oldest_items_and_their_selection() {
        let mut state = numbers(5).with_mode(Mode::Multiple).with_limit(5);
        let _ = state.update(Event::Press(1, Modifiers::empty()));
        let _ = state.update(Event::Press(3, Modifiers::COMMAND));
        state.scroll_to(0);
        let _ = state.update(Event::Received(vec![5, 6]));
        assert_eq!(state.len(), 5);
        assert_eq!(state.get(0), Some(&2));
        assert_eq!(state.selected(), [1], "item 3 is now at index 1");
        assert_eq!(state.highlighted(), Some(1));
        assert_eq!(state.reveal_target(), None, "the revealed row is gone");
        let _ = state.update(Event::Received(vec![7, 8]));
        assert!(state.selected().is_empty());
        assert_eq!(state.highlighted(), None);
        assert!(state.anchor.is_none());
        assert_eq!(numbers(5).with_limit(0).len(), 1, "a zero limit keeps one");
    }

    #[test]
    fn set_items_and_clear_reset_the_selection() {
        let mut state = numbers(5);
        let _ = state.update(Event::Press(2, Modifiers::empty()));
        state.set_items([10, 11]);
        assert_eq!(state.len(), 2);
        assert!(state.selected().is_empty());
        assert_eq!(state.highlighted(), None);
        assert_eq!(state.id(0), 5, "ids carry on");
        assert!(
            state.update(Event::Press(2, Modifiers::empty())).is_none(),
            "a press from the old items is ignored"
        );
        state.push(12);
        assert_eq!(state.len(), 3);
        state.clear();
        assert!(state.is_empty());
    }

    #[test]
    fn following_stops_off_the_end_and_resumes_at_it() {
        let mut state = numbers(50).with_follow(true);
        assert!(state.is_following());
        let _ = state.update(Event::Follow(false));
        assert!(!state.is_following());
        let _ = state.update(Event::Follow(true));
        assert!(state.is_following());
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(49));
        assert!(state.is_following(), "the last row is still the end");
        let _ = state.update(Event::Previous);
        assert!(!state.is_following());
        let _ = state.update(Event::Last);
        assert!(state.is_following());
        let _ = state.update(Event::Press(10, Modifiers::empty()));
        assert!(!state.is_following(), "pressing a row off the end stops");
        let _ = state.update(Event::Received(vec![50]));
        assert!(!state.is_following());
    }

    #[test]
    fn a_list_without_follow_never_follows() {
        let mut state = numbers(5);
        let _ = state.update(Event::Last);
        assert!(!state.is_following());
        let _ = state.update(Event::Follow(true));
        assert!(!state.is_following());
    }

    #[test]
    fn focus_on_a_following_list_lands_at_the_end() {
        let mut state = numbers(20).with_follow(true);
        let _ = state.update(Event::Focus(true));
        assert_eq!(state.highlighted(), Some(19));
    }

    #[test]
    fn the_default_keymap_maps_to_events() {
        let mut state = numbers(10).with_mode(Mode::Multiple);
        let keymap = default_keymap();
        let resolve = |state: &State<usize>, key: Key, modifiers: Modifiers| {
            state.key_event(&keymap, &press(key, modifiers))
        };
        let named = |named: Named| Key::Named(named);
        assert!(matches!(
            resolve(&state, named(Named::ArrowDown), Modifiers::empty()),
            Some(Event::Next)
        ));
        assert!(matches!(
            resolve(&state, named(Named::PageUp), Modifiers::empty()),
            Some(Event::PreviousPage)
        ));
        assert!(matches!(
            resolve(&state, named(Named::End), Modifiers::empty()),
            Some(Event::Last)
        ));
        assert!(matches!(
            resolve(&state, named(Named::ArrowDown), Modifiers::SHIFT),
            Some(Event::ExtendNext)
        ));
        assert!(matches!(
            resolve(&state, Key::Character("a".into()), Modifiers::COMMAND),
            Some(Event::SelectAll)
        ));
        assert!(
            resolve(&state, named(Named::Enter), Modifiers::empty()).is_none(),
            "nothing highlighted to activate"
        );
        let _ = state.update(Event::Next);
        assert!(matches!(
            resolve(&state, named(Named::Enter), Modifiers::empty()),
            Some(Event::ActivateHighlighted)
        ));
        assert!(matches!(
            resolve(&state, named(Named::Space), Modifiers::empty()),
            Some(Event::ToggleHighlighted)
        ));
        assert!(
            resolve(
                &State::<usize>::default(),
                named(Named::ArrowDown),
                Modifiers::empty()
            )
            .is_none(),
            "an empty list leaves keys alone"
        );
    }

    #[test]
    fn keymaps_can_be_overridden_and_unbound() {
        let keymap = default_keymap()
            .bind(Chord::character('j'), Action::Next)
            .unbind(&Chord::named(Named::ArrowDown));
        let state = numbers(3);
        let j = press(Key::Character("j".into()), Modifiers::empty());
        assert!(matches!(state.key_event(&keymap, &j), Some(Event::Next)));
        let down = press(Key::Named(Named::ArrowDown), Modifiers::empty());
        assert!(state.key_event(&keymap, &down).is_none());
    }

    #[test]
    fn extend_actions_fall_back_to_moving_in_single_mode() {
        let state = numbers(3);
        assert!(matches!(
            Action::ExtendNext.event(&state),
            Some(Event::Next)
        ));
        assert!(matches!(
            Action::ExtendPrevious.event(&state),
            Some(Event::Previous)
        ));
        assert!(Action::SelectAll.event(&state).is_none());
    }

    #[test]
    fn every_action_is_named_and_described() {
        for action in Action::ALL {
            assert!(!action.name().is_empty());
            assert!(action.description().ends_with('.'));
            assert!(!default_keymap().chords(action).is_empty(), "{action:?}");
        }
    }

    #[test]
    fn row_styles_cover_every_status_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = row_style(&tokens, Status::default());
            assert_eq!(idle.background, None);
            assert_eq!(idle.ring, None);
            let selected = row_style(
                &tokens,
                Status {
                    selected: true,
                    ..Status::default()
                },
            );
            assert_eq!(selected.background, Some(tokens.accent));
            let hovered = row_style(
                &tokens,
                Status {
                    hovered: true,
                    ..Status::default()
                },
            );
            assert_eq!(hovered.background, Some(fade(tokens.accent, 0.6)));
            let highlighted = row_style(
                &tokens,
                Status {
                    highlighted: true,
                    ..Status::default()
                },
            );
            assert_eq!(highlighted.ring, Some(tokens.ring));
            let disabled = row_style(
                &tokens,
                Status {
                    disabled: true,
                    hovered: true,
                    highlighted: true,
                    selected: true,
                },
            );
            assert_eq!(disabled.background, None);
            assert_eq!(disabled.ring, None);
            #[cfg(feature = "tree")]
            {
                use crate::data::tree;
                let selected_node = tree::Status {
                    selected: true,
                    ..tree::Status::default()
                };
                let node = tree::row_style(&tokens, selected_node);
                assert_eq!(selected.background, node.background, "rows match a tree's");
            }
            assert_eq!(frame_style(&tokens).border.color, tokens.border);
        }
    }

    #[test]
    fn the_builder_has_sensible_defaults() {
        let state = numbers(3);
        let list: VirtualList<'_, usize, ()> =
            virtual_list(&state, |_, _| iced::widget::text("").into());
        assert!(!list.is_enabled());
        assert_eq!(list.row_height, ROW_HEIGHT);
        assert_eq!(list.height, Length::Shrink);
        assert!(list.menu.is_none());
        assert_eq!(list.menu_keymap, context_menu::default_keymap());
        let menu: context_menu::State<u8, RowId> = context_menu::State::new([]);
        let list = list
            .row_height(0.0)
            .height(200)
            .context_menu(&menu, |_| ())
            .context_menu_keymap(Keymap::new())
            .on_event(|_| ());
        assert!(list.menu.is_some());
        assert!(list.menu_keymap.is_empty());
        assert!(list.is_enabled());
        assert_eq!(list.row_height, 1.0);
        assert_eq!(list.height, Length::Fixed(200.0));
        assert!(format!("{list:?}").contains("VirtualList"));
    }

    #[test]
    fn stream_emits_ready_first_then_batches() {
        let mut events = Box::pin(stream::<u32>());
        let Some(Event::Ready(mut sender)) = block_on(events.next()) else {
            panic!("first event must be Ready");
        };
        block_on(async {
            for item in 0..3 {
                sender.send(item).await.expect("channel open");
            }
        });
        let Some(Event::Received(batch)) = block_on(events.next()) else {
            panic!("then a batch");
        };
        assert_eq!(batch, [0, 1, 2]);
    }

    #[test]
    fn bursts_arrive_in_batches_of_the_batch_size() {
        let items = stream::iter(0..BATCH_SIZE + 3);
        let sizes: Vec<usize> = block_on(
            items
                .ready_chunks(BATCH_SIZE)
                .map(|batch| batch.len())
                .collect(),
        );
        assert_eq!(sizes, [BATCH_SIZE, 3]);
    }

    #[test]
    fn ready_passes_the_sender_on() {
        let (sender, _receiver) = mpsc::channel(1);
        let output = State::<u8>::default().update(Event::Ready(sender));
        assert!(matches!(output, Some(Output::Ready(_))));
    }
}
