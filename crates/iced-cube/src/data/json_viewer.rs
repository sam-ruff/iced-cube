//! A JSON document shown as rows that expand and collapse.
//!
//! [`State`] flattens a [`serde_json::Value`] into nodes, or parses JSON
//! text with [`State::parse`] and keeps the error when it is not valid. It
//! holds which objects and arrays are expanded, the highlighted node and
//! the search. Its [`update`](State::update) handles pointer and keyboard
//! navigation, expanding everything or to a depth, searching and copying.
//!
//! Objects and arrays show how many members they hold, `{3}` or `[12]`.
//! Values are coloured by type from the theme's [`Tokens`], see [`syntax`],
//! and a long value ends in an ellipsis on one line. Copy it to read it in
//! full. The view only builds the rows in view, so a document with
//! thousands of nodes stays quick.
//!
//! Copying never touches the clipboard itself: [`Output::CopyValue`] holds
//! the value as pretty-printed JSON and [`Output::CopyPath`] its path, such
//! as `$.users[2].name`, for the app to write. Paths start at `$`, the
//! document, and use `["key"]` for keys that are not plain identifiers.
//!
//! The viewer resolves its keys through a [`Keymap`] of [`Action`]s while
//! it has focus, which it takes when pressed; see [`default_keymap`].

use std::collections::HashSet;
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use iced::widget::{self, Space, column, container, mouse_area, row, text};
use iced::{Alignment, Border, Color, Element, Length, Theme, mouse};
use serde_json::Value;

use crate::data::ellipsis::ellipsis;
use crate::data::interaction::{Look, RowMenu, pressable, row_menu, scope};
use crate::data::rows::rows;
use crate::feedback::alert::{self, alert};
use crate::icon::{opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::context_menu;
use crate::overlay::dropdown_menu::{item, separator};
use crate::overlay::menu::{ICON_SIZE, ROW_PADDING};
use crate::primitives::button::Size;
use crate::primitives::icon_button::icon_button;
use crate::primitives::input::input;
use crate::theme::{Tokens, fade, mix, radius, space, text_size};

/// Height of every row, as in a menu.
pub const ROW_HEIGHT: f32 = 32.0;
/// Width of one level of indentation, which is also the chevron's slot.
pub const INDENT: f32 = 20.0;
/// The widest a key gets before it ends in an ellipsis, by default.
pub const KEY_WIDTH: f32 = 180.0;
/// The id of the search box, unless [`JsonViewer::search_id`] sets another.
pub const SEARCH: widget::Id = widget::Id::new("json-viewer-search");

/// A node in the document, numbered in document order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(usize);

impl NodeId {
    /// The node's position in document order, starting at the document
    /// itself.
    pub fn index(self) -> usize {
        self.0
    }
}

/// The type of a JSON value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Object,
    Array,
    String,
    Number,
    Bool,
    Null,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Object,
        Kind::Array,
        Kind::String,
        Kind::Number,
        Kind::Bool,
        Kind::Null,
    ];

    pub fn of(value: &Value) -> Kind {
        match value {
            Value::Object(_) => Kind::Object,
            Value::Array(_) => Kind::Array,
            Value::String(_) => Kind::String,
            Value::Number(_) => Kind::Number,
            Value::Bool(_) => Kind::Bool,
            Value::Null => Kind::Null,
        }
    }

    /// Whether values of this kind hold other values.
    pub fn is_container(self) -> bool {
        matches!(self, Kind::Object | Kind::Array)
    }
}

/// How a node is reached from its parent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Segment {
    /// A member of an object.
    Key(String),
    /// An element of an array.
    Index(usize),
}

/// Why JSON text could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    /// Where the problem is, counted from 1.
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for ParseError {}

impl From<serde_json::Error> for ParseError {
    fn from(error: serde_json::Error) -> Self {
        let (line, column) = (error.line(), error.column());
        let full = error.to_string();
        let suffix = format!(" at line {line} column {column}");
        let message = full.strip_suffix(&suffix).unwrap_or(&full).to_owned();
        Self {
            message,
            line,
            column,
        }
    }
}

#[derive(Debug, Clone)]
struct Item {
    segment: Option<Segment>,
    parent: Option<NodeId>,
    depth: usize,
    kind: Kind,
    /// A scalar as JSON, such as `"text"` or `12`. Empty for containers.
    text: String,
    /// The lower-case key and value the search looks in.
    key_lower: Option<String>,
    text_lower: String,
    children: Vec<NodeId>,
}

/// Everything that changes a viewer.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Expands or collapses a node, as its chevron does.
    Toggle(NodeId),
    Expand(NodeId),
    Collapse(NodeId),
    /// Expands a node and everything under it.
    ExpandSubtree(NodeId),
    /// Collapses a node and everything under it.
    CollapseSubtree(NodeId),
    ExpandAll,
    CollapseAll,
    /// Expands every object and array fewer than this many levels deep and
    /// collapses the rest, so `0` collapses everything.
    ExpandToDepth(usize),
    /// The pointer pressed a row.
    Press(NodeId),
    Next,
    Previous,
    First,
    Last,
    /// Expands the highlighted node, or moves to its first child when it
    /// is already expanded.
    ExpandOrEnter,
    /// Collapses the highlighted node, or moves to its parent when it is
    /// already collapsed.
    CollapseOrLeave,
    ToggleHighlighted,
    /// Expands the highlighted node and every sibling that has children.
    ExpandSiblings,
    /// Searches keys and values for this text, ignoring case, and expands
    /// the parents of every match.
    Search(String),
    NextMatch,
    PreviousMatch,
    /// Shows only matches, their parents and what is inside them.
    Filter(bool),
    CopyValue(NodeId),
    CopyPath(NodeId),
    /// Asks the app to focus the search box.
    FocusSearch,
    /// The viewer gained or lost focus. The widget sends this itself.
    Focus(bool),
}

/// What the app may need to act on after an [`Event`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// A value as pretty-printed JSON, to write to the clipboard.
    CopyValue(String),
    /// A path such as `$.users[2].name`, to write to the clipboard.
    CopyPath(String),
    /// Focus the search box, such as with
    /// `iced::widget::operation::focus(json_viewer::SEARCH)`.
    FocusSearch,
}

/// The document, what is expanded and highlighted, and the search.
#[derive(Debug, Clone)]
pub struct State {
    value: Value,
    items: Vec<Item>,
    roots: Vec<NodeId>,
    expanded: HashSet<NodeId>,
    highlighted: Option<NodeId>,
    focused: bool,
    query: String,
    matches: Vec<NodeId>,
    matched: HashSet<NodeId>,
    /// The ancestors of every match.
    trail: HashSet<NodeId>,
    current: Option<usize>,
    filter: bool,
    error: Option<ParseError>,
}

impl Default for State {
    fn default() -> Self {
        Self::new(Value::Null)
    }
}

impl State {
    /// A viewer of `value` with everything collapsed. An object or array
    /// shows its members as the top rows; anything else shows as one row.
    pub fn new(value: Value) -> Self {
        let (items, roots) = flatten(&value);
        Self {
            value,
            items,
            roots,
            expanded: HashSet::new(),
            highlighted: None,
            focused: false,
            query: String::new(),
            matches: Vec::new(),
            matched: HashSet::new(),
            trail: HashSet::new(),
            current: None,
            filter: false,
            error: None,
        }
    }

    /// Parses JSON text. Text that is not valid JSON gives an empty viewer
    /// that shows the [`error`](Self::error) in place of rows.
    pub fn parse(json: &str) -> Self {
        match serde_json::from_str(json) {
            Ok(value) => Self::new(value),
            Err(error) => Self {
                error: Some(error.into()),
                ..Self::new(Value::Null)
            }
            .without_rows(),
        }
    }

    fn without_rows(mut self) -> Self {
        self.items.clear();
        self.roots.clear();
        self
    }

