//! A compact strip of icon buttons: actions, toggles that stay pressed,
//! single-choice groups, separators and a spacer.
//!
//! [`State`] owns the entries and whether each toggle or choice is on.
//! The toolbar measures itself with iced's `responsive` widget, and the
//! entries that do not fit move, in order from the end, into a "More"
//! [dropdown menu](mod@crate::overlay::dropdown_menu) at the end of the strip,
//! where toggles become checkbox items and choices radio items. So a
//! toolbar built for a desktop window still works on a phone.
//!
//! The keyboard moves a highlight along the strip. The app routes key
//! presses from [`keys::subscription`] through [`State::key_event`]:
//! Ctrl+F10 focuses the toolbar, Left and Right move between the buttons
//! (and the More button), Enter or Space presses one and Escape leaves.
//! iced gives an app-drawn strip no focus of its own, so the toolbar
//! tracks its focus in [`State`] and draws the highlight as a ring.
//!
//! ```no_run
//! use iced::Element;
//! use iced_cube::lucide;
//! use iced_cube::toolbar::{self, Output, separator, toggle, toolbar};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Toolbar(toolbar::Event<&'static str>),
//! }
//!
//! struct App {
//!     toolbar: toolbar::State<&'static str>,
//! }
//!
//! impl App {
//!     fn new() -> Self {
//!         let toolbar = toolbar::State::new([
//!             toggle("bold", lucide!(Bold), "Bold", false).shortcut("Ctrl+B"),
//!             toggle("italic", lucide!(Italic), "Italic", false),
//!             separator(),
//!             toolbar::button("undo", lucide!(Undo2), "Undo"),
//!         ]);
//!         Self { toolbar }
//!     }
//!
//!     fn update(&mut self, message: Message) {
//!         let Message::Toolbar(event) = message;
//!         if let Some(Output::Toggled(id, on)) = self.toolbar.update(event) {
//!             println!("{id} is now {on}");
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         toolbar(&self.toolbar).on_event(Message::Toolbar).into()
//!     }
//! }
//! ```

use std::rc::Rc;

use iced::keyboard::key::Named;
use iced::widget::{self, column, container, responsive, row, sensor, space, stack};
use iced::{Alignment, Background, Border, Element, Length};

use crate::icon::Glyph;
use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::Align;
use crate::overlay::dropdown_menu::{self, dropdown_menu};
use crate::overlay::tooltip::Position;
use crate::primitives::button::Size;
use crate::primitives::icon_button::icon_button;
use crate::primitives::separator::vertical_separator;
use crate::theme::{Tokens, radius, space as gap};

pub use crate::overlay::dropdown_menu::Output;

/// Space between neighbouring entries.
pub const GAP: f32 = gap::XS;
/// Space between the strip's edge and its entries.
pub const PADDING: f32 = gap::XS;
/// Width a separator takes: the line and a margin on each side.
pub const SEPARATOR_WIDTH: f32 = 1.0 + 2.0 * gap::XS;
/// Height of a separator's line.
pub const SEPARATOR_HEIGHT: f32 = 20.0;
/// Width of the highlight ring around the focused button.
pub const RING_WIDTH: f32 = 2.0;

/// What a button does when pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A plain action, such as undo.
    Action,
    /// Stays pressed while on, such as bold.
    Toggle(bool),
    /// One choice among the choice buttons next to it, such as an
    /// alignment. A run of adjacent choices forms one group.
    Choice(bool),
}

/// One icon button.
#[derive(Debug, Clone, PartialEq)]
pub struct Item<Id> {
    pub id: Id,
    pub icon: Glyph,
    /// Shown as the tooltip and as the row label in the overflow menu.
    pub label: String,
    /// A hint such as `Ctrl+B`, added to the tooltip and shown in the
    /// overflow menu. It does not bind the keys.
    pub shortcut: Option<String>,
    pub disabled: bool,
    pub kind: Kind,
}

impl<Id> Item<Id> {
    /// Whether a toggle or choice is on.
    pub fn is_on(&self) -> bool {
        matches!(self.kind, Kind::Toggle(true) | Kind::Choice(true))
    }

    /// The tooltip: the label, with the shortcut in brackets.
    pub fn tooltip(&self) -> String {
        match &self.shortcut {
            Some(shortcut) => format!("{} ({shortcut})", self.label),
            None => self.label.clone(),
        }
    }
}

/// One slot of a toolbar.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry<Id> {
    Item(Item<Id>),
    /// A short vertical line between groups.
    Separator,
    /// Pushes the entries after it to the far end while everything fits.
    Spacer,
}

fn new_item<Id>(id: Id, icon: Glyph, label: impl Into<String>, kind: Kind) -> Entry<Id> {
    Entry::Item(Item {
        id,
        icon,
        label: label.into(),
        shortcut: None,
        disabled: false,
        kind,
    })
}

/// An action button.
pub fn button<Id>(id: Id, icon: Glyph, label: impl Into<String>) -> Entry<Id> {
    new_item(id, icon, label, Kind::Action)
}

/// A button that stays pressed while on.
pub fn toggle<Id>(id: Id, icon: Glyph, label: impl Into<String>, on: bool) -> Entry<Id> {
    new_item(id, icon, label, Kind::Toggle(on))
}

/// A button that turns itself on and the choices next to it off.
pub fn choice<Id>(id: Id, icon: Glyph, label: impl Into<String>, selected: bool) -> Entry<Id> {
    new_item(id, icon, label, Kind::Choice(selected))
}

