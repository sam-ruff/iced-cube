//! A menu that opens at the pointer on a right-click inside an area.
//!
//! It uses the dropdown menu's entries, rows and navigation: build the
//! entries with [`dropdown_menu::item`](crate::overlay::dropdown_menu::item)
//! and friends. [`State`] adds where the menu opened. From the keyboard,
//! Shift+F10 or the Menu key opens it at the area's top left corner, as
//! desktop apps do.
//!
//! Keyboard shortcuts resolve through a [`Keymap`] of [`Action`]s; see
//! [`default_keymap`], [`State::key_event`] and [`crate::keys`].

use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::key::Named;
use iced::{
    Element, Event as IcedEvent, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
};

use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{Align, Placement, Side, anchored};
use crate::overlay::dropdown_menu::{self, Entry, Output, WIDTH};

/// Changes to a context menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event<Id> {
    /// A right-click at this point, relative to the area's top left corner.
    /// Opens the menu there with nothing highlighted.
    Open(Point),
    /// Opens the menu at the area's top left corner on its first item.
    OpenFromKeyboard,
    /// Navigation and choosing, as in a dropdown menu. Ignored while closed.
    Menu(dropdown_menu::Event<Id>),
}

/// The menu and where it opened.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id> {
    menu: dropdown_menu::State<Id>,
    position: Point,
}

impl<Id: Copy + PartialEq> State<Id> {
    /// A closed menu. Item ids must be unique across submenus.
    pub fn new(entries: impl IntoIterator<Item = Entry<Id>>) -> Self {
        Self {
            menu: dropdown_menu::State::new(entries),
            position: Point::ORIGIN,
        }
    }

    /// The menu entries, highlight and open submenus.
    pub fn menu(&self) -> &dropdown_menu::State<Id> {
        &self.menu
    }

    pub fn is_open(&self) -> bool {
        self.menu.is_open()
    }

    /// Where the menu opened, relative to the area's top left corner.
    pub fn position(&self) -> Point {
        self.position
    }

    pub fn is_checked(&self, id: Id) -> bool {
        self.menu.is_checked(id)
    }

    /// Checks or unchecks an item. Checking a radio item unchecks its group.
    pub fn set_checked(&mut self, id: Id, checked: bool) {
        self.menu.set_checked(id, checked);
    }

    pub fn set_disabled(&mut self, id: Id, disabled: bool) {
        self.menu.set_disabled(id, disabled);
    }

    /// Applies an event and returns what was chosen, if anything.
    pub fn update(&mut self, event: Event<Id>) -> Option<Output<Id>> {
        match event {
            Event::Open(position) => {
                self.position = position;
                let _ = self.menu.update(dropdown_menu::Event::Close);
                self.menu.update(dropdown_menu::Event::Open)
            }
            Event::OpenFromKeyboard => {
                self.position = Point::ORIGIN;
                let _ = self.menu.update(dropdown_menu::Event::Close);
                self.menu.update(dropdown_menu::Event::First)
            }
            Event::Menu(_) if !self.is_open() => None,
            Event::Menu(event) => self.menu.update(event),
        }
    }

    /// Turns a key press into an event: first through `keymap`, then, while
    /// the menu is open, as typeahead for a letter or digit.
    pub fn key_event(&self, keymap: &Keymap<Action>, key: &keys::Event) -> Option<Event<Id>> {
        if let Some(action) = keymap.resolve_event(key) {
            return action.event(self);
        }
        if !self.is_open() {
            return None;
        }
        dropdown_menu::typeahead(key)
            .map(|character| Event::Menu(dropdown_menu::Event::Typeahead(character)))
    }
}

