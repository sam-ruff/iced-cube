//! A horizontal strip of top-level menus, such as File, Edit, View and
//! Help, along the top of a desktop window.
//!
//! Each menu is a [dropdown menu](mod@crate::overlay::dropdown_menu): the same
//! entries, rows, checkbox and radio items, submenus and shortcut hints.
//! [`State`] holds the menus, which one is open and which title has the
//! keyboard focus. Once a menu is open, moving the pointer onto another
//! title switches to that menu.
//!
//! The bar resolves its own keys while a menu is open: Left and Right move
//! between menus (opening and closing submenus first), and the rest go to
//! the open menu. While the bar is closed, the app routes key presses from
//! [`keys::subscription`] through [`State::key_event`]: F10 or Alt focuses
//! the bar, Left and Right move along it, Down opens the focused menu,
//! Escape leaves it, and Alt with a menu's mnemonic letter opens that menu.
//!
//! ```no_run
//! use iced::{Element, Subscription};
//! use iced_cube::dropdown_menu::{item, separator};
//! use iced_cube::keys;
//! use iced_cube::menubar::{self, Output, menu, menubar};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Menubar(menubar::Event<&'static str>),
//!     Key(keys::Event),
//! }
//!
//! struct App {
//!     menubar: menubar::State<&'static str>,
//! }
//!
//! impl App {
//!     fn new() -> Self {
//!         let menubar = menubar::State::new([
//!             menu("File", [item("new", "New").shortcut("Ctrl+N"), separator(), item("quit", "Quit")]),
//!             menu("Help", [item("about", "About")]),
//!         ]);
//!         Self { menubar }
//!     }
//!
//!     fn update(&mut self, message: Message) {
//!         let event = match message {
//!             Message::Menubar(event) => Some(event),
//!             Message::Key(key) => self.menubar.key_event(
//!                 &menubar::default_keymap(),
//!                 &iced_cube::dropdown_menu::default_keymap(),
//!                 &key,
//!             ),
//!         };
//!         if let Some(Output::Activated(id)) = event.and_then(|event| self.menubar.update(event)) {
//!             println!("Run {id}");
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         menubar(&self.menubar).on_event(Message::Menubar).into()
//!     }
//!
//!     fn subscription(&self) -> Subscription<Message> {
//!         keys::subscription().map(Message::Key)
//!     }
//! }
//! ```

use std::rc::Rc;

use iced::keyboard::{Key, Modifiers, key::Named};
use iced::widget::text::{LineHeight, Span};
use iced::widget::{self, column, container, mouse_area, rich_text, row, span, text};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow};

use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{Align, Placement, Side, anchored};
use crate::overlay::dropdown_menu::{self, Entry, Kind};
use crate::overlay::menu::LINE_HEIGHT;
use crate::theme::{Tokens, radius, space, text_size};

pub use crate::overlay::dropdown_menu::Output;

/// Padding inside a menu title: 4 pixels above and below, 8 at the sides.
pub const TRIGGER_PADDING: Padding = Padding {
    top: 4.0,
    bottom: 4.0,
    left: space::SM,
    right: space::SM,
};

/// One top-level menu: its title and its entries.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu<Id> {
    title: String,
    mnemonic: Option<char>,
    state: dropdown_menu::State<Id>,
}

/// A menu titled `title` holding dropdown menu `entries`. Its mnemonic is
/// the first letter of the title unless [`Menu::mnemonic`] sets another.
pub fn menu<Id: Copy + PartialEq>(
    title: impl Into<String>,
    entries: impl IntoIterator<Item = Entry<Id>>,
) -> Menu<Id> {
    Menu {
        title: title.into(),
        mnemonic: None,
        state: dropdown_menu::State::new(entries),
    }
}

impl<Id> Menu<Id> {
    /// The letter that opens this menu with Alt, or on its own while the
    /// bar has focus. It is underlined in the title while the bar has focus.
    pub fn mnemonic(mut self, letter: char) -> Self {
        self.mnemonic = Some(letter);
        self
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// The menu's entries and whether it is open.
    pub fn state(&self) -> &dropdown_menu::State<Id> {
        &self.state
    }

    /// The mnemonic in lower case: the one set, or the title's first letter.
    pub fn key(&self) -> Option<char> {
        self.mnemonic
            .or_else(|| self.title.chars().find(|c| c.is_alphanumeric()))
            .and_then(|c| c.to_lowercase().next())
    }
}

/// Changes to a menubar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event<Id> {
    /// A title was pressed: opens its menu, or closes it when it is open.
    Toggle(usize),
    /// The pointer moved onto a title. Switches to its menu while another
    /// menu is open, and does nothing otherwise.
    Hover(usize),
    /// An event for the menu at this position.
    Menu(usize, dropdown_menu::Event<Id>),
    /// Focuses the first title, or leaves the bar when it has focus.
    Focus,
    /// Moves to the next title, wrapping, and opens its menu if one is open.
    Next,
    /// Moves to the previous title, wrapping, and opens its menu if one is
    /// open.
    Previous,
    /// Opens the focused title's menu on its first item.
    Open,
    /// Opens the menu at this position on its first item, as its mnemonic
    /// does.
    OpenMenu(usize),
    /// Closes the open menu and keeps the focus on its title, or leaves the
    /// bar when no menu is open.
    Close,
    /// Closes everything and leaves the bar, as a press outside does.
    Dismiss,
}

