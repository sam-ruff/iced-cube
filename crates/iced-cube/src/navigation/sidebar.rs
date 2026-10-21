//! Persistent app navigation down one side of the window.
//!
//! [`State`] owns the groups and items, which item is active, the keyboard
//! highlight, which groups and parent items are open, and whether the
//! sidebar is collapsed to an icon rail. Its [`update`](State::update)
//! handles clicks and keyboard navigation and returns an [`Output`] when an
//! item is activated.
//!
//! [`sidebar`] draws the panel, and with [`content`](Sidebar::content)
//! the page beside it. On a window narrower than the
//! [breakpoint](Sidebar::breakpoint) the panel becomes an off-canvas
//! drawer instead: a trigger button opens it over a scrim, and a tap on
//! the scrim, Escape or choosing an item closes it.
//!
//! # Keyboard
//!
//! A press inside the sidebar gives it keyboard focus, and an open drawer
//! always has it. While focused the sidebar resolves its own [`Keymap`] of
//! [`Action`]s and captures the keys it uses; see [`default_keymap`]. Only
//! [`Action::ToggleRail`] is meant to work app-wide: route key presses from
//! [`keys::subscription`] through [`State::shortcut`] for that.

use std::rc::Rc;

use iced::Event as Input;
use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::{self, key::Named};
use iced::widget::{
    Space, button, column, container, mouse_area, opaque, row, scrollable, stack, text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Rectangle, Renderer, Shadow,
    Size, Theme, Vector, mouse, touch,
};

use crate::icon::{Glyph, themed};
use crate::keys::{self, Chord, Keymap};
use crate::overlay::menu::{self, Parts, RowStatus};
use crate::overlay::tooltip::{Position, tooltip};
use crate::primitives::icon_button::{IconButton, icon_button};
use crate::theme::{Tokens, fade, mix, radius, space, text_size};

/// Width of the expanded sidebar.
pub const WIDTH: f32 = 256.0;
/// Width of the icon rail.
pub const RAIL_WIDTH: f32 = 48.0;
/// Width of the drawer on narrow windows.
pub const DRAWER_WIDTH: f32 = 288.0;
/// Below this window width the sidebar becomes a drawer.
pub const BREAKPOINT: f32 = 640.0;
/// Height of the bar holding the trigger on narrow windows.
pub const BAR_HEIGHT: f32 = 48.0;

/// Side of the square buttons in the rail.
const RAIL_BUTTON: f32 = 32.0;

/// The widget id of the trigger in the narrow window bar, for tests.
pub const TRIGGER_ID: iced::widget::Id = iced::widget::Id::new("iced-cube-sidebar-trigger");

/// One navigation item. Items nest one level: children of children are
/// ignored.
#[derive(Debug, Clone, PartialEq)]
pub struct Item<Id> {
    pub id: Id,
    pub label: String,
    pub icon: Option<Glyph>,
    /// A short count or status on the right, such as `12`.
    pub badge: Option<String>,
    pub disabled: bool,
    pub children: Vec<Item<Id>>,
}

/// Creates an enabled item.
pub fn item<Id>(id: Id, label: impl Into<String>) -> Item<Id> {
    Item {
        id,
        label: label.into(),
        icon: None,
        badge: None,
        disabled: false,
        children: Vec::new(),
    }
}

impl<Id> Item<Id> {
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Nested items, shown below this one while it is expanded.
    pub fn children(mut self, children: impl IntoIterator<Item = Item<Id>>) -> Self {
        self.children = children
            .into_iter()
            .map(|mut child| {
                child.children.clear();
                child
            })
            .collect();
        self
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

/// A labelled group of items.
#[derive(Debug, Clone, PartialEq)]
pub struct Group<Id> {
    /// The heading. An empty label draws no heading.
    pub label: String,
    pub items: Vec<Item<Id>>,
    /// Whether the heading opens and closes the group.
    pub collapsible: bool,
}

/// Creates a group that is always open.
pub fn group<Id>(label: impl Into<String>, items: impl IntoIterator<Item = Item<Id>>) -> Group<Id> {
    Group {
        label: label.into(),
        items: items.into_iter().collect(),
        collapsible: false,
    }
}

impl<Id> Group<Id> {
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }
}

/// A row the keyboard can highlight: a collapsible group's heading or an
/// item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Row<Id> {
    Group(usize),
    Item(Id),
}

/// Changes to a sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event<Id> {
    /// Chooses an item, as a click does. A parent item opens or closes
    /// its children instead.
    Activate(Id),
    /// Highlights the next row, wrapping.
    Next,
    /// Highlights the previous row, wrapping.
    Previous,
    First,
    Last,
    /// Activates the highlighted item or opens and closes the highlighted
    /// group.
    ActivateHighlighted,
    /// Opens the highlighted group or parent, or moves into it when open.
    Expand,
    /// Closes the highlighted group or parent, or moves out to it.
    Collapse,
    ToggleGroup(usize),
    /// Opens or closes a parent item's children.
    ToggleItem(Id),
    ToggleRail,
    SetRail(bool),
    OpenDrawer,
    CloseDrawer,
    ToggleDrawer,
    /// What the trigger does: toggles the drawer on a narrow window and
    /// the rail otherwise.
    Toggle,
    /// Whether the window is narrower than the breakpoint. The sidebar
    /// sends this itself when the width crosses the breakpoint.
    Narrow(bool),
    /// The sidebar gained or lost keyboard focus.
    Focus(bool),
}

/// What an event did that the app should act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output<Id> {
    /// An item was activated: show its page.
    Activated(Id),
}

/// The navigation, the active item and how the sidebar is shown.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    groups: Vec<Group<Id>>,
    active: Option<Id>,
    highlighted: Option<Row<Id>>,
    closed: Vec<bool>,
    expanded: Vec<Id>,
    rail: bool,
    drawer: bool,
    narrow: bool,
    focused: bool,
    /// Whether the keyboard moved the highlight last, so it is drawn.
    keyboard: bool,
}

impl<Id: Copy + PartialEq> State<Id> {
    /// An expanded sidebar with every group open and nothing active.
    pub fn new(groups: impl IntoIterator<Item = Group<Id>>) -> Self {
        let groups: Vec<Group<Id>> = groups.into_iter().collect();
        Self {
            closed: vec![false; groups.len()],
            groups,
            active: None,
            highlighted: None,
            expanded: Vec::new(),
            rail: false,
            drawer: false,
            narrow: false,
            focused: false,
            keyboard: false,
        }
    }

    /// Starts with `id` active, opening its parent when it is a child.
    pub fn with_active(mut self, id: Id) -> Self {
        if self.item(id).is_some_and(|item| !item.disabled) {
            self.active = Some(id);
            if let Some(parent) = self.parent_of(id) {
                self.open_item(parent);
            }
        }
        self
    }

    /// Starts collapsed to the icon rail.
    pub fn with_rail(mut self, rail: bool) -> Self {
        self.rail = rail;
        self
    }

    pub fn groups(&self) -> &[Group<Id>] {
        &self.groups
    }

