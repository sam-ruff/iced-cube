//! Hierarchical items that expand and collapse, such as folders and files.
//!
//! [`State`] holds the nodes keyed by id, which are expanded, and the
//! selection. Its [`update`](State::update) handles expanding, pointer and
//! keyboard selection in the chosen [`Mode`], checkboxes with tri-state
//! parents, and children that load later. The view only builds the rows of
//! expanded nodes that are in view, so a tree of thousands of nodes stays
//! quick.
//!
//! A node made with [`Node::lazy`] has children that are not loaded yet.
//! Expanding it marks it loading and returns [`Output::Load`]; the app
//! fetches the children and sends them back through the channel that
//! [`subscription`] owns, as [`Loaded`] batches.
//!
//! The tree resolves its keys through a [`Keymap`] of [`Action`]s while it
//! has focus, which it takes when pressed; see [`default_keymap`]. Letters
//! and digits jump to the next node starting with them.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::ops::Range;
use std::rc::Rc;

use iced::futures::channel::mpsc;
use iced::futures::{Stream, StreamExt, stream};
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use iced::widget::{Space, column, container, mouse_area, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Subscription, Theme, mouse};

use crate::data::ellipsis::ellipsis;
use crate::data::interaction::{Look, RowMenu, pressable, row_menu, scope};
use crate::data::rows::rows;
use crate::feedback::badge::{self, badge};
use crate::icon::{Glyph, opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::context_menu;
use crate::overlay::menu::{self, ICON_SIZE, ROW_PADDING, RowStatus};
use crate::primitives::checkbox::{CheckState, checkbox};
use crate::theme::{Tokens, fade, radius, space, text_size};

/// Height of every row, as in a menu.
pub const ROW_HEIGHT: f32 = 32.0;
/// Width of one level of indentation, which is also the chevron's slot.
pub const INDENT: f32 = 20.0;
/// How many loaded batches a producer can send before it has to wait.
pub const CHANNEL_CAPACITY: usize = 64;
/// The most batches delivered in one [`Event::Received`].
pub const BATCH_SIZE: usize = 32;

/// How nodes are selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// Nodes can be highlighted and activated, but not selected.
    None,
    /// One node at a time.
    #[default]
    Single,
    /// Ctrl or Cmd click toggles a node, Shift click selects a range.
    Multiple,
    /// Every node has a checkbox. Checking a parent checks its children,
    /// and a parent shows whether all, some or none of them are checked.
    Checkbox,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::None, Mode::Single, Mode::Multiple, Mode::Checkbox];
}

/// What sits at the end of a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trailing {
    /// A small secondary badge, such as a count.
    Badge(String),
    /// Muted text, such as a size or a date.
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Kids<Id> {
    None,
    Nodes(Vec<Node<Id>>),
    Lazy,
}

/// One node: its label and look. Its children are given with
/// [`children`](Node::children), or later with [`Node::lazy`].
#[derive(Debug, Clone, PartialEq)]
pub struct Node<Id> {
    pub id: Id,
    pub label: String,
    pub icon: Option<Glyph>,
    /// Shown in place of `icon` while the node is expanded.
    pub open_icon: Option<Glyph>,
    pub trailing: Option<Trailing>,
    pub disabled: bool,
    kids: Kids<Id>,
}

/// Creates a leaf node.
pub fn node<Id>(id: Id, label: impl Into<String>) -> Node<Id> {
    Node {
        id,
        label: label.into(),
        icon: None,
        open_icon: None,
        trailing: None,
        disabled: false,
        kids: Kids::None,
    }
}

/// Children that arrived for a lazy node.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded<Id> {
    pub parent: Id,
    pub children: Vec<Node<Id>>,
}

/// The children of `parent`, to send through the channel.
pub fn loaded<Id>(parent: Id, children: impl IntoIterator<Item = Node<Id>>) -> Loaded<Id> {
    Loaded {
        parent,
        children: children.into_iter().collect(),
    }
}

impl<Id> Node<Id> {
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Icons for the collapsed and the expanded node.
    pub fn icons(mut self, closed: Glyph, open: Glyph) -> Self {
        self.icon = Some(closed);
        self.open_icon = Some(open);
        self
    }

    /// A closed folder icon that opens while expanded.
    pub fn folder(self) -> Self {
        self.icons(crate::lucide!(Folder), crate::lucide!(FolderOpen))
    }

    /// A file icon.
    pub fn file(self) -> Self {
        self.icon(crate::lucide!(File))
    }

    /// A small badge at the end of the row.
    pub fn badge(mut self, label: impl Into<String>) -> Self {
        self.trailing = Some(Trailing::Badge(label.into()));
        self
    }

    /// Muted text at the end of the row.
    pub fn trailing(mut self, label: impl Into<String>) -> Self {
        self.trailing = Some(Trailing::Text(label.into()));
        self
    }

    /// A disabled node cannot be highlighted, selected, checked, activated
    /// or expanded.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Node<Id>>) -> Self {
        self.kids = Kids::Nodes(children.into_iter().collect());
        self
    }

    /// Children that load when the node is first expanded.
    pub fn lazy(mut self) -> Self {
        self.kids = Kids::Lazy;
        self
    }
}

/// Where a node's children are.
#[derive(Debug, Clone, PartialEq)]
enum Children<Id> {
    None,
    Loaded(Vec<Id>),
    Unloaded,
    Loading,
}

#[derive(Debug, Clone)]
struct Slot<Id> {
    node: Node<Id>,
    parent: Option<Id>,
    children: Children<Id>,
}

/// One line of the rendered tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line<Id> {
    Node {
        id: Id,
        depth: usize,
    },
    /// Children on their way for the node above.
    Loading {
        depth: usize,
    },
}

impl<Id: PartialEq> Line<Id> {
    fn is(&self, id: &Id) -> bool {
        matches!(self, Line::Node { id: line, .. } if line == id)
    }
}

/// The sending half of the channel lazy children arrive on. Clone it for
/// each producer.
pub type Sender<Id> = mpsc::Sender<Loaded<Id>>;

/// Everything that changes a tree.
#[derive(Debug, Clone)]
pub enum Event<Id> {
    /// The channel is open. Keep the sender and hand clones to producers.
    Ready(Sender<Id>),
    /// Children from the channel, oldest first.
    Received(Vec<Loaded<Id>>),
    /// Expands or collapses a node, as its chevron does.
    Toggle(Id),
    Expand(Id),
    Collapse(Id),
    /// The pointer pressed a row with these modifiers held.
    Press(Id, Modifiers),
    /// A row was double clicked.
    Activate(Id),
    /// A checkbox changed, in [`Mode::Checkbox`].
    Check(Id, bool),
    Next,
    Previous,
    First,
    Last,
    /// Moves the highlight and extends the selection to it, in
    /// [`Mode::Multiple`].
    ExtendNext,
    ExtendPrevious,
    /// Expands the highlighted node, or moves to its first child when it
    /// is already expanded.
    ExpandOrEnter,
    /// Collapses the highlighted node, or moves to its parent when it is
    /// already collapsed.
    CollapseOrLeave,
    /// Expands the highlighted node and every sibling that has children.
    ExpandSiblings,
    /// Selects, toggles or checks the highlighted node, by [`Mode`].
    ToggleHighlighted,
    ActivateHighlighted,
    /// Selects every visible node, in [`Mode::Multiple`].
    SelectAll,
    /// Highlights the next visible node whose label starts with this
    /// character.
    Typeahead(char),
    /// The tree gained or lost focus. The widget sends this itself.
    Focus(bool),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone)]
pub enum Output<Id> {
    Ready(Sender<Id>),
    /// These lazy nodes were expanded and now show as loading. Fetch their
    /// children and send them through the channel.
    Load(Vec<Id>),
    /// A node was activated with Enter or a double click.
    Activated(Id),
    /// The selection changed. Holds every selected node, in tree order.
    Selected(Vec<Id>),
    /// Checkboxes changed. Holds every checked node, in tree order.
    Checked(Vec<Id>),
}

