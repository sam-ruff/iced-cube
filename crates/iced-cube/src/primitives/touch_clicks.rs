//! Lets content that only reads the mouse, such as a text editor or rich
//! text with links, respond to touch.
//!
//! The content gets every event unchanged. In addition, the first finger
//! is translated for the content alone:
//!
//! | Finger | The content also gets |
//! | --- | --- |
//! | Pressed and lifted within the slop | The cursor moved to the touch point, then a left press and release there |
//! | Moved beyond the slop | Nothing, so a scrollable around it scrolls as usual |
//! | Moved beyond the slop, with [`scroll`](TouchClicks::scroll) on | Wheel scrolling that follows the finger, and the moves are captured |
//!
//! A tap anywhere in the window is translated, so a tap outside a focused
//! editor unfocuses it, as a click would. The touch press and lift are
//! never captured, and a second finger cancels the tap.
//!
//! Wrap only the content that needs it. Widgets that already read touch,
//! such as buttons, would react twice if they were wrapped too, and so
//! would content on a platform that also turns touches into mouse events.
//!
//! ```no_run
//! use iced::widget::{rich_text, span};
//! use iced_cube::touch_clicks;
//!
//! #[derive(Debug, Clone)]
//! enum Message { Open(&'static str) }
//!
//! let text: iced::Element<'_, Message> = touch_clicks(
//!     rich_text![span("Read the "), span("guide").link("guide")]
//!         .on_link_click(Message::Open),
//! )
//! .into();
//! ```

use std::fmt;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::touch::{self, Finger};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

/// How far, in logical pixels, a finger may move and still be a tap.
pub const DEFAULT_SLOP: f32 = 8.0;
/// The line height a scroll-mode drag is measured against: that of a
/// textarea's 14 pixel text at 1.5 line height.
pub const DEFAULT_LINE_HEIGHT: f32 = 21.0;
/// How many wheel pixels a text editor reads as one line.
const WHEEL_PIXELS_PER_LINE: f32 = 4.0;

/// The wheel delta that scrolls a text editor with lines `line_height`
/// tall by the distance a finger moved, so the text follows the finger.
pub fn wheel_delta(finger_dy: f32, line_height: f32) -> mouse::ScrollDelta {
    let line_height = line_height.max(1.0);
    mouse::ScrollDelta::Pixels {
        x: 0.0,
        y: finger_dy * WHEEL_PIXELS_PER_LINE / line_height,
    }
}

/// What a finger move meant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Move {
    /// Not the tracked finger.
    Ignored,
    /// The tracked finger, still within the slop.
    Held,
    /// The tracked finger is dragging. `dy` is the vertical distance since
    /// the last move; `inside` is whether the drag started on the content.
    Dragged { dy: f32, inside: bool },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Track {
    finger: Finger,
    origin: Point,
    last: Point,
    target: mouse::Cursor,
    inside: bool,
    dragging: bool,
    cancelled: bool,
}

/// The finger logic of [`TouchClicks`], free of any widget.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Translator {
    track: Option<Track>,
}

impl Translator {
    /// A finger went down at `position` in window coordinates. `target` is
    /// the cursor a click would reach the content with, in the content's
    /// coordinates, and `inside` whether it is over the content. A layer
    /// above the content hides the cursor from it, as it would a mouse.
    pub fn press(&mut self, finger: Finger, position: Point, target: mouse::Cursor, inside: bool) {
        if let Some(track) = self.track.as_mut() {
            track.cancelled = true;
            return;
        }
        self.track = Some(Track {
            finger,
            origin: position,
            last: position,
            target,
            inside,
            dragging: false,
            cancelled: false,
        });
    }

    pub fn moved(&mut self, finger: Finger, position: Point, slop: f32) -> Move {
        let Some(track) = self.track.as_mut().filter(|track| track.finger == finger) else {
            return Move::Ignored;
        };
        if !track.dragging && track.origin.distance(position) > slop {
            track.dragging = true;
        }
        let dy = position.y - track.last.y;
        track.last = position;
        if track.dragging {
            Move::Dragged {
                dy,
                inside: track.inside,
            }
        } else {
            Move::Held
        }
    }

    /// The finger lifted. Returns the cursor to click with when it was a
    /// tap.
    pub fn lift(&mut self, finger: Finger) -> Option<mouse::Cursor> {
        let track = self.track.filter(|track| track.finger == finger)?;
        self.track = None;
        (!track.dragging && !track.cancelled).then_some(track.target)
    }

