//! A strip of open documents above an editor, one tab per document.
//!
//! [`State`] owns the documents in order, the active one and the order they
//! were last used in. Its [`update`](State::update) activates, reorders and
//! pins tabs, and turns close gestures into [`Output::CloseRequested`] so
//! the app decides what closing means, such as asking to save a modified
//! document first, before it calls [`State::close`].
//!
//! Each tab shows an optional icon, a label that ends in an ellipsis past
//! its maximum width, and a slot that holds the close button on the active
//! and hovered tab, or a dot while the document has unsaved changes. The
//! slot is always reserved, so labels do not move when it changes. Pinned
//! tabs sit first, compact and without a close button.
//!
//! With the pointer, a press activates a tab, dragging moves it (a line
//! marks where it will land), a middle click asks to close it and a right
//! click opens its context menu. The wheel scrolls tabs that do not fit,
//! and a button at the end of the strip lists every tab. The strip keeps
//! the active tab in view.
//!
//! Keyboard shortcuts resolve through a [`Keymap`] of [`Action`]s; see
//! [`default_keymap`] and [`crate::keys`].

use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse::{self, click};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::operation::scrollable::{AbsoluteOffset, RelativeOffset, Scrollable};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer as _, Shell};
use iced::alignment::Vertical;
use iced::border::Radius;
use iced::keyboard::{self, key::Named};
use iced::touch::{self, Finger};
use iced::widget::{self, button::Status, container, row};
use iced::{
    Alignment, Background, Border, Color, Element, Event as Input, Length, Padding, Point,
    Rectangle, Renderer, Size, Subscription, Theme, Vector,
};

use crate::data::ellipsis::ellipsis;
use crate::icon::{Glyph, opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{Align, Placement, Side, anchored};
use crate::overlay::context_menu::{self, KEYBOARD_GAP};
use crate::overlay::dropdown_menu::{self, Entry, item, radio_item, separator};
use crate::overlay::menu::{self, RowStatus};
use crate::overlay::scrolling::{fitted, scrolling};
use crate::overlay::tooltip;
use crate::primitives::button::Size as ButtonSize;
use crate::primitives::icon_button::icon_button;
use crate::theme::{Tokens, fade, mix, radius, space, text_size};

/// Height of the strip and its tabs.
pub const HEIGHT: f32 = 36.0;

/// Default widest a label grows before it ends in an ellipsis.
pub const MAX_LABEL_WIDTH: f32 = 200.0;

/// Widest label of a pinned tab without an icon.
pub const PINNED_LABEL_WIDTH: f32 = 80.0;

/// Widest the list of every tab grows before its labels end in an ellipsis.
pub const OVERFLOW_WIDTH: f32 = 320.0;

/// How far, in logical pixels, a pressed tab moves before it drags.
pub const DRAG_THRESHOLD: f32 = 4.0;

/// Side of the square slot for the close button or the modified dot.
const SLOT: f32 = 16.0;
const CLOSE_ICON: f32 = 12.0;
const ICON: f32 = 16.0;
const DOT: f32 = 8.0;
const PADDING_X: f32 = 12.0;
const DIVIDER_HEIGHT: f32 = 16.0;
const INDICATOR_WIDTH: f32 = 2.0;
/// Pixels one wheel line scrolls the strip.
const WHEEL_LINE: f32 = 40.0;

/// One open document.
#[derive(Debug, Clone, PartialEq)]
pub struct Document<Id> {
    pub id: Id,
    pub label: String,
    pub icon: Option<Glyph>,
    /// Unsaved changes, shown as a dot in the tab's slot.
    pub modified: bool,
    pub pinned: bool,
}

/// Creates a saved, unpinned document.
pub fn document<Id>(id: Id, label: impl Into<String>) -> Document<Id> {
    Document {
        id,
        label: label.into(),
        icon: None,
        modified: false,
        pinned: false,
    }
}

impl<Id> Document<Id> {
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub fn modified(mut self, modified: bool) -> Self {
        self.modified = modified;
        self
    }

    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }
}

/// An item in a tab's context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuItem {
    Close,
    CloseOthers,
    CloseToTheRight,
    CloseSaved,
    CloseAll,
    Pin,
    Unpin,
    /// An item the app added with [`State::with_menu_items`]. Choosing it
    /// returns [`Output::Custom`] with the tab it was chosen on.
    Custom(&'static str),
}

/// Changes to the documents.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event<Id> {
    /// Makes a document the active one.
    Activate(Id),
    /// Activates the next document, wrapping at the end.
    Next,
    /// Activates the previous document, wrapping at the start.
    Previous,
    /// Activates the next document in most-recently-used order. Repeated
    /// presses walk further down the list until [`Event::EndCycle`].
    RecentNext,
    /// Walks the most-recently-used list the other way.
    RecentPrevious,
    /// Ends a walk through the most-recently-used list, making the document
    /// it stopped on the most recent. Any other event ends it too.
    EndCycle,
    /// Asks to close one document.
    Close(Id),
    /// Asks to close every unpinned document except this one.
    CloseOthers(Id),
    /// Asks to close the unpinned documents after this one.
    CloseToTheRight(Id),
    /// Asks to close every unpinned document without unsaved changes.
    CloseSaved,
    /// Asks to close every unpinned document.
    CloseAll,
    /// Moves a document to an index. Pinned documents stay before the
    /// others, so the index is clamped to the document's group.
    Move(Id, usize),
    /// Pins or unpins a document. It moves to the end of the pinned tabs
    /// or the start of the others.
    Pin(Id, bool),
    /// Asks for a new document, from the new tab button, a double click on
    /// the empty strip or a shortcut.
    New,
    /// The context menu of a tab.
    Menu(context_menu::Event<MenuItem, Id>),
    /// The menu listing every document.
    Overflow(dropdown_menu::Event<Id>),
}

/// Sent by [`cycle_end`] when every modifier key is released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CycleEnd;

impl<Id> From<CycleEnd> for Event<Id> {
    fn from(_: CycleEnd) -> Self {
        Event::EndCycle
    }
}

/// Reports when the last modifier key is released, which ends a walk
/// through recent documents as editors do when Ctrl is let go.
pub fn cycle_end() -> Subscription<CycleEnd> {
    keyboard::listen().filter_map(|event| match event {
        keyboard::Event::ModifiersChanged(modifiers) if modifiers.is_empty() => Some(CycleEnd),
        _ => None,
    })
}

/// What an [`Event`] changed or asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output<Id> {
    /// A different document became active.
    Activated(Id),
    /// The user asked to close these documents, in strip order. Nothing is
    /// closed yet: the app saves or confirms as it needs to, then calls
    /// [`State::close`].
    CloseRequested(Vec<Id>),
    /// A document moved from one index to another.
    Moved { id: Id, from: usize, to: usize },
    /// A document was pinned or unpinned.
    Pinned(Id, bool),
    /// The user asked for a new document.
    NewTab,
    /// An app menu item was chosen on a tab.
    Custom(&'static str, Id),
}

/// The open documents, the active one and the order they were used in.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    documents: Vec<Document<Id>>,
    active: Option<Id>,
    /// Every document id, most recently used first.
    recent: Vec<Id>,
    /// Where a walk through `recent` has got to.
    cycle: Option<usize>,
    menu: context_menu::State<MenuItem, Id>,
    extra: Vec<Entry<MenuItem>>,
    overflow: dropdown_menu::State<Id>,
}

impl<Id: Copy + PartialEq> State<Id> {
    /// Opens `documents`, pinned ones first, with the first one active.
    /// Ids must be unique.
    pub fn new(documents: impl IntoIterator<Item = Document<Id>>) -> Self {
        let (mut documents, others): (Vec<_>, Vec<_>) =
            documents.into_iter().partition(|document| document.pinned);
        documents.extend(others);
        let recent = documents.iter().map(|document| document.id).collect();
        Self {
            active: documents.first().map(|document| document.id),
            documents,
            recent,
            cycle: None,
            menu: context_menu::State::new([]),
            extra: Vec::new(),
            overflow: dropdown_menu::State::new([]),
        }
    }

    /// Starts on `id`, when it exists.
    pub fn with_active(mut self, id: Id) -> Self {
        let _ = self.activate(id);
        self
    }

    /// Adds items to the end of every tab's context menu, such as "Copy
    /// path". Give them [`MenuItem::Custom`] ids.
    pub fn with_menu_items(mut self, entries: impl IntoIterator<Item = Entry<MenuItem>>) -> Self {
        self.extra = entries.into_iter().collect();
        self
    }

    /// The documents in strip order.
    pub fn documents(&self) -> &[Document<Id>] {
        &self.documents
    }

    pub fn document(&self, id: Id) -> Option<&Document<Id>> {
        self.documents.iter().find(|document| document.id == id)
    }

    pub fn active(&self) -> Option<Id> {
        self.active
    }

    pub fn is_active(&self, id: Id) -> bool {
        self.active == Some(id)
    }

    /// Document ids, most recently used first.
    pub fn recent(&self) -> &[Id] {
        &self.recent
    }

    /// The tab context menu.
    pub fn menu(&self) -> &context_menu::State<MenuItem, Id> {
        &self.menu
    }

    /// The menu listing every document.
    pub fn overflow(&self) -> &dropdown_menu::State<Id> {
        &self.overflow
    }

    /// How many documents are pinned. They are always the first ones.
    pub fn pinned_count(&self) -> usize {
        self.documents
            .iter()
            .take_while(|document| document.pinned)
            .count()
    }

    fn index_of(&self, id: Id) -> Option<usize> {
        self.documents.iter().position(|document| document.id == id)
    }

    fn active_index(&self) -> Option<usize> {
        self.active.and_then(|id| self.index_of(id))
    }

    /// Activates `document`, adding it at the end of its group (pinned or
    /// not) when it is not open yet.
    pub fn open(&mut self, document: Document<Id>) {
        self.end_cycle();
        let id = document.id;
        if self.index_of(id).is_none() {
            let index = if document.pinned {
                self.pinned_count()
            } else {
                self.documents.len()
            };
            self.documents.insert(index, document);
            self.recent.push(id);
        }
        let _ = self.activate(id);
    }

