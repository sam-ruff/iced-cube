//! What the tree and the data table share: a [`Scope`] that takes focus
//! when pressed and resolves keys only while it has it, a [`Pressable`] row
//! that reports clicks with the modifiers held, and a [`RowMenu`] that
//! gives every row the same context menu.

use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse::{Click, click};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer as _, Shell};
use iced::keyboard::{self, Modifiers};
use iced::touch::{self, Finger};
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Shadow, Size,
    Theme, Vector, mouse,
};

use crate::keys::{self, Keymap};
use crate::overlay::context_menu;

type OnKey<'a, Message> = Box<dyn Fn(&keys::Event) -> Option<Message> + 'a>;
type OnFocus<'a, Message> = Box<dyn Fn(bool) -> Message + 'a>;

/// Wraps content that takes focus when pressed and loses it on a press
/// elsewhere. While focused, key presses the content leaves go through
/// `on_key`; a returned message is sent and the key captured.
pub(crate) struct Scope<'a, Message> {
    content: Element<'a, Message>,
    on_key: Option<OnKey<'a, Message>>,
    on_focus: Option<OnFocus<'a, Message>>,
}

pub(crate) fn scope<'a, Message>(content: impl Into<Element<'a, Message>>) -> Scope<'a, Message> {
    Scope {
        content: content.into(),
        on_key: None,
        on_focus: None,
    }
}

impl<'a, Message> Scope<'a, Message> {
    pub(crate) fn on_key(mut self, on_key: impl Fn(&keys::Event) -> Option<Message> + 'a) -> Self {
        self.on_key = Some(Box::new(on_key));
        self
    }

    pub(crate) fn on_focus(mut self, on_focus: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_focus = Some(Box::new(on_focus));
        self
    }
}

/// Whether a scope has focus, and what it last told the app.
#[derive(Debug, Default)]
struct Focus {
    focused: bool,
    reported: bool,
}

impl Focusable for Focus {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}

/// Where a press landed, if the event is a press with a known position. A
/// press over a floating layer leaves the cursor unavailable underneath.
fn press_at(event: &Event, cursor: mouse::Cursor) -> Option<Point> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_))
        | Event::Touch(touch::Event::FingerPressed { .. }) => cursor.position(),
        _ => None,
    }
}

fn key_press(event: &Event) -> Option<keys::Event> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    Some(keys::Event {
        key: key.clone(),
        modifiers: *modifiers,
    })
}