    pub fn active(&self) -> Option<Id> {
        self.active
    }

    pub fn highlighted(&self) -> Option<Row<Id>> {
        self.highlighted
    }

    /// Whether the sidebar is set to collapse to a rail on wide windows.
    pub fn is_rail(&self) -> bool {
        self.rail
    }

    /// Whether the sidebar is drawn as a rail right now: collapsed on a
    /// wide window. The drawer always shows labels.
    pub fn is_compact(&self) -> bool {
        self.rail && !self.narrow
    }

    pub fn is_narrow(&self) -> bool {
        self.narrow
    }

    pub fn is_drawer_open(&self) -> bool {
        self.narrow && self.drawer
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Whether the highlighted row gets a ring: while the sidebar has keys
    /// and the keyboard moved the highlight last.
    pub fn shows_highlight(&self) -> bool {
        self.keyboard && self.has_keys()
    }

    /// Whether the sidebar resolves its keymap: while focused or while
    /// the drawer is open.
    pub fn has_keys(&self) -> bool {
        self.focused || self.is_drawer_open()
    }

    pub fn is_group_open(&self, group: usize) -> bool {
        !self.closed.get(group).copied().unwrap_or(true)
    }

    pub fn is_expanded(&self, id: Id) -> bool {
        self.expanded.contains(&id)
    }

    pub fn item(&self, id: Id) -> Option<&Item<Id>> {
        self.groups
            .iter()
            .flat_map(|group| &group.items)
            .find_map(|item| {
                if item.id == id {
                    return Some(item);
                }
                item.children.iter().find(|child| child.id == id)
            })
    }

    /// The parent of a child item.
    pub fn parent_of(&self, id: Id) -> Option<Id> {
        self.groups
            .iter()
            .flat_map(|group| &group.items)
            .find(|item| item.children.iter().any(|child| child.id == id))
            .map(|item| item.id)
    }

    fn group_of(&self, id: Id) -> Option<usize> {
        self.groups.iter().position(|group| {
            group
                .items
                .iter()
                .any(|item| item.id == id || item.children.iter().any(|child| child.id == id))
        })
    }

    /// Whether an item or one of its children is the active one, so a
    /// closed parent still shows where the page is.
    pub fn is_active_branch(&self, item: &Item<Id>) -> bool {
        Some(item.id) == self.active
            || self
                .active
                .is_some_and(|active| item.children.iter().any(|child| child.id == active))
    }

    /// The rows the keyboard moves through, in order: headings of
    /// collapsible groups and the enabled items that are showing.
    pub fn rows(&self) -> Vec<Row<Id>> {
        let compact = self.is_compact();
        let mut rows = Vec::new();
        for (index, group) in self.groups.iter().enumerate() {
            if group.collapsible && !compact {
                rows.push(Row::Group(index));
            }
            if !compact && !self.is_group_open(index) {
                continue;
            }
            for item in group.items.iter().filter(|item| !item.disabled) {
                rows.push(Row::Item(item.id));
                if compact || !self.is_expanded(item.id) {
                    continue;
                }
                rows.extend(
                    item.children
                        .iter()
                        .filter(|child| !child.disabled)
                        .map(|child| Row::Item(child.id)),
                );
            }
        }
        rows
    }

    /// Applies an event and returns the activated item, if any.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Next
            | Event::Previous
            | Event::First
            | Event::Last
            | Event::ActivateHighlighted
            | Event::Expand
            | Event::Collapse => self.keyboard = true,
            Event::Activate(_)
            | Event::ToggleGroup(_)
            | Event::ToggleItem(_)
            | Event::Focus(_)
            | Event::OpenDrawer
            | Event::ToggleDrawer
            | Event::Toggle => self.keyboard = false,
            _ => {}
        }
        match event {
            Event::Activate(id) => return self.activate(id),
            Event::Next => self.step(true),
            Event::Previous => self.step(false),
            Event::First => self.highlighted = self.rows().first().copied(),
            Event::Last => self.highlighted = self.rows().last().copied(),
            Event::ActivateHighlighted => match self.highlighted {
                Some(Row::Group(index)) => self.toggle_group(index),
                Some(Row::Item(id)) => return self.activate(id),
                None => self.step(true),
            },
            Event::Expand => self.expand(),
            Event::Collapse => self.collapse(),
            Event::ToggleGroup(index) => self.toggle_group(index),
            Event::ToggleItem(id) => self.toggle_item(id),
            Event::ToggleRail => self.set_rail(!self.rail),
            Event::SetRail(rail) => self.set_rail(rail),
            Event::OpenDrawer => self.open_drawer(),
            Event::CloseDrawer => self.drawer = false,
            Event::ToggleDrawer if self.drawer => self.drawer = false,
            Event::ToggleDrawer => self.open_drawer(),
            Event::Toggle if self.narrow => {
                return self.update(Event::ToggleDrawer);
            }
            Event::Toggle => self.set_rail(!self.rail),
            Event::Narrow(narrow) => {
                self.narrow = narrow;
                if !narrow {
                    self.drawer = false;
                }
                self.settle_highlight();
            }
            Event::Focus(focused) => {
                self.focused = focused;
                if focused && self.highlighted.is_none() {
                    self.highlight_active();
                }
            }
        }
        None
    }

    /// Turns a key press from [`keys::subscription`] into an event for the
    /// app-wide action, [`Action::ToggleRail`]. The sidebar resolves every
    /// other action itself while it has focus.
    pub fn shortcut(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event<Id>> {
        keymap
            .resolve_event(key)
            .filter(|action| *action == Action::ToggleRail)
            .and_then(|action| action.event(self))
    }

    fn activate(&mut self, id: Id) -> Option<Output<Id>> {
        let item = self.item(id).filter(|item| !item.disabled)?;
        if item.has_children() {
            if self.is_compact() {
                self.rail = false;
                self.open_item(id);
            } else {
                self.toggle_item(id);
            }
            self.highlighted = Some(Row::Item(id));
            return None;
        }
        self.active = Some(id);
        self.highlighted = Some(Row::Item(id));
        self.drawer = false;
        Some(Output::Activated(id))
    }

    fn step(&mut self, forward: bool) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let current = self
            .highlighted
            .and_then(|row| rows.iter().position(|candidate| *candidate == row));
        let len = rows.len();
        let next = match (current, forward) {
            (None, true) => 0,
            (None, false) => len - 1,
            (Some(index), true) => (index + 1) % len,
            (Some(index), false) => (index + len - 1) % len,
        };
        self.highlighted = rows.get(next).copied();
    }

    fn expand(&mut self) {
        match self.highlighted {
            Some(Row::Group(index)) if !self.is_group_open(index) => self.toggle_group(index),
            Some(Row::Group(index)) => {
                self.highlighted = self
                    .groups
                    .get(index)
                    .and_then(|group| group.items.iter().find(|item| !item.disabled))
                    .map(|item| Row::Item(item.id));
            }
            Some(Row::Item(id)) => {
                let Some(item) = self.item(id).filter(|item| item.has_children()) else {
                    return;
                };
                if self.is_compact() {
                    return;
                }
                if !self.is_expanded(id) {
                    self.open_item(id);
                    return;
                }
                if let Some(child) = item.children.iter().find(|child| !child.disabled) {
                    self.highlighted = Some(Row::Item(child.id));
                }
            }
            None => {}
        }
    }

    fn collapse(&mut self) {
        match self.highlighted {
            Some(Row::Group(index)) if self.is_group_open(index) => self.toggle_group(index),
            Some(Row::Group(_)) | None => {}
            Some(Row::Item(id)) => {
                if self.is_expanded(id) {
                    self.toggle_item(id);
                    return;
                }
                if let Some(parent) = self.parent_of(id) {
                    self.highlighted = Some(Row::Item(parent));
                    return;
                }
                let group = self
                    .group_of(id)
                    .filter(|&index| self.groups[index].collapsible && !self.is_compact());
                if let Some(index) = group {
                    self.highlighted = Some(Row::Group(index));
                }
            }
        }
    }

    fn toggle_group(&mut self, index: usize) {
        let Some(closed) = self.closed.get_mut(index) else {
            return;
        };
        if !self.groups[index].collapsible {
            return;
        }
        *closed = !*closed;
        if *closed && self.highlighted.is_some() {
            let inside = match self.highlighted {
                Some(Row::Item(id)) => self.group_of(id) == Some(index),
                _ => false,
            };
            if inside {
                self.highlighted = Some(Row::Group(index));
            }
        }
    }

    fn open_item(&mut self, id: Id) {
        if !self.expanded.contains(&id) {
            self.expanded.push(id);
        }
    }

    fn toggle_item(&mut self, id: Id) {
        if !self.item(id).is_some_and(Item::has_children) {
            return;
        }
        if !self.expanded.contains(&id) {
            self.open_item(id);
            return;
        }
        self.expanded.retain(|expanded| *expanded != id);
        if let Some(Row::Item(highlighted)) = self.highlighted
            && self.parent_of(highlighted) == Some(id)
        {
            self.highlighted = Some(Row::Item(id));
        }
    }

    fn set_rail(&mut self, rail: bool) {
        self.rail = rail;
        self.settle_highlight();
    }

    fn open_drawer(&mut self) {
        self.drawer = true;
        self.highlight_active();
    }

    /// Highlights the active item, or the first row, if it is showing.
    fn highlight_active(&mut self) {
        let rows = self.rows();
        let active = self
            .active
            .map(Row::Item)
            .filter(|row| rows.contains(row))
            .or_else(|| {
                self.active
                    .and_then(|id| self.parent_of(id))
                    .map(Row::Item)
                    .filter(|row| rows.contains(row))
            });
        self.highlighted = active.or_else(|| rows.first().copied());
    }

    /// Moves a highlight that is no longer showing to its parent or group,
    /// or drops it.
    fn settle_highlight(&mut self) {
        let Some(row) = self.highlighted else {
            return;
        };
        let rows = self.rows();
        if rows.contains(&row) {
            return;
        }
        let fallback = match row {
            Row::Item(id) => self
                .parent_of(id)
                .map(Row::Item)
                .filter(|row| rows.contains(row)),
            Row::Group(index) => self
                .groups
                .get(index)
                .and_then(|group| group.items.iter().find(|item| !item.disabled))
                .map(|item| Row::Item(item.id)),
        };
        self.highlighted = fallback;
    }
}