    /// Closes documents. When the active one closes, the most recently used
    /// one left becomes active, and is returned.
    pub fn close(&mut self, ids: &[Id]) -> Option<Id> {
        self.end_cycle();
        self.documents
            .retain(|document| !ids.contains(&document.id));
        self.recent.retain(|id| !ids.contains(id));
        let closed = self.active.is_some_and(|id| ids.contains(&id));
        if !closed {
            return None;
        }
        self.active = self.recent.first().copied();
        self.active
    }

    /// Marks a document as having unsaved changes, or as saved.
    pub fn set_modified(&mut self, id: Id, modified: bool) {
        if let Some(document) = self.document_mut(id) {
            document.modified = modified;
        }
    }

    pub fn set_label(&mut self, id: Id, label: impl Into<String>) {
        if let Some(document) = self.document_mut(id) {
            document.label = label.into();
        }
    }

    fn document_mut(&mut self, id: Id) -> Option<&mut Document<Id>> {
        self.documents.iter_mut().find(|document| document.id == id)
    }

    /// Applies an event and returns what changed, if anything.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        if !matches!(event, Event::RecentNext | Event::RecentPrevious) {
            self.end_cycle();
        }
        match event {
            Event::Activate(id) => self.activate(id),
            Event::Next => self.step(true),
            Event::Previous => self.step(false),
            Event::RecentNext => self.walk(true),
            Event::RecentPrevious => self.walk(false),
            Event::EndCycle => None,
            Event::Close(id) => self.request(|document| document.id == id),
            Event::CloseOthers(id) => {
                self.request(|document| document.id != id && !document.pinned)
            }
            Event::CloseToTheRight(id) => {
                let index = self.index_of(id)?;
                let right: Vec<Id> = self.documents[index + 1..]
                    .iter()
                    .filter(|document| !document.pinned)
                    .map(|document| document.id)
                    .collect();
                (!right.is_empty()).then_some(Output::CloseRequested(right))
            }
            Event::CloseSaved => self.request(|document| !document.modified && !document.pinned),
            Event::CloseAll => self.request(|document| !document.pinned),
            Event::Move(id, to) => self.move_to(id, to),
            Event::Pin(id, pinned) => self.pin(id, pinned),
            Event::New => Some(Output::NewTab),
            Event::Menu(event) => self.menu_event(event),
            Event::Overflow(event) => self.overflow_event(event),
        }
    }

    fn activate(&mut self, id: Id) -> Option<Output<Id>> {
        self.index_of(id)?;
        if self.active == Some(id) {
            return None;
        }
        self.active = Some(id);
        self.touch(id);
        Some(Output::Activated(id))
    }

    /// Moves `id` to the front of the recent list.
    fn touch(&mut self, id: Id) {
        self.recent.retain(|recent| *recent != id);
        self.recent.insert(0, id);
    }

    fn step(&mut self, forward: bool) -> Option<Output<Id>> {
        let len = self.documents.len();
        let current = self.active_index()?;
        let next = if forward {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        let id = self.documents[next].id;
        self.activate(id)
    }

    fn walk(&mut self, forward: bool) -> Option<Output<Id>> {
        let len = self.recent.len();
        if len < 2 {
            return None;
        }
        let current = self.cycle.unwrap_or(0);
        let next = if forward {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        self.cycle = Some(next);
        let id = self.recent[next];
        self.active = Some(id);
        Some(Output::Activated(id))
    }

    fn end_cycle(&mut self) {
        if self.cycle.take().is_none() {
            return;
        }
        if let Some(id) = self.active {
            self.touch(id);
        }
    }

    fn request(&self, close: impl Fn(&Document<Id>) -> bool) -> Option<Output<Id>> {
        let ids: Vec<Id> = self
            .documents
            .iter()
            .filter(|document| close(document))
            .map(|document| document.id)
            .collect();
        (!ids.is_empty()).then_some(Output::CloseRequested(ids))
    }

    fn move_to(&mut self, id: Id, to: usize) -> Option<Output<Id>> {
        let from = self.index_of(id)?;
        let pinned = self.pinned_count();
        let (start, end) = if from < pinned {
            (0, pinned - 1)
        } else {
            (pinned, self.documents.len() - 1)
        };
        let to = to.clamp(start, end);
        if to == from {
            return None;
        }
        let document = self.documents.remove(from);
        self.documents.insert(to, document);
        Some(Output::Moved { id, from, to })
    }

    fn pin(&mut self, id: Id, pinned: bool) -> Option<Output<Id>> {
        let from = self.index_of(id)?;
        if self.documents[from].pinned == pinned {
            return None;
        }
        let mut document = self.documents.remove(from);
        document.pinned = pinned;
        let to = self.pinned_count();
        self.documents.insert(to, document);
        Some(Output::Pinned(id, pinned))
    }

    fn menu_event(&mut self, event: context_menu::Event<MenuItem, Id>) -> Option<Output<Id>> {
        let event = match event {
            // The menu opens below the tab, not at the pointer, so it never
            // covers the tab's label.
            context_menu::Event::Open(target, _) => {
                context_menu::Event::Open(target, Point::new(0.0, HEIGHT + KEYBOARD_GAP))
            }
            event => event,
        };
        if let context_menu::Event::Open(target, _)
        | context_menu::Event::OpenFromKeyboard(target) = event
        {
            self.menu = context_menu::State::new(self.menu_entries(target)?);
        }
        let dropdown_menu::Output::Activated(item) = self.menu.update(event)? else {
            return None;
        };
        let target = *self.menu.target()?;
        let event = match item {
            MenuItem::Close => Event::Close(target),
            MenuItem::CloseOthers => Event::CloseOthers(target),
            MenuItem::CloseToTheRight => Event::CloseToTheRight(target),
            MenuItem::CloseSaved => Event::CloseSaved,
            MenuItem::CloseAll => Event::CloseAll,
            MenuItem::Pin => Event::Pin(target, true),
            MenuItem::Unpin => Event::Pin(target, false),
            MenuItem::Custom(name) => return Some(Output::Custom(name, target)),
        };
        self.update(event)
    }

    /// The context menu of the tab `target`, with the choices that would
    /// do nothing disabled.
    fn menu_entries(&self, target: Id) -> Option<Vec<Entry<MenuItem>>> {
        let index = self.index_of(target)?;
        let unpinned = |document: &&Document<Id>| !document.pinned;
        let others = self
            .documents
            .iter()
            .filter(unpinned)
            .any(|document| document.id != target);
        let right = self.documents[index + 1..].iter().any(|d| !d.pinned);
        let saved = self
            .documents
            .iter()
            .filter(unpinned)
            .any(|document| !document.modified);
        let any = self.documents.iter().any(|document| !document.pinned);
        let pin = if self.documents[index].pinned {
            item(MenuItem::Unpin, "Unpin")
        } else {
            item(MenuItem::Pin, "Pin")
        };

        let mut entries = vec![
            item(MenuItem::Close, "Close"),
            item(MenuItem::CloseOthers, "Close others").disabled(!others),
            item(MenuItem::CloseToTheRight, "Close to the right").disabled(!right),
            item(MenuItem::CloseSaved, "Close saved").disabled(!saved),
            item(MenuItem::CloseAll, "Close all").disabled(!any),
            separator(),
            pin,
        ];
        if !self.extra.is_empty() {
            entries.push(separator());
            entries.extend(self.extra.iter().cloned());
        }
        Some(entries)
    }

    fn overflow_event(&mut self, event: dropdown_menu::Event<Id>) -> Option<Output<Id>> {
        if !self.overflow.is_open() {
            self.overflow = dropdown_menu::State::new(self.documents.iter().map(|document| {
                let entry = radio_item(
                    document.id,
                    document.label.clone(),
                    self.active == Some(document.id),
                );
                if document.modified {
                    entry.shortcut("Modified")
                } else {
                    entry
                }
            }));
        }
        match self.overflow.update(event)? {
            dropdown_menu::Output::Activated(id) | dropdown_menu::Output::Selected(id) => {
                self.activate(id)
            }
            dropdown_menu::Output::Toggled(..) => None,
        }
    }
}

/// What a document tabs keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    RecentNext,
    RecentPrevious,
    Next,
    Previous,
    MoveLeft,
    MoveRight,
    Close,
    New,
    /// Activates the document at this position, counting from 1.
    Tab(u8),
    /// Activates the last document.
    LastTab,
    /// Opens the active tab's context menu.
    OpenMenu,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it would
    /// do nothing, such as moving the first tab left.
    pub fn event<Id: Copy + PartialEq>(self, state: &State<Id>) -> Option<Event<Id>> {
        let active = state.active;
        let index = state.active_index();
        match self {
            Action::RecentNext => Some(Event::RecentNext),
            Action::RecentPrevious => Some(Event::RecentPrevious),
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::MoveLeft => {
                let index = index?.checked_sub(1)?;
                Some(Event::Move(active?, index))
            }
            Action::MoveRight => Some(Event::Move(active?, index? + 1)),
            Action::Close => active.map(Event::Close),
            Action::New => Some(Event::New),
            Action::Tab(position) => {
                let index = usize::from(position).checked_sub(1)?;
                state
                    .documents
                    .get(index)
                    .map(|document| Event::Activate(document.id))
            }
            Action::LastTab => state
                .documents
                .last()
                .map(|document| Event::Activate(document.id)),
            Action::OpenMenu => {
                active.map(|id| Event::Menu(context_menu::Event::OpenFromKeyboard(id)))
            }
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::RecentNext,
        Action::RecentPrevious,
        Action::Next,
        Action::Previous,
        Action::MoveLeft,
        Action::MoveRight,
        Action::Close,
        Action::New,
        Action::Tab(1),
        Action::Tab(2),
        Action::Tab(3),
        Action::Tab(4),
        Action::Tab(5),
        Action::Tab(6),
        Action::Tab(7),
        Action::Tab(8),
        Action::LastTab,
        Action::OpenMenu,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::RecentNext => "RecentNext",
            Action::RecentPrevious => "RecentPrevious",
            Action::Next => "Next",
            Action::Previous => "Previous",
            Action::MoveLeft => "MoveLeft",
            Action::MoveRight => "MoveRight",
            Action::Close => "Close",
            Action::New => "New",
            Action::Tab(1) => "Tab1",
            Action::Tab(2) => "Tab2",
            Action::Tab(3) => "Tab3",
            Action::Tab(4) => "Tab4",
            Action::Tab(5) => "Tab5",
            Action::Tab(6) => "Tab6",
            Action::Tab(7) => "Tab7",
            Action::Tab(8) => "Tab8",
            Action::Tab(_) => "Tab",
            Action::LastTab => "LastTab",
            Action::OpenMenu => "OpenMenu",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::RecentNext => {
                "Activates the previously used document. Pressed again while the modifier is held, goes further back."
            }
            Action::RecentPrevious => "Walks the recently used documents the other way.",
            Action::Next => "Activates the tab to the right, wrapping at the end.",
            Action::Previous => "Activates the tab to the left, wrapping at the start.",
            Action::MoveLeft => "Moves the active tab one place left, within its group.",
            Action::MoveRight => "Moves the active tab one place right, within its group.",
            Action::Close => "Asks to close the active document.",
            Action::New => "Asks for a new document.",
            Action::Tab(1) => "Activates the first tab.",
            Action::Tab(2) => "Activates the second tab.",
            Action::Tab(3) => "Activates the third tab.",
            Action::Tab(4) => "Activates the fourth tab.",
            Action::Tab(5) => "Activates the fifth tab.",
            Action::Tab(6) => "Activates the sixth tab.",
            Action::Tab(7) => "Activates the seventh tab.",
            Action::Tab(8) => "Activates the eighth tab.",
            Action::Tab(_) => "Activates the tab at that position.",
            Action::LastTab => "Activates the last tab.",
            Action::OpenMenu => "Opens the active tab's context menu.",
        }
    }
}

