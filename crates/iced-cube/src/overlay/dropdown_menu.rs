//! A menu of actions that opens from a trigger button.
//!
//! [`State`] owns the entries (items, checkbox and radio items, submenus,
//! group labels and separators), whether the menu is open, and which row is
//! highlighted at each open level. Its [`update`](State::update) handles
//! pointer and keyboard navigation and returns an [`Output`] when an item
//! is chosen. The context menu uses the same model and rows.
//!
//! An open menu resolves its own keys through a [`Keymap`] of [`Action`]s,
//! so the app needs no subscription while it is open; see
//! [`default_keymap`]. A closed menu claims no keys: it opens from its
//! trigger, or from a chord the app binds to [`Action::Open`] and routes
//! through [`State::key_event`]. Rows are drawn in the shared
//! [menu look](crate::overlay::menu).

use std::rc::Rc;

use iced::Element;
use iced::keyboard::{Key, key::Named};
use iced::widget::column;

use crate::icon::Glyph;
use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{Align, Placement, Side, anchored};
use crate::overlay::menu::{self, Leading, Parts, RowStatus, Trailing};
use crate::theme::space;

/// Default menu width in logical pixels.
pub const WIDTH: f32 = 224.0;

/// What an item does when chosen.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind<Id> {
    /// A plain action.
    Action,
    /// A check mark that toggles.
    Checkbox(bool),
    /// One choice among the radio items next to it. A run of adjacent
    /// radio items forms one group.
    Radio(bool),
    /// Opens a nested menu.
    Submenu(Vec<Entry<Id>>),
}

/// A row that can be highlighted and chosen.
#[derive(Debug, Clone, PartialEq)]
pub struct Item<Id> {
    pub id: Id,
    pub label: String,
    pub icon: Option<Glyph>,
    /// A hint such as `Ctrl+S`, shown on the right. It does not bind the keys.
    pub shortcut: Option<String>,
    pub disabled: bool,
    pub destructive: bool,
    pub kind: Kind<Id>,
}

impl<Id> Item<Id> {
    fn enabled(&self) -> bool {
        !self.disabled
    }

    fn submenu(&self) -> Option<&[Entry<Id>]> {
        match &self.kind {
            Kind::Submenu(entries) => Some(entries),
            _ => None,
        }
    }
}

/// One line of a menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry<Id> {
    Item(Item<Id>),
    /// A heading for the items below it.
    Label(String),
    Separator,
}

fn new_item<Id>(id: Id, label: impl Into<String>, kind: Kind<Id>) -> Entry<Id> {
    Entry::Item(Item {
        id,
        label: label.into(),
        icon: None,
        shortcut: None,
        disabled: false,
        destructive: false,
        kind,
    })
}

/// An action item.
pub fn item<Id>(id: Id, label: impl Into<String>) -> Entry<Id> {
    new_item(id, label, Kind::Action)
}

/// An item with a check mark that toggles when chosen.
pub fn checkbox_item<Id>(id: Id, label: impl Into<String>, checked: bool) -> Entry<Id> {
    new_item(id, label, Kind::Checkbox(checked))
}

/// An item that checks itself and unchecks the radio items next to it.
pub fn radio_item<Id>(id: Id, label: impl Into<String>, checked: bool) -> Entry<Id> {
    new_item(id, label, Kind::Radio(checked))
}

/// An item that opens `entries` in a nested menu to its right.
pub fn submenu<Id>(
    id: Id,
    label: impl Into<String>,
    entries: impl IntoIterator<Item = Entry<Id>>,
) -> Entry<Id> {
    new_item(id, label, Kind::Submenu(entries.into_iter().collect()))
}

/// A heading for a group of items.
pub fn group_label<Id>(label: impl Into<String>) -> Entry<Id> {
    Entry::Label(label.into())
}

/// A line between groups.
pub fn separator<Id>() -> Entry<Id> {
    Entry::Separator
}

impl<Id> Entry<Id> {
    /// Adds a leading Lucide icon. Ignored on labels and separators, as are
    /// the other item settings.
    pub fn icon(self, glyph: Glyph) -> Self {
        self.with_item(|item| item.icon = Some(glyph))
    }

    pub fn shortcut(self, hint: impl Into<String>) -> Self {
        let hint = hint.into();
        self.with_item(|item| item.shortcut = Some(hint))
    }

    pub fn disabled(self, disabled: bool) -> Self {
        self.with_item(|item| item.disabled = disabled)
    }

    /// Draws the item in the destructive colour, for actions such as delete.
    pub fn destructive(self, destructive: bool) -> Self {
        self.with_item(|item| item.destructive = destructive)
    }

    pub fn as_item(&self) -> Option<&Item<Id>> {
        match self {
            Entry::Item(item) => Some(item),
            _ => None,
        }
    }

    fn with_item(mut self, change: impl FnOnce(&mut Item<Id>)) -> Self {
        if let Entry::Item(item) = &mut self {
            change(item);
        }
        self
    }

    fn enabled_item(&self) -> Option<&Item<Id>> {
        self.as_item().filter(|item| item.enabled())
    }
}

/// Changes to a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event<Id> {
    /// Opens with nothing highlighted, as a pointer does.
    Open,
    Close,
    Toggle,
    /// The pointer moved onto an item. A submenu item opens its submenu.
    Highlight(Id),
    /// Highlights the next enabled item, wrapping. Opens a closed menu on
    /// the first item.
    Next,
    /// Highlights the previous enabled item, wrapping. Opens a closed menu
    /// on the last item.
    Previous,
    First,
    Last,
    /// Opens the highlighted submenu and highlights its first item.
    OpenSubmenu,
    /// Closes the innermost submenu.
    CloseSubmenu,
    /// Chooses an item, as a click does.
    Activate(Id),
    /// Chooses the highlighted item, or opens its submenu. Opens a closed
    /// menu on the first item.
    ActivateHighlighted,
    /// Highlights the next item whose label starts with this character.
    Typeahead(char),
}