/// The menus, which one is open and which title has the keyboard focus.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    menus: Vec<Menu<Id>>,
    focused: Option<usize>,
    keyboard: bool,
}

impl<Id: Copy + PartialEq> State<Id> {
    /// A closed menubar. Item ids must be unique across every menu.
    pub fn new(menus: impl IntoIterator<Item = Menu<Id>>) -> Self {
        Self {
            menus: menus.into_iter().collect(),
            focused: None,
            keyboard: false,
        }
    }

    /// Whether the bar was last used from the keyboard, which underlines
    /// the mnemonics while it has focus.
    pub fn shows_mnemonics(&self) -> bool {
        self.keyboard && self.is_focused()
    }

    pub fn menus(&self) -> &[Menu<Id>] {
        &self.menus
    }

    /// The position of the open menu.
    pub fn open(&self) -> Option<usize> {
        self.menus.iter().position(|menu| menu.state.is_open())
    }

    pub fn is_open(&self) -> bool {
        self.open().is_some()
    }

    /// The title with the keyboard focus. The open menu's title keeps it.
    pub fn focused(&self) -> Option<usize> {
        self.focused
    }

    /// Whether the bar takes the keys that move along it.
    pub fn is_focused(&self) -> bool {
        self.focused.is_some()
    }

    /// The highlighted item in the open menu.
    pub fn highlighted(&self) -> Option<Id> {
        self.menus.iter().find_map(|menu| menu.state.highlighted())
    }

    /// Whether a checkbox or radio item in any menu is checked.
    pub fn is_checked(&self, id: Id) -> bool {
        self.menus.iter().any(|menu| menu.state.is_checked(id))
    }

    /// Checks or unchecks an item, in whichever menu holds it.
    pub fn set_checked(&mut self, id: Id, checked: bool) {
        for menu in &mut self.menus {
            if menu.state.item(id).is_some() {
                menu.state.set_checked(id, checked);
            }
        }
    }

    /// Enables or disables an item, in whichever menu holds it.
    pub fn set_disabled(&mut self, id: Id, disabled: bool) {
        for menu in &mut self.menus {
            if menu.state.item(id).is_some() {
                menu.state.set_disabled(id, disabled);
            }
        }
    }

    /// Applies an event and returns what was chosen, if anything. Choosing
    /// an item closes the menu and leaves the bar.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Toggle(_) | Event::Hover(_) => self.keyboard = false,
            Event::Focus | Event::Next | Event::Previous | Event::Open | Event::OpenMenu(_) => {
                self.keyboard = true;
            }
            Event::Menu(..) | Event::Close | Event::Dismiss => {}
        }
        match event {
            Event::Toggle(index) if self.open() == Some(index) => self.dismiss(),
            Event::Toggle(index) => self.switch(index, dropdown_menu::Event::Open),
            Event::Hover(index) => {
                if self.open().is_some_and(|open| open != index) {
                    self.switch(index, dropdown_menu::Event::Open);
                }
            }
            Event::Menu(_, dropdown_menu::Event::Close) => self.close(),
            Event::Menu(index, event) => return self.forward(index, event),
            Event::Focus if self.is_open() || self.is_focused() => self.dismiss(),
            Event::Focus => self.focused = (!self.menus.is_empty()).then_some(0),
            Event::Next => self.step(true),
            Event::Previous => self.step(false),
            Event::Open => {
                if let Some(index) = self.focused {
                    self.switch(index, dropdown_menu::Event::First);
                }
            }
            Event::OpenMenu(index) => self.switch(index, dropdown_menu::Event::First),
            Event::Close => self.close(),
            Event::Dismiss => self.dismiss(),
        }
        None
    }

    fn forward(&mut self, index: usize, event: dropdown_menu::Event<Id>) -> Option<Output<Id>> {
        let menu = self.menus.get_mut(index)?;
        let output = menu.state.update(event);
        if output.is_some() {
            self.dismiss();
            return output;
        }
        if menu.state.is_open() {
            self.close_others(index);
            self.focused = Some(index);
        }
        None
    }

    /// Opens the menu at `index` with `event`, closing any other.
    fn switch(&mut self, index: usize, event: dropdown_menu::Event<Id>) {
        let Some(menu) = self.menus.get_mut(index) else {
            return;
        };
        let _ = menu.state.update(dropdown_menu::Event::Close);
        let _ = menu.state.update(event);
        self.close_others(index);
        self.focused = Some(index);
    }

    fn close_others(&mut self, keep: usize) {
        for (index, menu) in self.menus.iter_mut().enumerate() {
            if index != keep {
                let _ = menu.state.update(dropdown_menu::Event::Close);
            }
        }
    }

    fn close_all(&mut self) {
        for menu in &mut self.menus {
            let _ = menu.state.update(dropdown_menu::Event::Close);
        }
    }

    fn close(&mut self) {
        if self.is_open() {
            self.close_all();
        } else {
            self.focused = None;
        }
    }

    fn dismiss(&mut self) {
        self.close_all();
        self.focused = None;
    }

    fn step(&mut self, forward: bool) {
        let len = self.menus.len();
        let Some(current) = self.open().or(self.focused) else {
            return;
        };
        if len == 0 {
            return;
        }
        let target = if forward {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        if self.is_open() {
            self.switch(target, dropdown_menu::Event::First);
        } else {
            self.focused = Some(target);
        }
    }

    /// Turns a key press into an event. While a menu is open, the bar's
    /// keymap goes first (Left and Right open and close submenus before
    /// they move between menus), then `menu_keymap` and typeahead. While
    /// closed, the bar claims [`Action::Focus`], its mnemonics with Alt,
    /// and, only while it has focus, its other actions and bare mnemonics.
    pub fn key_event(
        &self,
        keymap: &Keymap<Action>,
        menu_keymap: &Keymap<dropdown_menu::Action>,
        key: &keys::Event,
    ) -> Option<Event<Id>> {
        let action = resolve(keymap, key);
        if let Some(open) = self.open() {
            if let Some(event) = action.and_then(|action| action.event(self)) {
                return Some(event);
            }
            let menu = self.menus.get(open)?;
            return menu
                .state
                .key_event(menu_keymap, key)
                .map(|event| Event::Menu(open, event));
        }
        if let Some(event) = action.and_then(|action| action.event(self)) {
            return Some(event);
        }
        self.mnemonic(key).map(Event::OpenMenu)
    }

    /// The menu whose mnemonic a key press types: with Alt at any time, or
    /// on its own while the bar has focus.
    fn mnemonic(&self, key: &keys::Event) -> Option<usize> {
        let alone = key.modifiers.is_empty() || key.modifiers == Modifiers::SHIFT;
        let wanted = key.modifiers == Modifiers::ALT || (alone && self.is_focused());
        if !wanted {
            return None;
        }
        let Key::Character(typed) = &key.key else {
            return None;
        };
        let typed = typed.chars().next()?.to_lowercase().next()?;
        self.menus.iter().position(|menu| menu.key() == Some(typed))
    }

    /// Whether the deepest highlighted item of the open menu opens a submenu.
    fn on_submenu(&self) -> bool {
        let Some(id) = self.highlighted() else {
            return false;
        };
        self.menus.iter().any(|menu| {
            menu.state
                .item(id)
                .is_some_and(|item| !item.disabled && matches!(item.kind, Kind::Submenu(_)))
        })
    }

    fn open_depth(&self) -> usize {
        self.open()
            .and_then(|open| self.menus.get(open))
            .map_or(0, |menu| menu.state.depth())
    }
}

