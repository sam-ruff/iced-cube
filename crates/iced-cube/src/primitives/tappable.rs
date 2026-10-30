//! Makes any content respond to taps and long presses, by finger or mouse,
//! without stopping a surrounding scrollable from scrolling.
//!
//! A button captures the finger that presses it, so a list made of buttons
//! can only be dragged from outside its rows. A tappable never captures a
//! finger: it watches the press, and a finger that moves further than the
//! slop is a drag, which cancels the tap and leaves the scrolling to the
//! scrollable around it.
//!
//! | Input | Result |
//! | --- | --- |
//! | Finger or left button pressed and lifted within the slop | `on_tap` |
//! | Finger or left button held for the hold time | `on_long_press`, and the lift is ignored |
//! | Right click | `on_long_press` |
//! | Finger moved beyond the slop, a second finger, or a lost finger | Cancelled |
//!
//! Content gets every event first. A press the content captures, such as
//! one on a button nested in the row, belongs to the content: the row is not
//! tapped as well. Such a button also captures the finger, so a drag that
//! starts on it does not scroll; keep nested controls small, or make them
//! tappables too.
//!
//! ```no_run
//! use iced::widget::text;
//! use iced_cube::tappable;
//!
//! #[derive(Debug, Clone)]
//! enum Message { Open(u32), Menu(u32) }
//!
//! let row: iced::Element<'_, Message> = tappable(text("Shopping list"))
//!     .on_tap(Message::Open(7))
//!     .on_long_press(Message::Menu(7))
//!     .width(iced::Length::Fill)
//!     .padding(12)
//!     .into();
//! ```

use std::fmt;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::time::{Duration, Instant};
use iced::touch::{self, Finger};
use iced::widget::container;
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Renderer, Size,
    Theme, Vector, mouse, window,
};

use crate::theme::{Tokens, mix, radius};

/// How long a press must be held to count as a long press.
pub const DEFAULT_HOLD: Duration = Duration::from_millis(450);
/// How far, in logical pixels, a press may move and still be a tap.
pub const DEFAULT_SLOP: f32 = 8.0;

/// The interaction state a tappable is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Status {
    #[default]
    Idle,
    /// A mouse is over it. Never shown after a touch until the mouse moves.
    Hovered,
    /// A finger or button is down on it.
    Pressed,
}

impl Status {
    pub const ALL: [Status; 3] = [Status::Idle, Status::Hovered, Status::Pressed];
}

/// What a finished interaction means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    Tap,
    LongPress,
}

/// The finger or mouse button that started a press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pointer {
    Finger(Finger),
    Mouse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Press {
    pointer: Pointer,
    origin: Point,
    at: Instant,
    held: bool,
}

/// The gesture logic of a tappable, free of any widget, so it can be
/// driven and tested directly.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Tracker {
    press: Option<Press>,
    touched: bool,
}

impl Tracker {
    /// A press started at `position`. A second finger while one is down
    /// cancels instead, since two fingers are a pinch rather than a tap.
    pub fn press(&mut self, pointer: Pointer, position: Point, now: Instant) {
        if let Pointer::Finger(_) = pointer {
            self.touched = true;
        }
        if self.press.is_some() {
            self.press = None;
            return;
        }
        self.press = Some(Press {
            pointer,
            origin: position,
            at: now,
            held: false,
        });
    }

    /// The pointer moved. Beyond `slop` from where it started, the press is
    /// cancelled.
    pub fn moved(&mut self, pointer: Pointer, position: Point, slop: f32) {
        let Some(press) = self.press else {
            return;
        };
        if press.pointer == pointer && press.origin.distance(position) > slop {
            self.press = None;
        }
    }

    /// The pointer was lifted. A press that was not held and ends `inside`
    /// is a tap.
    pub fn release(&mut self, pointer: Pointer, inside: bool) -> Option<Gesture> {
        let press = self.press.filter(|press| press.pointer == pointer)?;
        self.press = None;
        (!press.held && inside).then_some(Gesture::Tap)
    }

    /// The pointer went away without a release.
    pub fn lost(&mut self, pointer: Pointer) {
        if self.press.is_some_and(|press| press.pointer == pointer) {
            self.press = None;
        }
    }