/// The default document tabs shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Ctrl+Tab` | [`Action::RecentNext`] |
/// | `Ctrl+Shift+Tab` | [`Action::RecentPrevious`] |
/// | `Ctrl+PageDown` | [`Action::Next`] |
/// | `Ctrl+PageUp` | [`Action::Previous`] |
/// | `Ctrl+Shift+PageUp` | [`Action::MoveLeft`] |
/// | `Ctrl+Shift+PageDown` | [`Action::MoveRight`] |
/// | `Ctrl+W` | [`Action::Close`] |
/// | `Ctrl+N` | [`Action::New`] |
/// | `Ctrl+1` to `Ctrl+8` | [`Action::Tab`] |
/// | `Ctrl+9` | [`Action::LastTab`] |
///
/// `Ctrl+Tab` walks the documents in most-recently-used order, and
/// `Ctrl+PageDown` and `Ctrl+PageUp` in strip order. Ctrl+W, Ctrl+N and the
/// digits use Cmd on macOS. Subscribe to [`cycle_end`] so a walk ends when
/// Ctrl is released.
///
/// [`Action::OpenMenu`] has no default chord. The strip has no keyboard
/// focus, so `Shift+F10` or `Menu` bound app-wide would also open any
/// other context menu on screen. Bind it when the tab strip is the only
/// one, or only while the editor has focus:
///
/// ```
/// use iced::keyboard::key::Named;
/// use iced_cube::keys::Chord;
/// use iced_cube::navigation::document_tabs::{Action, default_keymap};
///
/// let keymap = default_keymap()
///     .bind(Chord::named(Named::F10).shift(), Action::OpenMenu)
///     .bind(Chord::named(Named::ContextMenu), Action::OpenMenu);
/// ```
///
/// The segmented tabs also bind `Ctrl+Tab` and `Ctrl+Shift+Tab`. An app
/// showing both should unbind them from one of them.
pub fn default_keymap() -> Keymap<Action> {
    let keymap = Keymap::new()
        .bind(Chord::named(Named::Tab).ctrl(), Action::RecentNext)
        .bind(
            Chord::named(Named::Tab).ctrl().shift(),
            Action::RecentPrevious,
        )
        .bind(Chord::named(Named::PageDown).ctrl(), Action::Next)
        .bind(Chord::named(Named::PageUp).ctrl(), Action::Previous)
        .bind(Chord::named(Named::PageUp).ctrl().shift(), Action::MoveLeft)
        .bind(
            Chord::named(Named::PageDown).ctrl().shift(),
            Action::MoveRight,
        )
        .bind(Chord::character('w').command(), Action::Close)
        .bind(Chord::character('n').command(), Action::New);
    let keymap = ('1'..='8')
        .zip(1..)
        .fold(keymap, |keymap, (digit, position)| {
            keymap.bind(Chord::character(digit).command(), Action::Tab(position))
        });
    keymap.bind(Chord::character('9').command(), Action::LastTab)
}

type OnEvent<'a, Id, Message> = dyn Fn(Event<Id>) -> Message + 'a;

/// A document tab strip builder. Convert it into an [`Element`] to render.
pub struct DocumentTabs<'a, Id, Message> {
    state: &'a State<Id>,
    new_tab: bool,
    max_label_width: f32,
    context_menu_keymap: Keymap<context_menu::Action>,
    overflow_keymap: Keymap<dropdown_menu::Action>,
    on_event: Option<Box<OnEvent<'a, Id, Message>>>,
}

/// Renders the tab strip of `state`. Without
/// [`on_event`](DocumentTabs::on_event) every tab renders disabled.
pub fn document_tabs<Id, Message>(state: &State<Id>) -> DocumentTabs<'_, Id, Message> {
    DocumentTabs {
        state,
        new_tab: false,
        max_label_width: MAX_LABEL_WIDTH,
        context_menu_keymap: context_menu::default_keymap(),
        overflow_keymap: dropdown_menu::default_keymap(),
        on_event: None,
    }
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for DocumentTabs<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentTabs")
            .field("state", self.state)
            .field("new_tab", &self.new_tab)
            .field("max_label_width", &self.max_label_width)
            .finish_non_exhaustive()
    }
}

impl<'a, Id, Message> DocumentTabs<'a, Id, Message> {
    /// Shows a button after the last tab, and lets a double click on the
    /// empty strip ask for a new document. Both send [`Event::New`].
    pub fn new_tab(mut self, new_tab: bool) -> Self {
        self.new_tab = new_tab;
        self
    }

    /// The widest a label grows before it ends in an ellipsis. Defaults to
    /// [`MAX_LABEL_WIDTH`].
    pub fn max_label_width(mut self, width: f32) -> Self {
        self.max_label_width = width;
        self
    }

    /// Replaces the default keymap of the open tab context menu.
    pub fn context_menu_keymap(mut self, keymap: Keymap<context_menu::Action>) -> Self {
        self.context_menu_keymap = keymap;
        self
    }

    /// Replaces the default keymap of the open menu listing every tab.
    pub fn overflow_keymap(mut self, keymap: Keymap<dropdown_menu::Action>) -> Self {
        self.overflow_keymap = keymap;
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

/// How one tab is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Look {
    active: bool,
    hovered: bool,
    enabled: bool,
}

impl<'a, Id, Message> From<DocumentTabs<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(tabs: DocumentTabs<'a, Id, Message>) -> Self {
        let DocumentTabs {
            state,
            new_tab,
            max_label_width,
            context_menu_keymap,
            overflow_keymap,
            on_event,
        } = tabs;
        let on_event: Option<Rc<OnEvent<'a, Id, Message>>> = on_event.map(Rc::from);
        let enabled = on_event.is_some();

        let tabs: Vec<Element<'a, Message>> = state
            .documents
            .iter()
            .map(|document| {
                let active = state.is_active(document.id);
                let on_close = on_event
                    .as_ref()
                    .map(|on_event| on_event(Event::Close(document.id)));
                let look = Look {
                    active,
                    hovered: false,
                    enabled,
                };
                let base = tab_view(document, look, max_label_width, on_close.clone(), false);
                let Some(on_event) = on_event.clone() else {
                    return base;
                };
                let hovered = Look {
                    hovered: true,
                    ..look
                };
                let top = tab_view(document, hovered, max_label_width, on_close, true);
                context_menu::keyed(state.menu(), document.id, widget::hover(base, top))
                    .keymap(context_menu_keymap.clone())
                    .on_event(move |event| on_event(Event::Menu(event)))
                    .into()
            })
            .collect();

        let trailing: Element<'a, Message> = if new_tab {
            icon_button(crate::lucide!(Plus))
                .label("New tab")
                .tooltip(Some(tooltip::Position::Bottom))
                .size(ButtonSize::Sm)
                .on_press_maybe(on_event.as_ref().map(|on_event| on_event(Event::New)))
                .into()
        } else {
            widget::space().into()
        };

        let open = state.overflow().is_open();
        let toggle = on_event
            .as_ref()
            .map(|on_event| on_event(Event::Overflow(dropdown_menu::Event::Toggle)));
        let trigger = icon_button(crate::lucide!(ChevronDown))
            .label("All tabs")
            .tooltip(None)
            .size(ButtonSize::Sm)
            .pressed(open)
            .on_press_maybe(toggle);
        let mut overflow = anchored(trigger)
            .content(open.then(|| overflow_panel(state, on_event.as_deref())))
            .placement(Placement::new(Side::Bottom, Align::End))
            .fit_height(true)
            .dismiss_keys([]);
        if let Some(on_event) = on_event.clone().filter(|_| open) {
            overflow = overflow
                .on_dismiss(on_event(Event::Overflow(dropdown_menu::Event::Close)))
                .on_key(move |key| {
                    state
                        .overflow()
                        .key_event(&overflow_keymap, key)
                        .map(|event| on_event(Event::Overflow(event)))
                });
        }

        let ids: Vec<Id> = state.documents.iter().map(|document| document.id).collect();
        let on_gesture = on_event.map(|on_event| {
            Box::new(move |gesture: Gesture| {
                let event = match gesture {
                    Gesture::Press(index) => Event::Activate(*ids.get(index)?),
                    Gesture::Middle(index) => Event::Close(*ids.get(index)?),
                    Gesture::Drop { from, to } => Event::Move(*ids.get(from)?, to),
                    Gesture::DoubleClick if new_tab => Event::New,
                    Gesture::DoubleClick => return None,
                };
                Some(on_event(event))
            }) as Box<dyn Fn(Gesture) -> Option<Message> + 'a>
        });

        Element::new(Strip {
            content: row(tabs).into(),
            trailing,
            overflow: overflow.into(),
            active: state.active_index(),
            count: state.documents.len(),
            pinned: state.pinned_count(),
            on_gesture,
        })
    }
}