/// Resolves a key press through a keymap. A bare modifier key, such as
/// Alt, arrives with its own modifier held, so that modifier is ignored.
fn resolve<A: Clone>(keymap: &Keymap<A>, key: &keys::Event) -> Option<A> {
    let modifiers = match key.key {
        Key::Named(Named::Alt) => key.modifiers.difference(Modifiers::ALT),
        Key::Named(Named::Control) => key.modifiers.difference(Modifiers::CTRL),
        Key::Named(Named::Shift) => key.modifiers.difference(Modifiers::SHIFT),
        _ => key.modifiers,
    };
    keymap.resolve(&key.key, modifiers)
}

/// What a menubar keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Focuses the bar, or leaves it.
    Focus,
    NextMenu,
    PreviousMenu,
    /// Opens the focused title's menu.
    Open,
    Close,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing, so the key can go elsewhere: while a menu is open, Open is
    /// left to the menu and Left and Right work its submenus first; while
    /// the bar is closed and unfocused, only Focus does anything.
    pub fn event<Id: Copy + PartialEq>(self, state: &State<Id>) -> Option<Event<Id>> {
        if let Some(open) = state.open() {
            return match self {
                Action::Focus => Some(Event::Dismiss),
                Action::NextMenu if state.on_submenu() => {
                    Some(Event::Menu(open, dropdown_menu::Event::OpenSubmenu))
                }
                Action::NextMenu => Some(Event::Next),
                Action::PreviousMenu if state.open_depth() > 1 => {
                    Some(Event::Menu(open, dropdown_menu::Event::CloseSubmenu))
                }
                Action::PreviousMenu => Some(Event::Previous),
                Action::Close if state.open_depth() > 1 => {
                    Some(Event::Menu(open, dropdown_menu::Event::CloseSubmenu))
                }
                Action::Close => Some(Event::Close),
                Action::Open => None,
            };
        }
        if !state.is_focused() {
            return (self == Action::Focus).then_some(Event::Focus);
        }
        Some(match self {
            Action::Focus => Event::Focus,
            Action::NextMenu => Event::Next,
            Action::PreviousMenu => Event::Previous,
            Action::Open => Event::Open,
            Action::Close => Event::Close,
        })
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Focus,
        Action::NextMenu,
        Action::PreviousMenu,
        Action::Open,
        Action::Close,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Focus => "Focus",
            Action::NextMenu => "NextMenu",
            Action::PreviousMenu => "PreviousMenu",
            Action::Open => "Open",
            Action::Close => "Close",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Focus => "Focuses the first menu title, or leaves the bar and closes its menu.",
            Action::NextMenu => {
                "Moves to the next menu, or opens the highlighted submenu inside an open menu."
            }
            Action::PreviousMenu => {
                "Moves to the previous menu, or closes the open submenu inside an open menu."
            }
            Action::Open => "Opens the focused title's menu on its first item.",
            Action::Close => "Closes the open submenu or menu, or leaves the bar.",
        }
    }
}