/// What choosing an item did. The menu closes afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output<Id> {
    /// An action item was chosen.
    Activated(Id),
    /// A checkbox item changed to this value.
    Toggled(Id, bool),
    /// A radio item was chosen.
    Selected(Id),
}

/// The entries of a menu, whether it is open and what is highlighted.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    entries: Vec<Entry<Id>>,
    /// One slot per open level: the root menu, then each open submenu.
    levels: Vec<Option<usize>>,
}

impl<Id: Copy + PartialEq> State<Id> {
    /// A closed menu. Item ids must be unique across submenus.
    pub fn new(entries: impl IntoIterator<Item = Entry<Id>>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
            levels: Vec::new(),
        }
    }

    pub fn entries(&self) -> &[Entry<Id>] {
        &self.entries
    }

    pub fn is_open(&self) -> bool {
        !self.levels.is_empty()
    }

    /// How many menus are open: 0 when closed, 1 for the root menu, plus
    /// one per open submenu.
    pub fn depth(&self) -> usize {
        self.levels.len()
    }

    /// The highlighted item in the menu that has keyboard focus.
    pub fn highlighted(&self) -> Option<Id> {
        let path = self.active_path()?;
        self.item_at(&path).map(|item| item.id)
    }

    /// The highlighted row index in the menu `depth` levels deep.
    pub fn highlighted_at(&self, depth: usize) -> Option<usize> {
        self.levels.get(depth).copied().flatten()
    }

    pub fn item(&self, id: Id) -> Option<&Item<Id>> {
        let path = path_of(&self.entries, id)?;
        self.item_at(&path)
    }

    /// Whether a checkbox or radio item is checked.
    pub fn is_checked(&self, id: Id) -> bool {
        self.item(id)
            .is_some_and(|item| matches!(item.kind, Kind::Checkbox(true) | Kind::Radio(true)))
    }

    /// Checks or unchecks an item. Checking a radio item unchecks its group.
    pub fn set_checked(&mut self, id: Id, checked: bool) {
        let Some(path) = path_of(&self.entries, id) else {
            return;
        };
        let Some((&index, parents)) = path.split_last() else {
            return;
        };
        let Some(siblings) = entries_mut(&mut self.entries, parents) else {
            return;
        };
        let is_radio = matches!(
            siblings.get(index),
            Some(Entry::Item(Item {
                kind: Kind::Radio(_),
                ..
            }))
        );
        if is_radio && checked {
            for sibling in radio_group(siblings, index) {
                if let Some(Entry::Item(item)) = siblings.get_mut(sibling) {
                    item.kind = Kind::Radio(false);
                }
            }
        }
        if let Some(Entry::Item(item)) = siblings.get_mut(index) {
            match &mut item.kind {
                Kind::Checkbox(value) | Kind::Radio(value) => *value = checked,
                Kind::Action | Kind::Submenu(_) => {}
            }
        }
    }

    /// Enables or disables an item. A disabled item loses its highlight.
    pub fn set_disabled(&mut self, id: Id, disabled: bool) {
        let Some(path) = path_of(&self.entries, id) else {
            return;
        };
        let Some((&index, parents)) = path.split_last() else {
            return;
        };
        if let Some(Entry::Item(item)) =
            entries_mut(&mut self.entries, parents).and_then(|entries| entries.get_mut(index))
        {
            item.disabled = disabled;
        }
        let highlighted = self
            .levels
            .iter()
            .take(path.len())
            .copied()
            .eq(path.iter().copied().map(Some));
        if disabled && highlighted {
            self.levels.truncate(parents.len() + 1);
            if let Some(level) = self.levels.get_mut(parents.len()) {
                *level = None;
            }
        }
    }

    /// Applies an event and returns what was chosen, if anything.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Open => self.open(),
            Event::Close => self.levels.clear(),
            Event::Toggle if self.is_open() => self.levels.clear(),
            Event::Toggle => self.open(),
            Event::Highlight(id) => self.highlight(id),
            Event::Next | Event::First if !self.is_open() => self.open_on(Edge::First),
            Event::Previous | Event::Last if !self.is_open() => self.open_on(Edge::Last),
            Event::Next => self.step(Step::Next),
            Event::Previous => self.step(Step::Previous),
            Event::First => self.step(Step::Edge(Edge::First)),
            Event::Last => self.step(Step::Edge(Edge::Last)),
            Event::OpenSubmenu => self.open_submenu(),
            Event::CloseSubmenu => self.close_submenu(),
            Event::Typeahead(character) => self.typeahead(character),
            Event::Activate(id) => return self.activate(id),
            Event::ActivateHighlighted if !self.is_open() => self.open_on(Edge::First),
            Event::ActivateHighlighted => return self.activate_highlighted(),
        }
        None
    }

    /// Turns a key press into an event: first through `keymap`, then, while
    /// the menu is open, as typeahead for a letter or digit. An open menu
    /// does this itself; an app calls it for a closed menu's
    /// [`Action::Open`] chord.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event<Id>> {
        if let Some(action) = keymap.resolve_event(key) {
            return action.event(self);
        }
        if !self.is_open() {
            return None;
        }
        typeahead(key).map(Event::Typeahead)
    }

    fn open(&mut self) {
        if !self.is_open() {
            self.levels.push(None);
        }
    }

    fn open_on(&mut self, edge: Edge) {
        self.levels = vec![edge.find(&self.entries)];
    }

    /// The deepest level with a highlight, or the root.
    fn active_depth(&self) -> usize {
        self.levels.iter().rposition(Option::is_some).unwrap_or(0)
    }

    fn active_path(&self) -> Option<Vec<usize>> {
        let depth = self.active_depth();
        self.levels.get(..=depth)?.iter().copied().collect()
    }

    fn entries_at(&self, depth: usize) -> Option<&[Entry<Id>]> {
        let mut entries = self.entries.as_slice();
        for level in self.levels.get(..depth)? {
            entries = entries.get((*level)?)?.as_item()?.submenu()?;
        }
        Some(entries)
    }

    fn item_at(&self, path: &[usize]) -> Option<&Item<Id>> {
        let (&index, parents) = path.split_last()?;
        let mut entries = self.entries.as_slice();
        for &parent in parents {
            entries = entries.get(parent)?.as_item()?.submenu()?;
        }
        entries.get(index)?.as_item()
    }

    /// Moves the highlight in the active menu, closing deeper submenus.
    fn step(&mut self, step: Step) {
        let depth = self.active_depth();
        let Some(entries) = self.entries_at(depth) else {
            return;
        };
        let current = self.highlighted_at(depth);
        let target = match step {
            Step::Next => next_enabled(entries, current, true, |_| true),
            Step::Previous => next_enabled(entries, current, false, |_| true),
            Step::Edge(edge) => edge.find(entries),
        };
        self.set_highlight(depth, target);
    }

    fn typeahead(&mut self, character: char) {
        let depth = self.active_depth();
        let Some(entries) = self.entries_at(depth) else {
            return;
        };
        let wanted = character.to_lowercase().collect::<String>();
        let target = next_enabled(entries, self.highlighted_at(depth), true, |item| {
            item.label.to_lowercase().starts_with(&wanted)
        });
        self.set_highlight(depth, target);
    }

    fn set_highlight(&mut self, depth: usize, target: Option<usize>) {
        if target.is_none() {
            return;
        }
        self.levels.truncate(depth + 1);
        if let Some(level) = self.levels.get_mut(depth) {
            *level = target;
        }
    }

    fn highlight(&mut self, id: Id) {
        let Some(path) = path_of(&self.entries, id) else {
            return;
        };
        let Some(item) = self.item_at(&path).filter(|item| item.enabled()) else {
            return;
        };
        let opens = item.submenu().is_some();
        let target = some(&path);
        if self.levels.len() > target.len() && self.levels.starts_with(&target) {
            return;
        }
        self.levels = target;
        if opens {
            self.levels.push(None);
        }
    }

    fn open_submenu(&mut self) {
        let depth = self.active_depth();
        let Some(path) = self.active_path() else {
            return;
        };
        let Some(children) = self
            .item_at(&path)
            .filter(|item| item.enabled())
            .and_then(Item::submenu)
        else {
            return;
        };
        let first = Edge::First.find(children);
        self.levels.truncate(depth + 1);
        self.levels.push(first);
    }

    fn close_submenu(&mut self) {
        if self.levels.len() <= 1 {
            return;
        }
        let keep = self.active_depth().max(1);
        self.levels.truncate(keep);
    }

    fn activate(&mut self, id: Id) -> Option<Output<Id>> {
        let path = path_of(&self.entries, id)?;
        let item = self.item_at(&path).filter(|item| item.enabled())?;
        if item.submenu().is_some() {
            self.highlight(id);
            return None;
        }
        self.choose(id)
    }

    fn activate_highlighted(&mut self) -> Option<Output<Id>> {
        let path = self.active_path()?;
        let item = self.item_at(&path).filter(|item| item.enabled())?;
        if item.submenu().is_some() {
            self.open_submenu();
            return None;
        }
        let id = item.id;
        self.choose(id)
    }

    /// Applies a chosen item and closes the menu.
    fn choose(&mut self, id: Id) -> Option<Output<Id>> {
        let output = match self.item(id)?.kind {
            Kind::Action => Output::Activated(id),
            Kind::Checkbox(checked) => {
                self.set_checked(id, !checked);
                Output::Toggled(id, !checked)
            }
            Kind::Radio(_) => {
                self.set_checked(id, true);
                Output::Selected(id)
            }
            Kind::Submenu(_) => return None,
        };
        self.levels.clear();
        Some(output)
    }
}