fn tab_view<'a, Id, Message>(
    document: &'a Document<Id>,
    look: Look,
    max_label_width: f32,
    on_close: Option<Message>,
    fill: bool,
) -> Element<'a, Message>
where
    Id: 'a,
    Message: Clone + 'a,
{
    let Look {
        active,
        hovered,
        enabled,
    } = look;
    let colour = move |theme: &Theme| label_colour(&Tokens::of(theme), active, hovered, enabled);

    let mut content = row![].spacing(space::SM).align_y(Alignment::Center);
    if let Some(glyph) = document.icon {
        content = content.push(themed(glyph, ICON, opacity(enabled), colour));
    }
    if !document.pinned || document.icon.is_none() {
        let width = if document.pinned {
            PINNED_LABEL_WIDTH
        } else {
            max_label_width
        };
        let label = ellipsis(document.label.as_str())
            .width(Length::Shrink)
            .colour(colour);
        content = content.push(container(label).max_width(width));
    }
    if let Some(slot) = slot(document, look, on_close) {
        content = content.push(slot);
    }

    container(content)
        .width(if fill { Length::Fill } else { Length::Shrink })
        .height(HEIGHT)
        .padding(Padding::from([0.0, PADDING_X]))
        .align_y(Vertical::Center)
        .style(move |theme| tab_style(&Tokens::of(theme), active, hovered))
        .into()
}

/// The open list of every tab, one row per document with the active one
/// checked, scrolled to show the highlighted row or else the active one.
fn overflow_panel<'a, Id, Message>(
    state: &'a State<Id>,
    on_event: Option<&OnEvent<'a, Id, Message>>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let highlighted = state.overflow().highlighted();
    let rows = state.documents.iter().map(|document| {
        let status = match (on_event.is_some(), highlighted == Some(document.id)) {
            (false, _) => RowStatus::Disabled,
            (true, true) => RowStatus::Highlighted,
            (true, false) => RowStatus::Idle,
        };
        overflow_row(document, state.is_active(document.id), status, on_event)
    });
    let reveal = highlighted
        .and_then(|id| state.index_of(id))
        .or_else(|| state.active_index());
    menu::surface(scrolling(fitted(rows, OVERFLOW_WIDTH), reveal)).into()
}

/// One row of the list of every tab. The label ends in an ellipsis rather
/// than wrapping.
fn overflow_row<'a, Id, Message>(
    document: &'a Document<Id>,
    active: bool,
    status: RowStatus,
    on_event: Option<&OnEvent<'a, Id, Message>>,
) -> Element<'a, Message>
where
    Id: Copy + 'a,
    Message: Clone + 'a,
{
    let colours = move |theme: &Theme| menu::row_style(&Tokens::of(theme), status, false);
    let opacity = opacity(status != RowStatus::Disabled);

    let mark: Element<'a, Message> = if active {
        themed(
            crate::lucide!(Check),
            menu::ICON_SIZE,
            opacity,
            move |theme| colours(theme).text,
        )
        .into()
    } else {
        widget::space().into()
    };
    let mut content = row![
        container(mark)
            .center_x(menu::ICON_SIZE)
            .center_y(menu::ICON_SIZE)
    ]
    .spacing(space::SM)
    .align_y(Alignment::Center);
    if let Some(glyph) = document.icon {
        content = content.push(themed(glyph, menu::ICON_SIZE, opacity, move |theme| {
            colours(theme).icon
        }));
    }
    content = content.push(
        ellipsis(document.label.as_str())
            .width(Length::Fill)
            .colour(move |theme| colours(theme).text),
    );
    if document.modified {
        content = content.push(
            widget::text("Modified")
                .size(text_size::XS)
                .wrapping(widget::text::Wrapping::None)
                .style(move |theme| widget::text::Style {
                    color: Some(colours(theme).hint),
                }),
        );
    }

    let body = container(content)
        .padding(menu::ROW_PADDING)
        .width(Length::Fill)
        .style(move |theme| menu::row_container_style(colours(theme)));
    let Some(on_event) = on_event.filter(|_| status != RowStatus::Disabled) else {
        return menu::inset(body);
    };
    let id = document.id;
    menu::inset(
        widget::mouse_area(body)
            .on_enter(on_event(Event::Overflow(dropdown_menu::Event::Highlight(
                id,
            ))))
            .on_press(on_event(Event::Overflow(dropdown_menu::Event::Activate(
                id,
            ))))
            .interaction(mouse::Interaction::Pointer),
    )
}

/// The trailing slot of a tab: the close button on the active or hovered
/// tab, otherwise a dot while modified, otherwise empty space so the label
/// does not move. Pinned tabs only show the dot.
fn slot<'a, Id, Message>(
    document: &Document<Id>,
    look: Look,
    on_close: Option<Message>,
) -> Option<Element<'a, Message>>
where
    Message: Clone + 'a,
{
    let active = look.active;
    let close = !document.pinned && (look.hovered || (active && !document.modified));
    if close {
        return Some(close_button(on_close));
    }
    if document.modified {
        let dot = container(widget::space())
            .width(DOT)
            .height(DOT)
            .style(move |theme| dot_style(&Tokens::of(theme), active));
        return Some(container(dot).center(SLOT).into());
    }
    (!document.pinned).then(|| widget::space().width(SLOT).height(SLOT).into())
}

fn close_button<'a, Message: Clone + 'a>(on_close: Option<Message>) -> Element<'a, Message> {
    let enabled = on_close.is_some();
    widget::button(themed(
        crate::lucide!(X),
        CLOSE_ICON,
        opacity(enabled),
        |theme| Tokens::of(theme).foreground,
    ))
    .width(SLOT)
    .height(SLOT)
    .padding((SLOT - CLOSE_ICON) / 2.0)
    .on_press_maybe(on_close)
    .style(|theme, status| close_style(&Tokens::of(theme), status))
    .into()
}

/// The colour of a tab's label and icon.
pub fn label_colour(tokens: &Tokens, active: bool, hovered: bool, enabled: bool) -> Color {
    let colour = if active || hovered {
        tokens.foreground
    } else {
        tokens.muted_foreground
    };
    if enabled { colour } else { fade(colour, 0.5) }
}