/// The default menubar shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `F10`, `Alt` | [`Action::Focus`] |
/// | `ArrowRight` | [`Action::NextMenu`] |
/// | `ArrowLeft` | [`Action::PreviousMenu`] |
/// | `ArrowDown`, `Enter`, `Space` | [`Action::Open`] |
/// | `Escape` | [`Action::Close`] |
///
/// While a menu is open the bar resolves these itself, before the open
/// menu's own keymap. While the bar is closed, only Focus claims a key
/// unless the bar has focus, so the arrows stay free for the rest of the
/// app. Alt with a menu's mnemonic letter opens that menu at any time.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::F10), Action::Focus)
        .bind(Chord::named(Named::Alt), Action::Focus)
        .bind(Chord::named(Named::ArrowRight), Action::NextMenu)
        .bind(Chord::named(Named::ArrowLeft), Action::PreviousMenu)
        .bind(Chord::named(Named::ArrowDown), Action::Open)
        .bind(Chord::named(Named::Enter), Action::Open)
        .bind(Chord::named(Named::Space), Action::Open)
        .bind(Chord::named(Named::Escape), Action::Close)
}

/// How a menu title is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TriggerStatus {
    #[default]
    Idle,
    Hovered,
    /// Its menu is open, or it has the keyboard focus.
    Active,
    Disabled,
}

impl TriggerStatus {
    pub const ALL: [TriggerStatus; 4] = [
        TriggerStatus::Idle,
        TriggerStatus::Hovered,
        TriggerStatus::Active,
        TriggerStatus::Disabled,
    ];
}

/// The style of a menu title: transparent at rest, the accent under the
/// pointer, while open and while it has the keyboard focus.
pub fn trigger_style(tokens: &Tokens, status: TriggerStatus) -> widget::button::Style {
    let (background, text_color) = match status {
        TriggerStatus::Idle => (None, tokens.foreground),
        TriggerStatus::Hovered | TriggerStatus::Active => {
            (Some(tokens.accent), tokens.accent_foreground)
        }
        TriggerStatus::Disabled => (None, tokens.muted_foreground),
    };
    widget::button::Style {
        background: background.map(Background::Color),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius::SM.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// The style of the bar behind the titles.
pub fn bar_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.background)),
        text_color: Some(tokens.foreground),
        ..container::Style::default()
    }
}

fn trigger_status(status: widget::button::Status, active: bool) -> TriggerStatus {
    match status {
        widget::button::Status::Disabled => TriggerStatus::Disabled,
        _ if active => TriggerStatus::Active,
        widget::button::Status::Hovered | widget::button::Status::Pressed => TriggerStatus::Hovered,
        widget::button::Status::Active => TriggerStatus::Idle,
    }
}

type OnEvent<'a, Id, Message> = Rc<dyn Fn(Event<Id>) -> Message + 'a>;

/// A menubar builder. Convert it into an [`Element`] to render.
pub struct Menubar<'a, Id, Message> {
    state: &'a State<Id>,
    width: f32,
    keymap: Keymap<Action>,
    menu_keymap: Keymap<dropdown_menu::Action>,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for Menubar<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Menubar")
            .field("state", self.state)
            .field("width", &self.width)
            .field("keymap", &self.keymap)
            .field("menu_keymap", &self.menu_keymap)
            .finish_non_exhaustive()
    }
}

/// Renders the titles of `state`, with the open menu below its title.
/// Without [`on_event`](Menubar::on_event) every title renders disabled.
pub fn menubar<'a, Id, Message>(state: &'a State<Id>) -> Menubar<'a, Id, Message> {
    Menubar {
        state,
        width: dropdown_menu::WIDTH,
        keymap: default_keymap(),
        menu_keymap: dropdown_menu::default_keymap(),
        on_event: None,
    }
}

impl<'a, Id, Message> Menubar<'a, Id, Message> {
    /// Width of each menu and its submenus.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Replaces the [`default_keymap`] the bar resolves while a menu is open.
    pub fn keymap(mut self, keymap: Keymap<Action>) -> Self {
        self.keymap = keymap;
        self
    }