    /// Checks the clock. Once a press has been held for `hold`, it becomes
    /// a long press, reported once, and its release is then ignored.
    pub fn tick(&mut self, now: Instant, hold: Duration) -> Option<Gesture> {
        let press = self.press.as_mut()?;
        if press.held || now.saturating_duration_since(press.at) < hold {
            return None;
        }
        press.held = true;
        Some(Gesture::LongPress)
    }

    /// When the current press becomes a long press, if one is pending.
    pub fn deadline(&self, hold: Duration) -> Option<Instant> {
        let press = self.press.filter(|press| !press.held)?;
        Some(press.at + hold)
    }

    /// A real mouse moved, so hover can be shown again.
    pub fn mouse_moved(&mut self) {
        self.touched = false;
    }

    pub fn is_pressed(&self) -> bool {
        self.press.is_some()
    }

    /// The status to draw, given whether the cursor is over the bounds. A
    /// touch leaves the cursor where the finger was, so hover is ignored
    /// after a touch until the mouse moves.
    pub fn status(&self, cursor_over: bool) -> Status {
        if self.press.is_some() {
            return Status::Pressed;
        }
        if cursor_over && !self.touched {
            return Status::Hovered;
        }
        Status::Idle
    }
}

/// The default look: transparent at rest, a light tint on hover and a
/// stronger one while pressed.
pub fn style(tokens: &Tokens, status: Status) -> container::Style {
    let background = match status {
        Status::Idle => None,
        Status::Hovered => Some(mix(tokens.background, tokens.accent, 0.5)),
        Status::Pressed => Some(tokens.accent),
    };
    container::Style {
        background: background.map(Background::Color),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius::MD.into(),
        },
        ..container::Style::default()
    }
}

type StyleFn<'a> = Box<dyn Fn(&Theme, Status) -> container::Style + 'a>;

/// A tappable builder. Convert it into an [`Element`] to render.
///
/// Without [`on_tap`](Tappable::on_tap) or
/// [`on_long_press`](Tappable::on_long_press) it draws its content and
/// nothing else.
pub struct Tappable<'a, Message> {
    content: Element<'a, Message>,
    on_tap: Option<Message>,
    on_long_press: Option<Message>,
    hold: Duration,
    slop: f32,
    style: StyleFn<'a>,
    width: Length,
    height: Length,
    padding: Padding,
}

/// Wraps `content` so it reports taps and long presses.
pub fn tappable<'a, Message>(content: impl Into<Element<'a, Message>>) -> Tappable<'a, Message> {
    Tappable {
        content: content.into(),
        on_tap: None,
        on_long_press: None,
        hold: DEFAULT_HOLD,
        slop: DEFAULT_SLOP,
        style: Box::new(|theme, status| style(&Tokens::of(theme), status)),
        width: Length::Shrink,
        height: Length::Shrink,
        padding: Padding::ZERO,
    }
}

impl<Message> fmt::Debug for Tappable<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tappable")
            .field("on_tap", &self.on_tap.is_some())
            .field("on_long_press", &self.on_long_press.is_some())
            .field("hold", &self.hold)
            .field("slop", &self.slop)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("padding", &self.padding)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> Tappable<'a, Message> {
    /// The message for a tap or a left click.
    pub fn on_tap(mut self, message: Message) -> Self {
        self.on_tap = Some(message);
        self
    }

    /// The message for a press held for [`hold`](Self::hold), or a right
    /// click.
    pub fn on_long_press(mut self, message: Message) -> Self {
        self.on_long_press = Some(message);
        self
    }

    /// How long a press is held to count as a long press. Defaults to
    /// [`DEFAULT_HOLD`].
    pub fn hold(mut self, hold: Duration) -> Self {
        self.hold = hold;
        self
    }

    /// How far a press may move and still be a tap. Defaults to
    /// [`DEFAULT_SLOP`].
    pub fn slop(mut self, slop: f32) -> Self {
        self.slop = slop.max(0.0);
        self
    }

    /// Replaces the default [`style`].
    pub fn style(mut self, style: impl Fn(&Theme, Status) -> container::Style + 'a) -> Self {
        self.style = Box::new(style);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_tap.is_some() || self.on_long_press.is_some()
    }
}