    /// Starts with every object and array fewer than `depth` levels deep
    /// expanded.
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.expand_to_depth(depth);
        self
    }

    /// Starts with only the matches shown while searching.
    pub fn with_filter(mut self, filter: bool) -> Self {
        self.filter = filter;
        self
    }

    /// The whole document.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Why the text given to [`parse`](Self::parse) is not valid JSON.
    pub fn error(&self) -> Option<&ParseError> {
        self.error.as_ref()
    }

    /// How many nodes the document holds, the document itself included.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The top rows.
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn kind(&self, id: NodeId) -> Option<Kind> {
        self.item(id).map(|item| item.kind)
    }

    /// The key or index the node has in its parent, `None` for the document.
    pub fn segment(&self, id: NodeId) -> Option<&Segment> {
        self.item(id).and_then(|item| item.segment.as_ref())
    }

    /// The node's parent row, `None` for a top row.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.item(id).and_then(|item| item.parent)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.item(id).map_or(&[], |item| &item.children)
    }

    /// How far the row is indented: 0 for a top row.
    pub fn depth(&self, id: NodeId) -> usize {
        self.item(id).map_or(0, |item| item.depth)
    }

    pub fn is_expanded(&self, id: NodeId) -> bool {
        self.expanded.contains(&id)
    }

    /// The node the keyboard acts on.
    pub fn highlighted(&self) -> Option<NodeId> {
        self.highlighted
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// The search text.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The nodes whose key or value contains the search text, in document
    /// order.
    pub fn matches(&self) -> &[NodeId] {
        &self.matches
    }

    pub fn is_match(&self, id: NodeId) -> bool {
        self.matched.contains(&id)
    }

    /// The match [`Event::NextMatch`] last moved to.
    pub fn current_match(&self) -> Option<NodeId> {
        self.current
            .and_then(|index| self.matches.get(index))
            .copied()
    }

    /// Whether only matches are shown: the filter is on and there is a
    /// search.
    pub fn is_filtering(&self) -> bool {
        self.filter && !self.query.trim().is_empty()
    }

    /// Whether the filter is on, searching or not.
    pub fn filter(&self) -> bool {
        self.filter
    }

    /// The rows on screen when scrolled through, in document order.
    pub fn visible(&self) -> Vec<NodeId> {
        let filtering = self.is_filtering();
        let mut out = Vec::new();
        let mut stack: Vec<(NodeId, bool)> =
            self.roots.iter().rev().map(|&id| (id, false)).collect();
        while let Some((id, inside)) = stack.pop() {
            let matched = self.matched.contains(&id);
            if filtering && !inside && !matched && !self.trail.contains(&id) {
                continue;
            }
            out.push(id);
            if !self.expanded.contains(&id) {
                continue;
            }
            let inside = inside || matched;
            stack.extend(self.children(id).iter().rev().map(|&child| (child, inside)));
        }
        out
    }

    /// The node's path from the document, such as `$.users[2].name` or
    /// `$["first name"]`.
    pub fn path(&self, id: NodeId) -> String {
        let mut path = String::from("$");
        for segment in self.segments(id).into_iter().rev() {
            match segment {
                Segment::Index(index) => path.push_str(&format!("[{index}]")),
                Segment::Key(key) if is_identifier(key) => {
                    path.push('.');
                    path.push_str(key);
                }
                Segment::Key(key) => {
                    let quoted = serde_json::to_string(key).unwrap_or_default();
                    path.push_str(&format!("[{quoted}]"));
                }
            }
        }
        path
    }

    /// The node's value.
    pub fn value_at(&self, id: NodeId) -> Option<&Value> {
        self.item(id)?;
        self.segments(id)
            .into_iter()
            .rev()
            .try_fold(&self.value, |value, segment| match segment {
                Segment::Key(key) => value.get(key.as_str()),
                Segment::Index(index) => value.get(*index),
            })
    }

    /// The node's value as pretty-printed JSON.
    pub fn pretty(&self, id: NodeId) -> Option<String> {
        self.value_at(id)
            .and_then(|value| serde_json::to_string_pretty(value).ok())
    }

    /// Applies an event and returns what the app may need to act on.
    pub fn update(&mut self, event: Event) -> Option<Output> {
        match event {
            Event::Toggle(id) if self.is_expanded(id) => self.collapse(id),
            Event::Toggle(id) | Event::Expand(id) => self.expand(id),
            Event::Collapse(id) => self.collapse(id),
            Event::ExpandSubtree(id) => {
                let ids: Vec<NodeId> = self
                    .subtree(id)
                    .into_iter()
                    .filter(|&id| self.has_children(id))
                    .collect();
                self.expanded.extend(ids);
            }
            Event::CollapseSubtree(id) => {
                for id in self.subtree(id) {
                    let _ = self.expanded.remove(&id);
                }
                self.reveal_highlight();
            }
            Event::ExpandAll => {
                let all: Vec<NodeId> = (0..self.items.len())
                    .map(NodeId)
                    .filter(|&id| self.has_children(id))
                    .collect();
                self.expanded.extend(all);
            }
            Event::CollapseAll => {
                self.expanded.clear();
                self.reveal_highlight();
            }
            Event::ExpandToDepth(depth) => self.expand_to_depth(depth),
            Event::Press(id) if self.item(id).is_some() => self.highlighted = Some(id),
            Event::Press(_) => {}
            Event::Next => self.step(Step::Next),
            Event::Previous => self.step(Step::Previous),
            Event::First => self.step(Step::First),
            Event::Last => self.step(Step::Last),
            Event::ExpandOrEnter => self.expand_or_enter(),
            Event::CollapseOrLeave => self.collapse_or_leave(),
            Event::ToggleHighlighted => {
                if let Some(id) = self.highlighted {
                    return self.update(Event::Toggle(id));
                }
            }
            Event::ExpandSiblings => self.expand_siblings(),
            Event::Search(query) => self.search(query),
            Event::NextMatch => self.cycle(true),
            Event::PreviousMatch => self.cycle(false),
            Event::Filter(filter) => self.filter = filter,
            Event::CopyValue(id) => {
                self.highlight(id);
                return self.pretty(id).map(Output::CopyValue);
            }
            Event::CopyPath(id) => {
                self.highlight(id);
                return self.item(id).map(|_| Output::CopyPath(self.path(id)));
            }
            Event::FocusSearch => return Some(Output::FocusSearch),
            Event::Focus(focused) => {
                self.focused = focused;
                if focused && self.highlighted.is_none() {
                    self.highlighted = self.roots.first().copied();
                }
            }
        }
        None
    }

    /// Turns a key press into an event through `keymap`. A symbol typed
    /// with Shift, such as `*`, also matches its chord without Shift.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event> {
        let action = keymap.resolve_event(key).or_else(|| {
            let symbol = matches!(&key.key, Key::Character(text) if !text.chars().any(char::is_alphanumeric));
            (symbol && key.modifiers.shift())
                .then(|| keymap.resolve(&key.key, key.modifiers.difference(Modifiers::SHIFT)))
                .flatten()
        })?;
        action.event(self)
    }

    fn item(&self, id: NodeId) -> Option<&Item> {
        self.items.get(id.0)
    }

    fn has_children(&self, id: NodeId) -> bool {
        !self.children(id).is_empty()
    }

    fn highlight(&mut self, id: NodeId) {
        if self.item(id).is_some() {
            self.highlighted = Some(id);
        }
    }

    /// The segments from the node up to the document.
    fn segments(&self, id: NodeId) -> Vec<&Segment> {
        let mut segments = Vec::new();
        let mut current = Some(id);
        while let Some(node) = current {
            let Some(item) = self.item(node) else {
                break;
            };
            segments.extend(item.segment.as_ref());
            current = item.parent;
        }
        segments
    }

    /// The node and everything under it.
    fn subtree(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(node) = stack.pop() {
            if self.item(node).is_none() {
                continue;
            }
            out.push(node);
            stack.extend(self.children(node).iter().copied());
        }
        out
    }

    fn expand(&mut self, id: NodeId) {
        if self.has_children(id) {
            let _ = self.expanded.insert(id);
        }
    }

    fn collapse(&mut self, id: NodeId) {
        if self.expanded.remove(&id) {
            self.reveal_highlight();
        }
    }

    fn expand_to_depth(&mut self, depth: usize) {
        self.expanded = (0..self.items.len())
            .map(NodeId)
            .filter(|&id| self.depth(id) < depth && self.has_children(id))
            .collect();
        self.reveal_highlight();
    }

    /// Moves a highlight hidden in a collapsed branch up to the outermost
    /// collapsed node above it.
    fn reveal_highlight(&mut self) {
        let Some(id) = self.highlighted else {
            return;
        };
        let mut target = id;
        let mut current = self.parent(id);
        while let Some(parent) = current {
            if !self.expanded.contains(&parent) {
                target = parent;
            }
            current = self.parent(parent);
        }
        self.highlighted = Some(target);
    }

    fn step(&mut self, step: Step) {
        let rows = self.visible();
        let current = self
            .highlighted
            .and_then(|id| rows.iter().position(|&row| row == id));
        let target = match (step, current) {
            (Step::First, _) | (Step::Next, None) => rows.first(),
            (Step::Last, _) | (Step::Previous, None) => rows.last(),
            (Step::Next, Some(index)) => rows.get(index + 1).or(rows.get(index)),
            (Step::Previous, Some(index)) => rows.get(index.saturating_sub(1)),
        };
        if let Some(&target) = target {
            self.highlighted = Some(target);
        }
    }

    fn expand_or_enter(&mut self) {
        let Some(id) = self.highlighted else {
            return;
        };
        if !self.is_expanded(id) {
            self.expand(id);
            return;
        }
        let first = self
            .visible()
            .into_iter()
            .find(|&row| self.parent(row) == Some(id));
        if let Some(first) = first {
            self.highlighted = Some(first);
        }
    }

    fn collapse_or_leave(&mut self) {
        let Some(id) = self.highlighted else {
            return;
        };
        if self.is_expanded(id) {
            self.collapse(id);
            return;
        }
        if let Some(parent) = self.parent(id) {
            self.highlighted = Some(parent);
        }
    }

    fn expand_siblings(&mut self) {
        let Some(id) = self.highlighted else {
            return;
        };
        let siblings = match self.parent(id) {
            Some(parent) => self.children(parent).to_vec(),
            None => self.roots.clone(),
        };
        for sibling in siblings {
            self.expand(sibling);
        }
    }

    fn search(&mut self, query: String) {
        let needle = query.trim().to_lowercase();
        self.query = query;
        self.matches.clear();
        self.matched.clear();
        self.trail.clear();
        self.current = None;
        if needle.is_empty() {
            return;
        }
        self.matches = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.key_lower
                    .as_ref()
                    .is_some_and(|key| key.contains(&needle))
                    || item.text_lower.contains(&needle)
            })
            .map(|(index, _)| NodeId(index))
            .collect();
        self.matched = self.matches.iter().copied().collect();
        for &id in &self.matches {
            let mut current = self.parent(id);
            while let Some(parent) = current {
                if !self.trail.insert(parent) {
                    break;
                }
                current = self.parent(parent);
            }
        }
        self.expanded.extend(self.trail.iter().copied());
        if let Some(&first) = self.matches.first() {
            self.current = Some(0);
            self.highlighted = Some(first);
        }
    }

    fn cycle(&mut self, forward: bool) {
        let count = self.matches.len();
        if count == 0 {
            return;
        }
        let index = match (self.current, forward) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(index), true) => (index + 1) % count,
            (Some(index), false) => (index + count - 1) % count,
        };
        self.current = Some(index);
        self.highlighted = self.matches.get(index).copied();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Next,
    Previous,
    First,
    Last,
}