    /// Replaces the [dropdown menu keys](dropdown_menu::default_keymap) the
    /// open menu resolves after the bar's own.
    pub fn menu_keymap(mut self, keymap: Keymap<dropdown_menu::Action>) -> Self {
        self.menu_keymap = keymap;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Id, Message> From<Menubar<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(menubar: Menubar<'a, Id, Message>) -> Self {
        let Menubar {
            state,
            width,
            keymap,
            menu_keymap,
            on_event,
        } = menubar;
        let on_event: Option<OnEvent<'a, Id, Message>> = on_event.map(Rc::from);
        let keymaps = Rc::new((keymap, menu_keymap));

        let titles = state.menus.iter().enumerate().map(|(index, menu)| {
            title(state, index, menu, on_event.clone(), keymaps.clone(), width)
        });

        column![
            container(row(titles).spacing(2))
                .padding([space::XS, space::XS])
                .width(Length::Fill)
                .style(|theme| bar_style(&Tokens::of(theme))),
            crate::application::edge(),
        ]
        .width(Length::Fill)
        .into()
    }
}

/// One title with its menu floating below it while open.
fn title<'a, Id, Message>(
    state: &'a State<Id>,
    index: usize,
    menu: &'a Menu<Id>,
    on_event: Option<OnEvent<'a, Id, Message>>,
    keymaps: Rc<(Keymap<Action>, Keymap<dropdown_menu::Action>)>,
    width: f32,
) -> Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    let open = menu.state.is_open();
    let active = open || (!state.is_open() && state.focused == Some(index));
    let label = title_label(menu, state.shows_mnemonics());
    let trigger = widget::button(label)
        .padding(TRIGGER_PADDING)
        .style(move |theme, status| {
            trigger_style(&Tokens::of(theme), trigger_status(status, active))
        })
        .on_press_maybe(
            on_event
                .as_ref()
                .map(|on_event| on_event(Event::Toggle(index))),
        );

    let mut anchor = mouse_area(trigger);
    if let Some(on_event) = on_event.as_ref().filter(|_| state.is_open() && !open) {
        anchor = anchor.on_enter(on_event(Event::Hover(index)));
    }

    let on_menu = on_event
        .clone()
        .map(|on_event| move |event| on_event(Event::Menu(index, event)));
    let panel = open.then(|| {
        dropdown_menu::panel(
            &menu.state,
            on_menu
                .as_ref()
                .map(|f| f as &dyn Fn(dropdown_menu::Event<Id>) -> Message),
            width,
        )
    });
    let floating = anchored(anchor)
        .content(panel)
        .placement(Placement::new(Side::Bottom, Align::Start))
        .dismiss_keys([]);
    let Some(on_event) = on_event.filter(|_| open) else {
        return floating.into();
    };
    floating
        .on_dismiss(on_event(Event::Dismiss))
        .on_key(move |key| {
            let (keymap, menu_keymap) = &*keymaps;
            state.key_event(keymap, menu_keymap, key).map(&*on_event)
        })
        .into()
}

/// The title text, with its mnemonic underlined while the bar has focus.
fn title_label<'a, Message: 'a, Id>(menu: &Menu<Id>, underline: bool) -> Element<'a, Message> {
    let plain = || {
        text(menu.title.clone())
            .size(text_size::SM)
            .line_height(LineHeight::Absolute(LINE_HEIGHT.into()))
            .wrapping(text::Wrapping::None)
            .into()
    };
    if !underline {
        return plain();
    }
    let Some((before, letter, after)) = split_mnemonic(&menu.title, menu.key()) else {
        return plain();
    };
    let spans: Vec<Span<'a, ()>> = vec![span(before), span(letter).underline(true), span(after)];
    rich_text(spans)
        .size(text_size::SM)
        .line_height(LineHeight::Absolute(LINE_HEIGHT.into()))
        .wrapping(text::Wrapping::None)
        .into()
}