/// The surface of one tab. The active tab takes the page background, so it
/// joins the content below; the others sit flat on the strip.
pub fn tab_style(tokens: &Tokens, active: bool, hovered: bool) -> container::Style {
    let background = if active {
        Some(tokens.background)
    } else if hovered {
        Some(mix(tokens.muted, tokens.foreground, 0.06))
    } else {
        None
    };
    container::Style {
        background: background.map(Background::Color),
        text_color: Some(label_colour(tokens, active, hovered, true)),
        border: Border {
            radius: Radius {
                top_left: radius::MD,
                top_right: radius::MD,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The close button inside a tab.
pub fn close_style(tokens: &Tokens, status: Status) -> widget::button::Style {
    let background = match status {
        Status::Hovered | Status::Pressed => Some(fade(tokens.foreground, 0.1)),
        Status::Active | Status::Disabled => None,
    };
    widget::button::Style {
        background: background.map(Background::Color),
        text_color: tokens.foreground,
        border: Border {
            radius: radius::SM.into(),
            ..Border::default()
        },
        ..widget::button::Style::default()
    }
}

/// The dot that marks unsaved changes.
pub fn dot_style(tokens: &Tokens, active: bool) -> container::Style {
    let colour = if active {
        tokens.foreground
    } else {
        tokens.muted_foreground
    };
    container::Style {
        background: Some(Background::Color(colour)),
        border: Border {
            radius: radius::FULL.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Colours the strip draws itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripStyle {
    pub background: Color,
    /// The line along the bottom, broken under the active tab.
    pub line: Color,
    /// Short lines between inactive tabs.
    pub divider: Color,
    /// Where a dragged tab will land.
    pub indicator: Color,
}

pub fn strip_style(tokens: &Tokens) -> StripStyle {
    StripStyle {
        background: tokens.muted,
        line: tokens.border,
        divider: tokens.border,
        indicator: tokens.primary,
    }
}

/// The gap a pointer at `x` points to: the number of tabs whose centre lies
/// before it. `tabs` holds each tab's left edge and width, in order.
fn gap_at(tabs: &[(f32, f32)], x: f32) -> usize {
    tabs.iter()
        .take_while(|(left, width)| left + width / 2.0 < x)
        .count()
}

/// The gap a tab dragged from `from` may drop into, kept within its group:
/// the first `pinned` tabs, or the rest of the `count`.
fn clamp_gap(from: usize, gap: usize, pinned: usize, count: usize) -> usize {
    if from < pinned {
        gap.min(pinned)
    } else {
        gap.clamp(pinned, count)
    }
}

/// The index a tab dragged from `from` ends up at when dropped into `gap`,
/// or `None` when it would not move.
fn drop_index(from: usize, gap: usize) -> Option<usize> {
    let to = if gap > from { gap - 1 } else { gap };
    (to != from).then_some(to)
}

/// Pointer gestures on the strip, by tab index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gesture {
    Press(usize),
    Middle(usize),
    Drop { from: usize, to: usize },
    DoubleClick,
}

/// A press on a tab that may become a drag.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Press {
    index: usize,
    origin: Point,
    dragging: bool,
    gap: Option<usize>,
}

/// What the strip remembers between frames.
#[derive(Debug, Clone, Default)]
struct Memory {
    offset: f32,
    content_width: f32,
    viewport_width: f32,
    overflowing: bool,
    /// The active index and tab count the strip last scrolled into view.
    revealed: Option<(Option<usize>, usize)>,
    /// The content and viewport widths the strip last scrolled for.
    measured: Option<(f32, f32)>,
    press: Option<Press>,
    last_click: Option<mouse::Click>,
    /// A finger panning the strip: which one, and where it was last.
    touch: Option<(Finger, f32)>,
}

impl Scrollable for Memory {
    fn snap_to(&mut self, offset: RelativeOffset<Option<f32>>) {
        if let Some(x) = offset.x {
            self.offset = (x.clamp(0.0, 1.0) * self.max_offset()).round();
        }
    }

    fn scroll_to(&mut self, offset: AbsoluteOffset<Option<f32>>) {
        if let Some(x) = offset.x {
            self.offset = x.clamp(0.0, self.max_offset());
        }
    }

    fn scroll_by(&mut self, offset: AbsoluteOffset, _bounds: Rectangle, _content: Rectangle) {
        let _ = Memory::scroll_by(self, offset.x);
    }
}

impl Memory {
    fn max_offset(&self) -> f32 {
        (self.content_width - self.viewport_width).max(0.0)
    }

    fn scroll_by(&mut self, delta: f32) -> bool {
        let offset = (self.offset + delta).clamp(0.0, self.max_offset());
        let changed = offset != self.offset;
        self.offset = offset;
        changed
    }

    /// Scrolls the least distance that shows a tab spanning `left` to
    /// `right` in full.
    fn reveal(&mut self, left: f32, right: f32) {
        if right > self.offset + self.viewport_width {
            self.offset = right - self.viewport_width;
        }
        if left < self.offset {
            self.offset = left;
        }
        self.offset = self.offset.clamp(0.0, self.max_offset());
    }

    /// Keeps the active tab, spanning `active`, in view after a layout.
    ///
    /// When the active tab or the tab count changes, the strip scrolls the
    /// least distance. When only the sizes change, such as on the first
    /// layout, a resize or once fonts load, the offset is worked out afresh
    /// from the start of the strip, so it never depends on earlier sizes.
    fn follow(
        &mut self,
        key: (Option<usize>, usize),
        widths: (f32, f32),
        active: Option<(f32, f32)>,
    ) {
        (self.content_width, self.viewport_width) = widths;
        let moved = self.revealed != Some(key);
        let resized = self.measured != Some(widths);
        self.revealed = Some(key);
        self.measured = Some(widths);
        if !moved && resized {
            self.offset = 0.0;
        }
        if let Some((left, width)) = active.filter(|_| moved || resized) {
            self.reveal(left, left + width);
        }
        self.offset = self.offset.clamp(0.0, self.max_offset());
    }

    fn viewport(&self, bounds: Rectangle) -> Rectangle {
        Rectangle {
            width: self.viewport_width,
            ..bounds
        }
    }

    fn shift(&self) -> Vector {
        Vector::new(self.offset, 0.0)
    }

    /// The cursor in the scrolled content's coordinates.
    fn inner_cursor(&self, cursor: mouse::Cursor, viewport: Rectangle) -> mouse::Cursor {
        match cursor.position_over(viewport) {
            Some(position) => mouse::Cursor::Available(position + self.shift()),
            None => cursor.levitate() + self.shift(),
        }
    }
}

/// The tab strip: tabs that scroll sideways, a new tab button after them
/// and, while they overflow, a menu button listing them at the end.
struct Strip<'a, Message> {
    content: Element<'a, Message>,
    trailing: Element<'a, Message>,
    overflow: Element<'a, Message>,
    active: Option<usize>,
    count: usize,
    pinned: usize,
    on_gesture: Option<Box<dyn Fn(Gesture) -> Option<Message> + 'a>>,
}

impl<Message> Strip<'_, Message> {
    fn publish(&self, shell: &mut Shell<'_, Message>, gesture: Gesture) {
        if let Some(message) = self.on_gesture.as_ref().and_then(|on| on(gesture)) {
            shell.publish(message);
        }
    }
}

/// Each tab's left edge and width, relative to the content.
fn spans(content: Layout<'_>) -> Vec<(f32, f32)> {
    let origin = content.bounds().x;
    content
        .children()
        .map(|tab| {
            let bounds = tab.bounds();
            (bounds.x - origin, bounds.width)
        })
        .collect()
}

impl<Message> Widget<Message, Theme, Renderer> for Strip<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Memory>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Memory::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&self.content),
            Tree::new(&self.trailing),
            Tree::new(&self.overflow),
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content, &self.trailing, &self.overflow]);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        let [content_tree, trailing_tree, overflow_tree] = tree.children.as_mut_slice() else {
            return layout::Node::new(Size::ZERO);
        };
        let unbounded = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, max.height));
        let content = self
            .content
            .as_widget_mut()
            .layout(content_tree, renderer, &unbounded);
        let loose = layout::Limits::new(Size::ZERO, max);
        let trailing = self
            .trailing
            .as_widget_mut()
            .layout(trailing_tree, renderer, &loose);
        let overflow = self
            .overflow
            .as_widget_mut()
            .layout(overflow_tree, renderer, &loose);

        let content_size = content.size();
        let trailing_size = trailing.size();
        let overflow_size = overflow.size();
        let natural = content_size.width + trailing_size.width;
        let width = if max.width.is_finite() {
            max.width
        } else {
            natural
        };
        let height = content_size
            .height
            .max(trailing_size.height)
            .max(overflow_size.height)
            .max(limits.min().height);
        let overflowing = natural > width + 0.5;
        let viewport_width = if overflowing {
            (width - trailing_size.width - overflow_size.width).max(0.0)
        } else {
            content_size.width
        };
        let centre = |size: Size| (height - size.height) / 2.0;

        let spans: Vec<(f32, f32)> = content
            .children()
            .iter()
            .map(|tab| (tab.bounds().x, tab.bounds().width))
            .collect();

        let memory = tree.state.downcast_mut::<Memory>();
        memory.overflowing = overflowing;
        memory.follow(
            (self.active, self.count),
            (content_size.width, viewport_width),
            self.active.and_then(|index| spans.get(index).copied()),
        );

        let overflow_x = width - overflow_size.width;
        layout::Node::with_children(
            Size::new(width, height),
            vec![
                content.move_to(Point::new(0.0, centre(content_size))),
                trailing.move_to(Point::new(viewport_width, centre(trailing_size))),
                overflow.move_to(Point::new(overflow_x, centre(overflow_size))),
            ],
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let memory = tree.state.downcast_ref::<Memory>();
        let colours = strip_style(&Tokens::of(theme));
        let bounds = layout.bounds();
        let mut children = layout.children();
        let (Some(content), Some(trailing), Some(overflow)) =
            (children.next(), children.next(), children.next())
        else {
            return;
        };

        fill(renderer, bounds, colours.background);

        let clip = memory.viewport(bounds);
        let inner = memory.inner_cursor(cursor, clip);
        let tabs: Vec<Rectangle> = content.children().map(|tab| tab.bounds()).collect();
        renderer.with_layer(clip, |renderer| {
            renderer.with_translation(Vector::new(-memory.offset, 0.0), |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    content,
                    inner,
                    &(clip + memory.shift()),
                );
                self.draw_marks(renderer, memory, &tabs, colours, bounds);
            });
        });

        // Marks where tabs are cut off before the buttons.
        if memory.overflowing {
            let top = bounds.y + (bounds.height - DIVIDER_HEIGHT) / 2.0;
            fill(
                renderer,
                Rectangle::new(
                    Point::new((clip.x + clip.width).round(), top),
                    Size::new(1.0, DIVIDER_HEIGHT),
                ),
                colours.divider,
            );
        }

        // The bottom line, broken under the active tab where it is visible.
        let line = Rectangle {
            y: bounds.y + bounds.height - 1.0,
            height: 1.0,
            ..bounds
        };
        let gap = self
            .active
            .and_then(|index| tabs.get(index))
            .map(|tab| *tab - memory.shift())
            .and_then(|tab| tab.intersection(&clip));
        match gap {
            Some(tab) => {
                fill(
                    renderer,
                    Rectangle {
                        width: tab.x - line.x,
                        ..line
                    },
                    colours.line,
                );
                let right = tab.x + tab.width;
                fill(
                    renderer,
                    Rectangle {
                        x: right,
                        width: line.x + line.width - right,
                        ..line
                    },
                    colours.line,
                );
            }
            None => fill(renderer, line, colours.line),
        }

        self.trailing.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            trailing,
            cursor,
            viewport,
        );
        if memory.overflowing {
            self.overflow.as_widget().draw(
                &tree.children[2],
                renderer,
                theme,
                style,
                overflow,
                cursor,
                viewport,
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let mut children = layout.children();
        let (Some(content), Some(trailing), Some(overflow)) =
            (children.next(), children.next(), children.next())
        else {
            return;
        };
        let [content_tree, trailing_tree, overflow_tree] = tree.children.as_mut_slice() else {
            return;
        };
        let memory = tree.state.downcast_mut::<Memory>();
        let clip = memory.viewport(layout.bounds());
        let shift = memory.shift();
        operation.scrollable(None, clip, content.bounds(), shift, memory);
        operation.traverse(&mut |operation| {
            self.content
                .as_widget_mut()
                .operate(content_tree, content, renderer, operation);
        });
        self.trailing
            .as_widget_mut()
            .operate(trailing_tree, trailing, renderer, operation);
        self.overflow
            .as_widget_mut()
            .operate(overflow_tree, overflow, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Input,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let mut children = layout.children();
        let (Some(content), Some(trailing), Some(overflow)) =
            (children.next(), children.next(), children.next())
        else {
            return;
        };
        let spans = spans(content);

        if self.drag(tree, event, cursor, content.bounds().x, &spans, shell) {
            return;
        }

        let memory = tree.state.downcast_ref::<Memory>();
        let clip = memory.viewport(bounds);
        let inner = memory.inner_cursor(cursor, clip);
        let inner_viewport = clip + memory.shift();
        let overflowing = memory.overflowing;
        let [content_tree, trailing_tree, overflow_tree] = tree.children.as_mut_slice() else {
            return;
        };
        self.content.as_widget_mut().update(
            content_tree,
            event,
            content,
            inner,
            renderer,
            clipboard,
            shell,
            &inner_viewport,
        );
        self.trailing.as_widget_mut().update(
            trailing_tree,
            event,
            trailing,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if overflowing {
            self.overflow.as_widget_mut().update(
                overflow_tree,
                event,
                overflow,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
        if shell.is_event_captured() {
            return;
        }

        let memory = tree.state.downcast_mut::<Memory>();
        let content_x = content.bounds().x;
        self.pointer(
            memory, event, cursor, bounds, clip, content_x, &spans, shell,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let memory = tree.state.downcast_ref::<Memory>();
        if memory.press.is_some_and(|press| press.dragging) {
            return mouse::Interaction::Grabbing;
        }
        let bounds = layout.bounds();
        let mut children = layout.children();
        let (Some(content), Some(trailing), Some(overflow)) =
            (children.next(), children.next(), children.next())
        else {
            return mouse::Interaction::None;
        };
        let clip = memory.viewport(bounds);
        let inner = memory.inner_cursor(cursor, clip);
        let interaction = self
            .content
            .as_widget()
            .mouse_interaction(
                &tree.children[0],
                content,
                inner,
                &(clip + memory.shift()),
                renderer,
            )
            .max(self.trailing.as_widget().mouse_interaction(
                &tree.children[1],
                trailing,
                cursor,
                viewport,
                renderer,
            ));
        if !memory.overflowing {
            return interaction;
        }
        interaction.max(self.overflow.as_widget().mouse_interaction(
            &tree.children[2],
            overflow,
            cursor,
            viewport,
            renderer,
        ))
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let memory = tree.state.downcast_ref::<Memory>();
        let clip = memory.viewport(layout.bounds());
        let shift = memory.shift();
        let overflowing = memory.overflowing;
        let mut children = layout.children();
        let (Some(content), Some(trailing), Some(overflow)) =
            (children.next(), children.next(), children.next())
        else {
            return None;
        };
        let [content_tree, trailing_tree, overflow_tree] = tree.children.as_mut_slice() else {
            return None;
        };

        let mut overlays = Vec::new();
        overlays.extend(self.content.as_widget_mut().overlay(
            content_tree,
            content,
            renderer,
            &(clip + shift),
            translation - shift,
        ));
        overlays.extend(self.trailing.as_widget_mut().overlay(
            trailing_tree,
            trailing,
            renderer,
            viewport,
            translation,
        ));
        if overflowing {
            overlays.extend(self.overflow.as_widget_mut().overlay(
                overflow_tree,
                overflow,
                renderer,
                viewport,
                translation,
            ));
        }
        match overlays.len() {
            0 => None,
            1 => overlays.pop(),
            _ => Some(overlay::Group::with_children(overlays).overlay()),
        }
    }
}

fn fill(renderer: &mut Renderer, bounds: Rectangle, colour: Color) {
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return;
    }
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            snap: true,
            ..renderer::Quad::default()
        },
        Background::Color(colour),
    );
}

impl<Message> Strip<'_, Message> {
    /// Dividers between inactive tabs and the drop indicator, in content
    /// coordinates.
    fn draw_marks(
        &self,
        renderer: &mut Renderer,
        memory: &Memory,
        tabs: &[Rectangle],
        colours: StripStyle,
        bounds: Rectangle,
    ) {
        let top = bounds.y + (bounds.height - DIVIDER_HEIGHT) / 2.0;
        for (index, pair) in tabs.windows(2).enumerate() {
            let touches_active = self
                .active
                .is_some_and(|active| active == index || active == index + 1);
            if touches_active {
                continue;
            }
            let x = pair[1].x.round();
            fill(
                renderer,
                Rectangle::new(Point::new(x, top), Size::new(1.0, DIVIDER_HEIGHT)),
                colours.divider,
            );
        }

        let Some(gap) = memory
            .press
            .filter(|press| press.dragging)
            .and_then(|press| {
                let gap = press.gap?;
                drop_index(press.index, gap).map(|_| gap)
            })
        else {
            return;
        };
        let x = match (tabs.get(gap), tabs.last()) {
            (Some(tab), _) => tab.x,
            (None, Some(last)) => last.x + last.width,
            (None, None) => return,
        };
        let x = (x - INDICATOR_WIDTH / 2.0)
            .max(bounds.x)
            .min(bounds.x + memory.content_width - INDICATOR_WIDTH);
        fill(
            renderer,
            Rectangle::new(
                Point::new(x, bounds.y),
                Size::new(INDICATOR_WIDTH, bounds.height),
            ),
            colours.indicator,
        );
    }

    /// Follows a tab being dragged. Returns whether it used the event.
    fn drag(
        &self,
        tree: &mut Tree,
        event: &Input,
        cursor: mouse::Cursor,
        content_x: f32,
        spans: &[(f32, f32)],
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let memory = tree.state.downcast_mut::<Memory>();
        let Some(mut press) = memory.press else {
            return false;
        };
        match event {
            Input::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(position) = cursor.position() else {
                    return false;
                };
                if !press.dragging {
                    let far = press.origin.distance(position) > DRAG_THRESHOLD;
                    if !far || self.on_gesture.is_none() {
                        return false;
                    }
                    press.dragging = true;
                }
                let x = position.x + memory.offset - content_x;
                let gap = gap_at(spans, x);
                press.gap = Some(clamp_gap(press.index, gap, self.pinned, self.count));
                memory.press = Some(press);
                shell.request_redraw();
                shell.capture_event();
                true
            }
            Input::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                memory.press = None;
                if !press.dragging {
                    return false;
                }
                shell.request_redraw();
                shell.capture_event();
                let Some(to) = press.gap.and_then(|gap| drop_index(press.index, gap)) else {
                    return true;
                };
                self.publish(
                    shell,
                    Gesture::Drop {
                        from: press.index,
                        to,
                    },
                );
                true
            }
            _ => false,
        }
    }

    /// Presses, double clicks, the wheel and touch panning, once no child
    /// has used the event.
    #[allow(clippy::too_many_arguments)]
    fn pointer(
        &self,
        memory: &mut Memory,
        event: &Input,
        cursor: mouse::Cursor,
        bounds: Rectangle,
        clip: Rectangle,
        content_x: f32,
        spans: &[(f32, f32)],
        shell: &mut Shell<'_, Message>,
    ) {
        let tab_at = |memory: &Memory, cursor: mouse::Cursor| {
            let position = cursor.position_over(clip)?;
            let x = position.x + memory.offset - content_x;
            spans
                .iter()
                .position(|(left, width)| x >= *left && x < left + width)
        };
        match event {
            Input::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_over(bounds) else {
                    return;
                };
                if let Some(index) = tab_at(memory, cursor) {
                    memory.press = Some(Press {
                        index,
                        origin: position,
                        dragging: false,
                        gap: None,
                    });
                    self.publish(shell, Gesture::Press(index));
                    shell.capture_event();
                    return;
                }
                let click = mouse::Click::new(position, mouse::Button::Left, memory.last_click);
                memory.last_click = Some(click);
                if click.kind() == click::Kind::Double {
                    self.publish(shell, Gesture::DoubleClick);
                    shell.capture_event();
                }
            }
            Input::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                if let Some(index) = tab_at(memory, cursor) {
                    self.publish(shell, Gesture::Middle(index));
                    shell.capture_event();
                }
            }
            Input::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return;
                }
                let step = match *delta {
                    mouse::ScrollDelta::Lines { x, y } => {
                        (if x != 0.0 { x } else { y }) * WHEEL_LINE
                    }
                    mouse::ScrollDelta::Pixels { x, y } => {
                        if x.abs() > y.abs() {
                            x
                        } else {
                            y
                        }
                    }
                };
                if memory.scroll_by(-step) {
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Input::Touch(touch::Event::FingerPressed { id, position }) => {
                if !bounds.contains(*position) {
                    return;
                }
                memory.touch = Some((*id, position.x));
                if let Some(index) = tab_at(memory, cursor) {
                    self.publish(shell, Gesture::Press(index));
                    shell.capture_event();
                }
            }
            Input::Touch(touch::Event::FingerMoved { id, position }) => {
                let Some((finger, last)) = memory.touch.filter(|(finger, _)| finger == id) else {
                    return;
                };
                memory.touch = Some((finger, position.x));
                if memory.scroll_by(last - position.x) {
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Input::Touch(
                touch::Event::FingerLifted { id, .. } | touch::Event::FingerLost { id, .. },
            ) if memory.touch.is_some_and(|(finger, _)| finger == *id) => {
                memory.touch = None;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use iced::keyboard::{Key, Modifiers};

    fn state() -> State<u8> {
        State::new([
            document(1, "main.rs"),
            document(2, "lib.rs").modified(true),
            document(3, "README.md").pinned(true),
            document(4, "Cargo.toml"),
        ])
    }

    fn ids(state: &State<u8>) -> Vec<u8> {
        state
            .documents()
            .iter()
            .map(|document| document.id)
            .collect()
    }

    #[test]
    fn new_puts_pinned_documents_first_and_activates_the_first() {
        let state = state();
        assert_eq!(ids(&state), [3, 1, 2, 4]);
        assert_eq!(state.active(), Some(3));
        assert_eq!(state.recent(), [3, 1, 2, 4]);
        assert_eq!(state.pinned_count(), 1);
        assert!(state.is_active(3));
        assert!(!state.menu().is_open());
        assert!(!state.overflow().is_open());
    }

    #[test]
    fn empty_state_has_nothing_to_do() {
        let mut state: State<u8> = State::new([]);
        assert_eq!(state.active(), None);
        assert_eq!(state.update(Event::Next), None);
        assert_eq!(state.update(Event::RecentNext), None);
        assert_eq!(state.update(Event::CloseAll), None);
        assert_eq!(Action::Close.event(&state), None);
        assert_eq!(Action::LastTab.event(&state), None);
    }

    #[test]
    fn activate_reports_changes_only() {
        let mut state = state();
        assert_eq!(state.update(Event::Activate(2)), Some(Output::Activated(2)));
        assert_eq!(state.update(Event::Activate(2)), None);
        assert_eq!(state.update(Event::Activate(9)), None);
        assert_eq!(state.recent()[0], 2);
        assert_eq!(state.clone().with_active(4).active(), Some(4));
    }

    #[test]
    fn next_and_previous_wrap_in_strip_order() {
        let mut state = state().with_active(4);
        assert_eq!(state.update(Event::Next), Some(Output::Activated(3)));
        assert_eq!(state.update(Event::Previous), Some(Output::Activated(4)));
        assert_eq!(state.update(Event::Previous), Some(Output::Activated(2)));
    }

    #[test]
    fn recent_walks_until_the_cycle_ends() {
        let mut state = state();
        for id in [1, 2, 4] {
            let _ = state.update(Event::Activate(id));
        }
        assert_eq!(state.recent(), [4, 2, 1, 3]);
        assert_eq!(state.update(Event::RecentNext), Some(Output::Activated(2)));
        assert_eq!(state.update(Event::RecentNext), Some(Output::Activated(1)));
        assert_eq!(state.recent(), [4, 2, 1, 3], "the order holds mid-walk");
        assert_eq!(state.update(Event::EndCycle), None);
        assert_eq!(state.recent(), [1, 4, 2, 3]);
        assert_eq!(state.update(Event::RecentNext), Some(Output::Activated(4)));
        assert_eq!(state.update(Event::from(CycleEnd)), None);
        assert_eq!(state.recent(), [4, 1, 2, 3]);
    }

    #[test]
    fn recent_previous_starts_from_the_oldest_and_other_events_end_the_walk() {
        let mut state = state();
        assert_eq!(
            state.update(Event::RecentPrevious),
            Some(Output::Activated(4))
        );
        assert_eq!(state.update(Event::Activate(2)), Some(Output::Activated(2)));
        assert_eq!(state.recent(), [2, 4, 3, 1]);
        let mut single = State::new([document(1, "a")]);
        assert_eq!(single.update(Event::RecentNext), None);
    }

    #[test]
    fn close_events_request_without_closing() {
        let mut state = state();
        assert_eq!(
            state.update(Event::Close(3)),
            Some(Output::CloseRequested(vec![3]))
        );
        assert_eq!(state.update(Event::Close(9)), None);
        assert_eq!(
            state.update(Event::CloseOthers(1)),
            Some(Output::CloseRequested(vec![2, 4]))
        );
        assert_eq!(
            state.update(Event::CloseToTheRight(1)),
            Some(Output::CloseRequested(vec![2, 4]))
        );
        assert_eq!(state.update(Event::CloseToTheRight(4)), None);
        assert_eq!(
            state.update(Event::CloseSaved),
            Some(Output::CloseRequested(vec![1, 4]))
        );
        assert_eq!(
            state.update(Event::CloseAll),
            Some(Output::CloseRequested(vec![1, 2, 4]))
        );
        assert_eq!(state.documents().len(), 4);
    }

    #[test]
    fn closing_the_active_document_activates_the_most_recent() {
        let mut state = state();
        let _ = state.update(Event::Activate(4));
        let _ = state.update(Event::Activate(1));
        assert_eq!(state.close(&[1]), Some(4));
        assert_eq!(ids(&state), [3, 2, 4]);
        assert_eq!(state.close(&[2]), None, "the active one stays");
        assert_eq!(state.active(), Some(4));
        assert_eq!(state.close(&[3, 4]), None);
        assert_eq!(state.active(), None);
        assert!(state.recent().is_empty());
    }

    #[test]
    fn open_adds_to_the_end_of_its_group_or_activates() {
        let mut state = state();
        state.open(document(5, "new.rs"));
        assert_eq!(ids(&state), [3, 1, 2, 4, 5]);
        assert_eq!(state.active(), Some(5));
        state.open(document(6, "notes.md").pinned(true));
        assert_eq!(ids(&state), [3, 6, 1, 2, 4, 5]);
        state.open(document(1, "ignored"));
        assert_eq!(state.active(), Some(1));
        assert_eq!(state.document(1).map(|d| d.label.as_str()), Some("main.rs"));
        assert_eq!(state.recent()[..3], [1, 6, 5]);
    }

    #[test]
    fn modified_and_label_change_in_place() {
        let mut state = state();
        state.set_modified(1, true);
        state.set_label(1, "main2.rs");
        let main = state.document(1).cloned();
        assert_eq!(
            main,
            Some(document(1, "main2.rs").modified(true)),
            "{main:?}"
        );
        state.set_modified(9, true);
    }

    #[test]
    fn moves_stay_within_the_group() {
        let mut state = state();
        assert_eq!(
            state.update(Event::Move(1, 3)),
            Some(Output::Moved {
                id: 1,
                from: 1,
                to: 3
            })
        );
        assert_eq!(ids(&state), [3, 2, 4, 1]);
        assert_eq!(
            state.update(Event::Move(1, 0)),
            Some(Output::Moved {
                id: 1,
                from: 3,
                to: 1
            })
        );
        assert_eq!(state.update(Event::Move(1, 1)), None);
        assert_eq!(state.update(Event::Move(3, 5)), None, "pinned stays first");
        assert_eq!(state.update(Event::Move(4, 99)), None, "already last");
    }

    #[test]
    fn pinning_moves_to_the_group_boundary() {
        let mut state = state();
        assert_eq!(
            state.update(Event::Pin(4, true)),
            Some(Output::Pinned(4, true))
        );
        assert_eq!(ids(&state), [3, 4, 1, 2]);
        assert_eq!(state.update(Event::Pin(4, true)), None);
        assert_eq!(
            state.update(Event::Pin(3, false)),
            Some(Output::Pinned(3, false))
        );
        assert_eq!(ids(&state), [4, 3, 1, 2]);
        assert_eq!(state.pinned_count(), 1);
    }

    fn items(state: &State<u8>) -> Vec<(MenuItem, bool)> {
        state
            .menu()
            .menu()
            .entries()
            .iter()
            .filter_map(|entry| entry.as_item())
            .map(|item| (item.id, item.disabled))
            .collect()
    }

    #[test]
    fn the_context_menu_is_built_for_the_tab_it_opens_on() {
        let mut state = state().with_menu_items([item(MenuItem::Custom("copy-path"), "Copy path")]);
        let _ = state.update(Event::Menu(context_menu::Event::Open(4, Point::ORIGIN)));
        assert!(state.menu().is_open_on(&4));
        assert_eq!(
            items(&state),
            [
                (MenuItem::Close, false),
                (MenuItem::CloseOthers, false),
                (MenuItem::CloseToTheRight, true),
                (MenuItem::CloseSaved, false),
                (MenuItem::CloseAll, false),
                (MenuItem::Pin, false),
                (MenuItem::Custom("copy-path"), false),
            ]
        );
        let _ = state.update(Event::Menu(context_menu::Event::OpenFromKeyboard(3)));
        assert!(items(&state).contains(&(MenuItem::Unpin, false)));
        assert_eq!(
            state.update(Event::Menu(context_menu::Event::Open(9, Point::ORIGIN))),
            None
        );
    }

    #[test]
    fn menu_items_map_to_outputs() {
        let choose = |target: u8, item: MenuItem| {
            let mut state =
                state().with_menu_items([dropdown_menu::item(MenuItem::Custom("copy"), "Copy")]);
            let _ = state.update(Event::Menu(context_menu::Event::OpenFromKeyboard(target)));
            let output = state.update(Event::Menu(context_menu::Event::Menu(
                dropdown_menu::Event::Activate(item),
            )));
            assert!(!state.menu().is_open());
            (output, ids(&state))
        };
        assert_eq!(
            choose(1, MenuItem::Close).0,
            Some(Output::CloseRequested(vec![1]))
        );
        assert_eq!(
            choose(2, MenuItem::CloseOthers).0,
            Some(Output::CloseRequested(vec![1, 4]))
        );
        assert_eq!(
            choose(1, MenuItem::CloseToTheRight).0,
            Some(Output::CloseRequested(vec![2, 4]))
        );
        assert_eq!(
            choose(1, MenuItem::CloseSaved).0,
            Some(Output::CloseRequested(vec![1, 4]))
        );
        assert_eq!(
            choose(1, MenuItem::CloseAll).0,
            Some(Output::CloseRequested(vec![1, 2, 4]))
        );
        assert_eq!(
            choose(4, MenuItem::Pin),
            (Some(Output::Pinned(4, true)), vec![3, 4, 1, 2])
        );
        assert_eq!(choose(3, MenuItem::Unpin).0, Some(Output::Pinned(3, false)));
        assert_eq!(
            choose(2, MenuItem::Custom("copy")).0,
            Some(Output::Custom("copy", 2))
        );
    }

    #[test]
    fn the_overflow_menu_lists_every_document_and_activates_one() {
        let mut state = state();
        let _ = state.update(Event::Overflow(dropdown_menu::Event::Toggle));
        assert!(state.overflow().is_open());
        assert_eq!(state.overflow().entries().len(), 4);
        assert!(state.overflow().is_checked(3));
        let lib = state.overflow().item(2).cloned();
        assert_eq!(lib.and_then(|item| item.shortcut), Some("Modified".into()));
        assert_eq!(
            state.update(Event::Overflow(dropdown_menu::Event::Activate(4))),
            Some(Output::Activated(4))
        );
        assert!(!state.overflow().is_open());
        assert_eq!(state.update(Event::New), Some(Output::NewTab));
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Ctrl+Tab"), Some(Action::RecentNext));
        assert_eq!(
            press(&keymap, "Ctrl+Shift+Tab"),
            Some(Action::RecentPrevious)
        );
        assert_eq!(press(&keymap, "Ctrl+PageDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "Ctrl+PageUp"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Ctrl+Shift+PageUp"), Some(Action::MoveLeft));
        assert_eq!(
            press(&keymap, "Ctrl+Shift+PageDown"),
            Some(Action::MoveRight)
        );
        assert_eq!(press(&keymap, "Mod+W"), Some(Action::Close));
        assert_eq!(press(&keymap, "Mod+N"), Some(Action::New));
        assert_eq!(press(&keymap, "Mod+1"), Some(Action::Tab(1)));
        assert_eq!(press(&keymap, "Mod+8"), Some(Action::Tab(8)));
        assert_eq!(press(&keymap, "Mod+9"), Some(Action::LastTab));
        assert_eq!(press(&keymap, "Tab"), None);
        assert_eq!(press(&keymap, "PageDown"), None);
        for action in <Action as keys::Action>::ALL {
            let unbound = *action == Action::OpenMenu;
            assert_eq!(keymap.chords(action).is_empty(), unbound, "{action:?}");
            assert!(keys::Action::description(*action).ends_with('.'));
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn opening_the_tab_menu_is_left_to_context_menus_by_default() {
        let keymap = default_keymap();
        for chord in ["Shift+F10", "Menu"] {
            assert_eq!(press(&keymap, chord), None, "{chord}");
            let context = context_menu::default_keymap();
            assert_eq!(
                press_menu(&context, chord),
                Some(context_menu::Action::Open)
            );
        }
        let bound = keymap.bind("Shift+F10".parse().unwrap(), Action::OpenMenu);
        assert_eq!(press(&bound, "Shift+F10"), Some(Action::OpenMenu));
    }

    fn press_menu(
        keymap: &Keymap<context_menu::Action>,
        chord: &str,
    ) -> Option<context_menu::Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_leaves_plain_and_alt_keys_to_other_components() {
        let keymap = default_keymap();
        for chord in ["ArrowLeft", "ArrowRight", "Home", "End", "Alt+1", "Escape"] {
            assert_eq!(press(&keymap, chord), None, "{chord}");
        }
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&"Ctrl+Tab".parse().unwrap())
            .bind("Alt+PageDown".parse().unwrap(), Action::Next);
        assert_eq!(press(&keymap, "Ctrl+Tab"), None);
        assert_eq!(press(&keymap, "Alt+PageDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "Ctrl+PageDown"), Some(Action::Next));
        let event = keys::Event {
            key: Key::Character("w".into()),
            modifiers: Modifiers::COMMAND,
        };
        assert_eq!(keymap.resolve_event(&event), Some(Action::Close));
    }

    #[test]
    fn actions_map_to_events() {
        let state = state().with_active(1);
        assert_eq!(Action::RecentNext.event(&state), Some(Event::RecentNext));
        assert_eq!(
            Action::RecentPrevious.event(&state),
            Some(Event::RecentPrevious)
        );
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(Action::Previous.event(&state), Some(Event::Previous));
        assert_eq!(Action::MoveLeft.event(&state), Some(Event::Move(1, 0)));
        assert_eq!(Action::MoveRight.event(&state), Some(Event::Move(1, 2)));
        assert_eq!(Action::Close.event(&state), Some(Event::Close(1)));
        assert_eq!(Action::New.event(&state), Some(Event::New));
        assert_eq!(Action::Tab(1).event(&state), Some(Event::Activate(3)));
        assert_eq!(Action::Tab(4).event(&state), Some(Event::Activate(4)));
        assert_eq!(Action::Tab(5).event(&state), None);
        assert_eq!(Action::Tab(0).event(&state), None);
        assert_eq!(Action::LastTab.event(&state), Some(Event::Activate(4)));
        assert_eq!(
            Action::OpenMenu.event(&state),
            Some(Event::Menu(context_menu::Event::OpenFromKeyboard(1)))
        );
        let first = state.clone().with_active(3);
        assert_eq!(Action::MoveLeft.event(&first), None);

        let mut state = state;
        let event = Action::MoveRight.event(&state).unwrap();
        assert!(state.update(event).is_some());
        assert_eq!(ids(&state), [3, 2, 1, 4]);
    }

    #[test]
    fn gaps_follow_tab_centres() {
        let tabs = [(0.0, 100.0), (100.0, 60.0), (160.0, 80.0)];
        assert_eq!(gap_at(&tabs, -5.0), 0);
        assert_eq!(gap_at(&tabs, 49.0), 0);
        assert_eq!(gap_at(&tabs, 51.0), 1);
        assert_eq!(gap_at(&tabs, 131.0), 2);
        assert_eq!(gap_at(&tabs, 500.0), 3);
    }

    #[test]
    fn gaps_are_kept_within_the_group() {
        assert_eq!(clamp_gap(0, 4, 2, 5), 2);
        assert_eq!(clamp_gap(3, 0, 2, 5), 2);
        assert_eq!(clamp_gap(3, 5, 2, 5), 5);
        assert_eq!(clamp_gap(1, 0, 0, 3), 0);
    }

    #[test]
    fn dropping_next_to_itself_does_not_move() {
        assert_eq!(drop_index(2, 2), None);
        assert_eq!(drop_index(2, 3), None);
        assert_eq!(drop_index(2, 0), Some(0));
        assert_eq!(drop_index(0, 3), Some(2));
    }

    #[test]
    fn memory_reveals_and_clamps() {
        let mut memory = Memory {
            content_width: 600.0,
            viewport_width: 200.0,
            ..Memory::default()
        };
        memory.reveal(450.0, 550.0);
        assert_eq!(memory.offset, 350.0);
        memory.reveal(100.0, 180.0);
        assert_eq!(memory.offset, 100.0);
        assert!(memory.scroll_by(1000.0));
        assert_eq!(memory.offset, 400.0);
        assert!(!memory.scroll_by(10.0));
        assert!(memory.scroll_by(-1000.0));
        assert_eq!(memory.offset, 0.0);
    }

    /// The offset after laying the strip out at each viewport width in
    /// turn, with the active tab at 450 to 550 of 600 pixels of tabs.
    fn offset_after(widths: &[f32]) -> f32 {
        let mut memory = Memory::default();
        for width in widths {
            memory.follow((Some(4), 6), (600.0, *width), Some((450.0, 100.0)));
        }
        memory.offset
    }

    #[test]
    fn the_first_offset_does_not_depend_on_earlier_sizes() {
        assert_eq!(offset_after(&[300.0]), 250.0, "active tab ends at the edge");
        assert_eq!(offset_after(&[1000.0, 300.0]), 250.0);
        assert_eq!(offset_after(&[0.0, 300.0]), 250.0);
        assert_eq!(offset_after(&[200.0, 300.0]), 250.0);
        assert_eq!(offset_after(&[200.0, 300.0, 300.0]), 250.0);
    }

    #[test]
    fn a_new_active_tab_scrolls_the_least_distance_and_keeps_wheel_scrolling() {
        let mut memory = Memory::default();
        memory.follow((Some(4), 6), (600.0, 300.0), Some((450.0, 100.0)));
        memory.follow((Some(3), 6), (600.0, 300.0), Some((350.0, 100.0)));
        assert_eq!(memory.offset, 250.0, "already in view");
        memory.follow((Some(0), 6), (600.0, 300.0), Some((0.0, 100.0)));
        assert_eq!(memory.offset, 0.0);
        let _ = memory.scroll_by(120.0);
        memory.follow((Some(0), 6), (600.0, 300.0), Some((0.0, 100.0)));
        assert_eq!(memory.offset, 120.0, "a relayout leaves the wheel offset");
    }

    #[test]
    fn pointer_opens_place_the_menu_below_the_tab() {
        let mut state = state();
        let _ = state.update(Event::Menu(context_menu::Event::Open(
            1,
            Point::new(40.0, 12.0),
        )));
        assert!(state.menu().is_open_on(&1));
        assert_eq!(
            state.menu().position(),
            Some(Point::new(0.0, HEIGHT + KEYBOARD_GAP))
        );
        assert_eq!(state.menu().menu().highlighted(), None);
    }

    #[test]
    fn active_tab_joins_the_page_and_others_sit_on_the_strip() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let active = tab_style(&tokens, true, false);
            let idle = tab_style(&tokens, false, false);
            let hovered = tab_style(&tokens, false, true);
            assert_eq!(
                active.background,
                Some(Background::Color(tokens.background))
            );
            assert_eq!(idle.background, None);
            assert!(hovered.background.is_some());
            assert_ne!(hovered.background, active.background);
            assert_eq!(active.text_color, Some(tokens.foreground));
            assert_eq!(idle.text_color, Some(tokens.muted_foreground));
            assert_eq!(hovered.text_color, Some(tokens.foreground));
            assert_eq!(active.border.radius.bottom_left, 0.0);
            assert_eq!(active.border.radius.top_left, radius::MD);
            assert_eq!(strip_style(&tokens).background, tokens.muted);
            assert_ne!(strip_style(&tokens).background, tokens.background);
        }
    }

    #[test]
    fn disabled_labels_fade() {
        let tokens = Tokens::of(&light());
        let on = label_colour(&tokens, true, false, true);
        let off = label_colour(&tokens, true, false, false);
        assert!(off.a < on.a);
    }

    #[test]
    fn close_button_shows_a_surface_on_hover_only() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert!(close_style(&tokens, Status::Active).background.is_none());
            assert!(close_style(&tokens, Status::Disabled).background.is_none());
            assert!(close_style(&tokens, Status::Hovered).background.is_some());
            assert!(close_style(&tokens, Status::Pressed).background.is_some());
        }
    }

    #[test]
    fn modified_dot_is_stronger_on_the_active_tab() {
        let tokens = Tokens::of(&dark());
        assert_eq!(
            dot_style(&tokens, true).background,
            Some(Background::Color(tokens.foreground))
        );
        assert_eq!(
            dot_style(&tokens, false).background,
            Some(Background::Color(tokens.muted_foreground))
        );
    }

    #[test]
    fn slots_show_close_dot_or_space() {
        let look = |active, hovered| Look {
            active,
            hovered,
            enabled: true,
        };
        let saved = document(1, "a");
        let modified = document(1, "a").modified(true);
        let pinned = document(1, "a").pinned(true);
        let has_slot =
            |document: &Document<u8>, look| slot::<u8, ()>(document, look, None).is_some();
        assert!(has_slot(&saved, look(false, false)));
        assert!(has_slot(&modified, look(true, false)));
        assert!(!has_slot(&pinned, look(true, true)));
        assert!(has_slot(&pinned.clone().modified(true), look(false, false)));
    }

    #[test]
    fn builder_defaults() {
        let state = state();
        let tabs: DocumentTabs<'_, u8, ()> = document_tabs(&state);
        assert!(!tabs.new_tab);
        assert_eq!(tabs.max_label_width, MAX_LABEL_WIDTH);
        assert_eq!(tabs.context_menu_keymap, context_menu::default_keymap());
        assert!(!tabs.is_enabled());
        let tabs: DocumentTabs<'_, u8, ()> = document_tabs(&state)
            .new_tab(true)
            .max_label_width(120.0)
            .context_menu_keymap(Keymap::new())
            .overflow_keymap(Keymap::new())
            .on_event(|_| ());
        assert!(tabs.new_tab);
        assert_eq!(tabs.max_label_width, 120.0);
        assert!(tabs.context_menu_keymap.is_empty());
        assert!(tabs.overflow_keymap.is_empty());
        assert!(tabs.is_enabled());
    }
}