/// What a context menu keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
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
    /// The dropdown menu action this one drives, if it is not [`Action::Open`].
    pub fn menu_action(self) -> Option<dropdown_menu::Action> {
        use dropdown_menu::Action as Menu;
        match self {
            Action::Open => None,
            Action::Next => Some(Menu::Next),
            Action::Previous => Some(Menu::Previous),
            Action::First => Some(Menu::First),
            Action::Last => Some(Menu::Last),
            Action::Activate => Some(Menu::Activate),
            Action::Close => Some(Menu::Close),
            Action::OpenSubmenu => Some(Menu::OpenSubmenu),
            Action::CloseSubmenu => Some(Menu::CloseSubmenu),
        }
    }

    /// The [`Event`] this action sends to `state`, or `None` when it does
    /// nothing: only [`Action::Open`] works while the menu is closed, and
    /// only the others while it is open.
    pub fn event<Id: Copy + PartialEq>(self, state: &State<Id>) -> Option<Event<Id>> {
        let open = state.is_open();
        match self.menu_action() {
            None => (!open).then_some(Event::OpenFromKeyboard),
            Some(_) if !open => None,
            Some(action) => action.event(state.menu()).map(Event::Menu),
        }
    }
}

impl From<dropdown_menu::Action> for Action {
    fn from(action: dropdown_menu::Action) -> Self {
        use dropdown_menu::Action as Menu;
        match action {
            Menu::Next => Action::Next,
            Menu::Previous => Action::Previous,
            Menu::First => Action::First,
            Menu::Last => Action::Last,
            Menu::Activate => Action::Activate,
            Menu::Close => Action::Close,
            Menu::OpenSubmenu => Action::OpenSubmenu,
            Menu::CloseSubmenu => Action::CloseSubmenu,
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
        match self.menu_action() {
            None => "Open",
            Some(action) => keys::Action::name(action),
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Open => "Opens the menu at the top left corner of the area.",
            Action::Next => "Highlights the next enabled item, wrapping.",
            Action::Previous => "Highlights the previous enabled item, wrapping.",
            Action::Activate => "Chooses the highlighted item or opens its submenu.",
            action => action.menu_action().map_or("", keys::Action::description),
        }
    }
}

/// The default context menu shortcuts: the dropdown menu's, plus
///
/// | Keys | Action |
/// | --- | --- |
/// | `Shift+F10`, `Menu` | [`Action::Open`] |
///
/// Unlike a dropdown menu, the arrow keys, Enter and Space do nothing
/// while the menu is closed.
pub fn default_keymap() -> Keymap<Action> {
    let open = Keymap::new()
        .bind(Chord::named(Named::F10).shift(), Action::Open)
        .bind(Chord::named(Named::ContextMenu), Action::Open);
    dropdown_menu::default_keymap()
        .bindings()
        .fold(open, |keymap, (chord, action)| {
            keymap.bind(chord.clone(), Action::from(*action))
        })
}

/// A context menu builder. Convert it into an [`Element`] to render.
pub struct ContextMenu<'a, Id, Message> {
    state: &'a State<Id>,
    content: Element<'a, Message>,
    width: f32,
    on_event: Option<Box<dyn Fn(Event<Id>) -> Message + 'a>>,
}

impl<Id: std::fmt::Debug, Message> std::fmt::Debug for ContextMenu<'_, Id, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextMenu")
            .field("state", self.state)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

/// Makes `content` an area that opens the menu of `state` on a right-click.
/// Without [`on_event`](ContextMenu::on_event) right-clicks do nothing.
pub fn context_menu<'a, Id, Message>(
    state: &'a State<Id>,
    content: impl Into<Element<'a, Message>>,
) -> ContextMenu<'a, Id, Message> {
    ContextMenu {
        state,
        content: content.into(),
        width: WIDTH,
        on_event: None,
    }
}