pub fn separator<Id>() -> Entry<Id> {
    Entry::Separator
}

pub fn spacer<Id>() -> Entry<Id> {
    Entry::Spacer
}

impl<Id> Entry<Id> {
    /// Adds a shortcut hint. Ignored on separators and spacers, as is
    /// [`disabled`](Self::disabled).
    pub fn shortcut(self, hint: impl Into<String>) -> Self {
        let hint = hint.into();
        self.with_item(|item| item.shortcut = Some(hint))
    }

    pub fn disabled(self, disabled: bool) -> Self {
        self.with_item(|item| item.disabled = disabled)
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

    fn is_choice(&self) -> bool {
        matches!(
            self,
            Entry::Item(Item {
                kind: Kind::Choice(_),
                ..
            })
        )
    }

    /// Width in the strip for buttons `side` pixels square.
    fn width(&self, side: f32) -> f32 {
        match self {
            Entry::Item(_) => side,
            Entry::Separator => SEPARATOR_WIDTH,
            Entry::Spacer => 0.0,
        }
    }
}

/// How many entries fit in `width` before the rest move into the
/// overflow menu. Every entry fits when they all do; otherwise room is
/// kept for the More button and trailing separators and spacers go too.
pub fn fit<Id>(entries: &[Entry<Id>], width: f32, size: Size) -> usize {
    let side = size.metrics().height;
    let available = width - 2.0 * PADDING;
    let span = |entries: &[Entry<Id>]| -> f32 {
        let widths: f32 = entries.iter().map(|entry| entry.width(side)).sum();
        widths + GAP * entries.len().saturating_sub(1) as f32
    };
    if span(entries) <= available {
        return entries.len();
    }
    let mut shown = (0..entries.len())
        .rev()
        .find(|&count| {
            let before = entries.get(..count).unwrap_or_default();
            span(before) + if count > 0 { GAP } else { 0.0 } + side <= available
        })
        .unwrap_or(0);
    while shown > 0 && entries.get(shown - 1).and_then(Entry::as_item).is_none() {
        shown -= 1;
    }
    shown
}

/// The overflow menu's entries for everything from `start` on. Toggles
/// become checkbox items and choices radio items; spacers are dropped and
/// separators kept only between items.
pub fn overflow_entries<Id: Copy>(
    entries: &[Entry<Id>],
    start: usize,
) -> Vec<dropdown_menu::Entry<Id>> {
    let mut menu: Vec<dropdown_menu::Entry<Id>> = Vec::new();
    for entry in entries.get(start..).unwrap_or_default() {
        match entry {
            Entry::Item(item) => menu.push(overflow_item(item)),
            Entry::Separator => {
                if menu.last().is_some_and(|last| last.as_item().is_some()) {
                    menu.push(dropdown_menu::separator());
                }
            }
            Entry::Spacer => {}
        }
    }
    if menu.last().is_some_and(|last| last.as_item().is_none()) {
        menu.pop();
    }
    menu
}

fn overflow_item<Id: Copy>(item: &Item<Id>) -> dropdown_menu::Entry<Id> {
    let entry = match item.kind {
        Kind::Action => dropdown_menu::item(item.id, item.label.clone()),
        Kind::Toggle(on) => dropdown_menu::checkbox_item(item.id, item.label.clone(), on),
        Kind::Choice(on) => dropdown_menu::radio_item(item.id, item.label.clone(), on),
    };
    let entry = entry.icon(item.icon).disabled(item.disabled);
    match &item.shortcut {
        Some(shortcut) => entry.shortcut(shortcut.clone()),
        None => entry,
    }
}

/// Where the keyboard highlight is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Highlight<Id> {
    Item(Id),
    /// The More button that opens the overflow menu.
    More,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    Entry(usize),
    More,
}

/// Changes to a toolbar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event<Id> {
    /// The strip was measured at this width. The widget sends this itself.
    Resized(f32),
    /// A button in the strip was pressed.
    Press(Id),
    /// An event for the overflow menu, holding everything from this entry on.
    Overflow(usize, dropdown_menu::Event<Id>),
    /// Highlights the first enabled button, or leaves the toolbar when it
    /// has focus.
    Focus,
    /// Highlights the next button, wrapping.
    Next,
    /// Highlights the previous button, wrapping.
    Previous,
    First,
    Last,
    /// Presses the highlighted button, or opens the overflow menu on its
    /// first item.
    ActivateHighlighted,
    /// Closes the overflow menu, or leaves the toolbar.
    Close,
}

/// The entries, their on and off states, the overflow menu and the
/// keyboard highlight.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    entries: Vec<Entry<Id>>,
    size: Size,
    width: Option<f32>,
    overflow: dropdown_menu::State<Id>,
    split: usize,
    highlighted: Option<Stop>,
}

impl<Id: Copy + PartialEq> State<Id> {
    /// A toolbar of small buttons. Item ids must be unique.
    pub fn new(entries: impl IntoIterator<Item = Entry<Id>>) -> Self {
        let entries: Vec<Entry<Id>> = entries.into_iter().collect();
        let split = entries.len();
        Self {
            overflow: dropdown_menu::State::new(overflow_entries(&entries, split)),
            entries,
            size: Size::Sm,
            width: None,
            split,
            highlighted: None,
        }
    }