impl<'a, Message: Clone + 'a> From<Tappable<'a, Message>> for Element<'a, Message> {
    fn from(tappable: Tappable<'a, Message>) -> Self {
        let Tappable {
            content,
            on_tap,
            on_long_press,
            hold,
            slop,
            style,
            width,
            height,
            padding,
        } = tappable;
        Element::new(Surface {
            content: container(content)
                .width(width)
                .height(height)
                .padding(padding)
                .into(),
            on_tap,
            on_long_press,
            hold,
            slop,
            style,
        })
    }
}

struct Surface<'a, Message> {
    content: Element<'a, Message>,
    on_tap: Option<Message>,
    on_long_press: Option<Message>,
    hold: Duration,
    slop: f32,
    style: StyleFn<'a>,
}

#[derive(Debug, Default)]
struct State {
    tracker: Tracker,
    drawn: Status,
}

impl<Message> Surface<'_, Message> {
    fn is_enabled(&self) -> bool {
        self.on_tap.is_some() || self.on_long_press.is_some()
    }
}

/// Whether the cursor is over the part of `bounds` that is on screen.
fn over(cursor: mouse::Cursor, bounds: Rectangle, viewport: &Rectangle) -> bool {
    cursor.is_over(bounds) && cursor.is_over(*viewport)
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Surface<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
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
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let status = if self.is_enabled() {
            state.tracker.status(over(cursor, bounds, viewport))
        } else {
            Status::Idle
        };
        let appearance = (self.style)(theme, status);
        container::draw_background(renderer, &appearance, bounds);
        let style = renderer::Style {
            text_color: appearance.text_color.unwrap_or(style.text_color),
        };
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &style,
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
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
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
        if !self.is_enabled() {
            return;
        }

        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        let inside = over(cursor, bounds, viewport);
        let free = !shell.is_event_captured();
        let tracker = &mut state.tracker;
        let mut gesture = None;
        let mut now = Instant::now();

        match event {
            Event::Touch(touch::Event::FingerPressed { id, position }) => {
                if tracker.is_pressed() || (inside && free) {
                    tracker.press(Pointer::Finger(*id), *position, now);
                }
            }
            Event::Touch(touch::Event::FingerMoved { id, position }) => {
                tracker.moved(Pointer::Finger(*id), *position, self.slop);
            }
            Event::Touch(touch::Event::FingerLifted { id, .. }) => {
                gesture = tracker.release(Pointer::Finger(*id), true);
            }
            Event::Touch(touch::Event::FingerLost { id, .. }) => {
                tracker.lost(Pointer::Finger(*id));
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                tracker.mouse_moved();
                if let Some(position) = cursor.position() {
                    tracker.moved(Pointer::Mouse, position, self.slop);
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) => tracker.lost(Pointer::Mouse),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if inside && free => {
                if let Some(position) = cursor.position() {
                    tracker.press(Pointer::Mouse, position, now);
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                gesture = tracker.release(Pointer::Mouse, inside);
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
                if inside && free && self.on_long_press.is_some() =>
            {
                gesture = Some(Gesture::LongPress);
                shell.capture_event();
            }
            Event::Window(window::Event::RedrawRequested(at)) => now = *at,
            _ => {}
        }

        if gesture.is_none() && self.on_long_press.is_some() {
            gesture = tracker.tick(now, self.hold);
        }
        if self.on_long_press.is_some()
            && let Some(deadline) = tracker.deadline(self.hold)
        {
            shell.request_redraw_at(deadline);
        }

        let message = match gesture {
            Some(Gesture::Tap) => self.on_tap.clone(),
            Some(Gesture::LongPress) => self.on_long_press.clone(),
            None => None,
        };
        if let Some(message) = message {
            shell.publish(message);
        }

        let status = tracker.status(inside);
        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            state.drawn = status;
        } else if status != state.drawn {
            shell.request_redraw();
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
        let content = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        );
        if content != mouse::Interaction::None {
            return content;
        }
        if self.is_enabled() && over(cursor, layout.bounds(), viewport) {
            return mouse::Interaction::Pointer;
        }
        content
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
    use crate::theme::{dark, light};

    const FINGER: Pointer = Pointer::Finger(Finger(1));

    fn at(x: f32) -> Point {
        Point::new(x, 10.0)
    }

    #[test]
    fn builder_defaults() {
        let t: Tappable<'_, u8> = tappable("Row");
        assert_eq!(t.hold, DEFAULT_HOLD);
        assert_eq!(t.slop, DEFAULT_SLOP);
        assert_eq!(t.width, Length::Shrink);
        assert!(!t.is_enabled());
        assert!(tappable::<u8>("Row").on_tap(1).is_enabled());
        assert!(tappable::<u8>("Row").on_long_press(1).is_enabled());
        assert_eq!(tappable::<u8>("Row").slop(-3.0).slop, 0.0);
    }

    #[test]
    fn a_press_and_lift_within_the_slop_is_a_tap() {
        let mut tracker = Tracker::default();
        let now = Instant::now();
        tracker.press(FINGER, at(0.0), now);
        assert_eq!(tracker.status(true), Status::Pressed);
        tracker.moved(FINGER, at(DEFAULT_SLOP), DEFAULT_SLOP);
        assert_eq!(tracker.release(FINGER, true), Some(Gesture::Tap));
        assert_eq!(tracker.status(true), Status::Idle);
    }

    #[test]
    fn moving_beyond_the_slop_cancels() {
        let mut tracker = Tracker::default();
        tracker.press(FINGER, at(0.0), Instant::now());
        tracker.moved(FINGER, at(DEFAULT_SLOP + 1.0), DEFAULT_SLOP);
        assert!(!tracker.is_pressed());
        assert_eq!(tracker.release(FINGER, true), None);
    }

    #[test]
    fn other_fingers_are_ignored_and_a_second_press_cancels() {
        let mut tracker = Tracker::default();
        tracker.press(FINGER, at(0.0), Instant::now());
        tracker.moved(Pointer::Finger(Finger(2)), at(100.0), DEFAULT_SLOP);
        assert_eq!(tracker.release(Pointer::Finger(Finger(2)), true), None);
        assert!(tracker.is_pressed());
        tracker.press(Pointer::Finger(Finger(2)), at(50.0), Instant::now());
        assert!(!tracker.is_pressed());
    }

    #[test]
    fn holding_fires_one_long_press_and_swallows_the_lift() {
        let mut tracker = Tracker::default();
        let start = Instant::now();
        tracker.press(FINGER, at(0.0), start);
        assert_eq!(tracker.deadline(DEFAULT_HOLD), Some(start + DEFAULT_HOLD));
        assert_eq!(tracker.tick(start, DEFAULT_HOLD), None);
        let later = start + DEFAULT_HOLD;
        assert_eq!(tracker.tick(later, DEFAULT_HOLD), Some(Gesture::LongPress));
        assert_eq!(tracker.tick(later, DEFAULT_HOLD), None);
        assert_eq!(tracker.deadline(DEFAULT_HOLD), None);
        assert_eq!(tracker.release(FINGER, true), None);
    }

    #[test]
    fn lost_and_released_outside_produce_nothing() {
        let mut tracker = Tracker::default();
        tracker.press(FINGER, at(0.0), Instant::now());
        tracker.lost(FINGER);
        assert!(!tracker.is_pressed());
        tracker.press(Pointer::Mouse, at(0.0), Instant::now());
        assert_eq!(tracker.release(Pointer::Mouse, false), None);
    }

    #[test]
    fn hover_is_ignored_after_a_touch_until_the_mouse_moves() {
        let mut tracker = Tracker::default();
        assert_eq!(tracker.status(true), Status::Hovered);
        tracker.press(FINGER, at(0.0), Instant::now());
        let _ = tracker.release(FINGER, true);
        assert_eq!(tracker.status(true), Status::Idle);
        tracker.mouse_moved();
        assert_eq!(tracker.status(true), Status::Hovered);
        assert_eq!(tracker.status(false), Status::Idle);
    }

    #[test]
    fn default_style_is_clear_at_rest_and_stronger_when_pressed() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert!(style(&tokens, Status::Idle).background.is_none());
            let hovered = style(&tokens, Status::Hovered).background;
            let pressed = style(&tokens, Status::Pressed).background;
            assert!(hovered.is_some() && pressed.is_some());
            assert_ne!(hovered, pressed);
            assert_eq!(pressed, Some(Background::Color(tokens.accent)));
        }
    }
}