fn some(path: &[usize]) -> Vec<Option<usize>> {
    path.iter().copied().map(Some).collect()
}

#[derive(Debug, Clone, Copy)]
enum Edge {
    First,
    Last,
}

impl Edge {
    fn find<Id>(self, entries: &[Entry<Id>]) -> Option<usize> {
        let enabled = |(_, entry): &(usize, &Entry<Id>)| entry.enabled_item().is_some();
        match self {
            Edge::First => entries.iter().enumerate().find(enabled),
            Edge::Last => entries.iter().enumerate().rev().find(enabled),
        }
        .map(|(index, _)| index)
    }
}

#[derive(Debug, Clone, Copy)]
enum Step {
    Next,
    Previous,
    Edge(Edge),
}

/// The nearest enabled item after (or before) `from` that `accept`s,
/// wrapping round. With nothing highlighted, the search starts at an end.
fn next_enabled<Id>(
    entries: &[Entry<Id>],
    from: Option<usize>,
    forward: bool,
    accept: impl Fn(&Item<Id>) -> bool,
) -> Option<usize> {
    let len = entries.len();
    if len == 0 {
        return None;
    }
    let start = from.unwrap_or(if forward { len - 1 } else { 0 });
    let offset = if forward { 1 } else { len - 1 };
    (1..=len)
        .map(|n| (start + offset * n) % len)
        .find(|&index| {
            entries
                .get(index)
                .and_then(Entry::enabled_item)
                .is_some_and(&accept)
        })
}

/// The index path to an item, through submenus.
fn path_of<Id: Copy + PartialEq>(entries: &[Entry<Id>], id: Id) -> Option<Vec<usize>> {
    entries.iter().enumerate().find_map(|(index, entry)| {
        let item = entry.as_item()?;
        if item.id == id {
            return Some(vec![index]);
        }
        let mut path = path_of(item.submenu()?, id)?;
        path.insert(0, index);
        Some(path)
    })
}

fn entries_mut<'a, Id>(
    entries: &'a mut Vec<Entry<Id>>,
    parents: &[usize],
) -> Option<&'a mut Vec<Entry<Id>>> {
    let Some((&first, rest)) = parents.split_first() else {
        return Some(entries);
    };
    match entries.get_mut(first)? {
        Entry::Item(Item {
            kind: Kind::Submenu(children),
            ..
        }) => entries_mut(children, rest),
        _ => None,
    }
}