    /// Sets the button size. Toolbars default to [`Size::Sm`].
    pub fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn entries(&self) -> &[Entry<Id>] {
        &self.entries
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn item(&self, id: Id) -> Option<&Item<Id>> {
        self.entries
            .iter()
            .filter_map(Entry::as_item)
            .find(|item| item.id == id)
    }

    /// Whether a toggle or choice is on.
    pub fn is_on(&self, id: Id) -> bool {
        self.item(id).is_some_and(Item::is_on)
    }

    /// How many entries show in the strip at the last measured width. The
    /// rest are in the overflow menu. All of them until the strip is measured.
    pub fn visible(&self) -> usize {
        self.width.map_or(self.entries.len(), |width| {
            fit(&self.entries, width, self.size)
        })
    }

    /// The overflow menu, open or closed.
    pub fn overflow(&self) -> &dropdown_menu::State<Id> {
        &self.overflow
    }

    /// Where the keyboard highlight is, while the toolbar has focus.
    pub fn highlighted(&self) -> Option<Highlight<Id>> {
        match self.highlighted? {
            Stop::More => Some(Highlight::More),
            Stop::Entry(index) => self
                .entries
                .get(index)
                .and_then(Entry::as_item)
                .map(|item| Highlight::Item(item.id)),
        }
    }

    pub fn is_focused(&self) -> bool {
        self.highlighted.is_some()
    }

    /// Turns a toggle or choice on or off. Turning a choice on turns the
    /// other choices in its group off.
    pub fn set_on(&mut self, id: Id, on: bool) {
        let Some(index) = self.position(id) else {
            return;
        };
        let group = self.choice_group(index);
        let Some(Entry::Item(item)) = self.entries.get_mut(index) else {
            return;
        };
        match &mut item.kind {
            Kind::Toggle(value) => *value = on,
            Kind::Choice(_) if on => {
                for sibling in group {
                    if let Some(Entry::Item(item)) = self.entries.get_mut(sibling) {
                        item.kind = Kind::Choice(sibling == index);
                    }
                }
            }
            Kind::Choice(value) => *value = false,
            Kind::Action => return,
        }
        self.rebuild(self.split);
    }

    /// Enables or disables a button. A disabled button loses the highlight.
    pub fn set_disabled(&mut self, id: Id, disabled: bool) {
        let Some(index) = self.position(id) else {
            return;
        };
        if let Some(Entry::Item(item)) = self.entries.get_mut(index) {
            item.disabled = disabled;
        }
        if disabled && self.highlighted == Some(Stop::Entry(index)) {
            self.highlighted = self.stops().first().copied();
        }
        self.rebuild(self.split);
    }

    /// Applies an event and returns what a press did, if anything.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Resized(width) => {
                self.width = Some(width);
                self.keep_highlight_in_view();
            }
            Event::Press(id) => {
                self.highlighted = None;
                return self.press(id);
            }
            Event::Overflow(split, event) => return self.overflow_event(split, event),
            Event::Focus if self.is_focused() => self.leave(),
            Event::Focus => self.highlighted = self.stops().first().copied(),
            Event::Next => self.step(true),
            Event::Previous => self.step(false),
            Event::First if self.is_focused() => self.highlighted = self.stops().first().copied(),
            Event::Last if self.is_focused() => self.highlighted = self.stops().last().copied(),
            Event::First | Event::Last => {}
            Event::ActivateHighlighted => return self.activate_highlighted(),
            Event::Close if self.overflow.is_open() => {
                let _ = self.overflow.update(dropdown_menu::Event::Close);
            }
            Event::Close => self.highlighted = None,
        }
        None
    }

    fn leave(&mut self) {
        self.highlighted = None;
        let _ = self.overflow.update(dropdown_menu::Event::Close);
    }

    fn position(&self, id: Id) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.as_item().is_some_and(|item| item.id == id))
    }

    /// The indices of the run of choices around `index`.
    fn choice_group(&self, index: usize) -> std::ops::Range<usize> {
        let before = self.entries.get(..index).unwrap_or_default();
        let start = before
            .iter()
            .rposition(|entry| !entry.is_choice())
            .map_or(0, |position| position + 1);
        let after = self.entries.get(index..).unwrap_or_default();
        let end = after
            .iter()
            .position(|entry| !entry.is_choice())
            .map_or(self.entries.len(), |position| index + position);
        start..end
    }

    /// Applies a press on a button, wherever it is shown.
    fn press(&mut self, id: Id) -> Option<Output<Id>> {
        let item = self.item(id).filter(|item| !item.disabled)?;
        let output = match item.kind {
            Kind::Action => Output::Activated(id),
            Kind::Toggle(on) => Output::Toggled(id, !on),
            Kind::Choice(_) => Output::Selected(id),
        };
        match output {
            Output::Toggled(_, on) => self.set_on(id, on),
            Output::Selected(_) => self.set_on(id, true),
            Output::Activated(_) => {}
        }
        Some(output)
    }

    fn overflow_event(
        &mut self,
        split: usize,
        event: dropdown_menu::Event<Id>,
    ) -> Option<Output<Id>> {
        if split != self.split {
            self.rebuild(split);
        }
        let output = self.overflow.update(event)?;
        let id = match output {
            Output::Activated(id) | Output::Toggled(id, _) | Output::Selected(id) => id,
        };
        match output {
            Output::Toggled(_, on) => self.set_on(id, on),
            Output::Selected(_) => self.set_on(id, true),
            Output::Activated(_) => {}
        }
        Some(output)
    }

    /// Rebuilds the overflow menu for the entries from `split` on,
    /// keeping it open if it was.
    fn rebuild(&mut self, split: usize) {
        let open = self.overflow.is_open() && split == self.split;
        self.split = split;
        self.overflow = dropdown_menu::State::new(overflow_entries(&self.entries, split));
        if open {
            let _ = self.overflow.update(dropdown_menu::Event::Open);
        }
    }

    /// The highlight's stops: every enabled button in view, then More if
    /// anything overflows.
    fn stops(&self) -> Vec<Stop> {
        let visible = self.visible();
        let mut stops: Vec<Stop> = self
            .entries
            .iter()
            .take(visible)
            .enumerate()
            .filter(|(_, entry)| entry.as_item().is_some_and(|item| !item.disabled))
            .map(|(index, _)| Stop::Entry(index))
            .collect();
        if visible < self.entries.len() {
            stops.push(Stop::More);
        }
        stops
    }

    fn step(&mut self, forward: bool) {
        let Some(current) = self.highlighted else {
            return;
        };
        let stops = self.stops();
        let Some(position) = stops.iter().position(|stop| *stop == current) else {
            self.highlighted = stops.first().copied();
            return;
        };
        let len = stops.len();
        let target = if forward {
            (position + 1) % len
        } else {
            (position + len - 1) % len
        };
        self.highlighted = stops.get(target).copied();
    }

    fn keep_highlight_in_view(&mut self) {
        let Some(current) = self.highlighted else {
            return;
        };
        let stops = self.stops();
        if !stops.contains(&current) {
            self.highlighted = stops.last().copied();
        }
    }

    fn activate_highlighted(&mut self) -> Option<Output<Id>> {
        match self.highlighted? {
            Stop::More => {
                let visible = self.visible();
                if visible != self.split {
                    self.rebuild(visible);
                }
                let _ = self.overflow.update(dropdown_menu::Event::First);
                None
            }
            Stop::Entry(index) => {
                let id = self.entries.get(index)?.as_item()?.id;
                self.press(id)
            }
        }
    }

    /// Turns a key press into an event: [`Action::Focus`] at any time, the
    /// other actions only while the toolbar has focus. The open overflow
    /// menu resolves its own keys before any of these.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event<Id>> {
        keymap
            .resolve_event(key)
            .and_then(|action| action.event(self))
    }
}