/// Whether a key can follow a dot in a path.
fn is_identifier(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_' || first == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// The nodes of `value` in document order, and the top rows. The members
/// of an object or array that has any are the top rows; otherwise the
/// document is the only row.
fn flatten(value: &Value) -> (Vec<Item>, Vec<NodeId>) {
    let mut items = Vec::new();
    let root = push(&mut items, value, None, None, 0);
    let members = items
        .get(root.0)
        .map(|item| item.children.clone())
        .unwrap_or_default();
    if members.is_empty() {
        return (items, vec![root]);
    }
    for item in items.iter_mut().skip(1) {
        item.depth -= 1;
        if item.parent == Some(root) {
            item.parent = None;
        }
    }
    (items, members)
}

fn push(
    items: &mut Vec<Item>,
    value: &Value,
    segment: Option<Segment>,
    parent: Option<NodeId>,
    depth: usize,
) -> NodeId {
    let id = NodeId(items.len());
    let text = match value {
        Value::Object(_) | Value::Array(_) => String::new(),
        scalar => serde_json::to_string(scalar).unwrap_or_default(),
    };
    let raw = match value {
        Value::String(string) => string.as_str(),
        _ => text.as_str(),
    };
    let key_lower = match &segment {
        Some(Segment::Key(key)) => Some(key.to_lowercase()),
        _ => None,
    };
    items.push(Item {
        text_lower: raw.to_lowercase(),
        segment,
        parent,
        depth,
        kind: Kind::of(value),
        text,
        key_lower,
        children: Vec::new(),
    });
    let children: Vec<NodeId> = match value {
        Value::Object(map) => map
            .iter()
            .map(|(key, child)| {
                push(
                    items,
                    child,
                    Some(Segment::Key(key.clone())),
                    Some(id),
                    depth + 1,
                )
            })
            .collect(),
        Value::Array(list) => list
            .iter()
            .enumerate()
            .map(|(index, child)| {
                push(
                    items,
                    child,
                    Some(Segment::Index(index)),
                    Some(id),
                    depth + 1,
                )
            })
            .collect(),
        _ => Vec::new(),
    };
    if let Some(item) = items.get_mut(id.0) {
        item.children = children;
    }
    id
}

/// What a viewer keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Last,
    Expand,
    Collapse,
    Toggle,
    ExpandSiblings,
    ExpandAll,
    CollapseAll,
    CopyValue,
    CopyPath,
    Search,
    NextMatch,
    PreviousMatch,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing, so the key is left for others.
    pub fn event(self, state: &State) -> Option<Event> {
        let highlighted = state.highlighted();
        let searching = !state.matches().is_empty();
        match self {
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::First => Some(Event::First),
            Action::Last => Some(Event::Last),
            Action::Expand => highlighted.map(|_| Event::ExpandOrEnter),
            Action::Collapse => highlighted.map(|_| Event::CollapseOrLeave),
            Action::Toggle => highlighted
                .filter(|&id| state.has_children(id))
                .map(|_| Event::ToggleHighlighted),
            Action::ExpandSiblings => highlighted.map(|_| Event::ExpandSiblings),
            Action::ExpandAll => Some(Event::ExpandAll),
            Action::CollapseAll => Some(Event::CollapseAll),
            Action::CopyValue => highlighted.map(Event::CopyValue),
            Action::CopyPath => highlighted.map(Event::CopyPath),
            Action::Search => Some(Event::FocusSearch),
            Action::NextMatch => searching.then_some(Event::NextMatch),
            Action::PreviousMatch => searching.then_some(Event::PreviousMatch),
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
        Action::Toggle,
        Action::ExpandSiblings,
        Action::ExpandAll,
        Action::CollapseAll,
        Action::CopyValue,
        Action::CopyPath,
        Action::Search,
        Action::NextMatch,
        Action::PreviousMatch,
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
            Action::Toggle => "Toggle",
            Action::ExpandSiblings => "ExpandSiblings",
            Action::ExpandAll => "ExpandAll",
            Action::CollapseAll => "CollapseAll",
            Action::CopyValue => "CopyValue",
            Action::CopyPath => "CopyPath",
            Action::Search => "Search",
            Action::NextMatch => "NextMatch",
            Action::PreviousMatch => "PreviousMatch",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next row.",
            Action::Previous => "Highlights the previous row.",
            Action::First => "Highlights the first row.",
            Action::Last => "Highlights the last row.",
            Action::Expand => {
                "Expands the highlighted object or array, or moves to its first member when it is open."
            }
            Action::Collapse => {
                "Collapses the highlighted object or array, or moves to its parent when it is closed."
            }
            Action::Toggle => "Expands or collapses the highlighted object or array.",
            Action::ExpandSiblings => "Expands the highlighted node and all of its siblings.",
            Action::ExpandAll => "Expands every object and array.",
            Action::CollapseAll => "Collapses every object and array.",
            Action::CopyValue => "Copies the highlighted value as pretty-printed JSON.",
            Action::CopyPath => "Copies the path of the highlighted value.",
            Action::Search => "Moves to the search box.",
            Action::NextMatch => "Highlights the next search match.",
            Action::PreviousMatch => "Highlights the previous search match.",
        }
    }
}