/// What a sidebar keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    First,
    Last,
    Activate,
    Expand,
    Collapse,
    /// Collapses to the rail and back, or opens and closes the drawer on
    /// a narrow window.
    ToggleRail,
    /// Closes the drawer, or gives up keyboard focus.
    Close,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when
    /// [`Action::Close`] has nothing to close.
    pub fn event<Id: Copy + PartialEq>(self, state: &State<Id>) -> Option<Event<Id>> {
        let event = match self {
            Action::Next => Event::Next,
            Action::Previous => Event::Previous,
            Action::First => Event::First,
            Action::Last => Event::Last,
            Action::Activate => Event::ActivateHighlighted,
            Action::Expand => Event::Expand,
            Action::Collapse => Event::Collapse,
            Action::ToggleRail => Event::Toggle,
            Action::Close if state.is_drawer_open() => Event::CloseDrawer,
            Action::Close if state.is_focused() => Event::Focus(false),
            Action::Close => return None,
        };
        Some(event)
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Next,
        Action::Previous,
        Action::First,
        Action::Last,
        Action::Activate,
        Action::Expand,
        Action::Collapse,
        Action::ToggleRail,
        Action::Close,
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
            Action::Activate => "Activate",
            Action::Expand => "Expand",
            Action::Collapse => "Collapse",
            Action::ToggleRail => "ToggleRail",
            Action::Close => "Close",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Highlights the next row, wrapping.",
            Action::Previous => "Highlights the previous row, wrapping.",
            Action::First => "Highlights the first row.",
            Action::Last => "Highlights the last row.",
            Action::Activate => "Activates the highlighted item or opens and closes its group.",
            Action::Expand => "Opens the highlighted group or parent item, or moves into it.",
            Action::Collapse => "Closes the highlighted group or parent item, or moves out to it.",
            Action::ToggleRail => {
                "Collapses the sidebar to an icon rail and back, or toggles the drawer on a narrow window."
            }
            Action::Close => "Closes the drawer, or leaves the sidebar.",
        }
    }
}

/// The default sidebar shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowDown` | [`Action::Next`] |
/// | `ArrowUp` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
/// | `Enter`, `Space` | [`Action::Activate`] |
/// | `ArrowRight` | [`Action::Expand`] |
/// | `ArrowLeft` | [`Action::Collapse`] |
/// | `Ctrl+B` (`Cmd+B` on macOS) | [`Action::ToggleRail`] |
/// | `Escape` | [`Action::Close`] |
///
/// The sidebar resolves these itself while it has focus or its drawer is
/// open. Route [`keys::subscription`] through [`State::shortcut`] so
/// `Ctrl+B` works everywhere else too.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowDown), Action::Next)
        .bind(Chord::named(Named::ArrowUp), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
        .bind(Chord::named(Named::Enter), Action::Activate)
        .bind(Chord::named(Named::Space), Action::Activate)
        .bind(Chord::named(Named::ArrowRight), Action::Expand)
        .bind(Chord::named(Named::ArrowLeft), Action::Collapse)
        .bind(Chord::character('b').command(), Action::ToggleRail)
        .bind(Chord::named(Named::Escape), Action::Close)
}

/// The sidebar surface: a shade off the page in light themes, and lifted
/// like a card in dark ones.
pub fn surface_style(tokens: &Tokens) -> container::Style {
    let background = if tokens.is_dark {
        mix(tokens.background, tokens.foreground, 0.04)
    } else {
        mix(tokens.background, tokens.muted, 0.5)
    };
    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(tokens.foreground),
        ..container::Style::default()
    }
}

/// The one pixel edge between the sidebar and the page.
pub fn edge_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.border)),
        ..container::Style::default()
    }
}