/// What a toolbar keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Focuses the toolbar, or leaves it.
    Focus,
    Next,
    Previous,
    First,
    Last,
    Activate,
    Close,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing: only [`Action::Focus`] works while the toolbar has no focus.
    pub fn event<Id: Copy + PartialEq>(self, state: &State<Id>) -> Option<Event<Id>> {
        if !state.is_focused() {
            return (self == Action::Focus).then_some(Event::Focus);
        }
        Some(match self {
            Action::Focus => Event::Focus,
            Action::Next => Event::Next,
            Action::Previous => Event::Previous,
            Action::First => Event::First,
            Action::Last => Event::Last,
            Action::Activate => Event::ActivateHighlighted,
            Action::Close => Event::Close,
        })
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Focus,
        Action::Next,
        Action::Previous,
        Action::First,
        Action::Last,
        Action::Activate,
        Action::Close,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Focus => "Focus",
            Action::Next => "Next",
            Action::Previous => "Previous",
            Action::First => "First",
            Action::Last => "Last",
            Action::Activate => "Activate",
            Action::Close => "Close",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Focus => "Highlights the first button, or leaves the toolbar.",
            Action::Next => "Highlights the next button, wrapping, with More last.",
            Action::Previous => "Highlights the previous button, wrapping.",
            Action::First => "Highlights the first button.",
            Action::Last => "Highlights the last button, or More when anything overflows.",
            Action::Activate => "Presses the highlighted button, or opens the overflow menu.",
            Action::Close => "Leaves the toolbar.",
        }
    }
}

/// The default toolbar shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Ctrl+F10` | [`Action::Focus`] |
/// | `ArrowRight` | [`Action::Next`] |
/// | `ArrowLeft` | [`Action::Previous`] |
/// | `Home` | [`Action::First`] |
/// | `End` | [`Action::Last`] |
/// | `Enter`, `Space` | [`Action::Activate`] |
/// | `Escape` | [`Action::Close`] |
///
/// Route presses from [`keys::subscription`] through [`State::key_event`].
/// Only Focus claims a key until the toolbar has focus, so the arrows stay
/// free for the rest of the app; route the toolbar after anything that
/// should see the arrows first while it is focused. The open overflow
/// menu handles its own keys with the dropdown menu keymap.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::F10).ctrl(), Action::Focus)
        .bind(Chord::named(Named::ArrowRight), Action::Next)
        .bind(Chord::named(Named::ArrowLeft), Action::Previous)
        .bind(Chord::named(Named::Home), Action::First)
        .bind(Chord::named(Named::End), Action::Last)
        .bind(Chord::named(Named::Enter), Action::Activate)
        .bind(Chord::named(Named::Space), Action::Activate)
        .bind(Chord::named(Named::Escape), Action::Close)
}

/// The ring drawn around the highlighted button.
pub fn ring_style(tokens: &Tokens) -> container::Style {
    container::Style {
        border: Border {
            color: tokens.ring,
            width: RING_WIDTH,
            radius: radius::MD.into(),
        },
        ..container::Style::default()
    }
}