/// The nodes, their structure and what is expanded, selected and checked.
#[derive(Debug, Clone)]
pub struct State<Id> {
    roots: Vec<Id>,
    slots: HashMap<Id, Slot<Id>>,
    mode: Mode,
    expanded: HashSet<Id>,
    selected: HashSet<Id>,
    checked: HashSet<Id>,
    /// Parents with some, but not all, of their children checked.
    partial: HashSet<Id>,
    highlighted: Option<Id>,
    /// Where a Shift click or Shift arrow range starts.
    anchor: Option<Id>,
    focused: bool,
}

impl<Id: Copy + Eq + Hash> State<Id> {
    /// A tree in [`Mode::Single`] with every node collapsed. Ids must be
    /// unique across the whole tree.
    pub fn new(roots: impl IntoIterator<Item = Node<Id>>) -> Self {
        let mut state = Self {
            roots: Vec::new(),
            slots: HashMap::new(),
            mode: Mode::default(),
            expanded: HashSet::new(),
            selected: HashSet::new(),
            checked: HashSet::new(),
            partial: HashSet::new(),
            highlighted: None,
            anchor: None,
            focused: false,
        };
        state.roots = roots
            .into_iter()
            .map(|node| state.insert(node, None))
            .collect();
        state
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = mode;
        self
    }

    /// Starts with these nodes expanded. Lazy nodes stay collapsed until
    /// the user expands them, so nothing loads unasked.
    pub fn with_expanded(mut self, ids: impl IntoIterator<Item = Id>) -> Self {
        for id in ids {
            if self.is_loaded(id) && self.has_children(id) {
                let _ = self.expanded.insert(id);
            }
        }
        self
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn roots(&self) -> &[Id] {
        &self.roots
    }

    pub fn node(&self, id: Id) -> Option<&Node<Id>> {
        self.slots.get(&id).map(|slot| &slot.node)
    }

    pub fn parent(&self, id: Id) -> Option<Id> {
        self.slots.get(&id).and_then(|slot| slot.parent)
    }

    /// The loaded children of a node, empty for a leaf or a node whose
    /// children have not arrived.
    pub fn children(&self, id: Id) -> &[Id] {
        match self.slots.get(&id).map(|slot| &slot.children) {
            Some(Children::Loaded(children)) => children,
            _ => &[],
        }
    }

    /// Whether a node has children, or may have once they load.
    pub fn has_children(&self, id: Id) -> bool {
        match self.slots.get(&id).map(|slot| &slot.children) {
            Some(Children::Loaded(children)) => !children.is_empty(),
            Some(Children::Unloaded | Children::Loading) => true,
            Some(Children::None) | None => false,
        }
    }

    /// Whether a node's children are known: a leaf, or loaded.
    pub fn is_loaded(&self, id: Id) -> bool {
        matches!(
            self.slots.get(&id).map(|slot| &slot.children),
            Some(Children::None | Children::Loaded(_))
        )
    }

    pub fn is_loading(&self, id: Id) -> bool {
        matches!(
            self.slots.get(&id).map(|slot| &slot.children),
            Some(Children::Loading)
        )
    }

    /// Marks a node as loading, such as while the app refreshes it. Its
    /// children stay until new ones arrive.
    pub fn set_loading(&mut self, id: Id) {
        if let Some(slot) = self.slots.get_mut(&id)
            && !matches!(slot.children, Children::Loaded(_))
        {
            slot.children = Children::Loading;
        }
    }

    pub fn is_expanded(&self, id: Id) -> bool {
        self.expanded.contains(&id)
    }

    pub fn is_selected(&self, id: Id) -> bool {
        self.selected.contains(&id)
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// The node the keyboard acts on.
    pub fn highlighted(&self) -> Option<Id> {
        self.highlighted
    }

    /// The selected nodes, in tree order.
    pub fn selected(&self) -> Vec<Id> {
        self.walk(false)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| self.selected.contains(id))
            .collect()
    }

    /// The checked nodes, parents included, in tree order.
    pub fn checked(&self) -> Vec<Id> {
        self.walk(false)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| self.checked.contains(id))
            .collect()
    }

    /// Whether a node's checkbox is checked, unchecked, or indeterminate
    /// because only some of its children are checked.
    pub fn check_state(&self, id: Id) -> CheckState {
        if self.checked.contains(&id) {
            CheckState::Checked
        } else if self.partial.contains(&id) {
            CheckState::Indeterminate
        } else {
            CheckState::Unchecked
        }
    }

    /// The nodes on screen when scrolled through: roots and the children
    /// of expanded nodes, in tree order.
    pub fn visible(&self) -> Vec<Id> {
        self.walk(true).into_iter().map(|(id, _)| id).collect()
    }