    pub fn lost(&mut self, finger: Finger) {
        if self.track.is_some_and(|track| track.finger == finger) {
            self.track = None;
        }
    }

    /// Whether the tracked finger started over the content.
    pub fn is_inside(&self, finger: Finger) -> bool {
        self.track
            .is_some_and(|track| track.finger == finger && track.inside)
    }
}

/// A touch-clicks builder. Convert it into an [`Element`] to render.
pub struct TouchClicks<'a, Message> {
    content: Element<'a, Message>,
    slop: f32,
    scroll: bool,
    line_height: f32,
}

/// Wraps mouse-only `content` so taps reach it as clicks.
pub fn touch_clicks<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> TouchClicks<'a, Message> {
    TouchClicks {
        content: content.into(),
        slop: DEFAULT_SLOP,
        scroll: false,
        line_height: DEFAULT_LINE_HEIGHT,
    }
}

impl<Message> fmt::Debug for TouchClicks<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TouchClicks")
            .field("slop", &self.slop)
            .field("scroll", &self.scroll)
            .field("line_height", &self.line_height)
            .finish_non_exhaustive()
    }
}

impl<Message> TouchClicks<'_, Message> {
    /// How far a finger may move and still be a tap. Defaults to
    /// [`DEFAULT_SLOP`].
    pub fn slop(mut self, slop: f32) -> Self {
        self.slop = slop.max(0.0);
        self
    }

    /// For content that scrolls itself, such as a text editor: a vertical
    /// drag that starts on it becomes wheel scrolling, and the finger's
    /// moves are captured so a scrollable around it stays put.
    pub fn scroll(mut self, scroll: bool) -> Self {
        self.scroll = scroll;
        self
    }

    /// The content's line height, so a scroll-mode drag moves its text
    /// with the finger. Defaults to [`DEFAULT_LINE_HEIGHT`].
    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height.max(1.0);
        self
    }
}

impl<'a, Message: 'a> From<TouchClicks<'a, Message>> for Element<'a, Message> {
    fn from(clicks: TouchClicks<'a, Message>) -> Self {
        Element::new(Wrapper { clicks })
    }
}

struct Wrapper<'a, Message> {
    clicks: TouchClicks<'a, Message>,
}