impl<'a, Id, Message> ContextMenu<'a, Id, Message> {
    /// Width of the menu and its submenus.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn on_event(mut self, on_event: impl Fn(Event<Id>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

impl<'a, Id, Message> From<ContextMenu<'a, Id, Message>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(menu: ContextMenu<'a, Id, Message>) -> Self {
        let ContextMenu {
            state,
            content,
            width,
            on_event,
        } = menu;

        let on_event: Option<Rc<dyn Fn(Event<Id>) -> Message + 'a>> = on_event.map(Rc::from);

        let panel = match &on_event {
            _ if !state.is_open() => None,
            Some(on_event) => {
                let on_menu = |event: dropdown_menu::Event<Id>| on_event(Event::Menu(event));
                Some(dropdown_menu::panel(state.menu(), Some(&on_menu), width))
            }
            None => Some(dropdown_menu::panel(state.menu(), None, width)),
        };
        let dismiss = on_event
            .as_ref()
            .map(|on_event| on_event(Event::Menu(dropdown_menu::Event::Close)));

        let area = Area {
            content,
            on_open: on_event.map(|on_event| {
                Box::new(move |point| on_event(Event::Open(point)))
                    as Box<dyn Fn(Point) -> Message + 'a>
            }),
        };
        let anchored = anchored(Element::new(area))
            .content(panel)
            .at(state.position())
            .placement(Placement::new(Side::Bottom, Align::Start).gap(0.0))
            .dismiss_on_anchor_press(true);
        match dismiss {
            Some(message) => anchored.on_dismiss(message).into(),
            None => anchored.into(),
        }
    }
}

/// Wraps content and reports right-clicks on it with their position.
struct Area<'a, Message> {
    content: Element<'a, Message>,
    on_open: Option<Box<dyn Fn(Point) -> Message + 'a>>,
}