    /// How many nodes the tree holds.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Applies an event and returns what the app may need to act on.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Ready(sender) => return Some(Output::Ready(sender)),
            Event::Received(batch) => {
                let mut changed = false;
                for loaded in batch {
                    changed |= self.load(loaded);
                }
                return changed.then(|| Output::Checked(self.checked()));
            }
            Event::Toggle(id) if self.is_expanded(id) => self.collapse(id),
            Event::Toggle(id) | Event::Expand(id) => return self.expand([id]),
            Event::Collapse(id) => self.collapse(id),
            Event::Press(id, modifiers) => return self.press(id, modifiers),
            Event::Activate(id) if self.is_enabled(id) => {
                self.highlighted = Some(id);
                return Some(Output::Activated(id));
            }
            Event::Activate(_) => {}
            Event::Check(id, checked) => return self.check(id, checked),
            Event::Next => self.step(Step::Next),
            Event::Previous => self.step(Step::Previous),
            Event::First => self.step(Step::First),
            Event::Last => self.step(Step::Last),
            Event::ExtendNext => return self.extend(Step::Next),
            Event::ExtendPrevious => return self.extend(Step::Previous),
            Event::ExpandOrEnter => return self.expand_or_enter(),
            Event::CollapseOrLeave => self.collapse_or_leave(),
            Event::ExpandSiblings => return self.expand_siblings(),
            Event::ToggleHighlighted => return self.toggle_highlighted(),
            Event::ActivateHighlighted => {
                return self.highlighted.map(Output::Activated);
            }
            Event::SelectAll => return self.select_all(),
            Event::Typeahead(character) => self.typeahead(character),
            Event::Focus(focused) => {
                self.focused = focused;
                // Focus lands on the first selected node, or the first node,
                // so the keyboard has somewhere to start.
                if focused && self.highlighted.is_none() {
                    let nodes = self.navigable();
                    self.highlighted = nodes
                        .iter()
                        .copied()
                        .find(|id| self.selected.contains(id))
                        .or(nodes.first().copied());
                }
            }
        }
        None
    }

    /// Turns a key press into an event: first through `keymap`, then as
    /// typeahead for a letter or digit. A symbol typed with Shift, such as
    /// `*`, also matches its chord without Shift.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event<Id>> {
        let action = keymap.resolve_event(key).or_else(|| {
            let symbol = matches!(&key.key, Key::Character(text) if !text.chars().any(char::is_alphanumeric));
            (symbol && key.modifiers.shift())
                .then(|| {
                    keymap.resolve(&key.key, key.modifiers.difference(Modifiers::SHIFT))
                })
                .flatten()
        });
        if let Some(action) = action {
            return action.event(self);
        }
        typeahead(key).map(Event::Typeahead)
    }

    fn insert(&mut self, mut node: Node<Id>, parent: Option<Id>) -> Id {
        let id = node.id;
        let children = match std::mem::replace(&mut node.kids, Kids::None) {
            Kids::None => Children::None,
            Kids::Lazy => Children::Unloaded,
            Kids::Nodes(nodes) => Children::Loaded(
                nodes
                    .into_iter()
                    .map(|child| self.insert(child, Some(id)))
                    .collect(),
            ),
        };
        let _ = self.slots.insert(
            id,
            Slot {
                node,
                parent,
                children,
            },
        );
        id
    }

    fn is_enabled(&self, id: Id) -> bool {
        self.slots.get(&id).is_some_and(|slot| !slot.node.disabled)
    }

    /// Nodes in tree order with their depth: every loaded node, or only
    /// those under expanded parents.
    fn walk(&self, visible: bool) -> Vec<(Id, usize)> {
        let mut out = Vec::new();
        let mut stack: Vec<(Id, usize)> = self.roots.iter().rev().map(|&id| (id, 0)).collect();
        while let Some((id, depth)) = stack.pop() {
            out.push((id, depth));
            if visible && !self.expanded.contains(&id) {
                continue;
            }
            stack.extend(
                self.children(id)
                    .iter()
                    .rev()
                    .map(|&child| (child, depth + 1)),
            );
        }
        out
    }

    /// The rendered lines: visible nodes, and a loading line under each
    /// expanded node whose children are on their way.
    fn lines(&self) -> Vec<Line<Id>> {
        let mut lines = Vec::new();
        for (id, depth) in self.walk(true) {
            lines.push(Line::Node { id, depth });
            if self.is_expanded(id) && self.is_loading(id) {
                lines.push(Line::Loading { depth: depth + 1 });
            }
        }
        lines
    }

    /// Visible nodes that can take the highlight.
    fn navigable(&self) -> Vec<Id> {
        self.walk(true)
            .into_iter()
            .map(|(id, _)| id)
            .filter(|&id| self.is_enabled(id))
            .collect()
    }

    fn step(&mut self, step: Step) {
        let nodes = self.navigable();
        let current = self
            .highlighted
            .and_then(|id| nodes.iter().position(|&node| node == id));
        let target = match (step, current) {
            (Step::First, _) | (Step::Next, None) => nodes.first(),
            (Step::Last, _) | (Step::Previous, None) => nodes.last(),
            (Step::Next, Some(index)) => nodes.get(index + 1).or(nodes.get(index)),
            (Step::Previous, Some(index)) => nodes.get(index.saturating_sub(1)),
        };
        if let Some(&target) = target {
            self.highlighted = Some(target);
        }
    }

    fn expand(&mut self, ids: impl IntoIterator<Item = Id>) -> Option<Output<Id>> {
        let mut load = Vec::new();
        for id in ids {
            if !self.is_enabled(id) || !self.has_children(id) {
                continue;
            }
            let _ = self.expanded.insert(id);
            let Some(slot) = self.slots.get_mut(&id) else {
                continue;
            };
            if slot.children == Children::Unloaded {
                slot.children = Children::Loading;
                load.push(id);
            }
        }
        (!load.is_empty()).then_some(Output::Load(load))
    }

    fn collapse(&mut self, id: Id) {
        if !self.expanded.remove(&id) {
            return;
        }
        // A highlight inside the collapsed branch moves up to it.
        let hidden = self
            .highlighted
            .is_some_and(|highlighted| self.is_ancestor(id, highlighted));
        if hidden {
            self.highlighted = Some(id);
        }
    }

    fn is_ancestor(&self, ancestor: Id, id: Id) -> bool {
        let mut current = self.parent(id);
        while let Some(parent) = current {
            if parent == ancestor {
                return true;
            }
            current = self.parent(parent);
        }
        false
    }

    fn expand_or_enter(&mut self) -> Option<Output<Id>> {
        let id = self.highlighted?;
        if !self.is_expanded(id) {
            return self.expand([id]);
        }
        let first = self
            .children(id)
            .iter()
            .copied()
            .find(|&child| self.is_enabled(child));
        if let Some(first) = first {
            self.highlighted = Some(first);
        }
        None
    }

    fn collapse_or_leave(&mut self) {
        let Some(id) = self.highlighted else {
            return;
        };
        if self.is_expanded(id) {
            self.collapse(id);
            return;
        }
        if let Some(parent) = self.parent(id).filter(|&parent| self.is_enabled(parent)) {
            self.highlighted = Some(parent);
        }
    }

    fn expand_siblings(&mut self) -> Option<Output<Id>> {
        let id = self.highlighted?;
        let siblings = match self.parent(id) {
            Some(parent) => self.children(parent).to_vec(),
            None => self.roots.clone(),
        };
        self.expand(siblings)
    }

    fn selection_output(&self, before: &HashSet<Id>) -> Option<Output<Id>> {
        (self.selected != *before).then(|| Output::Selected(self.selected()))
    }

    fn press(&mut self, id: Id, modifiers: Modifiers) -> Option<Output<Id>> {
        if !self.is_enabled(id) {
            return None;
        }
        self.highlighted = Some(id);
        match self.mode {
            Mode::None => None,
            Mode::Single => {
                let before = std::mem::replace(&mut self.selected, HashSet::from([id]));
                self.anchor = Some(id);
                self.selection_output(&before)
            }
            Mode::Checkbox => self.check(id, !self.checked.contains(&id)),
            Mode::Multiple => {
                let before = self.selected.clone();
                let anchor = self
                    .anchor
                    .filter(|&anchor| self.navigable().contains(&anchor));
                match (modifiers.shift(), modifiers.command(), anchor) {
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
                        self.selected = HashSet::from([id]);
                        self.anchor = Some(id);
                    }
                }
                self.selection_output(&before)
            }
        }
    }

    /// Enabled visible nodes from `a` to `b`, in either order.
    fn range(&self, a: Id, b: Id) -> Vec<Id> {
        let nodes = self.navigable();
        let (Some(start), Some(end)) = (
            nodes.iter().position(|&id| id == a),
            nodes.iter().position(|&id| id == b),
        ) else {
            return vec![b];
        };
        let span = if start <= end {
            start..=end
        } else {
            end..=start
        };
        nodes.get(span).map(<[Id]>::to_vec).unwrap_or_default()
    }

    fn extend(&mut self, step: Step) -> Option<Output<Id>> {
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
        self.selected = self.range(anchor, highlighted).into_iter().collect();
        self.selection_output(&before)
    }

    fn toggle_highlighted(&mut self) -> Option<Output<Id>> {
        let id = self.highlighted?;
        match self.mode {
            Mode::None => None,
            Mode::Checkbox => self.check(id, !self.checked.contains(&id)),
            Mode::Single => self.press(id, Modifiers::empty()),
            Mode::Multiple => self.press(id, Modifiers::COMMAND),
        }
    }

    fn select_all(&mut self) -> Option<Output<Id>> {
        if self.mode != Mode::Multiple {
            return None;
        }
        let before = self.selected.clone();
        self.selected = self.navigable().into_iter().collect();
        self.selection_output(&before)
    }

    fn typeahead(&mut self, character: char) {
        let wanted: String = character.to_lowercase().collect();
        let nodes = self.navigable();
        let start = self
            .highlighted
            .and_then(|id| nodes.iter().position(|&node| node == id))
            .map_or(0, |index| index + 1);
        let found = (0..nodes.len())
            .filter_map(|offset| nodes.get((start + offset) % nodes.len()))
            .copied()
            .find(|&id| {
                self.node(id)
                    .is_some_and(|node| node.label.to_lowercase().starts_with(&wanted))
            });
        if let Some(found) = found {
            self.highlighted = Some(found);
        }
    }

    fn check(&mut self, id: Id, checked: bool) -> Option<Output<Id>> {
        if self.mode != Mode::Checkbox || !self.is_enabled(id) {
            return None;
        }
        let before = self.checked.clone();
        self.set_subtree(id, checked);
        self.refresh(self.parent(id));
        (self.checked != before).then(|| Output::Checked(self.checked()))
    }

    /// Checks or unchecks a node and its loaded descendants, leaving
    /// disabled descendants as they are.
    fn set_subtree(&mut self, id: Id, checked: bool) {
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            if current != id && !self.is_enabled(current) {
                continue;
            }
            let _ = self.partial.remove(&current);
            if checked {
                let _ = self.checked.insert(current);
            } else {
                let _ = self.checked.remove(&current);
            }
            stack.extend(self.children(current).iter().copied());
        }
    }

    /// Recomputes `start` and each node above it from its enabled children:
    /// checked when all are, indeterminate when some are.
    fn refresh(&mut self, start: Option<Id>) {
        let mut current = start;
        while let Some(parent) = current {
            let children: Vec<Id> = self
                .children(parent)
                .iter()
                .copied()
                .filter(|&child| self.is_enabled(child))
                .collect();
            if !children.is_empty() {
                let all = children.iter().all(|child| self.checked.contains(child));
                let some = children
                    .iter()
                    .any(|child| self.checked.contains(child) || self.partial.contains(child));
                if all {
                    let _ = self.checked.insert(parent);
                } else {
                    let _ = self.checked.remove(&parent);
                }
                if some && !all {
                    let _ = self.partial.insert(parent);
                } else {
                    let _ = self.partial.remove(&parent);
                }
            }
            current = self.parent(parent);
        }
    }

    /// Attaches children that arrived, replacing any the node had. Returns
    /// whether any checkbox changed.
    fn load(&mut self, loaded: Loaded<Id>) -> bool {
        let parent = loaded.parent;
        if !self.slots.contains_key(&parent) {
            return false;
        }
        for child in self.children(parent).to_vec() {
            self.remove(child);
        }
        let children: Vec<Id> = loaded
            .children
            .into_iter()
            .map(|child| self.insert(child, Some(parent)))
            .collect();
        if let Some(slot) = self.slots.get_mut(&parent) {
            slot.children = Children::Loaded(children.clone());
        }
        if children.is_empty() {
            let _ = self.expanded.remove(&parent);
        }
        if self.mode != Mode::Checkbox || !self.checked.contains(&parent) {
            return false;
        }
        for child in children {
            if self.is_enabled(child) {
                self.set_subtree(child, true);
            }
        }
        self.refresh(Some(parent));
        true
    }

    /// Forgets a node and everything under it.
    fn remove(&mut self, id: Id) {
        for child in self.children(id).to_vec() {
            self.remove(child);
        }
        let _ = self.slots.remove(&id);
        let _ = self.expanded.remove(&id);
        let _ = self.selected.remove(&id);
        let _ = self.checked.remove(&id);
        let _ = self.partial.remove(&id);
        if self.highlighted == Some(id) {
            self.highlighted = None;
        }
        if self.anchor == Some(id) {
            self.anchor = None;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Next,
    Previous,
    First,
    Last,
}

/// The character a key press types into the typeahead: a letter or digit
/// pressed without Ctrl, Alt or the logo key.
fn typeahead(key: &keys::Event) -> Option<char> {
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

/// What a tree keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Last,
    Expand,
    Collapse,
    Activate,
    Select,
    ExpandSiblings,
    ExtendNext,
    ExtendPrevious,
    SelectAll,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing, so the key is left for others.
    pub fn event<Id: Copy + Eq + Hash>(self, state: &State<Id>) -> Option<Event<Id>> {
        let highlighted = state.highlighted().is_some();
        let multiple = state.mode() == Mode::Multiple;
        match self {
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::First => Some(Event::First),
            Action::Last => Some(Event::Last),
            Action::Expand => highlighted.then_some(Event::ExpandOrEnter),
            Action::Collapse => highlighted.then_some(Event::CollapseOrLeave),
            Action::Activate => highlighted.then_some(Event::ActivateHighlighted),
            Action::Select => {
                (highlighted && state.mode() != Mode::None).then_some(Event::ToggleHighlighted)
            }
            Action::ExpandSiblings => highlighted.then_some(Event::ExpandSiblings),
            Action::ExtendNext if multiple => Some(Event::ExtendNext),
            Action::ExtendNext => Some(Event::Next),
            Action::ExtendPrevious if multiple => Some(Event::ExtendPrevious),
            Action::ExtendPrevious => Some(Event::Previous),
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
        Action::Expand,
        Action::Collapse,
        Action::Activate,
        Action::Select,
        Action::ExpandSiblings,
        Action::ExtendNext,
        Action::ExtendPrevious,
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
            Action::Expand => "Expand",
            Action::Collapse => "Collapse",
            Action::Activate => "Activate",
            Action::Select => "Select",
            Action::ExpandSiblings => "ExpandSiblings",
            Action::ExtendNext => "ExtendNext",
            Action::ExtendPrevious => "ExtendPrevious",
            Action::SelectAll => "SelectAll",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next visible node.",
            Action::Previous => "Highlights the previous visible node.",
            Action::First => "Highlights the first node.",
            Action::Last => "Highlights the last visible node.",
            Action::Expand => {
                "Expands the highlighted node, or moves to its first child when it is open."
            }
            Action::Collapse => {
                "Collapses the highlighted node, or moves to its parent when it is closed."
            }
            Action::Activate => "Activates the highlighted node.",
            Action::Select => "Selects, toggles or checks the highlighted node.",
            Action::ExpandSiblings => "Expands the highlighted node and all of its siblings.",
            Action::ExtendNext => "Moves down and extends the selection, in a multiple selection.",
            Action::ExtendPrevious => {
                "Moves up and extends the selection, in a multiple selection."
            }
            Action::SelectAll => "Selects every visible node, in a multiple selection.",
        }
    }
}

/// The default tree shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
/// | `ArrowRight` | [`Action::Expand`] |
/// | `ArrowLeft` | [`Action::Collapse`] |
/// | `Enter` | [`Action::Activate`] |
/// | `Space` | [`Action::Select`] |
/// | `*` | [`Action::ExpandSiblings`] |
/// | `Shift+ArrowDown` | [`Action::ExtendNext`] |
/// | `Shift+ArrowUp` | [`Action::ExtendPrevious`] |
/// | `Mod+A` (Ctrl, or Cmd on macOS) | [`Action::SelectAll`] |
///
/// Letters and digits jump to the next node starting with them. The tree
/// resolves these itself, only while it has focus, so they never clash
/// with other components. Pass a changed keymap with [`Tree::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
        .bind(Chord::named(Named::ArrowRight), Action::Expand)
        .bind(Chord::named(Named::ArrowLeft), Action::Collapse)
        .bind(Chord::named(Named::Enter), Action::Activate)
        .bind(Chord::named(Named::Space), Action::Select)
        .bind(Chord::character('*'), Action::ExpandSiblings)
        .bind(Chord::named(Named::ArrowDown).shift(), Action::ExtendNext)
        .bind(Chord::named(Named::ArrowUp).shift(), Action::ExtendPrevious)
        .bind(Chord::character('a').command(), Action::SelectAll)
}