/// The default viewer shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
/// | `ArrowRight` | [`Action::Expand`] |
/// | `ArrowLeft` | [`Action::Collapse`] |
/// | `Enter`, `Space` | [`Action::Toggle`] |
/// | `*` | [`Action::ExpandSiblings`] |
/// | `Shift+ArrowRight` | [`Action::ExpandAll`] |
/// | `Shift+ArrowLeft` | [`Action::CollapseAll`] |
/// | `Mod+C` (Ctrl, or Cmd on macOS) | [`Action::CopyValue`] |
/// | `Mod+Shift+C` | [`Action::CopyPath`] |
/// | `Mod+F` | [`Action::Search`] |
/// | `F3` | [`Action::NextMatch`] |
/// | `Shift+F3` | [`Action::PreviousMatch`] |
///
/// The viewer resolves these itself, only while it has focus, so they
/// never clash with other components. Pass a changed keymap with
/// [`JsonViewer::keymap`].
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
        .bind(Chord::named(Named::ArrowRight), Action::Expand)
        .bind(Chord::named(Named::ArrowLeft), Action::Collapse)
        .bind(Chord::named(Named::Enter), Action::Toggle)
        .bind(Chord::named(Named::Space), Action::Toggle)
        .bind(Chord::character('*'), Action::ExpandSiblings)
        .bind(Chord::named(Named::ArrowRight).shift(), Action::ExpandAll)
        .bind(Chord::named(Named::ArrowLeft).shift(), Action::CollapseAll)
        .bind(Chord::character('c').command(), Action::CopyValue)
        .bind(Chord::character('c').command().shift(), Action::CopyPath)
        .bind(Chord::character('f').command(), Action::Search)
        .bind(Chord::named(Named::F3), Action::NextMatch)
        .bind(Chord::named(Named::F3).shift(), Action::PreviousMatch)
}

/// An entry of the context menu from [`menu`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuItem {
    CopyValue,
    CopyPath,
    ExpandAll,
    CollapseAll,
}

impl MenuItem {
    pub const ALL: [MenuItem; 4] = [
        MenuItem::CopyValue,
        MenuItem::CopyPath,
        MenuItem::ExpandAll,
        MenuItem::CollapseAll,
    ];

    /// The event the entry sends for the node the menu opened on.
    pub fn event(self, id: NodeId) -> Event {
        match self {
            MenuItem::CopyValue => Event::CopyValue(id),
            MenuItem::CopyPath => Event::CopyPath(id),
            MenuItem::ExpandAll => Event::ExpandSubtree(id),
            MenuItem::CollapseAll => Event::CollapseSubtree(id),
        }
    }
}

/// A context menu to pass to [`JsonViewer::context_menu`]: copy the value
/// or its path, and expand or collapse everything inside it. Turn the
/// chosen entry into an event with [`MenuItem::event`].
pub fn menu() -> context_menu::State<MenuItem, NodeId> {
    context_menu::State::new([
        item(MenuItem::CopyValue, "Copy value").icon(crate::lucide!(Copy)),
        item(MenuItem::CopyPath, "Copy path").icon(crate::lucide!(Route)),
        separator(),
        item(MenuItem::ExpandAll, "Expand all").icon(crate::lucide!(ChevronsUpDown)),
        item(MenuItem::CollapseAll, "Collapse all").icon(crate::lucide!(ChevronsDownUp)),
    ])
}

/// The colours of keys, values and punctuation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Syntax {
    pub key: Color,
    /// Array indices and the `{3}` and `[12]` summaries.
    pub punctuation: Color,
    pub string: Color,
    pub number: Color,
    pub boolean: Color,
    pub null: Color,
}

/// Resolves the syntax colours from the theme: strings take the success
/// colour, numbers the warning colour and booleans the destructive colour.
/// In light themes they are darkened towards the text so they stay
/// readable on a white page.
pub fn syntax(tokens: &Tokens) -> Syntax {
    let readable = |colour: Color| {
        if tokens.is_dark {
            colour
        } else {
            mix(colour, tokens.foreground, 0.25)
        }
    };
    Syntax {
        key: tokens.foreground,
        punctuation: tokens.muted_foreground,
        string: readable(tokens.success),
        number: readable(tokens.warning),
        boolean: readable(tokens.destructive),
        null: tokens.muted_foreground,
    }
}

/// The colour of a value of this kind.
pub fn value_colour(tokens: &Tokens, kind: Kind) -> Color {
    let syntax = syntax(tokens);
    match kind {
        Kind::Object | Kind::Array => syntax.punctuation,
        Kind::String => syntax.string,
        Kind::Number => syntax.number,
        Kind::Bool => syntax.boolean,
        Kind::Null => syntax.null,
    }
}

/// How a row is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Status {
    /// The row the keyboard acts on.
    pub highlighted: bool,
    /// Draws the focus ring on the highlighted row.
    pub focused: bool,
    pub hovered: bool,
    /// The key or value contains the search text.
    pub matched: bool,
    /// The match the search moved to last.
    pub current: bool,
    pub disabled: bool,
}

/// The colours of a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStyle {
    pub background: Option<Color>,
    pub ring: Option<Color>,
}

/// Resolves a row's colours. The highlighted row takes the accent, as a
/// highlighted menu row does, and search matches a tint of the warning
/// colour, stronger on the current one.
pub fn row_style(tokens: &Tokens, status: Status) -> RowStyle {
    if status.disabled {
        return RowStyle {
            background: None,
            ring: None,
        };
    }
    let background = if status.highlighted {
        Some(tokens.accent)
    } else if status.current {
        Some(fade(tokens.warning, 0.35))
    } else if status.matched {
        Some(fade(tokens.warning, 0.15))
    } else if status.hovered {
        Some(fade(tokens.accent, 0.6))
    } else {
        None
    };
    RowStyle {
        background,
        ring: (status.highlighted && status.focused).then_some(tokens.ring),
    }
}

type OnEvent<'a, Message> = dyn Fn(Event) -> Message + 'a;
type BuildMenu<'a, Message> =
    dyn FnOnce(Keymap<context_menu::Action>) -> RowMenu<'a, NodeId, Message> + 'a;

/// A JSON viewer builder. Convert it into an [`Element`] to render.
pub struct JsonViewer<'a, Message> {
    state: &'a State,
    search: bool,
    placeholder: String,
    search_id: widget::Id,
    key_width: f32,
    width: Length,
    height: Length,
    keymap: Keymap<Action>,
    menu: Option<Box<BuildMenu<'a, Message>>>,
    menu_keymap: Keymap<context_menu::Action>,
    on_event: Option<Box<OnEvent<'a, Message>>>,
}

/// Renders `state`. Without [`on_event`](JsonViewer::on_event) it renders
/// disabled.
pub fn json_viewer<Message>(state: &State) -> JsonViewer<'_, Message> {
    JsonViewer {
        state,
        search: false,
        placeholder: "Search keys and values...".to_owned(),
        search_id: SEARCH,
        key_width: KEY_WIDTH,
        width: Length::Fill,
        height: Length::Shrink,
        keymap: default_keymap(),
        menu: None,
        menu_keymap: context_menu::default_keymap(),
        on_event: None,
    }
}