/// The style of the strip behind the buttons.
pub fn bar_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.background)),
        text_color: Some(tokens.foreground),
        ..container::Style::default()
    }
}

type OnEvent<'a, Id, Message> = Rc<dyn Fn(Event<Id>) -> Message + 'a>;

/// A toolbar builder. Convert it into an [`Element`] to render.
pub struct Toolbar<'a, Id, Message> {
    state: &'a State<Id>,
    tooltip: Option<Position>,
    more: String,
    menu_keymap: Keymap<dropdown_menu::Action>,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for Toolbar<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toolbar")
            .field("state", self.state)
            .field("tooltip", &self.tooltip)
            .field("more", &self.more)
            .field("menu_keymap", &self.menu_keymap)
            .finish_non_exhaustive()
    }
}

/// Renders the entries of `state` that fit, and a More menu with the rest.
/// Without [`on_event`](Toolbar::on_event) every button renders disabled.
pub fn toolbar<'a, Id, Message>(state: &'a State<Id>) -> Toolbar<'a, Id, Message> {
    Toolbar {
        state,
        tooltip: Some(Position::Bottom),
        more: String::from("More"),
        menu_keymap: dropdown_menu::default_keymap(),
        on_event: None,
    }
}

impl<'a, Id, Message> Toolbar<'a, Id, Message> {
    /// Where the button tooltips appear, or `None` for none. Defaults to
    /// below, since toolbars usually sit at the top of a window.
    pub fn tooltip(mut self, position: Option<Position>) -> Self {
        self.tooltip = position;
        self
    }

    /// The More button's label, shown as its tooltip.
    pub fn more_label(mut self, label: impl Into<String>) -> Self {
        self.more = label.into();
        self
    }

    /// Replaces the [dropdown menu keys](dropdown_menu::default_keymap)
    /// the open overflow menu resolves.
    pub fn menu_keymap(mut self, keymap: Keymap<dropdown_menu::Action>) -> Self {
        self.menu_keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Id, Message> From<Toolbar<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(toolbar: Toolbar<'a, Id, Message>) -> Self {
        let Toolbar {
            state,
            tooltip,
            more,
            menu_keymap,
            on_event,
        } = toolbar;
        let on_event: Option<OnEvent<'a, Id, Message>> = on_event.map(Rc::from);
        let on_resize = on_event.clone();
        let strip = responsive(move |size| {
            strip(
                state,
                size.width,
                tooltip,
                &more,
                &menu_keymap,
                on_event.clone(),
            )
        })
        .height(Length::Shrink);

        let measured: Element<'a, Message> = match on_resize {
            Some(on_event) => sensor(strip)
                .on_resize(move |size| on_event(Event::Resized(size.width)))
                .into(),
            None => strip.into(),
        };
        column![measured, crate::application::edge()]
            .width(Length::Fill)
            .into()
    }
}

/// The entries that fit in `width`, then the More button.
fn strip<'a, Id, Message>(
    state: &'a State<Id>,
    width: f32,
    tooltip: Option<Position>,
    more: &str,
    menu_keymap: &Keymap<dropdown_menu::Action>,
    on_event: Option<OnEvent<'a, Id, Message>>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let shown = fit(&state.entries, width, state.size);
    let mut children: Vec<Element<'a, Message>> = state
        .entries
        .iter()
        .take(shown)
        .enumerate()
        .map(|(index, entry)| match entry {
            Entry::Item(item) => {
                let highlighted = state.highlighted == Some(Stop::Entry(index));
                item_button(item, state.size, tooltip, highlighted, on_event.as_ref())
            }
            Entry::Separator => container(vertical_separator())
                .height(SEPARATOR_HEIGHT)
                .padding([0.0, gap::XS])
                .into(),
            Entry::Spacer => space::horizontal().into(),
        })
        .collect();
    if shown < state.entries.len() {
        children.push(more_button(
            state,
            shown,
            tooltip,
            more,
            menu_keymap,
            on_event.as_ref(),
        ));
    }

    container(
        row(children)
            .spacing(GAP)
            .align_y(Alignment::Center)
            .width(Length::Fill),
    )
    .padding(PADDING)
    .width(Length::Fill)
    .style(|theme| bar_style(&Tokens::of(theme)))
    .into()
}

fn item_button<'a, Id, Message>(
    item: &Item<Id>,
    size: Size,
    tooltip: Option<Position>,
    highlighted: bool,
    on_event: Option<&OnEvent<'a, Id, Message>>,
) -> Element<'a, Message>
where
    Id: Copy + 'a,
    Message: Clone + 'a,
{
    let on_press = on_event
        .filter(|_| !item.disabled)
        .map(|on_event| on_event(Event::Press(item.id)));
    let button = icon_button(item.icon)
        .label(item.tooltip())
        .tooltip(tooltip)
        .size(size)
        .pressed(item.is_on())
        .on_press_maybe(on_press);
    ringed(button, highlighted)
}

