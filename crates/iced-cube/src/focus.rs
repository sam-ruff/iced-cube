//! Keyboard focus for the library's buttons and controls.
//!
//! A control reports itself to iced's focus operations, so `focus_next`,
//! `focus_previous` and a dialog's Tab visit it in tree order alongside
//! text fields. Only keyboard focus is shown: a pointer press never
//! focuses a control, and any pointer press takes focus away, as it does
//! from a text field. While focused it draws a ring and its activation
//! keys press it.

#[cfg(any(
    feature = "button",
    feature = "checkbox",
    feature = "radio",
    feature = "switch",
    feature = "toggle"
))]
mod control;

#[cfg(any(
    feature = "button",
    feature = "checkbox",
    feature = "radio",
    feature = "switch",
    feature = "toggle"
))]
pub(crate) use control::{Ring, focusable};

use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::Focusable;
use iced::advanced::{Renderer as _, renderer};
use iced::widget;
use iced::{Background, Border, Color, Event, Rectangle, Renderer, Theme, mouse, touch};

use crate::theme::Tokens;

/// Width of the focus ring.
pub(crate) const RING_WIDTH: f32 = 2.0;

/// How far the ring sits outside the control, clear of its own border.
pub(crate) const RING_OUTSET: f32 = RING_WIDTH + 1.0;

/// Whether a control has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct State {
    focused: bool,
}

impl State {
    pub(crate) fn is_focused(self) -> bool {
        self.focused
    }

    /// Takes focus away on a pointer press. Returns whether it changed.
    pub(crate) fn update(&mut self, event: &Event) -> bool {
        if !self.focused || !is_pointer_press(event) {
            return false;
        }
        self.focused = false;
        true
    }
}

impl Focusable for State {
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

/// Reported through [`Operation::custom`] just before a control's
/// [`Operation::focusable`], so operations can tell controls from text
/// fields: the state passed to `custom` downcasts to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Control;

/// Reports a control to `operation`: the marker, then the focusable.
pub(crate) fn report(
    operation: &mut dyn Operation,
    id: Option<&widget::Id>,
    bounds: Rectangle,
    state: &mut State,
) {
    operation.custom(id, bounds, &mut Control);
    operation.focusable(id, bounds, state);
}

fn is_pointer_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Touch(touch::Event::FingerPressed { .. })
    )
}

/// The named key of a press without modifiers, if the event is one.
#[cfg(any(
    feature = "button",
    feature = "checkbox",
    feature = "radio",
    feature = "select",
    feature = "switch",
    feature = "toggle"
))]
pub(crate) fn named_press(event: &Event) -> Option<iced::keyboard::key::Named> {
    use iced::keyboard;

    let Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(named),
        modifiers,
        ..
    }) = event
    else {
        return None;
    };
    modifiers.is_empty().then_some(*named)
}

/// The bounds and corner radius of the ring around `target`. The radius
/// never exceeds half the ring's shorter side, so a large one draws a pill.
pub(crate) fn ring_shape(target: Rectangle, radius: f32) -> (Rectangle, f32) {
    let bounds = target.expand(RING_OUTSET);
    let radius = (radius + RING_OUTSET).min(bounds.width.min(bounds.height) / 2.0);
    (bounds, radius)
}

/// Draws the focus ring around `target` in the theme's ring colour.
pub(crate) fn draw_ring(renderer: &mut Renderer, theme: &Theme, target: Rectangle, radius: f32) {
    let (bounds, radius) = ring_shape(target, radius);
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: Border {
                color: Tokens::of(theme).ring,
                width: RING_WIDTH,
                radius: radius.into(),
            },
            ..renderer::Quad::default()
        },
        Background::Color(Color::TRANSPARENT),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::{self, key::Named};
    use iced::{Point, Size};

    fn key(named: Named, modifiers: keyboard::Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named),
            modified_key: keyboard::Key::Named(named),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    #[cfg(any(
        feature = "button",
        feature = "checkbox",
        feature = "radio",
        feature = "select",
        feature = "switch",
        feature = "toggle"
    ))]
    #[test]
    fn named_presses_ignore_chords_with_modifiers() {
        let none = keyboard::Modifiers::empty();
        assert_eq!(
            named_press(&key(Named::ArrowDown, none)),
            Some(Named::ArrowDown)
        );
        assert_eq!(
            named_press(&key(Named::Enter, keyboard::Modifiers::CTRL)),
            None
        );
        let moved = Event::Mouse(mouse::Event::CursorMoved {
            position: Point::ORIGIN,
        });
        assert_eq!(named_press(&moved), None);
    }

    #[test]
    fn only_operations_focus_and_a_pointer_press_unfocuses() {
        let mut state = State::default();
        let press = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        assert!(!state.update(&press), "a press never focuses");
        assert!(!state.is_focused());

        state.focus();
        assert!(Focusable::is_focused(&state));
        let moved = Event::Mouse(mouse::Event::CursorMoved {
            position: Point::ORIGIN,
        });
        assert!(!state.update(&moved));
        assert!(!state.update(&key(Named::Tab, keyboard::Modifiers::empty())));
        assert!(state.is_focused());

        let finger = Event::Touch(touch::Event::FingerPressed {
            id: touch::Finger(0),
            position: Point::ORIGIN,
        });
        assert!(state.update(&finger));
        assert!(!state.is_focused());
    }

    #[test]
    fn the_ring_sits_outside_the_control() {
        let target = Rectangle::new(Point::new(10.0, 10.0), Size::new(40.0, 20.0));
        let (bounds, radius) = ring_shape(target, 6.0);
        assert_eq!(bounds, target.expand(RING_OUTSET));
        assert_eq!(radius, 6.0 + RING_OUTSET);

        let (bounds, radius) = ring_shape(target, crate::theme::radius::FULL);
        assert_eq!(radius, bounds.height / 2.0, "a full radius draws a pill");
    }
}
