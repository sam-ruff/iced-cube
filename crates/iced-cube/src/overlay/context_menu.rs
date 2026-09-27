//! A menu that opens at the pointer on a right-click inside an area, or at
//! the finger on a long press.
//!
//! It uses the dropdown menu's entries, rows and navigation: build the
//! entries with [`dropdown_menu::item`](crate::overlay::dropdown_menu::item)
//! and friends. [`State`] adds where the menu opened. On a touch screen, a
//! finger held still on the area for [`LONG_PRESS`] opens it. From the
//! keyboard, Shift+F10 or the Menu key opens it just below the area, aligned
//! with its start, as desktop apps do.
//!
//! One [`State`] can serve many areas, such as every row of a list: give
//! each area a key with [`keyed`], and the open events carry the key of the
//! area they came from. [`State::target`] tells the app which row a chosen
//! item belongs to.
//!
//! Keyboard shortcuts resolve through a [`Keymap`] of [`Action`]s; see
//! [`default_keymap`]. The open menu handles its own keys; the app routes
//! the opening chord through [`State::key_event`].

use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::key::Named;
use iced::time::{Duration, Instant};
use iced::touch::{self, Finger};
use iced::{
    Element, Event as IcedEvent, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
    window,
};

use crate::keys::{self, Chord, Keymap};
use crate::overlay::anchored::{Align, Placement, Side, anchored};
use crate::overlay::dropdown_menu::{self, Entry, Output, WIDTH};

/// Changes to a context menu. `Key` names the area an open event came from;
/// it is `()` for a menu with a single area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event<Id, Key = ()> {
    /// A right-click or a long press on the area `Key` at this point,
    /// relative to the area's top left corner. Opens the menu there with
    /// nothing highlighted.
    Open(Key, Point),
    /// Opens the menu below the area `Key` on its first item.
    OpenFromKeyboard(Key),
    /// Navigation and choosing, as in a dropdown menu. Ignored while closed.
    Menu(dropdown_menu::Event<Id>),
}

/// The menu, which area it opened on and where.
#[derive(Debug, Clone, PartialEq)]
pub struct State<Id, Key = ()> {
    menu: dropdown_menu::State<Id>,
    position: Option<Point>,
    target: Option<Key>,
}

impl<Id: Copy + PartialEq, Key: Clone + PartialEq> State<Id, Key> {
    /// A closed menu. Item ids must be unique across submenus.
    pub fn new(entries: impl IntoIterator<Item = Entry<Id>>) -> Self {
        Self {
            menu: dropdown_menu::State::new(entries),
            position: None,
            target: None,
        }
    }

    /// The menu entries, highlight and open submenus.
    pub fn menu(&self) -> &dropdown_menu::State<Id> {
        &self.menu
    }

    pub fn is_open(&self) -> bool {
        self.menu.is_open()
    }

    /// Whether the menu is open on the area `key`.
    pub fn is_open_on(&self, key: &Key) -> bool {
        self.is_open() && self.target.as_ref() == Some(key)
    }

    /// The area the menu last opened on. It stays set after the menu closes,
    /// so the app can tell which row a chosen item belongs to.
    pub fn target(&self) -> Option<&Key> {
        self.target.as_ref()
    }