/// The scrim behind the open drawer.
pub fn scrim_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(fade(
            Color::BLACK,
            if tokens.is_dark { 0.7 } else { 0.5 },
        ))),
        ..container::Style::default()
    }
}

/// How an item row is drawn: in the shared menu row colours, highlighted
/// while active or under the pointer, with a ring while it has the
/// keyboard highlight.
pub fn item_style(
    tokens: &Tokens,
    active: bool,
    keyboard: bool,
    status: button::Status,
) -> button::Style {
    let row = match status {
        button::Status::Disabled => RowStatus::Disabled,
        _ if active => RowStatus::Highlighted,
        button::Status::Hovered | button::Status::Pressed => RowStatus::Highlighted,
        button::Status::Active => RowStatus::Idle,
    };
    let colours = menu::row_style(tokens, row, false);
    button::Style {
        background: colours.background.map(Background::Color),
        text_color: colours.text,
        border: Border {
            color: if keyboard {
                tokens.ring
            } else {
                Color::TRANSPARENT
            },
            width: if keyboard { 1.0 } else { 0.0 },
            radius: radius::SM.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// A sidebar builder. Convert it into an [`Element`] to render.
pub struct Sidebar<'a, Id, Message> {
    state: &'a State<Id>,
    header: Option<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
    content: Option<Element<'a, Message>>,
    width: Option<Length>,
    breakpoint: f32,
    trigger: bool,
    keymap: Keymap<Action>,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for Sidebar<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sidebar")
            .field("state", self.state)
            .field("width", &self.width)
            .field("breakpoint", &self.breakpoint)
            .field("trigger", &self.trigger)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

/// Renders the navigation of `state`. Without
/// [`on_event`](Sidebar::on_event) every item renders disabled.
pub fn sidebar<Id, Message>(state: &State<Id>) -> Sidebar<'_, Id, Message> {
    Sidebar {
        state,
        header: None,
        footer: None,
        content: None,
        width: None,
        breakpoint: BREAKPOINT,
        trigger: true,
        keymap: default_keymap(),
        on_event: None,
    }
}

impl<'a, Id, Message> Sidebar<'a, Id, Message> {
    /// Content above the groups, such as the app name and a switcher
    /// built with [`menu_button`].
    pub fn header(mut self, header: impl Into<Element<'a, Message>>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Content below the groups, such as the signed in user.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// The page beside the sidebar. With content, the sidebar turns into a
    /// drawer on narrow windows; without it, only the panel is drawn.
    pub fn content(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// The expanded width. Defaults to [`WIDTH`]; use [`Length::Fill`]
    /// inside a resizable panel.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// The window width below which the sidebar becomes a drawer.
    pub fn breakpoint(mut self, width: f32) -> Self {
        self.breakpoint = width;
        self
    }

    /// Whether narrow windows get a bar with the trigger above the
    /// content. Turn it off to place [`trigger`] in your own header.
    pub fn trigger(mut self, trigger: bool) -> Self {
        self.trigger = trigger;
        self
    }

    /// Replaces the [`default_keymap`] the focused sidebar resolves keys
    /// with.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

type OnEvent<'a, Id, Message> = Rc<dyn Fn(Event<Id>) -> Message + 'a>;

/// How the panel is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Docked,
    Rail,
    Drawer,
}

impl<'a, Id, Message> From<Sidebar<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(sidebar: Sidebar<'a, Id, Message>) -> Self {
        let Sidebar {
            state,
            header,
            footer,
            content,
            width,
            breakpoint,
            trigger: show_trigger,
            keymap,
            on_event,
        } = sidebar;
        let on_event: Option<OnEvent<'a, Id, Message>> = on_event.map(Rc::from);

        let Some(content) = content else {
            let mode = if state.rail { Mode::Rail } else { Mode::Docked };
            // Whatever sits beside a lone panel, such as a handle, draws
            // the edge.
            let panel = panel(state, header, footer, mode, width, false, on_event.clone());
            return watch(panel, state, Some(keymap), true, None, on_event);
        };

        if !state.narrow {
            let mode = if state.rail { Mode::Rail } else { Mode::Docked };
            let panel = panel(state, header, footer, mode, width, true, on_event.clone());
            let panel = watch(panel, state, Some(keymap), true, None, on_event.clone());
            let page = row![panel, container(content).width(Length::Fill)].height(Length::Fill);
            return watch(page.into(), state, None, false, Some(breakpoint), on_event);
        }

        let mut page = column![];
        if show_trigger {
            let bar = container(
                trigger(
                    on_event
                        .as_ref()
                        .map(|on_event| on_event(Event::ToggleDrawer)),
                )
                .tooltip(None)
                .id(TRIGGER_ID),
            )
            .padding([0.0, space::SM])
            .height(BAR_HEIGHT)
            .width(Length::Fill)
            .align_y(Alignment::Center);
            page = page.push(bar).push(edge(Length::Fill, Length::Fixed(1.0)));
        }
        page = page.push(container(content).width(Length::Fill).height(Length::Fill));

        let mut layers = stack![page].width(Length::Fill).height(Length::Fill);
        if state.is_drawer_open() {
            let scrim = container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|theme| scrim_style(&Tokens::of(theme)));
            let scrim: Element<'a, Message> = match &on_event {
                Some(on_event) => opaque(mouse_area(scrim).on_press(on_event(Event::CloseDrawer))),
                None => opaque(scrim),
            };
            let drawer = panel(
                state,
                header,
                footer,
                Mode::Drawer,
                None,
                true,
                on_event.clone(),
            );
            let drawer = watch(drawer, state, Some(keymap), false, None, on_event.clone());
            layers = layers.push(scrim).push(opaque(drawer));
        }
        watch(
            layers.into(),
            state,
            None,
            false,
            Some(breakpoint),
            on_event,
        )
    }
}

/// The icon button that opens the drawer or collapses the sidebar. Send
/// [`Event::Toggle`] from it to do whichever fits the window.
pub fn trigger<'a, Message>(on_press: Option<Message>) -> IconButton<'a, Message> {
    icon_button(crate::lucide!(PanelLeft))
        .label("Toggle sidebar")
        .on_press_maybe(on_press)
}

fn edge<'a, Message: 'a>(width: Length, height: Length) -> Element<'a, Message> {
    container(Space::new())
        .width(width)
        .height(height)
        .style(|theme| edge_style(&Tokens::of(theme)))
        .into()
}