/// Splits a title around the first letter matching its mnemonic.
fn split_mnemonic(title: &str, key: Option<char>) -> Option<(String, String, String)> {
    let key = key?;
    let (start, letter) = title
        .char_indices()
        .find(|(_, c)| c.to_lowercase().next() == Some(key))?;
    let end = start + letter.len_utf8();
    Some((
        title.get(..start)?.to_owned(),
        title.get(start..end)?.to_owned(),
        title.get(end..)?.to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::dropdown_menu::{checkbox_item, item, separator, submenu};
    use crate::theme::{dark, light};

    const NEW: u8 = 1;
    const QUIT: u8 = 2;
    const UNDO: u8 = 3;
    const RECENT: u8 = 4;
    const LAST: u8 = 5;
    const GRID: u8 = 6;
    const ABOUT: u8 = 7;

    fn state() -> State<u8> {
        State::new([
            menu(
                "File",
                [
                    item(NEW, "New").shortcut("Ctrl+N"),
                    submenu(RECENT, "Open recent", [item(LAST, "notes.md")]),
                    separator(),
                    item(QUIT, "Quit"),
                ],
            ),
            menu("Edit", [item(UNDO, "Undo")]),
            menu("View", [checkbox_item(GRID, "Show grid", true)]),
            menu("Help", [item(ABOUT, "About")]).mnemonic('p'),
        ])
    }

    fn key(key: Key, modifiers: Modifiers) -> keys::Event {
        keys::Event { key, modifiers }
    }

    fn named(named: Named) -> keys::Event {
        key(Key::Named(named), Modifiers::empty())
    }

    fn resolve_key(state: &State<u8>, event: &keys::Event) -> Option<Event<u8>> {
        state.key_event(&default_keymap(), &dropdown_menu::default_keymap(), event)
    }

    #[test]
    fn starts_closed_and_unfocused() {
        let state = state();
        assert_eq!(state.open(), None);
        assert!(!state.is_open() && !state.is_focused());
        assert_eq!(state.menus().len(), 4);
        assert_eq!(state.menus()[0].title(), "File");
    }

    #[test]
    fn mnemonics_default_to_the_first_letter() {
        let state = state();
        let keys: Vec<_> = state.menus().iter().map(Menu::key).collect();
        assert_eq!(keys, [Some('f'), Some('e'), Some('v'), Some('p')]);
        let empty: Menu<u8> = menu("", []);
        assert_eq!(empty.key(), None);
    }

    #[test]
    fn toggle_opens_and_closes_a_menu() {
        let mut state = state();
        let _ = state.update(Event::Toggle(1));
        assert_eq!(state.open(), Some(1));
        assert_eq!(state.focused(), Some(1));
        assert_eq!(
            state.highlighted(),
            None,
            "the pointer opens without a highlight"
        );
        let _ = state.update(Event::Toggle(1));
        assert!(!state.is_open() && !state.is_focused());
    }

    #[test]
    fn toggling_another_title_switches_menus() {
        let mut state = state();
        let _ = state.update(Event::Toggle(0));
        let _ = state.update(Event::Toggle(2));
        assert_eq!(state.open(), Some(2));
        assert!(!state.menus()[0].state().is_open());
    }

    #[test]
    fn hovering_switches_only_while_a_menu_is_open() {
        let mut state = state();
        let _ = state.update(Event::Hover(1));
        assert!(!state.is_open(), "hover alone opens nothing");

        let _ = state.update(Event::Toggle(0));
        let _ = state.update(Event::Hover(1));
        assert_eq!(state.open(), Some(1));
        let _ = state.update(Event::Hover(1));
        assert_eq!(state.open(), Some(1));
        let _ = state.update(Event::Hover(9));
        assert_eq!(state.open(), Some(1), "unknown titles are ignored");
    }

    #[test]
    fn choosing_an_item_returns_it_and_leaves_the_bar() {
        let mut state = state();
        let _ = state.update(Event::Toggle(0));
        assert_eq!(
            state.update(Event::Menu(0, dropdown_menu::Event::Activate(NEW))),
            Some(Output::Activated(NEW))
        );
        assert!(!state.is_open() && !state.is_focused());

        let _ = state.update(Event::Toggle(2));
        assert_eq!(
            state.update(Event::Menu(2, dropdown_menu::Event::Activate(GRID))),
            Some(Output::Toggled(GRID, false))
        );
        assert!(!state.is_checked(GRID));
    }

    #[test]
    fn menu_events_for_unknown_menus_do_nothing() {
        let mut state = state();
        assert_eq!(
            state.update(Event::Menu(9, dropdown_menu::Event::Open)),
            None
        );
        assert!(!state.is_open());
    }

    #[test]
    fn a_menu_opened_through_its_own_events_closes_the_others() {
        let mut state = state();
        let _ = state.update(Event::Toggle(0));
        let _ = state.update(Event::Menu(1, dropdown_menu::Event::Next));
        assert_eq!(state.open(), Some(1));
        assert_eq!(state.focused(), Some(1));
        assert!(!state.menus()[0].state().is_open());
    }

    #[test]
    fn focus_toggles_the_bar() {
        let mut state = state();
        let _ = state.update(Event::Focus);
        assert_eq!(state.focused(), Some(0));
        assert!(!state.is_open());
        let _ = state.update(Event::Focus);
        assert!(!state.is_focused());

        let mut empty: State<u8> = State::new([]);
        let _ = empty.update(Event::Focus);
        assert!(!empty.is_focused());
        let _ = empty.update(Event::Next);
        assert!(!empty.is_focused());
    }

    #[test]
    fn mnemonics_show_only_while_the_keyboard_drives_the_bar() {
        let mut state = state();
        assert!(!state.shows_mnemonics());
        let _ = state.update(Event::Focus);
        assert!(state.shows_mnemonics());
        let _ = state.update(Event::Open);
        assert!(state.shows_mnemonics());
        let _ = state.update(Event::Hover(2));
        assert!(!state.shows_mnemonics(), "the pointer takes over");
        let _ = state.update(Event::Dismiss);
        let _ = state.update(Event::Toggle(0));
        assert!(!state.shows_mnemonics());
    }

    #[test]
    fn next_and_previous_move_the_focus_and_wrap() {
        let mut state = state();
        let _ = state.update(Event::Next);
        assert!(!state.is_focused(), "an unfocused bar does not move");
        let _ = state.update(Event::Focus);
        let _ = state.update(Event::Previous);
        assert_eq!(state.focused(), Some(3));
        let _ = state.update(Event::Next);
        assert_eq!(state.focused(), Some(0));
        assert!(!state.is_open());
    }

    #[test]
    fn next_with_a_menu_open_opens_the_next_menu_on_its_first_item() {
        let mut state = state();
        let _ = state.update(Event::Toggle(0));
        let _ = state.update(Event::Next);
        assert_eq!(state.open(), Some(1));
        assert_eq!(state.highlighted(), Some(UNDO));
        let _ = state.update(Event::Previous);
        let _ = state.update(Event::Previous);
        assert_eq!(state.open(), Some(3));
        assert_eq!(state.highlighted(), Some(ABOUT));
    }

    #[test]
    fn open_opens_the_focused_menu_on_its_first_item() {
        let mut state = state();
        let _ = state.update(Event::Open);
        assert!(!state.is_open(), "nothing is focused");
        let _ = state.update(Event::Focus);
        let _ = state.update(Event::Next);
        let _ = state.update(Event::Open);
        assert_eq!(state.open(), Some(1));
        assert_eq!(state.highlighted(), Some(UNDO));
    }

    #[test]
    fn open_menu_opens_by_position() {
        let mut state = state();
        let _ = state.update(Event::OpenMenu(2));
        assert_eq!(state.open(), Some(2));
        assert_eq!(state.highlighted(), Some(GRID));
        let _ = state.update(Event::OpenMenu(7));
        assert_eq!(state.open(), Some(2));
    }

    #[test]
    fn close_returns_to_the_title_then_leaves() {
        let mut state = state();
        let _ = state.update(Event::OpenMenu(1));
        let _ = state.update(Event::Close);
        assert!(!state.is_open());
        assert_eq!(state.focused(), Some(1));
        let _ = state.update(Event::Close);
        assert!(!state.is_focused());

        let _ = state.update(Event::OpenMenu(1));
        let _ = state.update(Event::Menu(1, dropdown_menu::Event::Close));
        assert_eq!(state.focused(), Some(1), "a menu's own Close acts the same");
    }

    #[test]
    fn dismiss_closes_everything() {
        let mut state = state();
        let _ = state.update(Event::OpenMenu(1));
        let _ = state.update(Event::Dismiss);
        assert!(!state.is_open() && !state.is_focused());
    }

    #[test]
    fn checked_and_disabled_items_are_found_in_any_menu() {
        let mut state = state();
        assert!(state.is_checked(GRID));
        state.set_checked(GRID, false);
        assert!(!state.is_checked(GRID));
        state.set_disabled(UNDO, true);
        assert!(
            state.menus()[1]
                .state()
                .item(UNDO)
                .is_some_and(|item| item.disabled)
        );
        let _ = state.update(Event::OpenMenu(1));
        assert_eq!(state.highlighted(), None);
    }

    #[test]
    fn a_closed_unfocused_bar_claims_only_focus_and_alt_mnemonics() {
        let state = state();
        for named in [
            Named::ArrowDown,
            Named::ArrowLeft,
            Named::ArrowRight,
            Named::Escape,
        ] {
            assert_eq!(resolve_key(&state, &self::named(named)), None, "{named:?}");
        }
        assert_eq!(
            resolve_key(&state, &self::named(Named::F10)),
            Some(Event::Focus)
        );
        assert_eq!(
            resolve_key(&state, &key(Key::Named(Named::Alt), Modifiers::ALT)),
            Some(Event::Focus),
            "Alt arrives with its own modifier held"
        );
        assert_eq!(
            resolve_key(&state, &key(Key::Named(Named::Alt), Modifiers::empty())),
            Some(Event::Focus)
        );
        assert_eq!(
            resolve_key(&state, &key(Key::Character("e".into()), Modifiers::ALT)),
            Some(Event::OpenMenu(1))
        );
        assert_eq!(
            resolve_key(&state, &key(Key::Character("P".into()), Modifiers::ALT)),
            Some(Event::OpenMenu(3))
        );
        assert_eq!(
            resolve_key(&state, &key(Key::Character("e".into()), Modifiers::empty())),
            None,
            "bare letters need a focused bar"
        );
        assert_eq!(
            resolve_key(&state, &key(Key::Character("z".into()), Modifiers::ALT)),
            None,
            "letters without a menu stay free"
        );
    }

    #[test]
    fn a_focused_bar_moves_opens_and_takes_bare_mnemonics() {
        let mut state = state();
        let _ = state.update(Event::Focus);
        assert_eq!(
            resolve_key(&state, &named(Named::ArrowRight)),
            Some(Event::Next)
        );
        assert_eq!(
            resolve_key(&state, &named(Named::ArrowLeft)),
            Some(Event::Previous)
        );
        assert_eq!(
            resolve_key(&state, &named(Named::ArrowDown)),
            Some(Event::Open)
        );
        assert_eq!(resolve_key(&state, &named(Named::Enter)), Some(Event::Open));
        assert_eq!(
            resolve_key(&state, &named(Named::Escape)),
            Some(Event::Close)
        );
        assert_eq!(resolve_key(&state, &named(Named::F10)), Some(Event::Focus));
        assert_eq!(
            resolve_key(&state, &key(Key::Character("v".into()), Modifiers::empty())),
            Some(Event::OpenMenu(2))
        );
        assert_eq!(resolve_key(&state, &named(Named::Tab)), None);
    }

    #[test]
    fn an_open_menu_sends_left_and_right_to_submenus_first() {
        let mut state = state();
        let _ = state.update(Event::OpenMenu(0));
        assert_eq!(state.highlighted(), Some(NEW));
        assert_eq!(
            resolve_key(&state, &named(Named::ArrowRight)),
            Some(Event::Next)
        );
        assert_eq!(
            resolve_key(&state, &named(Named::ArrowLeft)),
            Some(Event::Previous)
        );

        let _ = state.update(Event::Menu(0, dropdown_menu::Event::Next));
        assert_eq!(state.highlighted(), Some(RECENT));
        let right = resolve_key(&state, &named(Named::ArrowRight));
        assert_eq!(
            right,
            Some(Event::Menu(0, dropdown_menu::Event::OpenSubmenu))
        );
        let _ = state.update(Event::Menu(0, dropdown_menu::Event::OpenSubmenu));
        assert_eq!(state.highlighted(), Some(LAST));
        assert_eq!(
            resolve_key(&state, &named(Named::ArrowLeft)),
            Some(Event::Menu(0, dropdown_menu::Event::CloseSubmenu))
        );
        assert_eq!(
            resolve_key(&state, &named(Named::Escape)),
            Some(Event::Menu(0, dropdown_menu::Event::CloseSubmenu))
        );
    }

    #[test]
    fn an_open_menu_leaves_the_rest_to_the_menu_keymap() {
        let mut state = state();
        let _ = state.update(Event::OpenMenu(0));
        assert_eq!(
            resolve_key(&state, &named(Named::ArrowDown)),
            Some(Event::Menu(0, dropdown_menu::Event::Next))
        );
        assert_eq!(
            resolve_key(&state, &named(Named::Enter)),
            Some(Event::Menu(0, dropdown_menu::Event::ActivateHighlighted))
        );
        assert_eq!(
            resolve_key(&state, &key(Key::Character("q".into()), Modifiers::empty())),
            Some(Event::Menu(0, dropdown_menu::Event::Typeahead('q')))
        );
        assert_eq!(
            resolve_key(&state, &named(Named::Escape)),
            Some(Event::Close)
        );
        assert_eq!(
            resolve_key(&state, &named(Named::F10)),
            Some(Event::Dismiss)
        );
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "F10"), Some(Action::Focus));
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::NextMenu));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::PreviousMenu));
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Open));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Open));
        assert_eq!(press(&keymap, "Space"), Some(Action::Open));
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));
        assert_eq!(
            keymap.resolve(&Key::Named(Named::Alt), Modifiers::empty()),
            Some(Action::Focus)
        );
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&Chord::named(Named::Alt))
            .bind("Ctrl+M".parse().unwrap(), Action::Focus);
        let state = state();
        let alt = key(Key::Named(Named::Alt), Modifiers::ALT);
        let menus = dropdown_menu::default_keymap();
        assert_eq!(state.key_event(&keymap, &menus, &alt), None);
        let ctrl_m = key(Key::Character("m".into()), Modifiers::CTRL);
        assert_eq!(
            state.key_event(&keymap, &menus, &ctrl_m),
            Some(Event::Focus)
        );
        assert_eq!(
            state.key_event(&Keymap::new(), &menus, &named(Named::F10)),
            None
        );
    }

    #[test]
    fn a_closed_unfocused_bar_maps_only_focus() {
        let state = state();
        assert_eq!(Action::Focus.event(&state), Some(Event::Focus));
        for &action in <Action as keys::Action>::ALL {
            if action != Action::Focus {
                assert_eq!(action.event(&state), None, "{action:?}");
            }
        }
    }

    #[test]
    fn an_open_bar_leaves_open_to_the_menu() {
        let mut state = state();
        let _ = state.update(Event::OpenMenu(1));
        assert_eq!(Action::Open.event(&state), None);
        assert_eq!(Action::Focus.event(&state), Some(Event::Dismiss));
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
    fn builder_defaults_and_overrides() {
        let state = state();
        let bar: Menubar<'_, u8, ()> = menubar(&state);
        assert_eq!(bar.width, dropdown_menu::WIDTH);
        assert_eq!(bar.keymap, default_keymap());
        assert_eq!(bar.menu_keymap, dropdown_menu::default_keymap());
        assert!(bar.on_event.is_none());

        let bar: Menubar<'_, u8, ()> = menubar(&state)
            .width(300.0)
            .keymap(Keymap::new())
            .menu_keymap(Keymap::new())
            .on_event(|_| ());
        assert_eq!(bar.width, 300.0);
        assert!(bar.keymap.is_empty() && bar.menu_keymap.is_empty());
        assert!(bar.on_event.is_some());
    }

    #[test]
    fn titles_use_the_accent_while_hovered_or_active() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = trigger_style(&tokens, TriggerStatus::Idle);
            assert_eq!(idle.background, None);
            assert_eq!(idle.text_color, tokens.foreground);
            for status in [TriggerStatus::Hovered, TriggerStatus::Active] {
                let style = trigger_style(&tokens, status);
                assert_eq!(style.background, Some(Background::Color(tokens.accent)));
                assert_eq!(style.text_color, tokens.accent_foreground);
                assert_eq!(style.border.radius, radius::SM.into());
            }
            let disabled = trigger_style(&tokens, TriggerStatus::Disabled);
            assert_eq!(disabled.text_color, tokens.muted_foreground);
            assert_eq!(disabled.background, None);
            assert_eq!(
                bar_style(&tokens).background,
                Some(Background::Color(tokens.background))
            );
        }
    }

    #[test]
    fn an_active_title_stays_active_under_the_pointer() {
        use widget::button::Status;
        assert_eq!(trigger_status(Status::Hovered, true), TriggerStatus::Active);
        assert_eq!(
            trigger_status(Status::Hovered, false),
            TriggerStatus::Hovered
        );
        assert_eq!(
            trigger_status(Status::Pressed, false),
            TriggerStatus::Hovered
        );
        assert_eq!(trigger_status(Status::Active, false), TriggerStatus::Idle);
        assert_eq!(
            trigger_status(Status::Disabled, true),
            TriggerStatus::Disabled
        );
    }

    #[test]
    fn mnemonics_split_the_title_around_the_first_match() {
        assert_eq!(
            split_mnemonic("Help", Some('p')),
            Some(("Hel".into(), "p".into(), String::new()))
        );
        assert_eq!(
            split_mnemonic("File", Some('f')),
            Some((String::new(), "F".into(), "ile".into()))
        );
        assert_eq!(split_mnemonic("File", Some('x')), None);
        assert_eq!(split_mnemonic("File", None), None);
    }
}