    /// Where a right-click opened the menu, relative to the area's top left
    /// corner, or `None` when it opened from the keyboard below the area.
    pub fn position(&self) -> Option<Point> {
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

    /// Applies an event and returns what was chosen, if anything. Opening
    /// on another area moves the menu there and resets it.
    pub fn update(&mut self, event: Event<Id, Key>) -> Option<Output<Id>> {
        match event {
            Event::Open(key, position) => {
                self.target = Some(key);
                self.position = Some(position);
                let _ = self.menu.update(dropdown_menu::Event::Close);
                self.menu.update(dropdown_menu::Event::Open)
            }
            Event::OpenFromKeyboard(key) => {
                self.target = Some(key);
                self.position = None;
                let _ = self.menu.update(dropdown_menu::Event::Close);
                self.menu.update(dropdown_menu::Event::First)
            }
            Event::Menu(_) if !self.is_open() => None,
            Event::Menu(event) => self.menu.update(event),
        }
    }

    /// Turns a key press into an event: first through `keymap`, then, while
    /// the menu is open, as typeahead for a letter or digit. `target` is the
    /// area the opening chord opens the menu on, such as the focused row;
    /// pass `()` for a menu with a single area.
    pub fn key_event(
        &self,
        keymap: &Keymap<Action>,
        key: &keys::Event,
        target: Key,
    ) -> Option<Event<Id, Key>> {
        if let Some(action) = keymap.resolve_event(key) {
            return action.event(self, target);
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
    /// only the others while it is open. [`Action::Open`] opens on `target`.
    pub fn event<Id: Copy + PartialEq, Key: Clone + PartialEq>(
        self,
        state: &State<Id, Key>,
        target: Key,
    ) -> Option<Event<Id, Key>> {
        let open = state.is_open();
        match self.menu_action() {
            None => (!open).then_some(Event::OpenFromKeyboard(target)),
            Some(_) if !open => None,
            Some(action) => action.event(state.menu()).map(Event::Menu),
        }
    }
}

impl From<dropdown_menu::Action> for Action {
    fn from(action: dropdown_menu::Action) -> Self {
        use dropdown_menu::Action as Menu;
        match action {
            Menu::Open => Action::Open,
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
            Action::Open => "Opens the menu below the area, on its first item.",
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
/// The open menu resolves its keys itself; pass a changed keymap with
/// [`ContextMenu::keymap`]. While it is closed, only [`Action::Open`] does
/// anything, and the app routes it from [`keys::subscription`] through
/// [`State::key_event`].
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

/// Gap between an area and a menu opened from the keyboard below it.
pub const KEYBOARD_GAP: f32 = 4.0;

/// How long a finger rests on an area before the menu opens.
pub const LONG_PRESS: Duration = Duration::from_millis(500);

/// How far, in logical pixels, a finger may drift during a long press. Any
/// further and it counts as a scroll or a drag.
pub const LONG_PRESS_SLOP: f32 = 10.0;

/// A finger held on an area. Time only comes in through [`LongPress::tick`],
/// which the area calls on every redraw, so the timing is testable.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct LongPress {
    finger: Option<Finger>,
    origin: Point,
    since: Option<Instant>,
    fired: bool,
}

impl LongPress {
    /// Starts timing a finger pressed at `position`, replacing any other.
    fn press(&mut self, finger: Finger, position: Point) {
        *self = Self {
            finger: Some(finger),
            origin: position,
            since: None,
            fired: false,
        };
    }

    /// Cancels the press once the finger drifts beyond [`LONG_PRESS_SLOP`].
    fn moved(&mut self, finger: Finger, position: Point) {
        if self.finger != Some(finger) || self.fired {
            return;
        }
        if self.origin.distance(position) > LONG_PRESS_SLOP {
            *self = Self::default();
        }
    }

    /// Ends the press. Returns whether it had opened the menu, in which case
    /// the lift must not reach the content as a tap.
    fn lift(&mut self, finger: Finger) -> bool {
        if self.finger != Some(finger) {
            return false;
        }
        let fired = self.fired;
        *self = Self::default();
        fired
    }

    /// Starts the clock on the first redraw after the press, and returns
    /// where the press began once, when it has been held for [`LONG_PRESS`].
    fn tick(&mut self, now: Instant) -> Option<Point> {
        if self.finger.is_none() || self.fired {
            return None;
        }
        let since = *self.since.get_or_insert(now);
        if now.saturating_duration_since(since) < LONG_PRESS {
            return None;
        }
        self.fired = true;
        Some(self.origin)
    }

    /// When the area next needs a redraw to keep timing, if it does.
    fn redraw(&self) -> Option<window::RedrawRequest> {
        if self.finger.is_none() || self.fired {
            return None;
        }
        Some(match self.since {
            None => window::RedrawRequest::NextFrame,
            Some(since) => window::RedrawRequest::At(since + LONG_PRESS),
        })
    }
}

type OnEvent<'a, Id, Key, Message> = dyn Fn(Event<Id, Key>) -> Message + 'a;

/// A context menu builder. Convert it into an [`Element`] to render.
pub struct ContextMenu<'a, Id, Message, Key = ()> {
    state: &'a State<Id, Key>,
    key: Key,
    content: Element<'a, Message>,
    width: f32,
    keymap: Keymap<Action>,
    on_event: Option<Box<OnEvent<'a, Id, Key, Message>>>,
}

impl<Id: std::fmt::Debug, Message, Key: std::fmt::Debug> std::fmt::Debug
    for ContextMenu<'_, Id, Message, Key>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextMenu")
            .field("state", self.state)
            .field("key", &self.key)
            .field("width", &self.width)
            .field("keymap", &self.keymap)
            .finish_non_exhaustive()
    }
}

/// Makes `content` an area that opens the menu of `state` on a right-click.
/// Without [`on_event`](ContextMenu::on_event) right-clicks do nothing.
pub fn context_menu<'a, Id, Message>(
    state: &'a State<Id>,
    content: impl Into<Element<'a, Message>>,
) -> ContextMenu<'a, Id, Message> {
    keyed(state, (), content)
}