/// The indices of the run of radio items around `index`.
fn radio_group<Id>(entries: &[Entry<Id>], index: usize) -> std::ops::Range<usize> {
    let is_radio = |entry: &Entry<Id>| {
        matches!(
            entry,
            Entry::Item(Item {
                kind: Kind::Radio(_),
                ..
            })
        )
    };
    let start = entries
        .get(..index)
        .and_then(|before| before.iter().rposition(|entry| !is_radio(entry)))
        .map_or(0, |position| position + 1);
    let end = entries
        .get(index..)
        .and_then(|after| after.iter().position(|entry| !is_radio(entry)))
        .map_or(entries.len(), |position| index + position);
    start..end
}

/// The character a key press types into the menu's typeahead: a letter or
/// digit pressed without Ctrl, Alt or the logo key.
pub fn typeahead(key: &keys::Event) -> Option<char> {
    let modifiers = key.modifiers;
    if modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }
    let Key::Character(text) = &key.key else {
        return None;
    };
    let mut chars = text.chars();
    let character = chars.next()?;
    if chars.next().is_some() || !character.is_alphanumeric() {
        return None;
    }
    Some(character)
}

/// What a menu keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Opens a closed menu on its first item. Not bound by default.
    Open,
    Next,
    Previous,
    First,
    Last,
    Activate,
    Close,
    OpenSubmenu,
    CloseSubmenu,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing: only [`Action::Open`] works while the menu is closed, and
    /// only the others while it is open. [`Action::Close`] closes just the
    /// innermost submenu when one is open.
    pub fn event<Id: Copy + PartialEq>(self, state: &State<Id>) -> Option<Event<Id>> {
        if !state.is_open() {
            return (self == Action::Open).then_some(Event::First);
        }
        match self {
            Action::Open => None,
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::First => Some(Event::First),
            Action::Last => Some(Event::Last),
            Action::Activate => Some(Event::ActivateHighlighted),
            Action::Close if state.depth() > 1 => Some(Event::CloseSubmenu),
            Action::Close => Some(Event::Close),
            Action::OpenSubmenu => Some(Event::OpenSubmenu),
            Action::CloseSubmenu => Some(Event::CloseSubmenu),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Open,
        Action::Next,
        Action::Previous,
        Action::First,
        Action::Last,
        Action::Activate,
        Action::Close,
        Action::OpenSubmenu,
        Action::CloseSubmenu,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Open => "Open",
            Action::Next => "Next",
            Action::Previous => "Previous",
            Action::First => "First",
            Action::Last => "Last",
            Action::Activate => "Activate",
            Action::Close => "Close",
            Action::OpenSubmenu => "OpenSubmenu",
            Action::CloseSubmenu => "CloseSubmenu",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Open => "Opens the menu on its first item.",
            Action::Next => "Highlights the next enabled item, wrapping.",
            Action::Previous => "Highlights the previous enabled item, wrapping.",
            Action::First => "Highlights the first enabled item.",
            Action::Last => "Highlights the last enabled item.",
            Action::Activate => "Chooses the highlighted item or opens its submenu.",
            Action::Close => "Closes the innermost submenu, or the menu.",
            Action::OpenSubmenu => "Opens the highlighted submenu.",
            Action::CloseSubmenu => "Closes the innermost submenu.",
        }
    }
}

/// The default menu shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
/// | `Enter`, `Space` | [`Action::Activate`] |
/// | `Escape` | [`Action::Close`] |
/// | `ArrowRight` | [`Action::OpenSubmenu`] |
/// | `ArrowLeft` | [`Action::CloseSubmenu`] |
///
/// Letters and digits jump to the next item starting with them. The open
/// menu handles all of these itself, before anything underneath sees
/// them; pass a changed keymap with [`DropdownMenu::keymap`]. A closed
/// menu claims none of them. [`Action::Open`] has no default chord
/// because a menu usually belongs to one button; bind one when a menu
/// deserves an app-wide shortcut.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
        .bind(Chord::named(Named::Enter), Action::Activate)
        .bind(Chord::named(Named::Space), Action::Activate)
        .bind(Chord::named(Named::Escape), Action::Close)
        .bind(Chord::named(Named::ArrowRight), Action::OpenSubmenu)
        .bind(Chord::named(Named::ArrowLeft), Action::CloseSubmenu)
}

/// A dropdown menu builder. Convert it into an [`Element`] to render.
pub struct DropdownMenu<'a, Id, Message> {
    state: &'a State<Id>,
    trigger: Element<'a, Message>,
    placement: Placement,
    width: f32,
    keymap: Keymap<Action>,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for DropdownMenu<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DropdownMenu")
            .field("state", self.state)
            .field("placement", &self.placement)
            .field("width", &self.width)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

/// Renders `trigger` with the menu of `state` below it while it is open.
///
/// The trigger opens the menu itself, usually a button whose message
/// carries [`Event::Toggle`]. Without [`on_event`](DropdownMenu::on_event)
/// every item renders disabled.
pub fn dropdown_menu<'a, Id, Message>(
    state: &'a State<Id>,
    trigger: impl Into<Element<'a, Message>>,
) -> DropdownMenu<'a, Id, Message> {
    DropdownMenu {
        state,
        trigger: trigger.into(),
        placement: Placement::new(Side::Bottom, Align::Start),
        width: WIDTH,
        keymap: default_keymap(),
        on_event: None,
    }
}

impl<'a, Id, Message> DropdownMenu<'a, Id, Message> {
    /// Which side of the trigger the menu opens on. Defaults to the bottom.
    pub fn side(mut self, side: Side) -> Self {
        self.placement.side = side;
        self
    }

    /// Defaults to the start, lining the menu up with the trigger's left edge.
    pub fn align(mut self, align: Align) -> Self {
        self.placement.align = align;
        self
    }

    /// Space between the trigger and the menu.
    pub fn gap(mut self, gap: f32) -> Self {
        self.placement.gap = gap;
        self
    }