impl<Message> Wrapper<'_, Message> {
    /// Sends synthetic events to the content with `cursor`. They get their
    /// own shell, so nothing they capture marks the real touch event as
    /// captured.
    #[allow(clippy::too_many_arguments)]
    fn deliver(
        &mut self,
        tree: &mut Tree,
        events: &[Event],
        cursor: mouse::Cursor,
        layout: Layout<'_>,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let mut messages = Vec::new();
        let mut inner = Shell::new(&mut messages);
        for event in events {
            self.clicks.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                &mut inner,
                viewport,
            );
        }
        let layout_invalid = inner.is_layout_invalid();
        let widgets_invalid = inner.are_widgets_invalid();
        let redraw = inner.redraw_request();
        for message in messages {
            shell.publish(message);
        }
        if layout_invalid {
            shell.invalidate_layout();
        }
        if widgets_invalid {
            shell.invalidate_widgets();
        }
        shell.request_redraw_at(redraw);
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Wrapper<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Translator>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Translator::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.clicks.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.clicks.content));
    }

    fn size(&self) -> Size<Length> {
        self.clicks.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.clicks.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.clicks
            .content
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
        self.clicks.content.as_widget().draw(
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
        self.clicks.content.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
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
        self.clicks.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let Event::Touch(touch) = event else {
            return;
        };
        let bounds = layout.bounds();
        match *touch {
            touch::Event::FingerPressed { id, position } => {
                let inside = cursor
                    .position()
                    .is_some_and(|target| bounds.contains(target) && viewport.contains(target));
                tree.state
                    .downcast_mut::<Translator>()
                    .press(id, position, cursor, inside);
            }
            touch::Event::FingerMoved { id, position } => {
                let translator = tree.state.downcast_mut::<Translator>();
                let inside = translator.is_inside(id);
                let change = translator.moved(id, position, self.clicks.slop);
                if !self.clicks.scroll || !inside || change == Move::Ignored {
                    return;
                }
                shell.capture_event();
                let Move::Dragged { dy, .. } = change else {
                    return;
                };
                let at = cursor
                    .position()
                    .filter(|position| bounds.contains(*position))
                    .unwrap_or(bounds.center());
                let wheel = Event::Mouse(mouse::Event::WheelScrolled {
                    delta: wheel_delta(dy, self.clicks.line_height),
                });
                self.deliver(
                    tree,
                    &[wheel],
                    mouse::Cursor::Available(at),
                    layout,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
            touch::Event::FingerLifted { id, .. } => {
                let Some(target) = tree.state.downcast_mut::<Translator>().lift(id) else {
                    return;
                };
                let moved = target
                    .position()
                    .map(|position| Event::Mouse(mouse::Event::CursorMoved { position }));
                let click: Vec<Event> = moved
                    .into_iter()
                    .chain([
                        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                    ])
                    .collect();
                self.deliver(
                    tree, &click, target, layout, renderer, clipboard, shell, viewport,
                );
            }
            touch::Event::FingerLost { id, .. } => {
                tree.state.downcast_mut::<Translator>().lost(id);
            }
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
        self.clicks.content.as_widget().mouse_interaction(
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
        self.clicks.content.as_widget_mut().overlay(
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

    const ONE: Finger = Finger(1);

    fn at(y: f32) -> Point {
        Point::new(10.0, y)
    }

    #[test]
    fn builder_defaults_and_clamps() {
        let clicks: TouchClicks<'_, ()> = touch_clicks("Text");
        assert_eq!(clicks.slop, DEFAULT_SLOP);
        assert!(!clicks.scroll);
        assert_eq!(clicks.line_height, DEFAULT_LINE_HEIGHT);
        let clamped: TouchClicks<'_, ()> = touch_clicks("Text").slop(-1.0).line_height(0.0);
        assert_eq!(clamped.slop, 0.0);
        assert_eq!(clamped.line_height, 1.0);
    }

    fn cursor(y: f32) -> mouse::Cursor {
        mouse::Cursor::Available(at(y))
    }

    #[test]
    fn a_tap_clicks_where_the_content_was_touched() {
        let mut translator = Translator::default();
        translator.press(ONE, at(0.0), cursor(40.0), true);
        assert_eq!(
            translator.moved(ONE, at(DEFAULT_SLOP), DEFAULT_SLOP),
            Move::Held
        );
        assert_eq!(translator.lift(ONE), Some(cursor(40.0)));
        assert_eq!(translator.lift(ONE), None);
    }

    #[test]
    fn a_tap_on_hidden_content_clicks_without_a_position() {
        let mut translator = Translator::default();
        let hidden = mouse::Cursor::Levitating(at(40.0));
        translator.press(ONE, at(40.0), hidden, false);
        assert_eq!(translator.lift(ONE), Some(hidden));
    }

    #[test]
    fn a_drag_is_not_a_click_and_reports_its_movement() {
        let mut translator = Translator::default();
        translator.press(ONE, at(0.0), cursor(0.0), true);
        assert_eq!(
            translator.moved(ONE, at(20.0), DEFAULT_SLOP),
            Move::Dragged {
                dy: 20.0,
                inside: true
            }
        );
        assert_eq!(
            translator.moved(ONE, at(15.0), DEFAULT_SLOP),
            Move::Dragged {
                dy: -5.0,
                inside: true
            }
        );
        assert_eq!(translator.lift(ONE), None);
    }

    #[test]
    fn a_second_finger_cancels_and_others_are_ignored() {
        let mut translator = Translator::default();
        translator.press(ONE, at(0.0), cursor(0.0), true);
        assert_eq!(
            translator.moved(Finger(2), at(90.0), DEFAULT_SLOP),
            Move::Ignored
        );
        translator.press(Finger(2), at(0.0), cursor(0.0), true);
        assert_eq!(translator.lift(Finger(2)), None);
        assert_eq!(translator.lift(ONE), None);

        translator.press(ONE, at(0.0), cursor(0.0), false);
        assert!(!translator.is_inside(ONE));
        translator.lost(ONE);
        assert_eq!(translator.lift(ONE), None);
    }

    #[test]
    fn wheel_deltas_follow_the_finger_in_lines() {
        let mouse::ScrollDelta::Pixels { x, y } = wheel_delta(-21.0, 21.0) else {
            panic!("expected pixels");
        };
        assert_eq!(x, 0.0);
        assert_eq!(y, -WHEEL_PIXELS_PER_LINE);
        let mouse::ScrollDelta::Pixels { y, .. } = wheel_delta(10.0, 0.0) else {
            panic!("expected pixels");
        };
        assert_eq!(y, 40.0);
    }
}