fn more_button<'a, Id, Message>(
    state: &'a State<Id>,
    split: usize,
    tooltip: Option<Position>,
    more: &str,
    menu_keymap: &Keymap<dropdown_menu::Action>,
    on_event: Option<&OnEvent<'a, Id, Message>>,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let open = state.overflow.is_open() && state.split == split;
    let trigger = icon_button(crate::lucide!(Ellipsis))
        .label(more.to_owned())
        .tooltip(tooltip.filter(|_| !open))
        .size(state.size)
        .pressed(open)
        .on_press_maybe(
            on_event.map(|on_event| on_event(Event::Overflow(split, dropdown_menu::Event::Toggle))),
        );
    let trigger = ringed(trigger, state.highlighted == Some(Stop::More));
    let Some(on_event) = on_event.filter(|_| open).cloned() else {
        return trigger;
    };
    dropdown_menu(&state.overflow, trigger)
        .align(Align::End)
        .keymap(menu_keymap.clone())
        .on_event(move |event| on_event(Event::Overflow(split, event)))
        .into()
}

/// Draws the highlight ring over a button while it has the keyboard focus.
fn ringed<'a, Message: Clone + 'a>(
    button: impl Into<Element<'a, Message>>,
    highlighted: bool,
) -> Element<'a, Message> {
    let button = button.into();
    if !highlighted {
        return button;
    }
    stack![
        button,
        container(widget::space())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme| ring_style(&Tokens::of(theme))),
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use iced::keyboard::{Key, Modifiers};

    const BOLD: u8 = 1;
    const ITALIC: u8 = 2;
    const LEFT: u8 = 3;
    const RIGHT: u8 = 4;
    const UNDO: u8 = 5;
    const REDO: u8 = 6;

    fn entries() -> Vec<Entry<u8>> {
        vec![
            toggle(BOLD, crate::lucide!(Bold), "Bold", true).shortcut("Ctrl+B"),
            toggle(ITALIC, crate::lucide!(Italic), "Italic", false),
            choice(LEFT, crate::lucide!(TextAlignStart), "Left", true),
            separator(),
            choice(RIGHT, crate::lucide!(TextAlignEnd), "Right", false),
            button(UNDO, crate::lucide!(Undo2), "Undo"),
        ]
    }

    fn state() -> State<u8> {
        State::new(entries())
    }

    /// Measured at 160 pixels, where three buttons fit before More.
    fn narrow() -> State<u8> {
        let mut state = state();
        let _ = state.update(Event::Resized(160.0));
        state
    }

    #[test]
    fn builders_set_item_fields_and_ignore_separators() {
        let entry: Entry<u8> = button(UNDO, crate::lucide!(Undo2), "Undo")
            .shortcut("Ctrl+Z")
            .disabled(true);
        let item = entry.as_item().unwrap();
        assert_eq!(item.label, "Undo");
        assert_eq!(item.icon, crate::lucide!(Undo2));
        assert_eq!(item.shortcut.as_deref(), Some("Ctrl+Z"));
        assert!(item.disabled);
        assert_eq!(item.kind, Kind::Action);
        assert_eq!(item.tooltip(), "Undo (Ctrl+Z)");
        assert_eq!(separator::<u8>().disabled(true), Entry::Separator);
        assert_eq!(spacer::<u8>().shortcut("X"), Entry::Spacer);
        assert!(separator::<u8>().as_item().is_none());
    }

    #[test]
    fn a_new_toolbar_is_small_unmeasured_and_unfocused() {
        let state = state();
        assert_eq!(state.size(), Size::Sm);
        assert_eq!(state.visible(), state.entries().len());
        assert!(!state.is_focused());
        assert_eq!(state.highlighted(), None);
        assert!(!state.overflow().is_open());
        assert!(state.is_on(BOLD) && state.is_on(LEFT));
        assert!(!state.is_on(ITALIC) && !state.is_on(UNDO));
        assert_eq!(state.with_size(Size::Lg).size(), Size::Lg);
    }

    #[test]
    fn everything_fits_when_there_is_room() {
        // Five 32 pixel buttons, a 9 pixel separator, five gaps and padding.
        let exact = 5.0 * 32.0 + SEPARATOR_WIDTH + 5.0 * GAP + 2.0 * PADDING;
        assert_eq!(fit(&entries(), exact, Size::Sm), 6);
        assert_eq!(fit(&entries(), exact + 100.0, Size::Sm), 6);
        assert_eq!(fit::<u8>(&[], 0.0, Size::Sm), 0);
    }

    #[test]
    fn overflow_keeps_room_for_more_and_drops_trailing_separators() {
        let exact = 5.0 * 32.0 + SEPARATOR_WIDTH + 5.0 * GAP + 2.0 * PADDING;
        assert_eq!(
            fit(&entries(), exact - 1.0, Size::Sm),
            3,
            "the separator before the cut goes too"
        );
        assert_eq!(fit(&entries(), 160.0, Size::Sm), 3);
        assert_eq!(fit(&entries(), 100.0, Size::Sm), 1);
        assert_eq!(fit(&entries(), 10.0, Size::Sm), 0);
        assert!(
            fit(&entries(), 160.0, Size::Lg) < 3,
            "bigger buttons fit fewer"
        );
    }

    #[test]
    fn spacers_take_no_room() {
        let entries = vec![
            button(UNDO, crate::lucide!(Undo2), "Undo"),
            spacer(),
            button(REDO, crate::lucide!(Redo2), "Redo"),
        ];
        let exact = 2.0 * 32.0 + 2.0 * GAP + 2.0 * PADDING;
        assert_eq!(fit(&entries, exact, Size::Sm), 3);
        assert_eq!(fit(&entries, exact - 1.0, Size::Sm), 1);
    }

    #[test]
    fn overflow_entries_map_kinds_and_tidy_separators() {
        let menu = overflow_entries(&entries(), 0);
        let kinds: Vec<_> = menu
            .iter()
            .map(|entry| entry.as_item().map(|item| item.kind.clone()))
            .collect();
        assert_eq!(
            kinds,
            [
                Some(dropdown_menu::Kind::Checkbox(true)),
                Some(dropdown_menu::Kind::Checkbox(false)),
                Some(dropdown_menu::Kind::Radio(true)),
                None,
                Some(dropdown_menu::Kind::Radio(false)),
                Some(dropdown_menu::Kind::Action),
            ]
        );
        let bold = menu[0].as_item().unwrap();
        assert_eq!(bold.shortcut.as_deref(), Some("Ctrl+B"));
        assert_eq!(bold.icon, Some(crate::lucide!(Bold)));

        let from_separator = overflow_entries(&entries(), 3);
        assert!(
            from_separator[0].as_item().is_some(),
            "no leading separator"
        );
        assert_eq!(from_separator.len(), 2);

        let tail = vec![
            button(UNDO, crate::lucide!(Undo2), "Undo"),
            separator(),
            spacer(),
        ];
        assert_eq!(overflow_entries(&tail, 0).len(), 1, "no trailing separator");
        assert!(overflow_entries(&tail, 9).is_empty());
    }

    #[test]
    fn pressing_toggles_selects_and_activates() {
        let mut state = state();
        assert_eq!(
            state.update(Event::Press(BOLD)),
            Some(Output::Toggled(BOLD, false))
        );
        assert!(!state.is_on(BOLD));
        assert_eq!(
            state.update(Event::Press(RIGHT)),
            Some(Output::Selected(RIGHT))
        );
        assert!(state.is_on(RIGHT));
        assert!(state.is_on(LEFT), "a separator ends the choice group");
        assert_eq!(
            state.update(Event::Press(UNDO)),
            Some(Output::Activated(UNDO))
        );
        assert_eq!(state.update(Event::Press(99)), None);
    }

    #[test]
    fn choices_in_one_run_exclude_each_other() {
        let mut state = State::new([
            choice(LEFT, crate::lucide!(TextAlignStart), "Left", true),
            choice(RIGHT, crate::lucide!(TextAlignEnd), "Right", false),
        ]);
        let _ = state.update(Event::Press(RIGHT));
        assert!(state.is_on(RIGHT) && !state.is_on(LEFT));
        state.set_on(RIGHT, false);
        assert!(!state.is_on(RIGHT) && !state.is_on(LEFT));
        state.set_on(LEFT, true);
        assert!(state.is_on(LEFT));
    }

    #[test]
    fn disabled_buttons_do_nothing_and_lose_the_highlight() {
        let mut state = state();
        state.set_disabled(UNDO, true);
        assert_eq!(state.update(Event::Press(UNDO)), None);
        state.set_on(UNDO, true);
        assert!(!state.is_on(UNDO), "actions have no on state");

        let _ = state.update(Event::Focus);
        assert_eq!(state.highlighted(), Some(Highlight::Item(BOLD)));
        state.set_disabled(BOLD, true);
        assert_eq!(state.highlighted(), Some(Highlight::Item(ITALIC)));
    }

    #[test]
    fn the_overflow_menu_holds_what_does_not_fit_and_syncs_back() {
        let mut state = narrow();
        assert_eq!(state.visible(), 3);
        let _ = state.update(Event::Overflow(3, dropdown_menu::Event::Toggle));
        assert!(state.overflow().is_open());
        assert!(state.overflow().item(RIGHT).is_some());
        assert!(state.overflow().item(BOLD).is_none());

        assert_eq!(
            state.update(Event::Overflow(3, dropdown_menu::Event::Activate(RIGHT))),
            Some(Output::Selected(RIGHT))
        );
        assert!(state.is_on(RIGHT));
        assert!(!state.overflow().is_open());

        let _ = state.update(Event::Overflow(1, dropdown_menu::Event::Toggle));
        assert!(
            state.overflow().item(ITALIC).is_some(),
            "rebuilt for a new split"
        );
        assert_eq!(
            state.update(Event::Overflow(1, dropdown_menu::Event::Activate(ITALIC))),
            Some(Output::Toggled(ITALIC, true))
        );
        assert!(state.is_on(ITALIC));
    }

    #[test]
    fn changing_a_toggle_keeps_an_open_overflow_menu_open() {
        let mut state = narrow();
        let _ = state.update(Event::Overflow(3, dropdown_menu::Event::Toggle));
        state.set_on(RIGHT, true);
        assert!(state.overflow().is_open());
        assert!(state.overflow().is_checked(RIGHT));
    }

    #[test]
    fn focus_moves_through_the_visible_buttons_then_more() {
        let mut state = narrow();
        let _ = state.update(Event::Next);
        assert!(!state.is_focused(), "an unfocused toolbar does not move");
        let _ = state.update(Event::Focus);
        let mut seen = vec![state.highlighted().unwrap()];
        for _ in 0..4 {
            let _ = state.update(Event::Next);
            seen.push(state.highlighted().unwrap());
        }
        assert_eq!(
            seen,
            [
                Highlight::Item(BOLD),
                Highlight::Item(ITALIC),
                Highlight::Item(LEFT),
                Highlight::More,
                Highlight::Item(BOLD),
            ]
        );
        let _ = state.update(Event::Previous);
        assert_eq!(state.highlighted(), Some(Highlight::More));
        let _ = state.update(Event::First);
        assert_eq!(state.highlighted(), Some(Highlight::Item(BOLD)));
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(Highlight::More));
        let _ = state.update(Event::Focus);
        assert!(!state.is_focused(), "focus again leaves");
    }

    #[test]
    fn first_and_last_need_focus() {
        let mut state = state();
        let _ = state.update(Event::First);
        let _ = state.update(Event::Last);
        assert!(!state.is_focused());
    }

    #[test]
    fn activating_the_highlight_presses_or_opens_more() {
        let mut state = narrow();
        let _ = state.update(Event::Focus);
        assert_eq!(
            state.update(Event::ActivateHighlighted),
            Some(Output::Toggled(BOLD, false))
        );
        assert!(state.is_focused(), "the keyboard keeps the focus");

        let _ = state.update(Event::Last);
        assert_eq!(state.update(Event::ActivateHighlighted), None);
        assert!(state.overflow().is_open());
        assert_eq!(state.overflow().highlighted(), Some(RIGHT));

        let _ = state.update(Event::Close);
        assert!(!state.overflow().is_open());
        assert_eq!(state.highlighted(), Some(Highlight::More));
        let _ = state.update(Event::Close);
        assert!(!state.is_focused());
        assert_eq!(state.update(Event::ActivateHighlighted), None);
    }

    #[test]
    fn a_pointer_press_ends_keyboard_focus() {
        let mut state = state();
        let _ = state.update(Event::Focus);
        let _ = state.update(Event::Press(ITALIC));
        assert!(!state.is_focused());
    }

    #[test]
    fn narrowing_moves_a_hidden_highlight_to_more() {
        let mut state = state();
        let _ = state.update(Event::Focus);
        let _ = state.update(Event::Last);
        assert_eq!(state.highlighted(), Some(Highlight::Item(UNDO)));
        let _ = state.update(Event::Resized(160.0));
        assert_eq!(state.highlighted(), Some(Highlight::More));
    }

    fn key(key: Key, modifiers: Modifiers) -> keys::Event {
        keys::Event { key, modifiers }
    }

    #[test]
    fn an_unfocused_toolbar_claims_only_focus() {
        let state = state();
        let keymap = default_keymap();
        let right = key(Key::Named(Named::ArrowRight), Modifiers::empty());
        let focus = key(Key::Named(Named::F10), Modifiers::CTRL);
        assert_eq!(state.key_event(&keymap, &right), None);
        assert_eq!(state.key_event(&keymap, &focus), Some(Event::Focus));
        for &action in <Action as keys::Action>::ALL {
            let expected = (action == Action::Focus).then_some(Event::Focus);
            assert_eq!(action.event(&state), expected, "{action:?}");
        }
    }

    #[test]
    fn a_focused_toolbar_maps_every_action() {
        let mut state = state();
        let _ = state.update(Event::Focus);
        assert_eq!(Action::Focus.event(&state), Some(Event::Focus));
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(Action::Previous.event(&state), Some(Event::Previous));
        assert_eq!(Action::First.event(&state), Some(Event::First));
        assert_eq!(Action::Last.event(&state), Some(Event::Last));
        assert_eq!(
            Action::Activate.event(&state),
            Some(Event::ActivateHighlighted)
        );
        assert_eq!(Action::Close.event(&state), Some(Event::Close));
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Ctrl+F10"), Some(Action::Focus));
        assert_eq!(press(&keymap, "F10"), None, "plain F10 is the menubar's");
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Home"), Some(Action::First));
        assert_eq!(press(&keymap, "End"), Some(Action::Last));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Space"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&"Space".parse().unwrap())
            .bind("Alt+T".parse().unwrap(), Action::Focus);
        assert_eq!(press(&keymap, "Space"), None);
        assert_eq!(press(&keymap, "Alt+T"), Some(Action::Focus));
        assert_eq!(press(&keymap, "Ctrl+F10"), Some(Action::Focus));
    }

    #[test]
    fn every_action_has_a_name_and_a_sentence() {
        for &action in <Action as keys::Action>::ALL {
            assert!(!keys::Action::name(action).is_empty());
            assert!(keys::Action::description(action).ends_with('.'));
        }
    }

    #[test]
    fn builder_defaults_and_overrides() {
        let state = state();
        let bar: Toolbar<'_, u8, ()> = toolbar(&state);
        assert_eq!(bar.tooltip, Some(Position::Bottom));
        assert_eq!(bar.more, "More");
        assert_eq!(bar.menu_keymap, dropdown_menu::default_keymap());
        assert!(bar.on_event.is_none());

        let bar: Toolbar<'_, u8, ()> = toolbar(&state)
            .tooltip(None)
            .more_label("Overflow")
            .menu_keymap(Keymap::new())
            .on_event(|_| ());
        assert_eq!(bar.tooltip, None);
        assert_eq!(bar.more, "Overflow");
        assert!(bar.menu_keymap.is_empty());
        assert!(bar.on_event.is_some());
    }

    #[test]
    fn ring_and_strip_styles_come_from_tokens() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let ring = ring_style(&tokens);
            assert_eq!(ring.border.color, tokens.ring);
            assert_eq!(ring.border.width, RING_WIDTH);
            assert_eq!(ring.border.radius, radius::MD.into());
            assert_eq!(ring.background, None);
            let bar = bar_style(&tokens);
            assert_eq!(bar.background, Some(Background::Color(tokens.background)));
            assert_eq!(bar.text_color, Some(tokens.foreground));
        }
    }
}