fn panel<'a, Id, Message>(
    state: &'a State<Id>,
    header: Option<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
    mode: Mode,
    width: Option<Length>,
    with_edge: bool,
    on_event: Option<OnEvent<'a, Id, Message>>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let on_event = on_event.as_deref();
    let keyboard = state.shows_highlight();
    let mut body = column![].spacing(space::SM).width(Length::Fill);
    for (index, group) in state.groups.iter().enumerate() {
        body = body.push(group_block(state, index, group, mode, keyboard, on_event));
    }

    let mut sections = column![].width(Length::Fill).height(Length::Fill);
    if let Some(header) = header {
        sections = sections.push(container(header).padding(space::SM).width(Length::Fill));
    }
    sections = sections.push(
        scrollable(body)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::hidden(),
            ))
            .height(Length::Fill),
    );
    if let Some(footer) = footer {
        sections = sections.push(container(footer).padding(space::SM).width(Length::Fill));
    }

    let width = match mode {
        Mode::Rail => Length::Fixed(RAIL_WIDTH),
        Mode::Docked => width.unwrap_or(Length::Fixed(WIDTH)),
        Mode::Drawer => Length::Fixed(DRAWER_WIDTH),
    };
    let surface = container(sections)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme| surface_style(&Tokens::of(theme)));
    let mut panel = row![surface].width(width).height(Length::Fill);
    if with_edge {
        panel = panel.push(edge(Length::Fixed(1.0), Length::Fill));
    }
    if mode != Mode::Drawer {
        return panel.into();
    }
    container(panel)
        .max_width(DRAWER_WIDTH)
        .height(Length::Fill)
        .style(|theme| drawer_style(&Tokens::of(theme)))
        .into()
}

/// The shadow the drawer casts over the page.
pub fn drawer_style(tokens: &Tokens) -> container::Style {
    container::Style {
        shadow: Shadow {
            color: fade(Color::BLACK, if tokens.is_dark { 0.5 } else { 0.15 }),
            offset: Vector::new(4.0, 0.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

fn group_block<'a, Id, Message>(
    state: &'a State<Id>,
    index: usize,
    group: &'a Group<Id>,
    mode: Mode,
    keyboard: bool,
    on_event: Option<&(dyn Fn(Event<Id>) -> Message + 'a)>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let mut block = column![].width(Length::Fill);
    if mode == Mode::Rail {
        for item in &group.items {
            block = block.push(rail_item(state, item, on_event));
        }
        return container(block.spacing(space::XS))
            .padding(space::SM)
            .width(Length::Fill)
            .into();
    }

    if !group.label.is_empty() {
        let highlighted = keyboard && state.highlighted == Some(Row::Group(index));
        block = block.push(group_heading(state, index, group, highlighted, on_event));
    }
    if state.is_group_open(index) || !group.collapsible {
        for item in &group.items {
            block = block.push(item_rows(state, item, keyboard, on_event));
        }
    }
    container(block.spacing(2.0))
        .padding([space::XS, space::XS])
        .width(Length::Fill)
        .into()
}

fn group_heading<'a, Id, Message>(
    state: &State<Id>,
    index: usize,
    group: &'a Group<Id>,
    highlighted: bool,
    on_event: Option<&(dyn Fn(Event<Id>) -> Message + 'a)>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let label = text(group.label.as_str())
        .size(text_size::XS)
        .font(crate::theme::semibold())
        .wrapping(text::Wrapping::None)
        .width(Length::Fill)
        .style(|theme: &Theme| menu::label_style(&Tokens::of(theme)));
    if !group.collapsible {
        return menu::inset(
            container(label)
                .padding(menu::ROW_PADDING)
                .height(menu::ROW_PADDING.top + menu::LINE_HEIGHT + menu::ROW_PADDING.bottom)
                .align_y(Alignment::Center),
        );
    }
    let chevron = if state.is_group_open(index) {
        crate::lucide!(ChevronDown)
    } else {
        crate::lucide!(ChevronRight)
    };
    let heading = row![
        label,
        themed(chevron, menu::ICON_SIZE, 1.0, |theme| {
            Tokens::of(theme).muted_foreground
        })
    ]
    .align_y(Alignment::Center);
    menu::inset(
        button(heading)
            .padding(menu::ROW_PADDING)
            .width(Length::Fill)
            .on_press_maybe(on_event.map(|on_event| on_event(Event::ToggleGroup(index))))
            .style(move |theme, status| item_style(&Tokens::of(theme), false, highlighted, status)),
    )
}

fn item_rows<'a, Id, Message>(
    state: &'a State<Id>,
    item: &'a Item<Id>,
    keyboard: bool,
    on_event: Option<&(dyn Fn(Event<Id>) -> Message + 'a)>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let expanded = state.is_expanded(item.id);
    let active = if item.has_children() {
        !expanded && state.is_active_branch(item)
    } else {
        state.active == Some(item.id)
    };
    let row_element = item_row(state, item, active, keyboard, on_event);
    if !item.has_children() || !expanded {
        return row_element;
    }

    let children = column(item.children.iter().map(|child| {
        let active = state.active == Some(child.id);
        item_row(state, child, active, keyboard, on_event)
    }))
    .spacing(2.0)
    .width(Length::Fill);
    let nested = row![
        edge(Length::Fixed(1.0), Length::Fill),
        container(children).padding(Padding::ZERO.left(space::SM)),
    ]
    .height(Length::Shrink);
    column![
        row_element,
        container(nested).padding(Padding {
            top: 2.0,
            bottom: 0.0,
            left: space::LG + space::XS,
            right: 0.0,
        })
    ]
    .spacing(0.0)
    .into()
}

fn item_row<'a, Id, Message>(
    state: &State<Id>,
    item: &'a Item<Id>,
    active: bool,
    keyboard: bool,
    on_event: Option<&(dyn Fn(Event<Id>) -> Message + 'a)>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let message = on_event
        .filter(|_| !item.disabled)
        .map(|on_event| on_event(Event::Activate(item.id)));
    let status = if message.is_some() {
        RowStatus::Idle
    } else {
        RowStatus::Disabled
    };
    let highlighted = keyboard && state.highlighted == Some(Row::Item(item.id));

    let parts = Parts {
        icon: item.icon,
        hint: item.badge.as_deref(),
        ..Parts::label(&item.label)
    };
    let mut content = menu::content(parts, status, false);
    if item.has_children() {
        let chevron = if state.is_expanded(item.id) {
            crate::lucide!(ChevronDown)
        } else {
            crate::lucide!(ChevronRight)
        };
        let opacity = crate::icon::opacity(message.is_some());
        content = content.push(themed(chevron, menu::ICON_SIZE, opacity, |theme| {
            Tokens::of(theme).muted_foreground
        }));
    }

    menu::inset(
        button(content)
            .padding(menu::ROW_PADDING)
            .width(Length::Fill)
            .on_press_maybe(message)
            .style(move |theme, status| {
                item_style(&Tokens::of(theme), active, highlighted, status)
            }),
    )
}

fn rail_item<'a, Id, Message>(
    state: &State<Id>,
    item: &'a Item<Id>,
    on_event: Option<&(dyn Fn(Event<Id>) -> Message + 'a)>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let message = on_event
        .filter(|_| !item.disabled)
        .map(|on_event| on_event(Event::Activate(item.id)));
    let enabled = message.is_some();
    let active = state.is_active_branch(item);
    let highlighted = state.shows_highlight() && state.highlighted == Some(Row::Item(item.id));

    let face: Element<'a, Message> = match item.icon {
        Some(glyph) => themed(
            glyph,
            menu::ICON_SIZE,
            crate::icon::opacity(enabled),
            |theme| Tokens::of(theme).foreground,
        )
        .into(),
        None => text(
            item.label
                .chars()
                .next()
                .map(String::from)
                .unwrap_or_default(),
        )
        .size(text_size::SM)
        .font(crate::theme::semibold())
        .into(),
    };
    let square = button(container(face).center(RAIL_BUTTON))
        .width(RAIL_BUTTON)
        .height(RAIL_BUTTON)
        .padding(0)
        .on_press_maybe(message)
        .style(move |theme, status| item_style(&Tokens::of(theme), active, highlighted, status));
    tooltip(square, item.label.as_str())
        .position(Position::Right)
        .into()
}

/// A header or footer button: a logo or avatar, a title and a subtitle,
/// with an up and down chevron to show it opens a menu. Wrap it in a
/// dropdown menu for an app switcher or a user menu.
pub struct MenuButton<'a, Message> {
    title: text::Fragment<'a>,
    subtitle: Option<text::Fragment<'a>>,
    icon: Option<Glyph>,
    initials: Option<text::Fragment<'a>>,
    compact: bool,
    on_press: Option<Message>,
}

impl<Message> std::fmt::Debug for MenuButton<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuButton")
            .field("title", &self.title)
            .field("subtitle", &self.subtitle)
            .field("compact", &self.compact)
            .finish_non_exhaustive()
    }
}