/// Makes `content` one of many areas sharing the menu of `state`, such as
/// a row in a list. The menu shows on this area only while it is open on
/// `key`, and the open events it sends carry `key`.
pub fn keyed<'a, Id, Key, Message>(
    state: &'a State<Id, Key>,
    key: Key,
    content: impl Into<Element<'a, Message>>,
) -> ContextMenu<'a, Id, Message, Key> {
    ContextMenu {
        state,
        key,
        content: content.into(),
        width: WIDTH,
        keymap: default_keymap(),
        on_event: None,
    }
}

impl<'a, Id, Message, Key> ContextMenu<'a, Id, Message, Key> {
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

    pub fn on_event(mut self, on_event: impl Fn(Event<Id, Key>) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }
}

/// Where the panel goes: at the right-click point, or just below the area,
/// aligned with its start, when the menu opened from the keyboard.
pub fn placement(position: Option<Point>) -> Placement {
    let gap = if position.is_some() {
        0.0
    } else {
        KEYBOARD_GAP
    };
    Placement::new(Side::Bottom, Align::Start).gap(gap)
}

impl<'a, Id, Message, Key> From<ContextMenu<'a, Id, Message, Key>> for Element<'a, Message>
where
    Id: Copy + PartialEq + 'a,
    Key: Clone + PartialEq + 'a,
    Message: Clone + 'a,
{
    fn from(menu: ContextMenu<'a, Id, Message, Key>) -> Self {
        let ContextMenu {
            state,
            key,
            content,
            width,
            keymap,
            on_event,
        } = menu;

        let open = state.is_open_on(&key);
        let on_event: Option<Rc<OnEvent<'a, Id, Key, Message>>> = on_event.map(Rc::from);

        let panel = match &on_event {
            _ if !open => None,
            Some(on_event) => {
                let on_menu = |event: dropdown_menu::Event<Id>| on_event(Event::Menu(event));
                Some(dropdown_menu::panel(state.menu(), Some(&on_menu), width))
            }
            None => Some(dropdown_menu::panel(state.menu(), None, width)),
        };

        let area = Area {
            content,
            on_open: on_event.clone().map(|on_event| {
                let key = key.clone();
                Box::new(move |point| on_event(Event::Open(key.clone(), point)))
                    as Box<dyn Fn(Point) -> Message + 'a>
            }),
        };
        let mut anchored = anchored(Element::new(area))
            .content(panel)
            .placement(placement(state.position()))
            .dismiss_on_anchor_press(true)
            .dismiss_keys([]);
        if let Some(position) = state.position() {
            anchored = anchored.at(position);
        }
        let Some(on_event) = on_event.filter(|_| open) else {
            return anchored.into();
        };
        anchored
            .on_dismiss(on_event(Event::Menu(dropdown_menu::Event::Close)))
            .on_key(move |press| state.key_event(&keymap, press, key.clone()).map(&*on_event))
            .into()
    }
}

/// Wraps content and reports right-clicks and long presses on it with their
/// position.
struct Area<'a, Message> {
    content: Element<'a, Message>,
    on_open: Option<Box<dyn Fn(Point) -> Message + 'a>>,
}

/// Where the cursor or finger is, relative to the area's top left corner.
fn relative(cursor: mouse::Cursor, bounds: Rectangle) -> Option<Point> {
    let position = cursor.position()?;
    Some(Point::new(position.x - bounds.x, position.y - bounds.y))
}