impl<Message> fmt::Debug for JsonViewer<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JsonViewer")
            .field("state", self.state)
            .field("search", &self.search)
            .field("placeholder", &self.placeholder)
            .field("key_width", &self.key_width)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> JsonViewer<'a, Message> {
    /// Shows a search box above the rows, with the match count, buttons
    /// for the previous and next match and a toggle that shows only
    /// matches.
    pub fn search(mut self, search: bool) -> Self {
        self.search = search;
        self
    }

    /// The search box placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// The search box's widget id, for focusing it. Defaults to [`SEARCH`];
    /// give each viewer its own when a window has several.
    pub fn search_id(mut self, id: impl Into<widget::Id>) -> Self {
        self.search_id = id.into();
        self
    }

    /// The widest a key gets before it ends in an ellipsis. Defaults to
    /// [`KEY_WIDTH`].
    pub fn key_width(mut self, width: f32) -> Self {
        self.key_width = width;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The height of the rows. A fixed or filling height scrolls them.
    /// Defaults to shrinking to fit them.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Replaces the [`default_keymap`] the focused viewer resolves keys with.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// Gives every row the context menu of `menu`, keyed by node, such as
    /// the one from [`menu()`]. It opens on a right-click or a
    /// long press, and on the highlighted row with its opening chord while
    /// the viewer has focus.
    pub fn context_menu<MenuId>(
        mut self,
        menu: &'a context_menu::State<MenuId, NodeId>,
        on_event: impl Fn(context_menu::Event<MenuId, NodeId>) -> Message + 'a,
    ) -> Self
    where
        MenuId: Copy + PartialEq + 'a,
        Message: Clone + 'a,
    {
        let on_event: Rc<dyn Fn(context_menu::Event<MenuId, NodeId>) -> Message + 'a> =
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

/// Everything a row needs from the builder.
struct Context<'a, Message> {
    state: &'a State,
    key_width: f32,
    on_event: Option<Rc<OnEvent<'a, Message>>>,
}

impl<'a, Message> From<JsonViewer<'a, Message>> for Element<'a, Message>
where
    Message: Clone + 'a,
{
    fn from(viewer: JsonViewer<'a, Message>) -> Self {
        let JsonViewer {
            state,
            search,
            placeholder,
            search_id,
            key_width,
            width,
            height,
            keymap,
            menu,
            menu_keymap,
            on_event,
        } = viewer;
        if let Some(error) = state.error() {
            return container(
                alert("Invalid JSON")
                    .variant(alert::Variant::Destructive)
                    .description(error.to_string()),
            )
            .width(width)
            .into();
        }
        let on_event: Option<Rc<OnEvent<'a, Message>>> = on_event.map(Rc::from);
        let menu = menu
            .filter(|_| on_event.is_some())
            .map(|build| Rc::new(build(menu_keymap)));
        let lines = state.visible();
        let highlight = state
            .highlighted()
            .and_then(|id| lines.iter().position(|&line| line == id));
        let count = lines.len();
        let context = Rc::new(Context {
            state,
            key_width,
            on_event: on_event.clone(),
        });

        let build = {
            let menu = menu.clone();
            move |range: Range<usize>| -> Element<'a, Message> {
                let lines = lines.get(range).unwrap_or_default();
                column(lines.iter().map(|&id| {
                    let row = row_view(&context, id);
                    match &menu {
                        Some(menu) => menu.wrap(id, row),
                        None => row,
                    }
                }))
                .into()
            }
        };
        let body: Element<'a, Message> = if count == 0 && state.is_filtering() {
            container(
                text(format!("Nothing matches \"{}\"", state.query().trim()))
                    .size(text_size::SM)
                    .wrapping(text::Wrapping::None)
                    .style(|theme: &Theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
            )
            .padding(ROW_PADDING)
            .height(ROW_HEIGHT)
            .into()
        } else {
            rows(count, ROW_HEIGHT, build)
                .highlight(highlight)
                .height(height)
                .into()
        };
        let mut parts = column![].spacing(space::SM).width(width);
        if search {
            parts = parts.push(toolbar(state, &placeholder, search_id, on_event.clone()));
        }
        let parts = parts.push(body);

        let Some(on_event) = on_event else {
            return parts.into();
        };
        let on_focus = on_event.clone();
        scope(parts)
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

/// The text beside the search box: which match is current, of how many.
pub fn match_label(state: &State) -> String {
    if state.query().trim().is_empty() {
        return String::new();
    }
    let total = state.matches().len();
    match (total, state.current) {
        (0, _) => "No matches".to_owned(),
        (_, Some(index)) => format!("{} of {total}", index + 1),
        (_, None) => format!("{total} found"),
    }
}

fn toolbar<'a, Message: Clone + 'a>(
    state: &'a State,
    placeholder: &str,
    id: widget::Id,
    on_event: Option<Rc<OnEvent<'a, Message>>>,
) -> Element<'a, Message> {
    let send = |event: Event| on_event.as_ref().map(|on_event| on_event(event));
    let searching = !state.matches().is_empty();
    let field = {
        let on_input = on_event
            .clone()
            .map(|on_event| move |query| on_event(Event::Search(query)));
        input(placeholder, state.query())
            .id(id)
            .icon(crate::lucide!(Search))
            .width(Length::Fill)
            .on_input_maybe(on_input)
            .on_submit_maybe(send(Event::NextMatch).filter(|_| searching))
    };
    let count = text(match_label(state))
        .size(text_size::XS)
        .wrapping(text::Wrapping::None)
        .style(|theme: &Theme| text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        });
    row![
        field,
        count,
        icon_button(crate::lucide!(ChevronUp))
            .label("Previous match")
            .size(Size::Sm)
            .on_press_maybe(send(Event::PreviousMatch).filter(|_| searching)),
        icon_button(crate::lucide!(ChevronDown))
            .label("Next match")
            .size(Size::Sm)
            .on_press_maybe(send(Event::NextMatch).filter(|_| searching)),
        icon_button(crate::lucide!(ListFilter))
            .label("Only matches")
            .size(Size::Sm)
            .pressed(state.filter())
            .on_press_maybe(send(Event::Filter(!state.filter()))),
    ]
    .spacing(space::XS)
    .align_y(Alignment::Center)
    .into()
}

/// The indentation for `depth` levels.
fn indent<'a, Message: 'a>(depth: usize) -> Element<'a, Message> {
    Space::new()
        .width(INDENT * depth as f32)
        .height(Length::Fill)
        .into()
}

fn muted<'a>(content: String, enabled: bool) -> text::Text<'a> {
    text(content)
        .size(text_size::SM)
        .wrapping(text::Wrapping::None)
        .style(move |theme: &Theme| {
            let tokens = Tokens::of(theme);
            text::Style {
                color: Some(if enabled {
                    syntax(&tokens).punctuation
                } else {
                    tokens.muted_foreground
                }),
            }
        })
}