impl<Message> Widget<Message, Theme, Renderer> for Area<'_, Message> {
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
        event: &IcedEvent,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
        if shell.is_event_captured() {
            return;
        }
        let IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event else {
            return;
        };
        let (Some(on_open), Some(position)) = (&self.on_open, cursor.position_in(layout.bounds()))
        else {
            return;
        };
        shell.publish(on_open(position));
        shell.capture_event();
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
    use crate::overlay::dropdown_menu::{checkbox_item, item, separator, submenu};
    use iced::keyboard::{Key, Modifiers};
    use iced::widget::text;

    fn state() -> State<u8> {
        State::new([
            item(1, "Back"),
            item(2, "Forward").disabled(true),
            item(3, "Reload"),
            submenu(4, "More tools", [item(5, "Save page")]),
            separator(),
            checkbox_item(6, "Show bookmarks", true),
        ])
    }

    #[test]
    fn starts_closed_at_the_origin() {
        let state = state();
        assert!(!state.is_open());
        assert_eq!(state.position(), Point::ORIGIN);
        assert_eq!(state.menu().entries().len(), 6);
    }

    #[test]
    fn right_click_opens_at_the_point_without_a_highlight() {
        let mut state = state();
        assert_eq!(state.update(Event::Open(Point::new(40.0, 30.0))), None);
        assert!(state.is_open());
        assert_eq!(state.position(), Point::new(40.0, 30.0));
        assert_eq!(state.menu().highlighted(), None);
    }

    #[test]
    fn a_second_right_click_moves_the_menu_and_resets_it() {
        let mut state = state();
        let _ = state.update(Event::Open(Point::new(40.0, 30.0)));
        let _ = state.update(Event::Menu(dropdown_menu::Event::Highlight(4)));
        assert_eq!(state.menu().depth(), 2);
        let _ = state.update(Event::Open(Point::new(5.0, 6.0)));
        assert_eq!(state.position(), Point::new(5.0, 6.0));
        assert_eq!(state.menu().depth(), 1);
        assert_eq!(state.menu().highlighted(), None);
    }

    #[test]
    fn keyboard_opens_at_the_origin_on_the_first_item() {
        let mut state = state();
        let _ = state.update(Event::Open(Point::new(40.0, 30.0)));
        let _ = state.update(Event::OpenFromKeyboard);
        assert_eq!(state.position(), Point::ORIGIN);
        assert_eq!(state.menu().highlighted(), Some(1));
    }

    #[test]
    fn menu_events_are_ignored_while_closed() {
        let mut state = state();
        assert_eq!(state.update(Event::Menu(dropdown_menu::Event::Next)), None);
        assert!(!state.is_open());
        assert_eq!(
            state.update(Event::Menu(dropdown_menu::Event::Activate(1))),
            None
        );
    }

    #[test]
    fn choosing_items_returns_outputs_and_closes() {
        let mut state = state();
        let _ = state.update(Event::Open(Point::ORIGIN));
        assert_eq!(
            state.update(Event::Menu(dropdown_menu::Event::Activate(6))),
            Some(Output::Toggled(6, false))
        );
        assert!(!state.is_open());
        assert!(!state.is_checked(6));
        state.set_checked(6, true);
        assert!(state.is_checked(6));
        state.set_disabled(3, true);
        assert!(state.menu().item(3).is_some_and(|item| item.disabled));
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_adds_open_to_the_menu_keys() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "Shift+F10"), Some(Action::Open));
        assert_eq!(press(&keymap, "Menu"), Some(Action::Open));
        assert_eq!(press(&keymap, "F10"), None);
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Next));
        assert_eq!(press(&keymap, "ArrowUp"), Some(Action::Previous));
        assert_eq!(press(&keymap, "Home"), Some(Action::First));
        assert_eq!(press(&keymap, "End"), Some(Action::Last));
        assert_eq!(press(&keymap, "Enter"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Space"), Some(Action::Activate));
        assert_eq!(press(&keymap, "Escape"), Some(Action::Close));
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::OpenSubmenu));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::CloseSubmenu));
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind(&"Menu".parse().unwrap())
            .bind("Ctrl+M".parse().unwrap(), Action::Open);
        assert_eq!(press(&keymap, "Menu"), None);
        assert_eq!(press(&keymap, "Ctrl+M"), Some(Action::Open));
        assert_eq!(press(&keymap, "Shift+F10"), Some(Action::Open));
    }

    #[test]
    fn open_works_only_while_closed_and_the_rest_only_while_open() {
        let mut state = state();
        assert_eq!(Action::Open.event(&state), Some(Event::OpenFromKeyboard));
        assert_eq!(Action::Next.event(&state), None);
        assert_eq!(Action::Activate.event(&state), None);

        let _ = state.update(Event::OpenFromKeyboard);
        assert_eq!(Action::Open.event(&state), None);
        assert_eq!(
            Action::Next.event(&state),
            Some(Event::Menu(dropdown_menu::Event::Next))
        );
        assert_eq!(
            Action::Close.event(&state),
            Some(Event::Menu(dropdown_menu::Event::Close))
        );
    }

    #[test]
    fn every_action_round_trips_through_the_menu_action() {
        for &action in <Action as keys::Action>::ALL {
            match action.menu_action() {
                Some(menu) => assert_eq!(Action::from(menu), action),
                None => assert_eq!(action, Action::Open),
            }
            assert!(keys::Action::description(action).ends_with('.'));
            assert!(!keys::Action::name(action).is_empty());
        }
    }

    #[test]
    fn key_event_uses_the_keymap_then_typeahead() {
        let keymap = default_keymap();
        let mut state = state();
        let letter = keys::Event {
            key: Key::Character("r".into()),
            modifiers: Modifiers::empty(),
        };
        assert_eq!(state.key_event(&keymap, &letter), None);
        let _ = state.update(Event::OpenFromKeyboard);
        assert_eq!(
            state.key_event(&keymap, &letter),
            Some(Event::Menu(dropdown_menu::Event::Typeahead('r')))
        );
        let menu_key = keys::Event {
            key: Key::Named(Named::ContextMenu),
            modifiers: Modifiers::empty(),
        };
        assert_eq!(state.key_event(&keymap, &menu_key), None);
    }

    #[test]
    fn builder_defaults() {
        let state = state();
        let menu: ContextMenu<'_, u8, ()> = context_menu(&state, text("Area"));
        assert_eq!(menu.width, WIDTH);
        assert!(menu.on_event.is_none());
        let menu: ContextMenu<'_, u8, ()> = context_menu(&state, text("Area"))
            .width(180.0)
            .on_event(|_| ());
        assert_eq!(menu.width, 180.0);
        assert!(menu.on_event.is_some());
    }
}