    /// Width of the menu and its submenus.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Replaces the [`default_keymap`] the open menu resolves its keys with.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Id, Message> From<DropdownMenu<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(menu: DropdownMenu<'a, Id, Message>) -> Self {
        let DropdownMenu {
            state,
            trigger,
            placement,
            width,
            keymap,
            on_event,
        } = menu;
        let on_event: Option<Rc<dyn Fn(Event<Id>) -> Message + 'a>> = on_event.map(Rc::from);
        let panel = state
            .is_open()
            .then(|| panel(state, on_event.as_deref(), width));
        let anchored = anchored(trigger)
            .content(panel)
            .placement(placement)
            .dismiss_keys([]);
        let Some(on_event) = on_event.filter(|_| state.is_open()) else {
            return anchored.into();
        };
        anchored
            .on_dismiss(on_event(Event::Close))
            .on_key(move |key| state.key_event(&keymap, key).map(&*on_event))
            .into()
    }
}

/// The open menu: the root panel, with open submenus floating beside
/// their items.
pub(crate) fn panel<'a, Id, Message>(
    state: &'a State<Id>,
    on_event: Option<&dyn Fn(Event<Id>) -> Message>,
    width: f32,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    level(state, &state.entries, 0, on_event, width)
}

fn level<'a, Id, Message>(
    state: &'a State<Id>,
    entries: &'a [Entry<Id>],
    depth: usize,
    on_event: Option<&dyn Fn(Event<Id>) -> Message>,
    width: f32,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let highlighted = state.highlighted_at(depth);
    let submenu_open = state.depth() > depth + 1;
    let indented = has_marks(entries);
    let slot = reserves_slot(entries);

    let rows = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| match entry {
            Entry::Separator => menu::separator(),
            Entry::Label(label) => menu::group_label(label, indented),
            Entry::Item(item) => {
                let is_highlighted = highlighted == Some(index);
                let row = item_row(item, is_highlighted, slot, on_event);
                let Some(children) = item.submenu().filter(|_| is_highlighted && submenu_open)
                else {
                    return row;
                };
                // The root menu dismisses everything, so a submenu lets
                // outside presses through to it.
                anchored(row)
                    .content(Some(level(state, children, depth + 1, on_event, width)))
                    .placement(
                        Placement::new(Side::Right, Align::Start)
                            .gap(space::XS)
                            .offset(-(space::XS + 1.0)),
                    )
                    .pass_through(true)
                    .into()
            }
        });

    menu::surface(column(rows).width(width)).into()
}

/// Whether a menu level has checkbox or radio items, whose group labels
/// then line up with the item labels.
fn has_marks<Id>(entries: &[Entry<Id>]) -> bool {
    entries.iter().any(|entry| {
        matches!(
            entry.as_item().map(|item| &item.kind),
            Some(Kind::Checkbox(_) | Kind::Radio(_))
        )
    })
}

/// Whether every row of a menu level gets a leading slot, so labels line
/// up: when any item has a check mark, a radio dot or an icon.
fn reserves_slot<Id>(entries: &[Entry<Id>]) -> bool {
    has_marks(entries)
        || entries
            .iter()
            .any(|entry| entry.as_item().is_some_and(|item| item.icon.is_some()))
}

/// What fills an item's leading slot: its mark while checked, otherwise
/// its icon, otherwise nothing.
fn leading<Id>(item: &Item<Id>, slot: bool) -> Leading {
    match (&item.kind, slot) {
        (_, false) => Leading::None,
        (Kind::Checkbox(true), true) => Leading::Check,
        (Kind::Radio(true), true) => Leading::Dot,
        (_, true) => Leading::Empty,
    }
}