/// Creates a header or footer button titled `title`.
pub fn menu_button<'a, Message>(title: impl text::IntoFragment<'a>) -> MenuButton<'a, Message> {
    MenuButton {
        title: title.into_fragment(),
        subtitle: None,
        icon: None,
        initials: None,
        compact: false,
        on_press: None,
    }
}

impl<'a, Message> MenuButton<'a, Message> {
    pub fn subtitle(mut self, subtitle: impl text::IntoFragment<'a>) -> Self {
        self.subtitle = Some(subtitle.into_fragment());
        self
    }

    /// A logo icon on a primary square, as for an app or team.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Initials on a muted square, as for a person.
    pub fn initials(mut self, initials: impl text::IntoFragment<'a>) -> Self {
        self.initials = Some(initials.into_fragment());
        self
    }

    /// Shows only the square, for the rail. Pass
    /// [`State::is_compact`].
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }
}

impl<'a, Message: Clone + 'a> From<MenuButton<'a, Message>> for Element<'a, Message> {
    fn from(menu_button: MenuButton<'a, Message>) -> Self {
        let MenuButton {
            title,
            subtitle,
            icon,
            initials,
            compact,
            on_press,
        } = menu_button;
        let logo = icon.is_some();
        let face: Element<'a, Message> = match (icon, initials) {
            (Some(glyph), _) => themed(glyph, menu::ICON_SIZE, 1.0, |theme| {
                Tokens::of(theme).primary_foreground
            })
            .into(),
            (None, Some(initials)) => text(initials)
                .size(text_size::XS)
                .font(crate::theme::semibold())
                .wrapping(text::Wrapping::None)
                .into(),
            (None, None) => themed(crate::lucide!(User), menu::ICON_SIZE, 1.0, |theme| {
                Tokens::of(theme).foreground
            })
            .into(),
        };
        let square = container(face)
            .center(RAIL_BUTTON)
            .style(move |theme| avatar_style(&Tokens::of(theme), logo));

        let content: Element<'a, Message> = if compact {
            square.into()
        } else {
            let mut lines = column![
                text(title)
                    .size(text_size::SM)
                    .font(crate::theme::semibold())
                    .wrapping(text::Wrapping::None)
            ];
            if let Some(subtitle) = subtitle {
                lines = lines.push(
                    text(subtitle)
                        .size(text_size::XS)
                        .wrapping(text::Wrapping::None)
                        .style(|theme: &Theme| text::Style {
                            color: Some(Tokens::of(theme).muted_foreground),
                        }),
                );
            }
            row![
                square,
                container(lines).width(Length::Fill).clip(true),
                themed(
                    crate::lucide!(ChevronsUpDown),
                    menu::ICON_SIZE,
                    1.0,
                    |theme| { Tokens::of(theme).muted_foreground }
                ),
            ]
            .spacing(space::SM)
            .align_y(Alignment::Center)
            .into()
        };

        let width = if compact {
            Length::Fixed(RAIL_BUTTON)
        } else {
            Length::Fill
        };
        let padding = if compact { 0.0 } else { space::SM };
        button(content)
            .padding(padding)
            .width(width)
            .on_press_maybe(on_press)
            .style(|theme, status| {
                let status = match status {
                    button::Status::Disabled => button::Status::Active,
                    status => status,
                };
                item_style(&Tokens::of(theme), false, false, status)
            })
            .into()
    }
}