impl<Message> Area<'_, Message> {
    /// Times a finger held on the area and opens the menu once it has been
    /// held long enough. The cursor follows the finger, and unlike the
    /// event's position it is already translated inside scroll areas.
    fn long_press(
        &self,
        press: &mut LongPress,
        event: &IcedEvent,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(on_open) = &self.on_open else {
            return;
        };
        match event {
            IcedEvent::Touch(touch::Event::FingerPressed { id, .. }) => {
                if let Some(position) = cursor.position_in(bounds) {
                    press.press(*id, position);
                }
            }
            IcedEvent::Touch(touch::Event::FingerMoved { id, .. }) => {
                if let Some(position) = relative(cursor, bounds) {
                    press.moved(*id, position);
                }
            }
            IcedEvent::Window(window::Event::RedrawRequested(now)) => {
                if let Some(position) = press.tick(*now) {
                    shell.publish(on_open(position));
                }
            }
            _ => {}
        }
        if let Some(request) = press.redraw() {
            shell.request_redraw_at(request);
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Area<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<LongPress>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(LongPress::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
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
            .operate(&mut tree.children[0], layout, renderer, operation);
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
        let press = tree.state.downcast_mut::<LongPress>();
        // The finger that opened the menu lifts over it, not as a tap.
        if let IcedEvent::Touch(
            touch::Event::FingerLifted { id, .. } | touch::Event::FingerLost { id, .. },
        ) = event
            && press.lift(*id)
        {
            shell.capture_event();
            return;
        }
        self.long_press(press, event, layout.bounds(), cursor, shell);

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
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
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
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

    fn rows() -> State<u8, &'static str> {
        State::new([item(1, "Open"), item(3, "Rename"), item(6, "Delete")])
    }

    #[test]
    fn starts_closed_with_no_target() {
        let state = state();
        assert!(!state.is_open());
        assert_eq!(state.position(), None);
        assert_eq!(state.target(), None);
        assert!(!state.is_open_on(&()));
        assert_eq!(state.menu().entries().len(), 6);
    }

    #[test]
    fn right_click_opens_at_the_point_without_a_highlight() {
        let mut state = state();
        assert_eq!(state.update(Event::Open((), Point::new(40.0, 30.0))), None);
        assert!(state.is_open());
        assert!(state.is_open_on(&()));
        assert_eq!(state.position(), Some(Point::new(40.0, 30.0)));
        assert_eq!(state.menu().highlighted(), None);
    }

    #[test]
    fn a_second_right_click_moves_the_menu_and_resets_it() {
        let mut state = state();
        let _ = state.update(Event::Open((), Point::new(40.0, 30.0)));
        let _ = state.update(Event::Menu(dropdown_menu::Event::Highlight(4)));
        assert_eq!(state.menu().depth(), 2);
        let _ = state.update(Event::Open((), Point::new(5.0, 6.0)));
        assert_eq!(state.position(), Some(Point::new(5.0, 6.0)));
        assert_eq!(state.menu().depth(), 1);
        assert_eq!(state.menu().highlighted(), None);
    }

    #[test]
    fn keyboard_opens_below_the_area_on_the_first_item() {
        let mut state = state();
        let _ = state.update(Event::Open((), Point::new(40.0, 30.0)));
        let _ = state.update(Event::OpenFromKeyboard(()));
        assert_eq!(state.position(), None);
        assert_eq!(state.menu().highlighted(), Some(1));
    }

    #[test]
    fn keyboard_placement_sits_below_the_area_and_pointer_placement_at_the_point() {
        let below = placement(None);
        assert_eq!(
            below,
            Placement::new(Side::Bottom, Align::Start).gap(KEYBOARD_GAP)
        );
        let at_pointer = placement(Some(Point::new(3.0, 4.0)));
        assert_eq!(
            at_pointer,
            Placement::new(Side::Bottom, Align::Start).gap(0.0)
        );
    }

    #[test]
    fn one_state_serves_many_rows() {
        let mut state = rows();
        let _ = state.update(Event::Open("a.txt", Point::new(10.0, 8.0)));
        assert!(state.is_open_on(&"a.txt"));
        assert!(!state.is_open_on(&"b.txt"));

        let _ = state.update(Event::OpenFromKeyboard("b.txt"));
        assert!(state.is_open_on(&"b.txt"));
        assert!(!state.is_open_on(&"a.txt"));
        assert_eq!(state.target(), Some(&"b.txt"));
    }

    #[test]
    fn the_target_outlives_the_menu_so_a_choice_knows_its_row() {
        let mut state = rows();
        let _ = state.update(Event::Open("b.txt", Point::ORIGIN));
        assert_eq!(
            state.update(Event::Menu(dropdown_menu::Event::Activate(6))),
            Some(Output::Activated(6))
        );
        assert!(!state.is_open());
        assert!(!state.is_open_on(&"b.txt"));
        assert_eq!(state.target(), Some(&"b.txt"));
    }

    #[test]
    fn the_opening_chord_opens_on_the_row_it_is_given() {
        let state = rows();
        let open = keys::Event {
            key: Key::Named(Named::F10),
            modifiers: Modifiers::SHIFT,
        };
        assert_eq!(
            state.key_event(&default_keymap(), &open, "c.txt"),
            Some(Event::OpenFromKeyboard("c.txt"))
        );
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
        let _ = state.update(Event::Open((), Point::ORIGIN));
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
        assert_eq!(
            Action::Open.event(&state, ()),
            Some(Event::OpenFromKeyboard(()))
        );
        assert_eq!(Action::Next.event(&state, ()), None);
        assert_eq!(Action::Activate.event(&state, ()), None);

        let _ = state.update(Event::OpenFromKeyboard(()));
        assert_eq!(Action::Open.event(&state, ()), None);
        assert_eq!(
            Action::Next.event(&state, ()),
            Some(Event::Menu(dropdown_menu::Event::Next))
        );
        assert_eq!(
            Action::Close.event(&state, ()),
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
        assert_eq!(state.key_event(&keymap, &letter, ()), None);
        let _ = state.update(Event::OpenFromKeyboard(()));
        assert_eq!(
            state.key_event(&keymap, &letter, ()),
            Some(Event::Menu(dropdown_menu::Event::Typeahead('r')))
        );
        let menu_key = keys::Event {
            key: Key::Named(Named::ContextMenu),
            modifiers: Modifiers::empty(),
        };
        assert_eq!(state.key_event(&keymap, &menu_key, ()), None);
    }

    const FINGER: Finger = Finger(7);

    #[test]
    fn a_long_press_fires_once_after_the_delay_at_its_origin() {
        let start = Instant::now();
        let mut press = LongPress::default();
        assert_eq!(press.redraw(), None);
        assert_eq!(press.tick(start), None);

        press.press(FINGER, Point::new(12.0, 8.0));
        assert_eq!(press.redraw(), Some(window::RedrawRequest::NextFrame));
        assert_eq!(press.tick(start), None);
        assert_eq!(
            press.redraw(),
            Some(window::RedrawRequest::At(start + LONG_PRESS))
        );
        assert_eq!(press.tick(start + Duration::from_millis(499)), None);
        assert_eq!(press.tick(start + LONG_PRESS), Some(Point::new(12.0, 8.0)));
        assert_eq!(press.tick(start + LONG_PRESS * 2), None);
        assert_eq!(press.redraw(), None);
        assert!(press.lift(FINGER), "the lift after firing is swallowed");
        assert!(!press.lift(FINGER));
    }

    #[test]
    fn lifting_early_is_a_tap() {
        let start = Instant::now();
        let mut press = LongPress::default();
        press.press(FINGER, Point::ORIGIN);
        let _ = press.tick(start);
        assert!(!press.lift(FINGER));
        assert_eq!(press.tick(start + LONG_PRESS), None);
    }

    #[test]
    fn drifting_beyond_the_slop_cancels_but_a_wobble_does_not() {
        let start = Instant::now();
        let mut press = LongPress::default();
        press.press(FINGER, Point::new(50.0, 50.0));
        let _ = press.tick(start);
        press.moved(FINGER, Point::new(56.0, 54.0));
        press.moved(Finger(8), Point::new(200.0, 200.0));
        assert!(press.tick(start + LONG_PRESS).is_some());

        press.press(FINGER, Point::new(50.0, 50.0));
        let _ = press.tick(start);
        press.moved(FINGER, Point::new(50.0, 50.0 + LONG_PRESS_SLOP + 1.0));
        assert_eq!(press.tick(start + LONG_PRESS), None);
        assert_eq!(press.redraw(), None);
    }

    #[test]
    fn another_finger_lifting_leaves_the_press_alone() {
        let start = Instant::now();
        let mut press = LongPress::default();
        press.press(FINGER, Point::ORIGIN);
        let _ = press.tick(start);
        assert!(!press.lift(Finger(8)));
        assert!(press.tick(start + LONG_PRESS).is_some());
    }

    #[test]
    fn builder_defaults() {
        let state = state();
        let menu: ContextMenu<'_, u8, ()> = context_menu(&state, text("Area"));
        assert_eq!(menu.width, WIDTH);
        assert_eq!(menu.keymap, default_keymap());
        assert!(menu.on_event.is_none());
        let menu: ContextMenu<'_, u8, ()> = context_menu(&state, text("Area"))
            .width(180.0)
            .keymap(Keymap::new())
            .on_event(|_| ());
        assert_eq!(menu.width, 180.0);
        assert!(menu.keymap.is_empty());
        assert!(menu.on_event.is_some());

        let rows = rows();
        let row: ContextMenu<'_, u8, (), &str> = keyed(&rows, "a.txt", text("a.txt"));
        assert_eq!(row.key, "a.txt");
    }
}