impl<Message> Widget<Message, Theme, Renderer> for Scope<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Focus>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Focus::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
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
        let Tree {
            state, children, ..
        } = tree;
        operation.focusable(None, layout.bounds(), state.downcast_mut::<Focus>());
        operation.traverse(&mut |operation| {
            self.content
                .as_widget_mut()
                .operate(&mut children[0], layout, renderer, operation);
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Tree {
            state, children, ..
        } = tree;
        let focus = state.downcast_mut::<Focus>();
        if let Some(position) = press_at(event, cursor) {
            focus.focused = layout.bounds().contains(position);
        }
        if focus.focused != focus.reported {
            focus.reported = focus.focused;
            if let Some(on_focus) = &self.on_focus {
                shell.publish(on_focus(focus.focused));
            }
        }

        self.content.as_widget_mut().update(
            &mut children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        if shell.is_event_captured() || !focus.focused {
            return;
        }
        let Some(press) = key_press(event) else {
            return;
        };
        let Some(message) = self.on_key.as_ref().and_then(|on_key| on_key(&press)) else {
            return;
        };
        shell.publish(message);
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

impl<'a, Message: 'a> From<Scope<'a, Message>> for Element<'a, Message> {
    fn from(scope: Scope<'a, Message>) -> Self {
        Element::new(scope)
    }
}

type Wrap<'a, Key, Message> = dyn Fn(Key, Element<'a, Message>) -> Element<'a, Message> + 'a;
type MenuKey<'a, Key, Message> = dyn Fn(&keys::Event, Key) -> Option<Message> + 'a;
#[cfg(feature = "data-table")]
type Open<'a, Key, Message> = dyn Fn(Key) -> Message + 'a;

/// One context menu shared by every row, told apart by the row's key.
pub(crate) struct RowMenu<'a, Key, Message> {
    wrap: Box<Wrap<'a, Key, Message>>,
    key: Box<MenuKey<'a, Key, Message>>,
    #[cfg(feature = "data-table")]
    open: Box<Open<'a, Key, Message>>,
}

impl<'a, Key, Message> RowMenu<'a, Key, Message> {
    /// Makes a row an area of the menu.
    pub(crate) fn wrap(&self, key: Key, row: Element<'a, Message>) -> Element<'a, Message> {
        (self.wrap)(key, row)
    }

    /// Resolves the opening chord, opening the menu on `target`.
    pub(crate) fn key(&self, press: &keys::Event, target: Key) -> Option<Message> {
        (self.key)(press, target)
    }

    /// The message that opens the menu below the row `key`, for a button.
    #[cfg(feature = "data-table")]
    pub(crate) fn open(&self, key: Key) -> Message {
        (self.open)(key)
    }
}

pub(crate) fn row_menu<'a, Id, Key, Message>(
    state: &'a context_menu::State<Id, Key>,
    keymap: Keymap<context_menu::Action>,
    on_event: Rc<dyn Fn(context_menu::Event<Id, Key>) -> Message + 'a>,
) -> RowMenu<'a, Key, Message>
where
    Id: Copy + PartialEq + 'a,
    Key: Clone + PartialEq + 'a,
    Message: Clone + 'a,
{
    let wrap = {
        let keymap = keymap.clone();
        let on_event = on_event.clone();
        move |key: Key, row: Element<'a, Message>| -> Element<'a, Message> {
            let on_event = on_event.clone();
            context_menu::keyed(state, key, row)
                .keymap(keymap.clone())
                .on_event(move |event| on_event(event))
                .into()
        }
    };
    let key = {
        let on_event = on_event.clone();
        move |press: &keys::Event, target: Key| {
            state
                .key_event(&keymap, press, target)
                .map(|event| on_event(event))
        }
    };
    RowMenu {
        wrap: Box::new(wrap),
        key: Box::new(key),
        #[cfg(feature = "data-table")]
        open: Box::new(move |key| on_event(context_menu::Event::OpenFromKeyboard(key))),
    }
}

/// How far, in logical pixels, a finger may drift before a tap on a row
/// counts as a scroll instead.
pub(crate) const TAP_SLOP: f32 = 10.0;

/// How a [`Pressable`] row is painted: a background under the content, and
/// a border and a line along the bottom over it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Look {
    pub background: Option<Color>,
    pub border: Border,
    pub divider: Option<Color>,
}

type Paint<'a> = Box<dyn Fn(&Theme, bool) -> Look + 'a>;

/// A row that paints itself for the pointer hovering over it and reports
/// presses with the modifiers held, and double clicks. A tap only counts
/// when the finger lifts without moving, so a list still scrolls by touch.
pub(crate) struct Pressable<'a, Message> {
    content: Element<'a, Message>,
    on_press: Option<Box<dyn Fn(Modifiers) -> Message + 'a>>,
    on_double_click: Option<Message>,
    look: Paint<'a>,
}

pub(crate) fn pressable<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    look: impl Fn(&Theme, bool) -> Look + 'a,
) -> Pressable<'a, Message> {
    Pressable {
        content: content.into(),
        on_press: None,
        on_double_click: None,
        look: Box::new(look),
    }
}

impl<'a, Message> Pressable<'a, Message> {
    pub(crate) fn on_press(mut self, on_press: impl Fn(Modifiers) -> Message + 'a) -> Self {
        self.on_press = Some(Box::new(on_press));
        self
    }

    pub(crate) fn on_double_click(mut self, message: Option<Message>) -> Self {
        self.on_double_click = message;
        self
    }
}