fn item_row<'a, Id, Message>(
    item: &'a Item<Id>,
    highlighted: bool,
    slot: bool,
    on_event: Option<&dyn Fn(Event<Id>) -> Message>,
) -> Element<'a, Message>
where
    Id: Copy + 'a,
    Message: Clone + 'a,
{
    let enabled = on_event.filter(|_| item.enabled());
    let status = match (enabled.is_some(), highlighted) {
        (false, _) => RowStatus::Disabled,
        (true, true) => RowStatus::Highlighted,
        (true, false) => RowStatus::Idle,
    };
    let leading = leading(item, slot);
    let trailing = if item.submenu().is_some() {
        Trailing::Chevron
    } else {
        Trailing::None
    };
    let parts = Parts {
        icon: item.icon,
        hint: item.shortcut.as_deref(),
        leading,
        trailing,
        ..Parts::label(&item.label)
    };
    let messages = enabled.map(|on_event| {
        (
            on_event(Event::Highlight(item.id)),
            on_event(Event::Activate(item.id)),
        )
    });
    menu::item(parts, status, item.destructive, messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::Modifiers;
    use iced::widget::text;

    const COPY: u8 = 1;
    const PASTE: u8 = 2;
    const DELETE: u8 = 3;
    const SHARE: u8 = 4;
    const EMAIL: u8 = 5;
    const LINK: u8 = 6;
    const GRID: u8 = 7;
    const SMALL: u8 = 8;
    const LARGE: u8 = 9;
    const PREVIEW: u8 = 10;

    fn state() -> State<u8> {
        State::new([
            group_label("Edit"),
            item(COPY, "Copy").shortcut("Ctrl+C"),
            item(PASTE, "Paste").disabled(true),
            item(DELETE, "Delete").destructive(true),
            separator(),
            submenu(
                SHARE,
                "Share",
                [item(EMAIL, "Email"), item(LINK, "Copy link")],
            ),
            checkbox_item(GRID, "Show grid", true),
            separator(),
            radio_item(SMALL, "Small", true),
            radio_item(LARGE, "Large", false),
            item(PREVIEW, "Preview"),
        ])
    }

    fn open() -> State<u8> {
        let mut state = state();
        let _ = state.update(Event::Open);
        state
    }

    #[test]
    fn builders_set_item_fields_and_ignore_labels() {
        let entry: Entry<u8> = item(1, "Save")
            .icon(crate::lucide!(Save))
            .shortcut("Ctrl+S")
            .disabled(true)
            .destructive(true);
        let item = entry.as_item().unwrap();
        assert_eq!(item.label, "Save");
        assert_eq!(item.icon, Some(crate::lucide!(Save)));
        assert_eq!(item.shortcut.as_deref(), Some("Ctrl+S"));
        assert!(item.disabled && item.destructive);
        assert_eq!(item.kind, Kind::Action);

        let label: Entry<u8> = group_label("Group")
            .icon(crate::lucide!(Save))
            .disabled(true);
        assert_eq!(label, Entry::Label("Group".into()));
        assert_eq!(separator::<u8>().shortcut("X"), Entry::Separator);
    }

    #[test]
    fn starts_closed_and_opens_without_a_highlight() {
        let mut state = state();
        assert!(!state.is_open());
        assert_eq!(state.depth(), 0);
        assert_eq!(state.update(Event::Open), None);
        assert!(state.is_open());
        assert_eq!(state.highlighted(), None);
        let _ = state.update(Event::Open);
        assert_eq!(state.depth(), 1);
    }

    #[test]
    fn toggle_and_close() {
        let mut state = state();
        let _ = state.update(Event::Toggle);
        assert!(state.is_open());
        let _ = state.update(Event::Toggle);
        assert!(!state.is_open());
        let _ = state.update(Event::Next);
        let _ = state.update(Event::Close);
        assert!(!state.is_open());
        assert_eq!(state.highlighted(), None);
    }

    #[test]
    fn next_skips_labels_separators_and_disabled_items_and_wraps() {
        let mut state = open();
        let mut seen = Vec::new();
        for _ in 0..8 {
            let _ = state.update(Event::Next);
            seen.push(state.highlighted().unwrap());
        }
        assert_eq!(
            seen,
            [COPY, DELETE, SHARE, GRID, SMALL, LARGE, PREVIEW, COPY]
        );
    }

    #[test]
    fn previous_wraps_backwards() {
        let mut state = open();
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(PREVIEW));
        let _ = state.update(Event::First);
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(PREVIEW));
    }

    #[test]
    fn first_and_last() {
        let mut state = open();
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(PREVIEW));
        let _ = state.update(Event::First);
        assert_eq!(state.highlighted(), Some(COPY));
    }

    #[test]
    fn moving_opens_a_closed_menu_on_an_end() {
        let mut down = state();
        let _ = down.update(Event::Next);
        assert!(down.is_open());
        assert_eq!(down.highlighted(), Some(COPY));

        let mut up = state();
        let _ = up.update(Event::Previous);
        assert_eq!(up.highlighted(), Some(PREVIEW));

        let mut enter = state();
        assert_eq!(enter.update(Event::ActivateHighlighted), None);
        assert_eq!(enter.highlighted(), Some(COPY));
    }

    #[test]
    fn empty_and_all_disabled_menus_never_highlight() {
        let mut empty: State<u8> = State::new([]);
        let _ = empty.update(Event::Next);
        assert!(empty.is_open());
        assert_eq!(empty.highlighted(), None);
        assert_eq!(empty.update(Event::ActivateHighlighted), None);

        let mut disabled = State::new([item(1u8, "A").disabled(true), separator()]);
        let _ = disabled.update(Event::Next);
        let _ = disabled.update(Event::Last);
        assert_eq!(disabled.highlighted(), None);
        assert_eq!(disabled.update(Event::Activate(1)), None);
    }

    #[test]
    fn activating_an_action_returns_it_and_closes() {
        let mut state = open();
        assert_eq!(
            state.update(Event::Activate(COPY)),
            Some(Output::Activated(COPY))
        );
        assert!(!state.is_open());
    }

    #[test]
    fn disabled_and_unknown_items_do_nothing() {
        let mut state = open();
        assert_eq!(state.update(Event::Activate(PASTE)), None);
        assert_eq!(state.update(Event::Activate(99)), None);
        let _ = state.update(Event::Highlight(PASTE));
        assert_eq!(state.highlighted(), None);
        assert!(state.is_open());
    }

    #[test]
    fn enter_activates_the_highlighted_item() {
        let mut state = open();
        let _ = state.update(Event::Next);
        let _ = state.update(Event::Next);
        assert_eq!(
            state.update(Event::ActivateHighlighted),
            Some(Output::Activated(DELETE))
        );
        let mut nothing = open();
        assert_eq!(nothing.update(Event::ActivateHighlighted), None);
        assert!(nothing.is_open());
    }

    #[test]
    fn checkbox_items_toggle() {
        let mut state = open();
        assert!(state.is_checked(GRID));
        assert_eq!(
            state.update(Event::Activate(GRID)),
            Some(Output::Toggled(GRID, false))
        );
        assert!(!state.is_checked(GRID));
        let _ = state.update(Event::Open);
        assert_eq!(
            state.update(Event::Activate(GRID)),
            Some(Output::Toggled(GRID, true))
        );
    }

    #[test]
    fn radio_items_check_one_of_their_run() {
        let mut state = open();
        assert_eq!(
            state.update(Event::Activate(LARGE)),
            Some(Output::Selected(LARGE))
        );
        assert!(state.is_checked(LARGE));
        assert!(!state.is_checked(SMALL));
        state.set_checked(SMALL, true);
        assert!(state.is_checked(SMALL));
        assert!(!state.is_checked(LARGE));
        state.set_checked(SMALL, false);
        assert!(!state.is_checked(SMALL));
    }

    #[test]
    fn radio_group_is_the_adjacent_run() {
        let entries: Vec<Entry<u8>> = vec![
            radio_item(1, "A", false),
            separator(),
            radio_item(2, "B", false),
            radio_item(3, "C", false),
            item(4, "D"),
            radio_item(5, "E", false),
        ];
        assert_eq!(radio_group(&entries, 0), 0..1);
        assert_eq!(radio_group(&entries, 2), 2..4);
        assert_eq!(radio_group(&entries, 3), 2..4);
        assert_eq!(radio_group(&entries, 5), 5..6);
    }

    #[test]
    fn set_checked_ignores_actions_and_unknown_ids() {
        let mut state = state();
        state.set_checked(COPY, true);
        state.set_checked(99, true);
        assert!(!state.is_checked(COPY));
        assert_eq!(state, self::state());
    }

    #[test]
    fn set_disabled_drops_the_highlight() {
        let mut state = open();
        let _ = state.update(Event::Highlight(DELETE));
        state.set_disabled(DELETE, true);
        assert_eq!(state.highlighted(), None);
        assert!(state.item(DELETE).unwrap().disabled);
        state.set_disabled(PASTE, false);
        let _ = state.update(Event::First);
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(PASTE));
    }

    #[test]
    fn disabling_an_item_in_another_submenu_keeps_the_active_highlight() {
        let mut state = State::new([
            submenu(1, "Share", [item(2, "Email")]),
            submenu(3, "Export", [item(4, "PDF")]),
        ]);
        let _ = state.update(Event::Open);
        let _ = state.update(Event::Highlight(2));

        state.set_disabled(4, true);

        assert_eq!(state.highlighted(), Some(2));
        assert_eq!(
            state.update(Event::ActivateHighlighted),
            Some(Output::Activated(2))
        );
    }

    #[test]
    fn pointer_on_a_submenu_opens_it_without_a_highlight() {
        let mut state = open();
        let _ = state.update(Event::Highlight(SHARE));
        assert_eq!(state.depth(), 2);
        assert_eq!(state.highlighted(), Some(SHARE));
        assert_eq!(state.highlighted_at(1), None);

        let _ = state.update(Event::Highlight(LINK));
        assert_eq!(state.highlighted(), Some(LINK));
        let _ = state.update(Event::Highlight(SHARE));
        assert_eq!(
            state.highlighted(),
            Some(LINK),
            "returning to the parent keeps the submenu"
        );

        let _ = state.update(Event::Highlight(COPY));
        assert_eq!(state.depth(), 1, "another item closes the submenu");
    }

    #[test]
    fn keyboard_opens_and_closes_submenus() {
        let mut state = open();
        let _ = state.update(Event::Highlight(SHARE));
        let _ = state.update(Event::CloseSubmenu);
        assert_eq!(state.depth(), 1);
        assert_eq!(state.highlighted(), Some(SHARE));

        let _ = state.update(Event::OpenSubmenu);
        assert_eq!(state.highlighted(), Some(EMAIL));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(LINK));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(EMAIL), "wraps inside the submenu");

        let _ = state.update(Event::CloseSubmenu);
        assert_eq!(state.depth(), 1);
        assert_eq!(state.highlighted(), Some(SHARE));
        let _ = state.update(Event::CloseSubmenu);
        assert!(state.is_open(), "the root menu stays open");
    }

    #[test]
    fn enter_on_a_submenu_opens_it_and_items_inside_activate() {
        let mut state = open();
        let _ = state.update(Event::Highlight(SHARE));
        let _ = state.update(Event::CloseSubmenu);
        assert_eq!(state.update(Event::ActivateHighlighted), None);
        assert_eq!(state.highlighted(), Some(EMAIL));
        assert_eq!(
            state.update(Event::ActivateHighlighted),
            Some(Output::Activated(EMAIL))
        );
        assert!(!state.is_open());
    }

    #[test]
    fn clicking_a_submenu_item_opens_it() {
        let mut state = open();
        assert_eq!(state.update(Event::Activate(SHARE)), None);
        assert_eq!(state.depth(), 2);
        assert_eq!(
            state.update(Event::Activate(LINK)),
            Some(Output::Activated(LINK))
        );
    }

    #[test]
    fn open_submenu_ignores_plain_items() {
        let mut state = open();
        let _ = state.update(Event::First);
        let _ = state.update(Event::OpenSubmenu);
        assert_eq!(state.depth(), 1);
    }

    #[test]
    fn moving_in_the_parent_closes_the_submenu() {
        let mut state = open();
        let _ = state.update(Event::Highlight(SHARE));
        let _ = state.update(Event::Next);
        assert_eq!(state.depth(), 1);
        assert_eq!(state.highlighted(), Some(GRID));
    }

    #[test]
    fn typeahead_jumps_to_matching_labels_and_cycles() {
        let mut state = open();
        let _ = state.update(Event::Typeahead('s'));
        assert_eq!(state.highlighted(), Some(SHARE));
        let _ = state.update(Event::Typeahead('S'));
        assert_eq!(state.highlighted(), Some(GRID));
        let _ = state.update(Event::Typeahead('s'));
        assert_eq!(state.highlighted(), Some(SMALL));
        let _ = state.update(Event::Typeahead('s'));
        assert_eq!(state.highlighted(), Some(SHARE));
        let _ = state.update(Event::Typeahead('p'));
        assert_eq!(
            state.highlighted(),
            Some(PREVIEW),
            "skips the disabled Paste"
        );
        let _ = state.update(Event::Typeahead('z'));
        assert_eq!(state.highlighted(), Some(PREVIEW));
    }

    #[test]
    fn typeahead_starts_at_the_first_match() {
        let mut state = open();
        let _ = state.update(Event::Typeahead('c'));
        assert_eq!(state.highlighted(), Some(COPY));
    }

    fn key(key: Key, modifiers: Modifiers) -> keys::Event {
        keys::Event { key, modifiers }
    }

    fn character(c: &str) -> Key {
        Key::Character(c.into())
    }

    #[test]
    fn typeahead_accepts_plain_letters_and_digits_only() {
        let none = Modifiers::empty();
        assert_eq!(typeahead(&key(character("a"), none)), Some('a'));
        assert_eq!(typeahead(&key(character("A"), Modifiers::SHIFT)), Some('A'));
        assert_eq!(typeahead(&key(character("7"), none)), Some('7'));
        assert_eq!(typeahead(&key(character("a"), Modifiers::CTRL)), None);
        assert_eq!(typeahead(&key(character("a"), Modifiers::ALT)), None);
        assert_eq!(typeahead(&key(character(" "), none)), None);
        assert_eq!(typeahead(&key(character("/"), none)), None);
        assert_eq!(typeahead(&key(character("ab"), none)), None);
        assert_eq!(typeahead(&key(Key::Named(Named::Enter), none)), None);
    }

    #[test]
    fn key_event_uses_the_keymap_then_typeahead() {
        let keymap = default_keymap();
        let closed = state();
        let down = key(Key::Named(Named::ArrowDown), Modifiers::empty());
        let enter = key(Key::Named(Named::Enter), Modifiers::empty());
        let letter = key(character("d"), Modifiers::empty());
        assert_eq!(
            closed.key_event(&keymap, &down),
            None,
            "closed menus claim no keys"
        );
        assert_eq!(closed.key_event(&keymap, &enter), None);
        assert_eq!(closed.key_event(&keymap, &letter), None);

        let bound = default_keymap().bind(Chord::named(Named::F2), Action::Open);
        let f2 = key(Key::Named(Named::F2), Modifiers::empty());
        assert_eq!(closed.key_event(&bound, &f2), Some(Event::First));

        let open = open();
        assert_eq!(
            open.key_event(&keymap, &letter),
            Some(Event::Typeahead('d'))
        );
        let escape = key(Key::Named(Named::Escape), Modifiers::empty());
        assert_eq!(open.key_event(&keymap, &escape), Some(Event::Close));
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowUp"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Home"), Some(Action::First));
        assert_eq!(press(&keymap, "End"), Some(Action::Last));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Space"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::OpenSubmenu));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::CloseSubmenu));
        assert_eq!(press(&keymap, "Tab"), None);
        for action in <Action as keys::Action>::ALL {
            let bound = !keymap.chords(action).is_empty();
            assert_eq!(bound, *action != Action::Open, "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&"Space".parse().unwrap())
            .bind("J".parse().unwrap(), Action::Next);
        assert_eq!(press(&keymap, "Space"), None);
        assert_eq!(press(&keymap, "j"), Some(Action::Next));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Activate));
    }

    #[test]
    fn a_closed_menu_only_answers_open() {
        let closed = state();
        assert_eq!(Action::Open.event(&closed), Some(Event::First));
        for &action in <Action as keys::Action>::ALL {
            if action != Action::Open {
                assert_eq!(action.event(&closed), None, "{action:?}");
            }
        }
    }

    #[test]
    fn an_open_menu_maps_every_action_but_open() {
        let open = open();
        assert_eq!(Action::Open.event(&open), None);
        assert_eq!(Action::Next.event(&open), Some(Event::Next));
        assert_eq!(Action::Previous.event(&open), Some(Event::Previous));
        assert_eq!(Action::First.event(&open), Some(Event::First));
        assert_eq!(Action::Last.event(&open), Some(Event::Last));
        assert_eq!(
            Action::Activate.event(&open),
            Some(Event::ActivateHighlighted)
        );
        assert_eq!(Action::Close.event(&open), Some(Event::Close));
        assert_eq!(Action::OpenSubmenu.event(&open), Some(Event::OpenSubmenu));
        assert_eq!(Action::CloseSubmenu.event(&open), Some(Event::CloseSubmenu));
    }

    #[test]
    fn close_in_a_submenu_closes_only_the_submenu() {
        let mut state = open();
        let _ = state.update(Event::Highlight(SHARE));
        assert_eq!(Action::Close.event(&state), Some(Event::CloseSubmenu));
        let _ = state.update(Event::CloseSubmenu);
        assert_eq!(Action::Close.event(&state), Some(Event::Close));
    }

    #[test]
    fn every_action_has_a_name_and_a_sentence() {
        for &action in <Action as keys::Action>::ALL {
            assert!(!keys::Action::name(action).is_empty());
            assert!(keys::Action::description(action).ends_with('.'));
        }
    }

    #[test]
    fn builder_defaults() {
        let state = state();
        let menu: DropdownMenu<'_, u8, ()> = dropdown_menu(&state, text("Open"));
        assert_eq!(menu.placement, Placement::new(Side::Bottom, Align::Start));
        assert_eq!(menu.width, WIDTH);
        assert_eq!(menu.keymap, default_keymap());
        assert!(menu.on_event.is_none());

        let menu: DropdownMenu<'_, u8, ()> = dropdown_menu(&state, text("Open"))
            .side(Side::Top)
            .align(Align::End)
            .gap(8.0)
            .width(300.0)
            .keymap(Keymap::new())
            .on_event(|_| ());
        assert_eq!(menu.placement.side, Side::Top);
        assert_eq!(menu.placement.align, Align::End);
        assert_eq!(menu.placement.gap, 8.0);
        assert_eq!(menu.width, 300.0);
        assert!(menu.keymap.is_empty());
        assert!(menu.on_event.is_some());
    }

    #[test]
    fn a_slot_is_reserved_for_marks_or_icons() {
        let plain: Vec<Entry<u8>> = vec![item(COPY, "Copy"), separator()];
        assert!(!reserves_slot(&plain) && !has_marks(&plain));

        let icons: Vec<Entry<u8>> = vec![
            item(COPY, "Copy").icon(crate::lucide!(Copy)),
            item(PASTE, "Paste"),
        ];
        assert!(reserves_slot(&icons) && !has_marks(&icons));

        let marks: Vec<Entry<u8>> = vec![item(COPY, "Copy"), radio_item(SMALL, "Small", false)];
        assert!(reserves_slot(&marks) && has_marks(&marks));
    }

    #[test]
    fn the_slot_holds_the_mark_while_checked_and_the_icon_otherwise() {
        let entries: Vec<Entry<u8>> = vec![
            checkbox_item(GRID, "Grid", true).icon(crate::lucide!(Grid3x3)),
            checkbox_item(GRID, "Grid", false).icon(crate::lucide!(Grid3x3)),
            radio_item(SMALL, "Small", true),
            radio_item(LARGE, "Large", false),
            item(COPY, "Copy").icon(crate::lucide!(Copy)),
        ];
        let slots: Vec<Leading> = entries
            .iter()
            .filter_map(Entry::as_item)
            .map(|item| leading(item, true))
            .collect();
        assert_eq!(
            slots,
            [
                Leading::Check,
                Leading::Empty,
                Leading::Dot,
                Leading::Empty,
                Leading::Empty
            ]
        );
        let unslotted = entries.iter().filter_map(Entry::as_item);
        assert!(
            unslotted
                .map(|item| leading(item, false))
                .all(|l| l == Leading::None)
        );
    }
}