fn row_view<'a, Message: Clone + 'a>(
    context: &Context<'a, Message>,
    id: NodeId,
) -> Element<'a, Message> {
    let state = context.state;
    let Some(item) = state.item(id) else {
        return Space::new().into();
    };
    let on_event = context.on_event.as_ref();
    let enabled = on_event.is_some();
    let expanded = state.is_expanded(id);
    let highlighted = state.highlighted() == Some(id);
    let status = Status {
        highlighted,
        focused: state.is_focused(),
        hovered: false,
        matched: state.is_match(id),
        current: state.current_match() == Some(id),
        disabled: !enabled,
    };

    let chevron: Element<'a, Message> = if state.has_children(id) {
        let glyph = if expanded {
            crate::lucide!(ChevronDown)
        } else {
            crate::lucide!(ChevronRight)
        };
        let icon = container(themed(glyph, ICON_SIZE, opacity(enabled), |theme| {
            Tokens::of(theme).muted_foreground
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
        .spacing(space::XS)
        .align_y(Alignment::Center)
        .height(Length::Fill);
    match &item.segment {
        Some(Segment::Key(key)) => {
            let key = ellipsis(key.as_str())
                .width(Length::Shrink)
                .colour(move |theme| {
                    let tokens = Tokens::of(theme);
                    if enabled {
                        syntax(&tokens).key
                    } else {
                        tokens.muted_foreground
                    }
                });
            content = content.push(
                row![
                    container(key).max_width(context.key_width),
                    muted(":".to_owned(), enabled)
                ]
                .align_y(Alignment::Center),
            );
        }
        Some(Segment::Index(index)) => {
            content = content.push(muted(format!("{index}:"), enabled));
        }
        None => {}
    }
    let kind = item.kind;
    let value: Element<'a, Message> = match kind {
        Kind::Object | Kind::Array => {
            let (open, close) = if kind == Kind::Object {
                ('{', '}')
            } else {
                ('[', ']')
            };
            let count = item.children.len();
            let summary = if count == 0 {
                format!("{open}{close}")
            } else {
                format!("{open}{count}{close}")
            };
            row![muted(summary, enabled), Space::new().width(Length::Fill)].into()
        }
        _ => ellipsis(item.text.as_str())
            .colour(move |theme| {
                let tokens = Tokens::of(theme);
                if enabled {
                    value_colour(&tokens, kind)
                } else {
                    tokens.muted_foreground
                }
            })
            .into(),
    };
    content = content.push(value);
    if let Some(on_event) = on_event.filter(|_| highlighted) {
        content = content.push(
            row![
                icon_button(crate::lucide!(Copy))
                    .label("Copy value")
                    .size(Size::Sm)
                    .on_press(on_event(Event::CopyValue(id))),
                icon_button(crate::lucide!(Route))
                    .label("Copy path")
                    .size(Size::Sm)
                    .on_press(on_event(Event::CopyPath(id))),
            ]
            .align_y(Alignment::Center),
        );
    }

    let body = container(row![indent(item.depth), chevron, content].height(Length::Fill))
        .padding(
            iced::Padding::ZERO
                .left(ROW_PADDING.left)
                .right(ROW_PADDING.right),
        )
        .width(Length::Fill)
        .height(ROW_HEIGHT);
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
    let toggle = state.has_children(id).then(|| on_event(Event::Toggle(id)));
    row.on_press(move |_| on_event(Event::Press(id)))
        .on_double_click(toggle)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use serde_json::json;

    /// ```text
    /// name: "Ada"
    /// tags: [2]
    ///   0: "admin"
    ///   1: "ops"
    /// address: {2}
    ///   city: "London"
    ///   first line: null
    /// active: true
    /// age: 36
    /// ```
    fn sample() -> State {
        State::new(json!({
            "name": "Ada",
            "tags": ["admin", "ops"],
            "address": { "city": "London", "first line": null },
            "active": true,
            "age": 36
        }))
    }

    fn id(state: &State, path: &str) -> NodeId {
        (0..state.len())
            .map(NodeId)
            .find(|&id| state.path(id) == path)
            .unwrap_or_else(|| panic!("no node at {path}"))
    }

    fn key(key: Key, modifiers: Modifiers) -> keys::Event {
        keys::Event { key, modifiers }
    }

    #[test]
    fn flattens_in_document_order_with_the_members_as_top_rows() {
        let state = sample();
        assert_eq!(state.len(), 10);
        let paths: Vec<String> = state.roots().iter().map(|&id| state.path(id)).collect();
        assert_eq!(
            paths,
            ["$.name", "$.tags", "$.address", "$.active", "$.age"]
        );
        let tags = id(&state, "$.tags");
        assert_eq!(state.kind(tags), Some(Kind::Array));
        assert_eq!(state.children(tags).len(), 2);
        assert_eq!(state.parent(tags), None);
        assert_eq!(state.depth(tags), 0);
        let first = state.children(tags)[0];
        assert_eq!(state.parent(first), Some(tags));
        assert_eq!(state.depth(first), 1);
        assert_eq!(state.segment(first), Some(&Segment::Index(0)));
        assert_eq!(state.visible().len(), 5, "everything starts collapsed");
    }

    #[test]
    fn a_scalar_or_an_empty_document_is_one_row() {
        for value in [json!(42), json!("hi"), json!({}), json!([]), json!(null)] {
            let state = State::new(value.clone());
            assert_eq!(state.roots(), &[NodeId(0)], "{value}");
            assert_eq!(state.path(NodeId(0)), "$");
            assert_eq!(state.value_at(NodeId(0)), Some(&value));
        }
        assert_eq!(State::default().len(), 1);
    }

    #[test]
    fn kinds_and_containers() {
        assert_eq!(Kind::ALL.len(), 6);
        assert!(Kind::Object.is_container() && Kind::Array.is_container());
        assert!(!Kind::String.is_container());
        assert_eq!(Kind::of(&json!(1.5)), Kind::Number);
        assert_eq!(Kind::of(&json!(false)), Kind::Bool);
    }

    #[test]
    fn invalid_text_keeps_the_error_and_has_no_rows() {
        let state = State::parse("{\n  \"a\": 1,\n  \"b\": }");
        let error = state.error().expect("an error");
        assert_eq!((error.line, error.column), (3, 8));
        assert_eq!(error.message, "expected value");
        assert_eq!(error.to_string(), "expected value at line 3, column 8");
        assert!(state.is_empty());
        assert!(state.visible().is_empty());

        let valid = State::parse("[1, 2]");
        assert!(valid.error().is_none());
        assert_eq!(valid.roots().len(), 2);
    }

    #[test]
    fn paths_use_dots_for_identifiers_and_brackets_otherwise() {
        let state = State::new(json!({
            "users": [{ "name": "Ada" }, { "first name": "Grace", "$id": 2, "9lives": true }]
        }));
        assert_eq!(id(&state, "$.users[0].name").index(), 3);
        let second = state.children(id(&state, "$.users"))[1];
        let paths: Vec<String> = state
            .children(second)
            .iter()
            .map(|&id| state.path(id))
            .collect();
        assert_eq!(
            paths,
            [
                "$.users[1][\"first name\"]",
                "$.users[1].$id",
                "$.users[1][\"9lives\"]"
            ]
        );
        assert!(!is_identifier(""));
        assert!(is_identifier("_a1"));
    }

    #[test]
    fn copying_gives_pretty_json_and_the_path() {
        let mut state = sample();
        let address = id(&state, "$.address");
        assert_eq!(
            state.update(Event::CopyValue(address)),
            Some(Output::CopyValue(
                "{\n  \"city\": \"London\",\n  \"first line\": null\n}".into()
            ))
        );
        assert_eq!(state.highlighted(), Some(address));
        let city = state.children(address)[0];
        assert_eq!(
            state.update(Event::CopyValue(city)),
            Some(Output::CopyValue("\"London\"".into()))
        );
        assert_eq!(
            state.update(Event::CopyPath(city)),
            Some(Output::CopyPath("$.address.city".into()))
        );
        assert_eq!(state.update(Event::CopyValue(NodeId(999))), None);
        assert_eq!(state.update(Event::CopyPath(NodeId(999))), None);
    }

    #[test]
    fn expanding_and_collapsing() {
        let mut state = sample();
        let tags = id(&state, "$.tags");
        let name = id(&state, "$.name");
        let _ = state.update(Event::Toggle(tags));
        assert!(state.is_expanded(tags));
        assert_eq!(state.visible().len(), 7);
        let _ = state.update(Event::Expand(name));
        assert!(!state.is_expanded(name), "scalars never expand");
        let _ = state.update(Event::Toggle(tags));
        assert!(!state.is_expanded(tags));

        let _ = state.update(Event::ExpandAll);
        assert_eq!(state.visible().len(), 9);
        let _ = state.update(Event::CollapseAll);
        assert_eq!(state.visible().len(), 5);

        let empty = State::new(json!({ "a": {}, "b": [] }));
        let mut empty = empty;
        let _ = empty.update(Event::ExpandAll);
        assert!(empty.visible().iter().all(|&id| !empty.is_expanded(id)));
    }

    #[test]
    fn expand_to_depth_opens_only_shallow_containers() {
        let value = json!({ "a": { "b": { "c": { "d": 1 } } } });
        let state = State::new(value.clone()).with_depth(2);
        assert_eq!(state.visible().len(), 3, "a, b and c");
        let mut state = state;
        let _ = state.update(Event::ExpandToDepth(0));
        assert_eq!(state.visible().len(), 1);
        let _ = state.update(Event::ExpandToDepth(10));
        assert_eq!(state.visible().len(), 4);
    }

    #[test]
    fn subtrees_expand_and_collapse_together() {
        let mut state = State::new(json!({ "a": { "b": { "c": 1 } }, "d": { "e": 2 } }));
        let a = id(&state, "$.a");
        let _ = state.update(Event::ExpandSubtree(a));
        assert_eq!(state.visible().len(), 4);
        let c = id(&state, "$.a.b.c");
        let _ = state.update(Event::Press(c));
        let _ = state.update(Event::CollapseSubtree(a));
        assert_eq!(state.visible().len(), 2);
        assert_eq!(state.highlighted(), Some(a), "the highlight moves up");
        let _ = state.update(Event::Expand(a));
        assert!(!state.is_expanded(id(&state, "$.a.b")));
    }

    #[test]
    fn collapsing_moves_a_hidden_highlight_to_the_outermost_closed_node() {
        let mut state = State::new(json!({ "a": { "b": { "c": 1 } } })).with_depth(5);
        let c = id(&state, "$.a.b.c");
        let _ = state.update(Event::Press(c));
        let _ = state.update(Event::CollapseAll);
        assert_eq!(state.highlighted(), Some(id(&state, "$.a")));
    }

    #[test]
    fn keyboard_navigation() {
        let mut state = sample();
        let _ = state.update(Event::Focus(true));
        assert!(state.is_focused());
        assert_eq!(state.highlighted(), Some(id(&state, "$.name")));
        let _ = state.update(Event::Next);
        let tags = id(&state, "$.tags");
        assert_eq!(state.highlighted(), Some(tags));
        let _ = state.update(Event::ExpandOrEnter);
        assert!(state.is_expanded(tags));
        let _ = state.update(Event::ExpandOrEnter);
        assert_eq!(state.highlighted(), Some(id(&state, "$.tags[0]")));
        let _ = state.update(Event::CollapseOrLeave);
        assert_eq!(state.highlighted(), Some(tags), "leaves to the parent");
        let _ = state.update(Event::CollapseOrLeave);
        assert!(!state.is_expanded(tags));
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(id(&state, "$.age")));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(id(&state, "$.age")), "stays");
        let _ = state.update(Event::First);
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(id(&state, "$.name")));
        let _ = state.update(Event::CollapseOrLeave);
        assert_eq!(state.highlighted(), Some(id(&state, "$.name")), "no parent");

        let _ = state.update(Event::Press(tags));
        let _ = state.update(Event::ToggleHighlighted);
        assert!(state.is_expanded(tags));
        let _ = state.update(Event::ExpandSiblings);
        assert!(state.is_expanded(id(&state, "$.address")));
        let _ = state.update(Event::Focus(false));
        assert!(!state.is_focused());
    }

    #[test]
    fn search_finds_keys_and_values_and_expands_their_parents() {
        let mut state = sample();
        let _ = state.update(Event::Search("LONDON".into()));
        let city = id(&state, "$.address.city");
        assert_eq!(state.matches(), &[city]);
        assert!(state.is_expanded(id(&state, "$.address")));
        assert_eq!(state.highlighted(), Some(city));
        assert_eq!(state.current_match(), Some(city));
        assert_eq!(match_label(&state), "1 of 1");

        let _ = state.update(Event::Search("a".into()));
        let found: Vec<String> = state.matches().iter().map(|&id| state.path(id)).collect();
        assert_eq!(
            found,
            [
                "$.name",
                "$.tags",
                "$.tags[0]",
                "$.address",
                "$.active",
                "$.age"
            ]
        );
        let _ = state.update(Event::Search("1".into()));
        assert!(state.matches().is_empty(), "indices are not searched");
        assert_eq!(match_label(&state), "No matches");
        let _ = state.update(Event::Search("  ".into()));
        assert!(state.matches().is_empty());
        assert_eq!(match_label(&state), "");
        let _ = state.update(Event::Search("36".into()));
        assert_eq!(state.matches(), &[id(&state, "$.age")], "numbers as text");
    }

    #[test]
    fn matches_cycle_both_ways() {
        let mut state = sample();
        let _ = state.update(Event::NextMatch);
        assert_eq!(state.current_match(), None, "nothing to cycle");
        let _ = state.update(Event::Search("ad".into()));
        let matches = state.matches().to_vec();
        assert_eq!(matches.len(), 3);
        let _ = state.update(Event::NextMatch);
        assert_eq!(state.current_match(), Some(matches[1]));
        assert_eq!(state.highlighted(), Some(matches[1]));
        let _ = state.update(Event::NextMatch);
        let _ = state.update(Event::NextMatch);
        assert_eq!(state.current_match(), Some(matches[0]), "wraps");
        let _ = state.update(Event::PreviousMatch);
        assert_eq!(state.current_match(), Some(matches[2]), "wraps back");
        assert_eq!(match_label(&state), "3 of 3");
    }

    #[test]
    fn the_filter_shows_matches_their_parents_and_their_insides() {
        let mut state = State::new(json!({
            "users": [{ "name": "Ada", "role": "admin" }, { "name": "Grace" }],
            "meta": { "count": 2 }
        }))
        .with_filter(true);
        assert!(state.filter() && !state.is_filtering());
        let _ = state.update(Event::Search("grace".into()));
        assert!(state.is_filtering());
        let shown: Vec<String> = state.visible().iter().map(|&id| state.path(id)).collect();
        assert_eq!(shown, ["$.users", "$.users[1]", "$.users[1].name"]);

        let _ = state.update(Event::Search("users".into()));
        let _ = state.update(Event::Expand(id(&state, "$.users")));
        let shown: Vec<String> = state.visible().iter().map(|&id| state.path(id)).collect();
        assert_eq!(
            shown,
            ["$.users", "$.users[0]", "$.users[1]", "$.users[1].name"],
            "a match shows what is inside"
        );
        let _ = state.update(Event::Filter(false));
        assert_eq!(state.visible().len(), 5);
        let _ = state.update(Event::Search("nothing".into()));
        let _ = state.update(Event::Filter(true));
        assert!(state.visible().is_empty());
    }

    #[test]
    fn a_large_document_flattens_and_searches_quickly() {
        let users: Vec<Value> = (0..2_000)
            .map(|index| json!({ "id": index, "name": format!("user {index}"), "active": index % 2 == 0 }))
            .collect();
        let started = std::time::Instant::now();
        let mut state = State::new(json!({ "users": users }));
        assert_eq!(state.len(), 1 + 1 + 2_000 * 4);
        let _ = state.update(Event::ExpandAll);
        assert_eq!(state.visible().len(), 1 + 2_000 * 4);
        let _ = state.update(Event::Search("user 1999".into()));
        assert_eq!(state.matches().len(), 1);
        let _ = state.update(Event::Last);
        assert!(started.elapsed().as_secs() < 5, "{:?}", started.elapsed());
    }

    #[test]
    fn deeply_nested_values_are_fine() {
        let mut value = json!(1);
        for _ in 0..100 {
            value = json!([value]);
        }
        let mut state = State::new(value);
        let _ = state.update(Event::ExpandAll);
        assert_eq!(state.visible().len(), 100);
        let last = *state.visible().last().expect("a row");
        assert_eq!(state.path(last).matches('[').count(), 100);
        assert_eq!(
            state.update(Event::CopyValue(last)),
            Some(Output::CopyValue("1".into()))
        );
    }

    #[test]
    fn strings_are_shown_escaped_on_one_line() {
        let state = State::new(json!({ "note": "two\nlines \"quoted\"" }));
        let note = state.roots()[0];
        assert_eq!(
            state.item(note).map(|item| item.text.as_str()),
            Some("\"two\\nlines \\\"quoted\\\"\"")
        );
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        let press = |chord: &str| {
            let chord: Chord = chord.parse().ok()?;
            keymap.resolve(chord.key(), chord.modifiers())
        };
        assert_eq!(press("ArrowDown"), Some(Action::Next));
        assert_eq!(press("ArrowUp"), Some(Action::Previous));
        assert_eq!(press("Home"), Some(Action::First));
        assert_eq!(press("End"), Some(Action::Last));
        assert_eq!(press("ArrowRight"), Some(Action::Expand));
        assert_eq!(press("ArrowLeft"), Some(Action::Collapse));
        assert_eq!(press("Enter"), Some(Action::Toggle));
        assert_eq!(press("Space"), Some(Action::Toggle));
        assert_eq!(press("*"), Some(Action::ExpandSiblings));
        assert_eq!(press("Shift+ArrowRight"), Some(Action::ExpandAll));
        assert_eq!(press("Shift+ArrowLeft"), Some(Action::CollapseAll));
        assert_eq!(press("Mod+C"), Some(Action::CopyValue));
        assert_eq!(press("Mod+Shift+C"), Some(Action::CopyPath));
        assert_eq!(press("Mod+F"), Some(Action::Search));
        assert_eq!(press("F3"), Some(Action::NextMatch));
        assert_eq!(press("Shift+F3"), Some(Action::PreviousMatch));
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
        let mut state = sample();
        let _ = state.update(Event::First);
        assert!(
            state
                .key_event(&keymap, &key(Key::Character("*".into()), Modifiers::SHIFT))
                .is_none()
        );
        assert_eq!(
            state.key_event(
                &keymap,
                &key(Key::Character("l".into()), Modifiers::empty())
            ),
            Some(Event::ExpandOrEnter)
        );
        assert!(default_keymap().clear().is_empty());
    }

    #[test]
    fn key_events_resolve_shifted_symbols_and_copy_chords() {
        let keymap = default_keymap();
        let mut state = sample();
        let _ = state.update(Event::First);
        let name = id(&state, "$.name");
        assert_eq!(
            state.key_event(&keymap, &key(Key::Character("*".into()), Modifiers::SHIFT)),
            Some(Event::ExpandSiblings)
        );
        assert_eq!(
            state.key_event(
                &keymap,
                &key(Key::Character("c".into()), Modifiers::COMMAND)
            ),
            Some(Event::CopyValue(name))
        );
        assert_eq!(
            state.key_event(
                &keymap,
                &key(
                    Key::Character("C".into()),
                    Modifiers::COMMAND | Modifiers::SHIFT
                )
            ),
            Some(Event::CopyPath(name))
        );
        assert!(
            state
                .key_event(
                    &keymap,
                    &key(Key::Character("x".into()), Modifiers::empty())
                )
                .is_none()
        );
    }

    #[test]
    fn actions_map_to_events_only_when_they_apply() {
        let mut state = sample();
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(Action::Expand.event(&state), None, "nothing highlighted");
        assert_eq!(Action::CopyValue.event(&state), None);
        assert_eq!(Action::NextMatch.event(&state), None, "no search");
        assert_eq!(Action::Search.event(&state), Some(Event::FocusSearch));
        assert_eq!(Action::ExpandAll.event(&state), Some(Event::ExpandAll));
        assert_eq!(Action::CollapseAll.event(&state), Some(Event::CollapseAll));
        let _ = state.update(Event::First);
        assert_eq!(Action::Toggle.event(&state), None, "a scalar");
        let _ = state.update(Event::Next);
        assert_eq!(Action::Toggle.event(&state), Some(Event::ToggleHighlighted));
        assert_eq!(Action::Collapse.event(&state), Some(Event::CollapseOrLeave));
        assert_eq!(
            Action::ExpandSiblings.event(&state),
            Some(Event::ExpandSiblings)
        );
        let tags = id(&state, "$.tags");
        assert_eq!(Action::CopyPath.event(&state), Some(Event::CopyPath(tags)));
        let _ = state.update(Event::Search("o".into()));
        assert_eq!(Action::NextMatch.event(&state), Some(Event::NextMatch));
        assert_eq!(
            Action::PreviousMatch.event(&state),
            Some(Event::PreviousMatch)
        );
        assert_eq!(state.update(Event::FocusSearch), Some(Output::FocusSearch));
    }

    #[test]
    fn menu_items_map_to_events() {
        let node = NodeId(3);
        assert_eq!(MenuItem::ALL.len(), 4);
        assert_eq!(MenuItem::CopyValue.event(node), Event::CopyValue(node));
        assert_eq!(MenuItem::CopyPath.event(node), Event::CopyPath(node));
        assert_eq!(MenuItem::ExpandAll.event(node), Event::ExpandSubtree(node));
        assert_eq!(
            MenuItem::CollapseAll.event(node),
            Event::CollapseSubtree(node)
        );
        assert!(!menu().is_open());
    }

    #[test]
    fn syntax_colours_come_from_tokens_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let syntax = syntax(&tokens);
            assert_eq!(syntax.key, tokens.foreground);
            assert_eq!(syntax.punctuation, tokens.muted_foreground);
            assert_eq!(syntax.null, tokens.muted_foreground);
            let colours = [syntax.string, syntax.number, syntax.boolean];
            for (index, colour) in colours.iter().enumerate() {
                assert_ne!(*colour, tokens.foreground);
                assert!(colours.iter().skip(index + 1).all(|other| other != colour));
            }
            if tokens.is_dark {
                assert_eq!(syntax.string, tokens.success);
            } else {
                assert_eq!(syntax.string, mix(tokens.success, tokens.foreground, 0.25));
            }
            assert_eq!(value_colour(&tokens, Kind::Number), syntax.number);
            assert_eq!(value_colour(&tokens, Kind::Bool), syntax.boolean);
            assert_eq!(value_colour(&tokens, Kind::Null), syntax.null);
            assert_eq!(value_colour(&tokens, Kind::Object), syntax.punctuation);
            assert_eq!(value_colour(&tokens, Kind::Array), syntax.punctuation);
            assert_eq!(value_colour(&tokens, Kind::String), syntax.string);
        }
    }

    #[test]
    fn rows_in_every_status_and_theme() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = row_style(&tokens, Status::default());
            assert_eq!(
                idle,
                RowStyle {
                    background: None,
                    ring: None
                }
            );

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

            let matched = row_style(
                &tokens,
                Status {
                    matched: true,
                    ..Status::default()
                },
            );
            let current = row_style(
                &tokens,
                Status {
                    matched: true,
                    current: true,
                    ..Status::default()
                },
            );
            let alpha = |style: RowStyle| style.background.map_or(0.0, |colour| colour.a);
            assert!(alpha(current) > alpha(matched) && alpha(matched) > 0.0);

            let highlighted = row_style(
                &tokens,
                Status {
                    highlighted: true,
                    matched: true,
                    ..Status::default()
                },
            );
            assert_eq!(highlighted.background, Some(tokens.accent));
            assert_eq!(highlighted.ring, None, "the ring needs focus");
            let focused = row_style(
                &tokens,
                Status {
                    highlighted: true,
                    focused: true,
                    ..Status::default()
                },
            );
            assert_eq!(focused.ring, Some(tokens.ring));

            let disabled = row_style(
                &tokens,
                Status {
                    highlighted: true,
                    focused: true,
                    hovered: true,
                    matched: true,
                    current: true,
                    disabled: true,
                },
            );
            assert_eq!(
                disabled,
                RowStyle {
                    background: None,
                    ring: None
                }
            );
        }
    }

    #[test]
    fn builder_defaults_and_options() {
        let state = sample();
        let view: JsonViewer<'_, ()> = json_viewer(&state);
        assert!(!view.search && !view.is_enabled());
        assert_eq!((view.width, view.height), (Length::Fill, Length::Shrink));
        assert_eq!(view.key_width, KEY_WIDTH);
        assert_eq!(view.keymap, default_keymap());
        let view = view
            .search(true)
            .placeholder("Find")
            .search_id("other")
            .key_width(120.0)
            .width(300)
            .height(200)
            .keymap(Keymap::new())
            .context_menu_keymap(Keymap::new())
            .on_event(|_| ());
        assert!(view.search && view.is_enabled());
        assert_eq!(view.placeholder, "Find");
        assert_eq!(view.key_width, 120.0);
        assert_eq!(view.width, Length::Fixed(300.0));
        assert!(view.keymap.is_empty() && view.menu_keymap.is_empty());
        assert!(format!("{view:?}").starts_with("JsonViewer"));
    }
}