#[derive(Debug, Default)]
struct Press {
    modifiers: Modifiers,
    hovered: bool,
    last_click: Option<Click>,
    finger: Option<(Finger, Point)>,
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Pressable<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Press>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Press::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
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
        let bounds = layout.bounds();
        let look = (self.look)(theme, self.on_press.is_some() && cursor.is_over(bounds));
        if let Some(background) = look.background {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: Border {
                        width: 0.0,
                        ..look.border
                    },
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(background),
            );
        }
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        if let Some(divider) = look.divider {
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle {
                        y: bounds.y + bounds.height - 1.0,
                        height: 1.0,
                        ..bounds
                    },
                    snap: true,
                    ..Quad::default()
                },
                Background::Color(divider),
            );
        }
        if look.border.width > 0.0 {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border: look.border,
                    shadow: Shadow::default(),
                    snap: true,
                },
                Background::Color(Color::TRANSPARENT),
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
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let press = tree.state.downcast_mut::<Press>();
        if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            press.modifiers = *modifiers;
        }
        let hovered = cursor.is_over(bounds);
        if hovered != press.hovered {
            press.hovered = hovered;
            shell.request_redraw();
        }

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
        let press = tree.state.downcast_mut::<Press>();
        if shell.is_event_captured() {
            press.finger = None;
            return;
        }
        let Some(on_press) = &self.on_press else {
            return;
        };

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_over(bounds) else {
                    return;
                };
                shell.publish(on_press(press.modifiers));
                let click = Click::new(position, mouse::Button::Left, press.last_click);
                press.last_click = Some(click);
                if click.kind() == click::Kind::Double
                    && let Some(message) = &self.on_double_click
                {
                    shell.publish(message.clone());
                }
                shell.capture_event();
            }
            Event::Touch(touch::Event::FingerPressed { id, .. }) => {
                press.finger = cursor.position_over(bounds).map(|position| (*id, position));
            }
            Event::Touch(touch::Event::FingerMoved { id, .. }) => {
                let drifted = press.finger.is_some_and(|(finger, origin)| {
                    finger == *id
                        && cursor
                            .position()
                            .is_some_and(|position| origin.distance(position) > TAP_SLOP)
                });
                if drifted {
                    press.finger = None;
                }
            }
            Event::Touch(touch::Event::FingerLifted { id, .. }) => {
                let tapped = press.finger.take().is_some_and(|(finger, _)| finger == *id)
                    && cursor.is_over(bounds);
                if tapped {
                    shell.publish(on_press(Modifiers::empty()));
                    shell.capture_event();
                }
            }
            Event::Touch(touch::Event::FingerLost { .. }) => press.finger = None,
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let inner = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        );
        if inner != mouse::Interaction::None
            || self.on_press.is_none()
            || !cursor.is_over(layout.bounds())
        {
            return inner;
        }
        mouse::Interaction::Pointer
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

impl<'a, Message: Clone + 'a> From<Pressable<'a, Message>> for Element<'a, Message> {
    fn from(pressable: Pressable<'a, Message>) -> Self {
        Element::new(pressable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_only_counts_with_a_known_position() {
        let pressed = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let at = Point::new(4.0, 5.0);
        assert_eq!(press_at(&pressed, mouse::Cursor::Available(at)), Some(at));
        assert_eq!(press_at(&pressed, mouse::Cursor::Levitating(at)), None);
        assert_eq!(press_at(&pressed, mouse::Cursor::Unavailable), None);
        let moved = Event::Mouse(mouse::Event::CursorMoved { position: at });
        assert_eq!(press_at(&moved, mouse::Cursor::Available(at)), None);
    }

    #[test]
    fn focus_follows_the_focusable_calls() {
        let mut focus = Focus::default();
        assert!(!focus.is_focused());
        focus.focus();
        assert!(focus.is_focused());
        focus.unfocus();
        assert!(!focus.is_focused());
    }

    #[test]
    fn key_presses_carry_their_modifiers() {
        let event = Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("a".into()),
            modified_key: keyboard::Key::Character("a".into()),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: Modifiers::CTRL,
            text: None,
            repeat: false,
        });
        let press = key_press(&event).expect("a key press");
        assert_eq!(press.modifiers, Modifiers::CTRL);
        assert_eq!(key_press(&Event::Mouse(mouse::Event::CursorLeft)), None);
    }
}