/// Owns the channel lazy children arrive on. Emits [`Event::Ready`] first,
/// then batches.
pub fn subscription<Id: Send + 'static>() -> Subscription<Event<Id>> {
    Subscription::run(stream::<Id>)
}

/// The stream behind [`subscription`], for driving it without a runtime.
pub fn stream<Id: Send + 'static>() -> impl Stream<Item = Event<Id>> {
    let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
    stream::once(async move { Event::Ready(sender) })
        .chain(receiver.ready_chunks(BATCH_SIZE).map(Event::Received))
}

/// How a row is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Status {
    pub selected: bool,
    pub hovered: bool,
    /// Carries the keyboard highlight while the tree has focus.
    pub highlighted: bool,
    pub disabled: bool,
}

/// The colours of a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStyle {
    pub background: Option<Color>,
    pub text: Color,
    pub icon: Color,
    /// The ring around the row carrying the keyboard highlight.
    pub ring: Option<Color>,
}

/// Resolves a row's colours from the shared menu rows: a selected row is
/// drawn as a highlighted menu row, and a hovered one with a lighter tint.
pub fn row_style(tokens: &Tokens, status: Status) -> RowStyle {
    let menu_status = match (status.disabled, status.selected) {
        (true, _) => RowStatus::Disabled,
        (false, true) => RowStatus::Highlighted,
        (false, false) => RowStatus::Idle,
    };
    let base = menu::row_style(tokens, menu_status, false);
    let hover = (status.hovered && !status.disabled).then(|| fade(tokens.accent, 0.6));
    RowStyle {
        background: base.background.or(hover),
        text: base.text,
        icon: base.icon,
        ring: (status.highlighted && !status.disabled).then_some(tokens.ring),
    }
}