/// The square behind a menu button's logo or initials.
pub fn avatar_style(tokens: &Tokens, logo: bool) -> container::Style {
    let (background, text_color) = if logo {
        (tokens.primary, tokens.primary_foreground)
    } else {
        (tokens.muted, tokens.foreground)
    };
    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(text_color),
        border: Border {
            radius: radius::MD.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Wraps part of the sidebar to watch for presses, keys and width.
fn watch<'a, Id, Message>(
    content: Element<'a, Message>,
    state: &'a State<Id>,
    keymap: Option<Keymap<Action>>,
    track_focus: bool,
    breakpoint: Option<f32>,
    on_event: Option<OnEvent<'a, Id, Message>>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: 'a,
{
    let Some(on_event) = on_event else {
        return content;
    };
    Element::new(Watch {
        content,
        state,
        keymap,
        track_focus,
        breakpoint,
        on_event,
    })
}

/// Gives the sidebar keyboard focus on a press inside it and takes it
/// away on a press outside, resolves its keymap while it has keys, and
/// reports when the width crosses the breakpoint.
struct Watch<'a, Id, Message> {
    content: Element<'a, Message>,
    state: &'a State<Id>,
    keymap: Option<Keymap<Action>>,
    track_focus: bool,
    breakpoint: Option<f32>,
    on_event: OnEvent<'a, Id, Message>,
}

fn is_press(event: &Input) -> bool {
    matches!(
        event,
        Input::Mouse(mouse::Event::ButtonPressed(_))
            | Input::Touch(touch::Event::FingerPressed { .. })
    )
}

fn press_position(event: &Input, cursor: mouse::Cursor) -> Option<iced::Point> {
    match event {
        Input::Touch(touch::Event::FingerPressed { position, .. }) => Some(*position),
        _ => cursor.position(),
    }
}

impl<Id, Message> Widget<Message, Theme, Renderer> for Watch<'_, Id, Message>
where
    Id: Copy + PartialEq,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
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
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
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
        if let Some(breakpoint) = self.breakpoint {
            let narrow = layout.bounds().width < breakpoint;
            if narrow != self.state.narrow {
                shell.publish((self.on_event)(Event::Narrow(narrow)));
            }
        }

        if let (Some(keymap), Input::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })) =
            (&self.keymap, event)
            && self.state.has_keys()
            && let Some(action) = keymap.resolve(key, *modifiers)
        {
            if let Some(event) = action.event(self.state) {
                shell.publish((self.on_event)(event));
            }
            shell.capture_event();
            return;
        }

        if self.track_focus && is_press(event) {
            let inside =
                press_position(event, cursor).is_some_and(|point| layout.bounds().contains(point));
            if inside != self.state.focused {
                shell.publish((self.on_event)(Event::Focus(inside)));
            }
        }

        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
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
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Page {
        Home,
        Inbox,
        Projects,
        Alpha,
        Beta,
        Archived,
        Settings,
        Billing,
    }

    use Page::*;

    fn state() -> State<Page> {
        State::new([
            group(
                "Platform",
                [
                    item(Home, "Home"),
                    item(Inbox, "Inbox").badge("12"),
                    item(Projects, "Projects").children([
                        item(Alpha, "Alpha"),
                        item(Beta, "Beta"),
                        item(Archived, "Archived").disabled(true),
                    ]),
                ],
            ),
            group(
                "Account",
                [item(Settings, "Settings"), item(Billing, "Billing")],
            )
            .collapsible(true),
        ])
    }

    fn rows(state: &State<Page>) -> Vec<Row<Page>> {
        state.rows()
    }

    #[test]
    fn builders_set_fields() {
        let entry = item(Home, "Home")
            .icon(crate::lucide!(House))
            .badge("3")
            .disabled(true);
        assert_eq!(entry.badge.as_deref(), Some("3"));
        assert!(entry.icon.is_some() && entry.disabled);

        let nested = item(Projects, "Projects")
            .children([item(Alpha, "Alpha").children([item(Beta, "Beta")])]);
        assert!(nested.children[0].children.is_empty(), "one level deep");

        let group = group("G", [item(Home, "Home")]).collapsible(true);
        assert!(group.collapsible);

        let state = State::new([group]);
        assert!(!state.is_rail() && !state.is_narrow() && !state.is_focused());
        assert_eq!(state.active(), None);
    }

    #[test]
    fn with_active_opens_the_parent_and_ignores_disabled() {
        let state = state().with_active(Beta);
        assert_eq!(state.active(), Some(Beta));
        assert!(state.is_expanded(Projects));
        assert_eq!(state.clone().with_active(Archived).active(), Some(Beta));
    }

    #[test]
    fn rows_follow_what_is_showing() {
        let mut state = state();
        assert_eq!(
            rows(&state),
            [
                Row::Item(Home),
                Row::Item(Inbox),
                Row::Item(Projects),
                Row::Group(1),
                Row::Item(Settings),
                Row::Item(Billing)
            ]
        );
        let _ = state.update(Event::ToggleItem(Projects));
        assert!(rows(&state).contains(&Row::Item(Alpha)));
        assert!(!rows(&state).contains(&Row::Item(Archived)), "disabled");
        let _ = state.update(Event::ToggleGroup(1));
        assert!(!rows(&state).contains(&Row::Item(Settings)));
        assert!(rows(&state).contains(&Row::Group(1)));
    }

    #[test]
    fn the_rail_shows_top_level_items_only() {
        let state = state().with_rail(true);
        assert!(state.is_compact());
        assert_eq!(
            rows(&state),
            [
                Row::Item(Home),
                Row::Item(Inbox),
                Row::Item(Projects),
                Row::Item(Settings),
                Row::Item(Billing)
            ]
        );
        let mut narrow = state.clone();
        let _ = narrow.update(Event::Narrow(true));
        assert!(
            narrow.is_rail() && !narrow.is_compact(),
            "the drawer shows labels"
        );
    }

    #[test]
    fn activating_a_leaf_makes_it_active() {
        let mut state = state();
        assert_eq!(
            state.update(Event::Activate(Inbox)),
            Some(Output::Activated(Inbox))
        );
        assert_eq!(state.active(), Some(Inbox));
        assert_eq!(state.highlighted(), Some(Row::Item(Inbox)));
        assert_eq!(state.update(Event::Activate(Archived)), None);
        assert_eq!(state.active(), Some(Inbox));
    }

    #[test]
    fn activating_a_parent_toggles_its_children() {
        let mut state = state();
        assert_eq!(state.update(Event::Activate(Projects)), None);
        assert!(state.is_expanded(Projects));
        let _ = state.update(Event::Activate(Projects));
        assert!(!state.is_expanded(Projects));
    }

    #[test]
    fn a_parent_in_the_rail_expands_the_sidebar() {
        let mut state = state().with_rail(true);
        let _ = state.update(Event::Activate(Projects));
        assert!(!state.is_rail());
        assert!(state.is_expanded(Projects));
    }

    #[test]
    fn up_and_down_wrap_through_rows() {
        let mut state = state();
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(Row::Item(Home)));
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(Row::Item(Billing)));
        let _ = state.update(Event::Next);
        assert_eq!(state.highlighted(), Some(Row::Item(Home)));
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(Row::Item(Billing)));
        let _ = state.update(Event::First);
        assert_eq!(state.highlighted(), Some(Row::Item(Home)));
    }

    #[test]
    fn enter_activates_the_highlight() {
        let mut state = state();
        let _ = state.update(Event::Next);
        let _ = state.update(Event::Next);
        assert_eq!(
            state.update(Event::ActivateHighlighted),
            Some(Output::Activated(Inbox))
        );
        let _ = state.update(Event::Last);
        let _ = state.update(Event::Previous);
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(Row::Group(1)));
        let _ = state.update(Event::ActivateHighlighted);
        assert!(!state.is_group_open(1));
    }

    #[test]
    fn right_and_left_open_close_and_move_through_parents() {
        let mut state = state();
        state.highlighted = Some(Row::Item(Projects));
        let _ = state.update(Event::Expand);
        assert!(state.is_expanded(Projects));
        let _ = state.update(Event::Expand);
        assert_eq!(state.highlighted(), Some(Row::Item(Alpha)));
        let _ = state.update(Event::Collapse);
        assert_eq!(state.highlighted(), Some(Row::Item(Projects)));
        let _ = state.update(Event::Collapse);
        assert!(!state.is_expanded(Projects));
    }

    #[test]
    fn right_and_left_open_and_close_groups() {
        let mut state = state();
        state.highlighted = Some(Row::Item(Settings));
        let _ = state.update(Event::Collapse);
        assert_eq!(state.highlighted(), Some(Row::Group(1)));
        let _ = state.update(Event::Collapse);
        assert!(!state.is_group_open(1));
        let _ = state.update(Event::Expand);
        assert!(state.is_group_open(1));
        let _ = state.update(Event::Expand);
        assert_eq!(state.highlighted(), Some(Row::Item(Settings)));
        state.highlighted = Some(Row::Item(Home));
        let _ = state.update(Event::Collapse);
        assert_eq!(state.highlighted(), Some(Row::Item(Home)), "group is fixed");
    }

    #[test]
    fn closing_moves_a_hidden_highlight_out() {
        let mut state = state().with_active(Alpha);
        state.highlighted = Some(Row::Item(Alpha));
        let _ = state.update(Event::ToggleItem(Projects));
        assert_eq!(state.highlighted(), Some(Row::Item(Projects)));

        state.highlighted = Some(Row::Item(Billing));
        let _ = state.update(Event::ToggleGroup(1));
        assert_eq!(state.highlighted(), Some(Row::Group(1)));
        let _ = state.update(Event::ToggleGroup(0));
        assert!(state.is_group_open(0), "fixed groups stay open");
    }

    #[test]
    fn toggling_the_rail_moves_the_highlight_to_a_showing_row() {
        let mut state = state().with_active(Alpha);
        state.highlighted = Some(Row::Item(Alpha));
        let _ = state.update(Event::ToggleRail);
        assert!(state.is_rail());
        assert_eq!(state.highlighted(), Some(Row::Item(Projects)));
        state.highlighted = Some(Row::Group(1));
        let _ = state.update(Event::SetRail(true));
        assert_eq!(state.highlighted(), Some(Row::Item(Settings)));
    }

    #[test]
    fn the_trigger_toggles_the_rail_or_the_drawer() {
        let mut state = state().with_active(Inbox);
        let _ = state.update(Event::Toggle);
        assert!(state.is_rail());
        let _ = state.update(Event::Narrow(true));
        let _ = state.update(Event::Toggle);
        assert!(state.is_drawer_open());
        assert_eq!(state.highlighted(), Some(Row::Item(Inbox)));
        let _ = state.update(Event::Toggle);
        assert!(!state.is_drawer_open());
    }

    #[test]
    fn the_drawer_closes_on_activation_and_when_the_window_widens() {
        let mut state = state();
        let _ = state.update(Event::Narrow(true));
        let _ = state.update(Event::OpenDrawer);
        assert!(state.has_keys());
        let _ = state.update(Event::Activate(Home));
        assert!(!state.is_drawer_open());

        let _ = state.update(Event::OpenDrawer);
        let _ = state.update(Event::Narrow(false));
        assert!(!state.is_drawer_open());
        let _ = state.update(Event::Narrow(true));
        assert!(!state.is_drawer_open(), "stays closed");
        let _ = state.update(Event::ToggleDrawer);
        let _ = state.update(Event::CloseDrawer);
        assert!(!state.is_drawer_open());
    }

    #[test]
    fn the_ring_shows_once_the_keyboard_moves_the_highlight() {
        let mut state = state().with_active(Home);
        let _ = state.update(Event::Focus(true));
        assert!(!state.shows_highlight(), "a press does not draw the ring");
        let _ = state.update(Event::Next);
        assert!(state.shows_highlight());
        let _ = state.update(Event::Activate(Inbox));
        assert!(!state.shows_highlight(), "a click hides it again");
        let _ = state.update(Event::Next);
        let _ = state.update(Event::Focus(false));
        assert!(!state.shows_highlight(), "only while the sidebar has keys");
    }

    #[test]
    fn focus_highlights_the_active_item() {
        let mut state = state().with_active(Beta);
        let _ = state.update(Event::Focus(true));
        assert!(state.is_focused());
        assert_eq!(state.highlighted(), Some(Row::Item(Beta)));
        let _ = state.update(Event::Focus(false));
        assert!(!state.has_keys());

        let mut closed = state.clone();
        closed.highlighted = None;
        let _ = closed.update(Event::ToggleItem(Projects));
        let _ = closed.update(Event::Focus(true));
        assert_eq!(closed.highlighted(), Some(Row::Item(Projects)));
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
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::Expand));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::Collapse));
        assert_eq!(press(&keymap, "Mod+B"), Some(Action::ToggleRail));
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));
        assert_eq!(press(&keymap, "B"), None);
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&"Mod+B".parse().unwrap())
            .bind("Ctrl+Shift+S".parse().unwrap(), Action::ToggleRail);
        assert_eq!(press(&keymap, "Mod+B"), None);
        assert_eq!(press(&keymap, "Ctrl+Shift+S"), Some(Action::ToggleRail));
    }

    #[test]
    fn actions_map_to_events() {
        let mut state = state();
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(
            Action::Activate.event(&state),
            Some(Event::ActivateHighlighted)
        );
        assert_eq!(Action::ToggleRail.event(&state), Some(Event::Toggle));
        assert_eq!(Action::Close.event(&state), None);
        let _ = state.update(Event::Focus(true));
        assert_eq!(Action::Close.event(&state), Some(Event::Focus(false)));
        let _ = state.update(Event::Narrow(true));
        let _ = state.update(Event::OpenDrawer);
        assert_eq!(Action::Close.event(&state), Some(Event::CloseDrawer));
    }

    #[test]
    fn shortcut_only_resolves_the_app_wide_action() {
        let state = state();
        let keymap = default_keymap();
        let key = |chord: &str| {
            let chord: Chord = chord.parse().unwrap();
            keys::Event {
                key: chord.key().clone(),
                modifiers: chord.modifiers(),
            }
        };
        assert_eq!(state.shortcut(&keymap, &key("Mod+B")), Some(Event::Toggle));
        assert_eq!(state.shortcut(&keymap, &key("ArrowDown")), None);
    }

    #[test]
    fn rows_highlight_when_active_hovered_or_pressed() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = item_style(&tokens, false, false, button::Status::Active);
            assert_eq!(idle.background, None);
            assert_eq!(idle.text_color, tokens.foreground);
            for status in [button::Status::Hovered, button::Status::Pressed] {
                let style = item_style(&tokens, false, false, status);
                assert_eq!(style.background, Some(Background::Color(tokens.accent)));
            }
            let active = item_style(&tokens, true, false, button::Status::Active);
            assert_eq!(active.background, Some(Background::Color(tokens.accent)));
            let disabled = item_style(&tokens, true, false, button::Status::Disabled);
            assert_eq!(disabled.background, None);
            assert_eq!(disabled.text_color, tokens.muted_foreground);
        }
    }

    #[test]
    fn the_keyboard_highlight_draws_a_ring() {
        let tokens = Tokens::of(&light());
        let ring = item_style(&tokens, false, true, button::Status::Active);
        assert_eq!(ring.border.color, tokens.ring);
        assert_eq!(ring.border.width, 1.0);
        let plain = item_style(&tokens, false, false, button::Status::Active);
        assert_eq!(plain.border.width, 0.0);
    }

    #[test]
    fn surfaces_sit_between_page_and_card() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let surface = surface_style(&tokens).background;
            assert!(surface.is_some());
            assert_ne!(surface, Some(Background::Color(tokens.background)));
            assert_eq!(
                edge_style(&tokens).background,
                Some(Background::Color(tokens.border))
            );
            assert!(scrim_style(&tokens).background.is_some());
        }
        let logo = avatar_style(&Tokens::of(&light()), true);
        assert_eq!(
            logo.text_color,
            Some(Tokens::of(&light()).primary_foreground)
        );
    }

    #[test]
    fn builder_defaults() {
        let state = state();
        let sidebar: Sidebar<'_, Page, ()> = sidebar(&state);
        assert_eq!(sidebar.breakpoint, BREAKPOINT);
        assert!(sidebar.trigger);
        assert!(sidebar.content.is_none());
        assert!(sidebar.on_event.is_none());
        assert_eq!(sidebar.keymap, default_keymap());
    }
}