/// The colour of the indentation guides.
pub fn guide_colour(tokens: &Tokens) -> Color {
    tokens.border
}

type OnEvent<'a, Id, Message> = dyn Fn(Event<Id>) -> Message + 'a;
type BuildMenu<'a, Id, Message> =
    dyn FnOnce(Keymap<context_menu::Action>) -> RowMenu<'a, Id, Message> + 'a;

/// A tree builder. Convert it into an [`Element`] to render.
pub struct Tree<'a, Id, Message> {
    state: &'a State<Id>,
    guides: bool,
    width: Length,
    height: Length,
    keymap: Keymap<Action>,
    menu: Option<Box<BuildMenu<'a, Id, Message>>>,
    menu_keymap: Keymap<context_menu::Action>,
    on_event: Option<Box<OnEvent<'a, Id, Message>>>,
}

/// Renders `state`. Without [`on_event`](Tree::on_event) every row renders
/// disabled.
pub fn tree<Id, Message>(state: &State<Id>) -> Tree<'_, Id, Message> {
    Tree {
        state,
        guides: false,
        width: Length::Fill,
        height: Length::Shrink,
        keymap: default_keymap(),
        menu: None,
        menu_keymap: context_menu::default_keymap(),
        on_event: None,
    }
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for Tree<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("state", self.state)
            .field("guides", &self.guides)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

impl<'a, Id, Message> Tree<'a, Id, Message> {
    /// Draws a vertical line at each level of indentation.
    pub fn guides(mut self, guides: bool) -> Self {
        self.guides = guides;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// A fixed or filling height scrolls the rows. Defaults to shrinking to
    /// fit them.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Replaces the [`default_keymap`] the focused tree resolves keys with.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// Gives every node the context menu of `menu`, keyed by node id. It
    /// opens on a right-click or a long press, and on the highlighted node
    /// with its opening chord while the tree has focus.
    pub fn context_menu<MenuId>(
        mut self,
        menu: &'a context_menu::State<MenuId, Id>,
        on_event: impl Fn(context_menu::Event<MenuId, Id>) -> Message + 'a,
    ) -> Self
    where
        MenuId: Copy + PartialEq + 'a,
        Id: Clone + PartialEq + 'a,
        Message: Clone + 'a,
    {
        let on_event: Rc<dyn Fn(context_menu::Event<MenuId, Id>) -> Message + 'a> =
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

impl<'a, Id, Message> From<Tree<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + Eq + Hash + 'a,
    Message: Clone + 'a,
{
    fn from(tree: Tree<'a, Id, Message>) -> Self {
        let Tree {
            state,
            guides,
            width,
            height,
            keymap,
            menu,
            menu_keymap,
            on_event,
        } = tree;
        let on_event: Option<Rc<OnEvent<'a, Id, Message>>> = on_event.map(Rc::from);
        let menu = menu
            .filter(|_| on_event.is_some())
            .map(|build| Rc::new(build(menu_keymap)));
        let lines = state.lines();
        let highlight = state
            .highlighted
            .and_then(|id| lines.iter().position(|line| line.is(&id)));
        let count = lines.len();

        let build = {
            let on_event = on_event.clone();
            let menu = menu.clone();
            move |range: Range<usize>| -> Element<'a, Message> {
                let lines = lines.get(range).unwrap_or_default();
                column(lines.iter().map(|line| match *line {
                    Line::Node { id, depth } => {
                        let row = node_view(state, id, depth, guides, on_event.as_ref());
                        match &menu {
                            Some(menu) if state.is_enabled(id) => menu.wrap(id, row),
                            _ => row,
                        }
                    }
                    Line::Loading { depth } => loading_view(depth, guides),
                }))
                .into()
            }
        };
        let list = container(
            rows(count, ROW_HEIGHT, build)
                .highlight(highlight)
                .height(height),
        )
        .width(width);

        let Some(on_event) = on_event else {
            return list.into();
        };
        let on_focus = on_event.clone();
        scope(list)
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

/// The indentation for `depth` levels, with a guide at each when asked.
fn indent<'a, Message: 'a>(depth: usize, guides: bool) -> Element<'a, Message> {
    let levels =
        (0..depth).map(|_| {
            let guide: Element<'a, Message> =
                if guides {
                    container(container(Space::new()).width(1).height(Length::Fill).style(
                        |theme| container::Style {
                            background: Some(Background::Color(guide_colour(&Tokens::of(theme)))),
                            ..container::Style::default()
                        },
                    ))
                    .padding(iced::Padding::ZERO.left(ICON_SIZE / 2.0))
                    .into()
                } else {
                    Space::new().into()
                };
            container(guide).width(INDENT).height(Length::Fill).into()
        });
    row(levels).height(Length::Fill).into()
}

fn line_frame<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(
            iced::Padding::ZERO
                .left(ROW_PADDING.left)
                .right(ROW_PADDING.right),
        )
        .width(Length::Fill)
        .height(ROW_HEIGHT)
        .into()
}

fn loading_view<'a, Message: 'a>(depth: usize, guides: bool) -> Element<'a, Message> {
    let muted = |theme: &Theme| Tokens::of(theme).muted_foreground;
    line_frame(
        row![
            indent(depth, guides),
            Space::new().width(INDENT),
            row![
                themed(crate::lucide!(LoaderCircle), ICON_SIZE, 1.0, muted),
                text("Loading...")
                    .size(text_size::SM)
                    .wrapping(text::Wrapping::None)
                    .style(move |theme| text::Style {
                        color: Some(muted(theme)),
                    }),
            ]
            .spacing(space::SM)
            .align_y(Alignment::Center)
            .height(Length::Fill),
        ]
        .height(Length::Fill),
    )
}

fn node_view<'a, Id, Message>(
    state: &'a State<Id>,
    id: Id,
    depth: usize,
    guides: bool,
    on_event: Option<&Rc<OnEvent<'a, Id, Message>>>,
) -> Element<'a, Message>
where
    Id: Copy + Eq + Hash + 'a,
    Message: Clone + 'a,
{
    let Some(node) = state.node(id) else {
        return Space::new().into();
    };
    let on_event = on_event.filter(|_| !node.disabled);
    let enabled = on_event.is_some();
    let expanded = state.is_expanded(id);
    let status = Status {
        selected: state.mode != Mode::Checkbox && state.is_selected(id),
        hovered: false,
        highlighted: state.focused && state.highlighted == Some(id),
        disabled: !enabled,
    };
    let colours = move |theme: &Theme| row_style(&Tokens::of(theme), status);

    let chevron: Element<'a, Message> = if state.has_children(id) {
        let glyph = if expanded {
            crate::lucide!(ChevronDown)
        } else {
            crate::lucide!(ChevronRight)
        };
        let icon = container(themed(glyph, ICON_SIZE, opacity(enabled), move |theme| {
            colours(theme).icon
        }))
        .width(INDENT)
        .height(Length::Fill)
        .center_y(Length::Fill);
        match on_event {
            Some(on_event) => mouse_area(icon)
                .on_press(on_event(Event::Toggle(id)))
                .interaction(mouse::Interaction::Pointer)
                .into(),
            None => icon.into(),
        }
    } else {
        Space::new().width(INDENT).into()
    };

    let mut content = row![]
        .spacing(space::SM)
        .align_y(Alignment::Center)
        .height(Length::Fill);
    if state.mode == Mode::Checkbox {
        let check = on_event.map(|on_event| move |checked| on_event(Event::Check(id, checked)));
        content = content.push(checkbox(state.check_state(id)).on_toggle_maybe(check));
    }
    let glyph = if expanded {
        node.open_icon.or(node.icon)
    } else {
        node.icon
    };
    if let Some(glyph) = glyph {
        content = content.push(themed(glyph, ICON_SIZE, opacity(enabled), move |theme| {
            colours(theme).icon
        }));
    }
    content = content.push(ellipsis(node.label.as_str()).colour(move |theme| colours(theme).text));
    match &node.trailing {
        Some(Trailing::Badge(label)) => {
            content = content.push(badge(label.as_str()).variant(badge::Variant::Secondary));
        }
        Some(Trailing::Text(label)) => {
            content = content.push(
                text(label.as_str())
                    .size(text_size::XS)
                    .wrapping(text::Wrapping::None)
                    .style(move |theme: &Theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
            );
        }
        None => {}
    }

    let body = line_frame(row![indent(depth, guides), chevron, content].height(Length::Fill));
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
    use crate::theme::{dark, light};

    /// ```text
    /// 1 src
    ///   2 main.rs
    ///   3 lib
    ///     4 mod.rs
    ///     5 old.rs (disabled)
    ///     6 util.rs
    /// 7 docs (lazy)
    /// 8 readme.md
    /// ```
    fn sample() -> State<u32> {
        State::new([
            node(1, "src").folder().children([
                node(2, "main.rs").file(),
                node(3, "lib").folder().children([
                    node(4, "mod.rs"),
                    node(5, "old.rs").disabled(true),
                    node(6, "util.rs"),
                ]),
            ]),
            node(7, "docs").folder().lazy(),
            node(8, "readme.md").file(),
        ])
    }

    fn open() -> State<u32> {
        sample().with_expanded([1, 3])
    }

    fn selected(output: Option<Output<u32>>) -> Vec<u32> {
        match output {
            Some(Output::Selected(ids)) => ids,
            other => panic!("expected a selection, got {other:?}"),
        }
    }

    fn checked(output: Option<Output<u32>>) -> Vec<u32> {
        match output {
            Some(Output::Checked(ids)) => ids,
            other => panic!("expected checked nodes, got {other:?}"),
        }
    }

    fn key(key: Key, modifiers: Modifiers) -> keys::Event {
        keys::Event { key, modifiers }
    }

    #[test]
    fn builds_the_structure_collapsed() {
        let state = sample();
        assert_eq!(state.len(), 8);
        assert_eq!(state.roots(), &[1, 7, 8]);
        assert_eq!(state.children(1), &[2, 3]);
        assert_eq!(state.parent(4), Some(3));
        assert_eq!(state.parent(1), None);
        assert_eq!(state.visible(), vec![1, 7, 8]);
        assert!(state.has_children(7) && !state.is_loaded(7));
        assert!(!state.has_children(8) && state.is_loaded(8));
        assert_eq!(state.mode(), Mode::Single);
        assert!(State::<u8>::new([]).is_empty());
    }

    #[test]
    fn with_expanded_skips_leaves_and_lazy_nodes() {
        let state = sample().with_expanded([1, 7, 8]);
        assert!(state.is_expanded(1));
        assert!(!state.is_expanded(7), "lazy nodes load only when asked");
        assert!(!state.is_expanded(8));
        assert_eq!(state.visible(), vec![1, 2, 3, 7, 8]);
    }

    #[test]
    fn toggle_expands_and_collapses() {
        let mut state = sample();
        assert!(state.update(Event::Toggle(1)).is_none());
        assert_eq!(state.visible(), vec![1, 2, 3, 7, 8]);
        let _ = state.update(Event::Toggle(1));
        assert_eq!(state.visible(), vec![1, 7, 8]);
        let _ = state.update(Event::Expand(8));
        assert!(!state.is_expanded(8), "a leaf does not expand");
    }

    #[test]
    fn collapsing_moves_a_hidden_highlight_to_the_parent() {
        let mut state = open();
        let _ = state.update(Event::Press(4, Modifiers::empty()));
        let _ = state.update(Event::Collapse(1));
        assert_eq!(state.highlighted(), Some(1));
    }

    #[test]
    fn expanding_a_lazy_node_starts_loading() {
        let mut state = sample();
        let output = state.update(Event::Expand(7));
        assert!(matches!(output, Some(Output::Load(ids)) if ids == [7]));
        assert!(state.is_loading(7) && state.is_expanded(7));
        assert_eq!(
            state.lines(),
            vec![
                Line::Node { id: 1, depth: 0 },
                Line::Node { id: 7, depth: 0 },
                Line::Loading { depth: 1 },
                Line::Node { id: 8, depth: 0 },
            ]
        );
        assert!(state.update(Event::Expand(7)).is_none(), "already loading");
    }

    #[test]
    fn loaded_children_replace_the_loading_line() {
        let mut state = sample();
        let _ = state.update(Event::Expand(7));
        let _ = state.update(Event::Received(vec![loaded(
            7,
            [node(70, "guide.md"), node(71, "api").lazy()],
        )]));
        assert!(!state.is_loading(7));
        assert_eq!(state.visible(), vec![1, 7, 70, 71, 8]);
        assert_eq!(state.parent(71), Some(7));
        assert!(state.has_children(71));
    }

    #[test]
    fn reloading_replaces_old_children_and_forgets_them() {
        let mut state = open();
        let _ = state.update(Event::Press(4, Modifiers::empty()));
        let _ = state.update(Event::Received(vec![loaded(3, [node(9, "new.rs")])]));
        assert_eq!(state.children(3), &[9]);
        assert!(state.node(4).is_none());
        assert_eq!(state.highlighted(), None);
        assert!(state.selected().is_empty());
    }

    #[test]
    fn loading_no_children_collapses_the_node() {
        let mut state = sample();
        let _ = state.update(Event::Expand(7));
        let _ = state.update(Event::Received(vec![loaded(7, [])]));
        assert!(!state.is_expanded(7) && !state.has_children(7));
        assert!(
            state
                .update(Event::Received(vec![loaded(99, [])]))
                .is_none()
        );
    }

    #[test]
    fn set_loading_marks_unloaded_nodes_only() {
        let mut state = sample();
        state.set_loading(7);
        assert!(state.is_loading(7));
        state.set_loading(1);
        assert!(!state.is_loading(1), "loaded children stay");
    }

    #[test]
    fn arrows_move_over_visible_enabled_nodes_without_wrapping() {
        let mut state = open();
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(1));
        for _ in 0..4 {
            let _ = state.update(Event::Next);
        }
        assert_eq!(state.highlighted(), Some(6), "skips the disabled node");
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(8));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(8), "no wrap");
        let _ = state.update(Event::First);
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(1));

        let mut fresh = open();
        let _ = fresh.update(Event::Previous);
        assert_eq!(fresh.highlighted(), Some(8));
    }

    #[test]
    fn right_expands_then_enters_and_left_collapses_then_leaves() {
        let mut state = sample();
        let _ = state.update(Event::First);
        let _ = state.update(Event::ExpandOrEnter);
        assert!(state.is_expanded(1));
        assert_eq!(state.highlighted(), Some(1));
        let _ = state.update(Event::ExpandOrEnter);
        assert_eq!(state.highlighted(), Some(2));
        let _ = state.update(Event::ExpandOrEnter);
        assert_eq!(state.highlighted(), Some(2), "a leaf goes nowhere");
        let _ = state.update(Event::CollapseOrLeave);
        assert_eq!(state.highlighted(), Some(1));
        let _ = state.update(Event::CollapseOrLeave);
        assert!(!state.is_expanded(1));
        let _ = state.update(Event::CollapseOrLeave);
        assert_eq!(state.highlighted(), Some(1), "a root has no parent");
    }

    #[test]
    fn right_on_a_lazy_node_loads_it() {
        let mut state = sample();
        let _ = state.update(Event::Press(7, Modifiers::empty()));
        assert!(matches!(
            state.update(Event::ExpandOrEnter),
            Some(Output::Load(ids)) if ids == [7]
        ));
    }

    #[test]
    fn star_expands_every_sibling_and_loads_lazy_ones() {
        let mut state = sample();
        let _ = state.update(Event::First);
        let output = state.update(Event::ExpandSiblings);
        assert!(matches!(output, Some(Output::Load(ids)) if ids == [7]));
        assert!(state.is_expanded(1) && state.is_expanded(7));
        assert!(!state.is_expanded(8));
    }

    #[test]
    fn typeahead_cycles_through_matching_labels() {
        let mut state = open();
        let _ = state.update(Event::Typeahead('M'));
        assert_eq!(state.highlighted(), Some(2));
        let _ = state.update(Event::Typeahead('m'));
        assert_eq!(state.highlighted(), Some(4));
        let _ = state.update(Event::Typeahead('m'));
        assert_eq!(state.highlighted(), Some(2), "wraps round");
        let _ = state.update(Event::Typeahead('o'));
        assert_eq!(state.highlighted(), Some(2), "disabled old.rs is skipped");
        let _ = state.update(Event::Typeahead('z'));
        assert_eq!(state.highlighted(), Some(2));
    }

    #[test]
    fn single_selection_follows_presses() {
        let mut state = open();
        assert_eq!(
            selected(state.update(Event::Press(2, Modifiers::empty()))),
            [2]
        );
        assert_eq!(
            selected(state.update(Event::Press(4, Modifiers::CTRL))),
            [4]
        );
        assert!(state.update(Event::Press(4, Modifiers::empty())).is_none());
        assert!(state.update(Event::Press(5, Modifiers::empty())).is_none());
        assert_eq!(state.selected(), [4], "disabled nodes are not selected");
    }

    #[test]
    fn multiple_selection_with_command_and_shift() {
        let mut state = open().with_mode(Mode::Multiple);
        let _ = state.update(Event::Press(2, Modifiers::empty()));
        assert_eq!(
            selected(state.update(Event::Press(4, Modifiers::COMMAND))),
            [2, 4]
        );
        assert_eq!(
            selected(state.update(Event::Press(2, Modifiers::COMMAND))),
            [4]
        );
        assert_eq!(
            selected(state.update(Event::Press(7, Modifiers::SHIFT))),
            [2, 3, 4, 6, 7],
            "a range from the anchor, skipping the disabled node"
        );
        let _ = state.update(Event::Press(8, Modifiers::empty()));
        let _ = state.update(Event::Press(2, Modifiers::COMMAND));
        assert_eq!(
            selected(state.update(Event::Press(4, Modifiers::SHIFT | Modifiers::COMMAND))),
            [2, 3, 4, 8],
            "Ctrl+Shift adds the range"
        );
        assert_eq!(
            selected(state.update(Event::Press(3, Modifiers::empty()))),
            [3]
        );
    }

    #[test]
    fn shift_arrows_extend_from_the_anchor() {
        let mut state = open().with_mode(Mode::Multiple);
        let _ = state.update(Event::Press(2, Modifiers::empty()));
        assert_eq!(selected(state.update(Event::ExtendNext)), [2, 3]);
        assert_eq!(selected(state.update(Event::ExtendNext)), [2, 3, 4]);
        assert_eq!(selected(state.update(Event::ExtendPrevious)), [2, 3]);
        let _ = state.update(Event::ExtendPrevious);
        assert_eq!(selected(state.update(Event::ExtendPrevious)), [1, 2]);
    }

    #[test]
    fn extend_only_moves_outside_a_multiple_selection() {
        let mut state = open();
        let _ = state.update(Event::Press(2, Modifiers::empty()));
        assert!(state.update(Event::ExtendNext).is_none());
        assert_eq!(state.highlighted(), Some(3));
        assert_eq!(state.selected(), [2]);
    }

    #[test]
    fn space_selects_by_mode() {
        let mut single = open();
        let _ = single.update(Event::First);
        assert_eq!(selected(single.update(Event::ToggleHighlighted)), [1]);

        let mut multiple = open().with_mode(Mode::Multiple);
        let _ = multiple.update(Event::First);
        let _ = multiple.update(Event::ToggleHighlighted);
        let _ = multiple.update(Event::Next);
        assert_eq!(selected(multiple.update(Event::ToggleHighlighted)), [1, 2]);
        assert_eq!(selected(multiple.update(Event::ToggleHighlighted)), [1]);

        let mut none = open().with_mode(Mode::None);
        let _ = none.update(Event::First);
        assert!(none.update(Event::ToggleHighlighted).is_none());
        assert!(none.update(Event::Press(2, Modifiers::empty())).is_none());
        assert_eq!(none.highlighted(), Some(2));
    }

    #[test]
    fn select_all_takes_every_visible_enabled_node() {
        let mut state = open().with_mode(Mode::Multiple);
        assert_eq!(
            selected(state.update(Event::SelectAll)),
            [1, 2, 3, 4, 6, 7, 8]
        );
        assert!(open().update(Event::SelectAll).is_none());
    }

    #[test]
    fn activation_reports_the_node() {
        let mut state = open();
        assert!(state.update(Event::ActivateHighlighted).is_none());
        assert!(matches!(
            state.update(Event::Activate(2)),
            Some(Output::Activated(2))
        ));
        assert_eq!(state.highlighted(), Some(2));
        assert!(matches!(
            state.update(Event::ActivateHighlighted),
            Some(Output::Activated(2))
        ));
        assert!(state.update(Event::Activate(5)).is_none());
    }

    #[test]
    fn checking_a_parent_checks_its_enabled_children() {
        let mut state = open().with_mode(Mode::Checkbox);
        assert_eq!(checked(state.update(Event::Check(3, true))), [3, 4, 6]);
        assert_eq!(state.check_state(3), CheckState::Checked);
        assert_eq!(
            state.check_state(5),
            CheckState::Unchecked,
            "disabled stays"
        );
        assert_eq!(state.check_state(1), CheckState::Indeterminate);

        assert_eq!(
            checked(state.update(Event::Check(2, true))),
            [1, 2, 3, 4, 6]
        );
        assert_eq!(state.check_state(1), CheckState::Checked);

        assert_eq!(checked(state.update(Event::Check(4, false))), [2, 6]);
        assert_eq!(state.check_state(3), CheckState::Indeterminate);
        assert_eq!(state.check_state(1), CheckState::Indeterminate);

        assert_eq!(
            checked(state.update(Event::Check(1, false))),
            Vec::<u32>::new()
        );
        assert_eq!(state.check_state(3), CheckState::Unchecked);
    }

    #[test]
    fn an_indeterminate_parent_checks_everything_on_the_next_click() {
        let mut state = open().with_mode(Mode::Checkbox);
        let _ = state.update(Event::Check(4, true));
        assert_eq!(state.check_state(3), CheckState::Indeterminate);
        let next = state.check_state(3).toggled().is_checked();
        assert_eq!(checked(state.update(Event::Check(3, next))), [3, 4, 6]);
    }

    #[test]
    fn presses_and_space_toggle_checks_in_checkbox_mode() {
        let mut state = open().with_mode(Mode::Checkbox);
        assert_eq!(
            checked(state.update(Event::Press(6, Modifiers::empty()))),
            [6]
        );
        assert_eq!(state.check_state(3), CheckState::Indeterminate);
        assert_eq!(
            checked(state.update(Event::ToggleHighlighted)),
            Vec::<u32>::new()
        );
        assert!(state.update(Event::Check(5, true)).is_none(), "disabled");
        assert!(state.selected().is_empty(), "checkbox mode never selects");
        assert!(
            open().update(Event::Check(2, true)).is_none(),
            "other modes"
        );
    }

    #[test]
    fn children_loading_under_a_checked_parent_arrive_checked() {
        let mut state = sample().with_mode(Mode::Checkbox);
        let _ = state.update(Event::Check(7, true));
        let _ = state.update(Event::Expand(7));
        let output = state.update(Event::Received(vec![loaded(
            7,
            [node(70, "a"), node(71, "b").disabled(true)],
        )]));
        assert_eq!(checked(output), [7, 70]);
        assert_eq!(state.check_state(7), CheckState::Checked);
    }

    #[test]
    fn disabled_nodes_ignore_everything() {
        let mut state = open();
        let _ = state.update(Event::Press(5, Modifiers::empty()));
        assert_eq!(state.highlighted(), None);
        let disabled = State::new([node(1, "a").disabled(true).children([node(2, "b")])]);
        let mut disabled = disabled;
        assert!(disabled.update(Event::Toggle(1)).is_none());
        assert!(!disabled.is_expanded(1));
    }

    #[test]
    fn focus_is_tracked_and_lands_on_a_node() {
        let mut state = sample();
        let _ = state.update(Event::Focus(true));
        assert!(state.is_focused());
        assert_eq!(state.highlighted(), Some(1), "the first node");
        let _ = state.update(Event::Focus(false));
        assert!(!state.is_focused());
        assert_eq!(state.highlighted(), Some(1), "the highlight stays");

        let mut selected = open();
        let _ = selected.update(Event::Press(6, Modifiers::empty()));
        let _ = selected.update(Event::Collapse(3));
        let _ = selected.update(Event::Focus(true));
        assert_eq!(
            selected.highlighted(),
            Some(3),
            "an existing highlight is kept"
        );

        let mut fresh = open().with_mode(Mode::Multiple);
        let _ = fresh.update(Event::SelectAll);
        let _ = fresh.update(Event::Focus(true));
        assert_eq!(fresh.highlighted(), Some(1), "the first selected node");
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
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::Expand));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::Collapse));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Space"), Some(Action::Select));
        assert_eq!(press(&keymap, "*"), Some(Action::ExpandSiblings));
        assert_eq!(press(&keymap, "Shift+ArrowDown"), Some(Action::ExtendNext));
        assert_eq!(
            press(&keymap, "Shift+ArrowUp"),
            Some(Action::ExtendPrevious)
        );
        assert_eq!(press(&keymap, "Mod+A"), Some(Action::SelectAll));
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
            assert!(keys::Action::description(*action).ends_with('.'));
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&Chord::character('*'))
            .bind(Chord::character('l'), Action::Expand);
        let state = open();
        assert!(
            state
                .key_event(&keymap, &key(Key::Character("*".into()), Modifiers::SHIFT))
                .is_none()
        );
        let mut state = state;
        let _ = state.update(Event::First);
        assert!(matches!(
            state.key_event(
                &keymap,
                &key(Key::Character("l".into()), Modifiers::empty())
            ),
            Some(Event::ExpandOrEnter)
        ));
        assert!(default_keymap().clear().is_empty());
    }

    #[test]
    fn key_events_use_the_keymap_then_typeahead() {
        let keymap = default_keymap();
        let mut state = open();
        let _ = state.update(Event::First);
        let star = key(Key::Character("*".into()), Modifiers::SHIFT);
        assert!(matches!(
            state.key_event(&keymap, &star),
            Some(Event::ExpandSiblings)
        ));
        let letter = key(Key::Character("r".into()), Modifiers::empty());
        assert!(matches!(
            state.key_event(&keymap, &letter),
            Some(Event::Typeahead('r'))
        ));
        let ctrl_letter = key(Key::Character("r".into()), Modifiers::CTRL);
        assert!(state.key_event(&keymap, &ctrl_letter).is_none());
        let shifted_letter = key(Key::Character("R".into()), Modifiers::SHIFT);
        assert!(matches!(
            state.key_event(&keymap, &shifted_letter),
            Some(Event::Typeahead('R'))
        ));
    }

    #[test]
    fn actions_map_to_events_only_when_they_apply() {
        let mut state = open();
        assert!(Action::Next.event(&state).is_some());
        assert!(
            Action::Expand.event(&state).is_none(),
            "nothing highlighted"
        );
        assert!(Action::Activate.event(&state).is_none());
        assert!(
            Action::SelectAll.event(&state).is_none(),
            "single selection"
        );
        let _ = state.update(Event::First);
        assert!(matches!(
            Action::Expand.event(&state),
            Some(Event::ExpandOrEnter)
        ));
        assert!(matches!(
            Action::Collapse.event(&state),
            Some(Event::CollapseOrLeave)
        ));
        assert!(matches!(
            Action::Select.event(&state),
            Some(Event::ToggleHighlighted)
        ));
        assert!(matches!(
            Action::ExtendNext.event(&state),
            Some(Event::Next)
        ));
        assert!(matches!(
            Action::ExtendPrevious.event(&state),
            Some(Event::Previous)
        ));

        let multiple = open().with_mode(Mode::Multiple);
        assert!(matches!(
            Action::SelectAll.event(&multiple),
            Some(Event::SelectAll)
        ));
        assert!(matches!(
            Action::ExtendNext.event(&multiple),
            Some(Event::ExtendNext)
        ));

        let mut none = open().with_mode(Mode::None);
        let _ = none.update(Event::First);
        assert!(Action::Select.event(&none).is_none());
    }

    #[test]
    fn stream_emits_ready_first_then_batches() {
        use iced::futures::SinkExt;
        use iced::futures::executor::block_on;

        let mut events = Box::pin(stream::<u32>());
        let Some(Event::Ready(mut sender)) = block_on(events.next()) else {
            panic!("first event must be Ready");
        };
        block_on(async {
            for parent in [1, 2, 3] {
                sender.send(loaded(parent, [])).await.ok();
            }
        });
        let Some(Event::Received(batch)) = block_on(events.next()) else {
            panic!("expected a batch");
        };
        let parents: Vec<_> = batch.iter().map(|loaded| loaded.parent).collect();
        assert_eq!(parents, [1, 2, 3]);
    }

    #[test]
    fn ready_hands_the_sender_back() {
        let (sender, _receiver) = mpsc::channel(1);
        assert!(matches!(
            sample().update(Event::Ready(sender)),
            Some(Output::Ready(_))
        ));
    }

    #[test]
    fn rows_follow_the_menu_look_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = row_style(&tokens, Status::default());
            assert_eq!(idle.background, None);
            assert_eq!(idle.text, tokens.foreground);
            assert_eq!(idle.ring, None);

            let selected = row_style(
                &tokens,
                Status {
                    selected: true,
                    ..Status::default()
                },
            );
            assert_eq!(selected.background, Some(tokens.accent));
            assert_eq!(selected.text, tokens.accent_foreground);

            let hovered = row_style(
                &tokens,
                Status {
                    hovered: true,
                    ..Status::default()
                },
            );
            assert!(
                hovered
                    .background
                    .is_some_and(|colour| colour.a < tokens.accent.a)
            );

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
            assert_eq!(disabled.text, tokens.muted_foreground);
            assert_eq!(disabled.ring, None);
            assert_eq!(guide_colour(&tokens), tokens.border);
        }
    }

    #[test]
    fn builder_defaults_and_options() {
        let state = sample();
        let view: Tree<'_, u32, ()> = tree(&state);
        assert!(!view.guides && !view.is_enabled());
        assert_eq!((view.width, view.height), (Length::Fill, Length::Shrink));
        assert_eq!(view.keymap, default_keymap());
        let view = view
            .guides(true)
            .width(240)
            .height(300)
            .keymap(Keymap::new())
            .on_event(|_| ());
        assert!(view.guides && view.is_enabled());
        assert_eq!(view.width, Length::Fixed(240.0));
        assert!(view.keymap.is_empty());
    }

    #[test]
    fn node_builders() {
        let folder = node(1, "src").folder().badge("3");
        assert_eq!(folder.icon, Some(crate::lucide!(Folder)));
        assert_eq!(folder.open_icon, Some(crate::lucide!(FolderOpen)));
        assert_eq!(folder.trailing, Some(Trailing::Badge("3".into())));
        let file = node(2, "a.txt").file().trailing("4 KB").disabled(true);
        assert_eq!(file.icon, Some(crate::lucide!(File)));
        assert_eq!(file.trailing, Some(Trailing::Text("4 KB".into())));
        assert!(file.disabled);
        assert_eq!(Mode::ALL.len(), 4);
    }
}
