//! A small label shown while the pointer rests on an element, or while
//! something inside the element has keyboard focus.
//!
//! A finger shows it while held. Once the finger lifts, the bubble stays
//! closed until a mouse is used, so a tap on a phone does not leave it open.
//!
//! iced has no general focus model, so the tooltip follows the widgets that
//! report focus to operations: text fields, text editors and the focusable
//! parts of this library. Focus that arrives with a pointer press, such as
//! clicking into a field, does not show the bubble.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::time::{Duration, Instant};
use iced::widget::{self, container, text};
use iced::{
    Background, Border, Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Theme,
    Vector, mouse, touch, window,
};

use crate::after_touch::after_touch;
use crate::theme::{Tokens, radius, space, text_size};

/// Where the bubble appears relative to its element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Position {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
    /// Follows the pointer. A bubble shown for keyboard focus sits on top.
    FollowCursor,
}

impl Position {
    pub const ALL: [Position; 5] = [
        Position::Top,
        Position::Bottom,
        Position::Left,
        Position::Right,
        Position::FollowCursor,
    ];
}

/// Gap between the element and the bubble's margin, in logical pixels.
pub const GAP: f32 = 6.0;

/// The bubble's margin: the closest it comes to the window edges, and part
/// of the space between it and its element.
pub const EDGE: f32 = 5.0;

/// Labels longer than this many characters are sentences rather than
/// names, and read at [`text_size::SM`] instead of [`text_size::XS`].
pub const LONG_LABEL: usize = 40;

/// The text size of the bubble: `explicit` when set, otherwise small for a
/// short label and a size up for a longer one.
pub fn label_size(label: &str, explicit: Option<f32>) -> f32 {
    if let Some(size) = explicit {
        return size;
    }
    if label.chars().count() > LONG_LABEL {
        return text_size::SM;
    }
    text_size::XS
}

/// A tooltip builder. Convert it into an [`Element`] to render.
pub struct Tooltip<'a, Message> {
    content: Element<'a, Message>,
    label: text::Fragment<'a>,
    position: Position,
    delay: Duration,
    max_width: Option<f32>,
    text_size: Option<f32>,
}

impl<Message> std::fmt::Debug for Tooltip<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tooltip")
            .field("label", &self.label)
            .field("position", &self.position)
            .field("delay", &self.delay)
            .field("max_width", &self.max_width)
            .field("text_size", &self.text_size)
            .finish_non_exhaustive()
    }
}

/// Shows `label` in a bubble while the pointer is over `content`, or while
/// something inside `content` has keyboard focus.
pub fn tooltip<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    label: impl text::IntoFragment<'a>,
) -> Tooltip<'a, Message> {
    Tooltip {
        content: content.into(),
        label: label.into_fragment(),
        position: Position::default(),
        delay: Duration::ZERO,
        max_width: None,
        text_size: None,
    }
}

impl<'a, Message> Tooltip<'a, Message> {
    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// How long the pointer has to rest before the bubble appears. Keyboard
    /// focus shows it straight away.
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// The widest the bubble grows, in logical pixels. A longer label wraps
    /// onto more lines. Without it the bubble is as wide as the label, up
    /// to the width of the window.
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// The label's text size. Defaults to [`label_size`]: small for a few
    /// words and a size up for a sentence.
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = Some(size);
        self
    }
}

impl<'a, Message: 'a> From<Tooltip<'a, Message>> for Element<'a, Message> {
    fn from(tooltip: Tooltip<'a, Message>) -> Self {
        let size = label_size(&tooltip.label, tooltip.text_size);
        #[cfg(feature = "icedwright")]
        let label = tooltip.label.clone();
        let mut bubble = container(text(tooltip.label).size(size))
            .padding([space::XS + 2.0, space::MD])
            .style(|theme| style(&Tokens::of(theme)));
        if let Some(width) = tooltip.max_width {
            bubble = bubble.max_width(width);
        }
        let bubble: Element<'a, Message> = bubble.into();
        #[cfg(feature = "icedwright")]
        let bubble = icedwright_probe::semantic(bubble)
            .role(icedwright_probe::role::Tooltip)
            .name(label.clone())
            .component("iced_cube::tooltip")
            .into();

        after_touch(Element::new(Tip {
            content: tooltip.content,
            bubble,
            position: tooltip.position,
            delay: tooltip.delay,
            #[cfg(feature = "icedwright")]
            label,
        }))
    }
}

/// The bubble style: inverted colours so it stands out from the page.
pub fn style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.primary)),
        text_color: Some(tokens.primary_foreground),
        border: Border {
            radius: radius::MD.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// How focus reached the content, if it has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    None,
    /// Focus came with a pointer press, so the bubble stays hidden.
    Pointer,
    /// Focus came any other way, such as Tab, and shows the bubble.
    Keyboard,
}

impl Focus {
    /// The focus after an event, given whether the content is `focused`
    /// now and whether the event was a pointer press.
    pub fn next(self, focused: bool, pointer_press: bool) -> Focus {
        if !focused {
            return Focus::None;
        }
        if pointer_press {
            return Focus::Pointer;
        }
        if self == Focus::None {
            return Focus::Keyboard;
        }
        self
    }
}

/// Whether the pointer has shown the bubble.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Hover {
    #[default]
    Idle,
    Resting {
        at: Instant,
    },
    Open {
        cursor: Point,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct State {
    hover: Hover,
    focus: Focus,
}

impl State {
    fn shown(&self) -> bool {
        matches!(self.hover, Hover::Open { .. }) || self.focus == Focus::Keyboard
    }

    fn cursor(&self) -> Option<Point> {
        match self.hover {
            Hover::Open { cursor } => Some(cursor),
            _ => None,
        }
    }
}

fn is_pointer_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Touch(touch::Event::FingerPressed { .. })
    )
}

/// Whether focus may have changed with `event`. Pointer movement never
/// moves focus, so it skips the check.
fn may_move_focus(event: &Event) -> bool {
    !matches!(
        event,
        Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::WheelScrolled { .. })
            | Event::Touch(touch::Event::FingerMoved { .. })
    )
}

/// Finds out whether any widget it visits has focus.
#[derive(Debug, Default)]
struct Probe {
    focused: bool,
}

impl Operation for Probe {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn Focusable,
    ) {
        self.focused |= state.is_focused();
    }
}

struct Tip<'a, Message> {
    content: Element<'a, Message>,
    bubble: Element<'a, Message>,
    position: Position,
    delay: Duration,
    #[cfg(feature = "icedwright")]
    label: text::Fragment<'a>,
}

impl<Message> Tip<'_, Message> {
    fn update_hover(
        &self,
        state: &mut State,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
    ) {
        if !matches!(
            event,
            Event::Mouse(_) | Event::Window(window::Event::RedrawRequested(_))
        ) {
            return;
        }
        let now = crate::time::now();
        let over = cursor.position_over(layout.bounds());

        match (state.hover, over) {
            (Hover::Idle, Some(position)) => {
                if self.delay == Duration::ZERO {
                    state.hover = Hover::Open { cursor: position };
                    shell.invalidate_layout();
                } else {
                    state.hover = Hover::Resting { at: now };
                }
                shell.request_redraw_at(now + self.delay);
            }
            (Hover::Resting { .. }, None) => state.hover = Hover::Idle,
            (Hover::Resting { at }, _) if now.saturating_duration_since(at) < self.delay => {
                shell.request_redraw_at(at + self.delay);
            }
            (Hover::Resting { .. }, Some(position)) => {
                state.hover = Hover::Open { cursor: position };
                shell.invalidate_layout();
            }
            (Hover::Open { cursor: last }, Some(position))
                if self.position == Position::FollowCursor && last != position =>
            {
                state.hover = Hover::Open { cursor: position };
                shell.request_redraw();
            }
            (Hover::Open { .. }, None) => {
                state.hover = Hover::Idle;
                shell.invalidate_layout();
                if !matches!(event, Event::Window(window::Event::RedrawRequested(_))) {
                    shell.request_redraw();
                }
            }
            (Hover::Open { .. }, Some(_)) | (Hover::Idle, None) => {}
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Tip<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content), Tree::new(&self.bubble)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.content, &self.bubble]);
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
        #[cfg(feature = "icedwright")]
        icedwright_probe::report(operation, layout.bounds(), || {
            icedwright_probe::Annotation::new()
                .tooltip(self.label.to_string())
                .component("iced_cube::tooltip")
        });
        #[cfg(not(feature = "icedwright"))]
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
        let state = tree.state.downcast_mut::<State>();
        let was_shown = state.shown();
        self.update_hover(state, event, layout, cursor, shell);

        let content = &mut tree.children[0];
        self.content.as_widget_mut().update(
            content, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
        if !may_move_focus(event) {
            return;
        }

        let mut probe = Probe::default();
        self.content
            .as_widget_mut()
            .operate(content, layout, renderer, &mut probe);
        let state = tree.state.downcast_mut::<State>();
        state.focus = state.focus.next(probe.focused, is_pointer_press(event));
        if state.shown() != was_shown {
            shell.invalidate_layout();
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
        let state = *tree.state.downcast_ref::<State>();
        let [content_tree, bubble_tree] = tree.children.as_mut_slice() else {
            return None;
        };

        let content = self.content.as_widget_mut().overlay(
            content_tree,
            layout,
            renderer,
            viewport,
            translation,
        );
        let bubble = state.shown().then(|| {
            overlay::Element::new(Box::new(Bubble {
                position: layout.position() + translation,
                content_bounds: layout.bounds(),
                cursor: state.cursor(),
                placement: self.position,
                bubble: &mut self.bubble,
                tree: bubble_tree,
            }))
        });

        let children: Vec<_> = content.into_iter().chain(bubble).collect();
        (!children.is_empty()).then(|| overlay::Group::with_children(children).overlay())
    }
}

/// Where the bubble goes: its top left corner, given the content's
/// position and bounds, the bubble's size and the pointer, before it is
/// kept inside the window.
fn place(
    placement: Position,
    position: Point,
    content: Rectangle,
    bubble: Size,
    cursor: Option<Point>,
) -> Point {
    let x_centre = position.x + (content.width - bubble.width) / 2.0;
    let y_centre = position.y + (content.height - bubble.height) / 2.0;
    let gap = GAP + EDGE;
    let above = Point::new(x_centre, position.y - bubble.height - gap);

    match (placement, cursor) {
        (Position::Top, _) | (Position::FollowCursor, None) => above,
        (Position::Bottom, _) => Point::new(x_centre, position.y + content.height + gap),
        (Position::Left, _) => Point::new(position.x - bubble.width - gap, y_centre),
        (Position::Right, _) => Point::new(position.x + content.width + gap, y_centre),
        (Position::FollowCursor, Some(cursor)) => {
            let shift = position - content.position();
            Point::new(cursor.x, cursor.y - bubble.height) + shift
        }
    }
}

/// The side the bubble goes on: `placement`, or the opposite side when
/// `placement` would run out of `viewport` and the opposite would not, so
/// a bubble near an edge is never pushed back over its own control.
fn side_that_fits(
    placement: Position,
    position: Point,
    content: Rectangle,
    bubble: Size,
    viewport: Rectangle,
) -> Position {
    // Only the side's own axis matters: sliding along the edge is left to
    // `keep_inside`.
    let fits = |side| {
        let corner = place(side, position, content, bubble, None);
        match side {
            Position::Top | Position::Bottom => {
                corner.y - EDGE >= viewport.y
                    && corner.y + bubble.height + EDGE <= viewport.y + viewport.height
            }
            Position::Left | Position::Right | Position::FollowCursor => {
                corner.x - EDGE >= viewport.x
                    && corner.x + bubble.width + EDGE <= viewport.x + viewport.width
            }
        }
    };
    let opposite = match placement {
        Position::Top => Position::Bottom,
        Position::Bottom => Position::Top,
        Position::Left => Position::Right,
        Position::Right => Position::Left,
        Position::FollowCursor => return placement,
    };
    if !fits(placement) && fits(opposite) {
        return opposite;
    }
    placement
}

/// Moves `bounds` back inside `viewport`, leaving it where it is if it
/// already fits.
fn keep_inside(mut bounds: Rectangle, viewport: Rectangle) -> Rectangle {
    if bounds.x < viewport.x {
        bounds.x = viewport.x;
    } else if viewport.x + viewport.width < bounds.x + bounds.width {
        bounds.x = viewport.x + viewport.width - bounds.width;
    }
    if bounds.y < viewport.y {
        bounds.y = viewport.y;
    } else if viewport.y + viewport.height < bounds.y + bounds.height {
        bounds.y = viewport.y + viewport.height - bounds.height;
    }
    bounds
}

struct Bubble<'a, 'b, Message> {
    position: Point,
    content_bounds: Rectangle,
    cursor: Option<Point>,
    placement: Position,
    bubble: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
}

impl<Message> overlay::Overlay<Message, Theme, Renderer> for Bubble<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);
        let limits = layout::Limits::new(Size::ZERO, viewport.size()).shrink(Padding::new(EDGE));
        let node = self
            .bubble
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);
        let size = node.size();

        let side = side_that_fits(
            self.placement,
            self.position,
            self.content_bounds,
            size,
            viewport,
        );
        let corner = place(side, self.position, self.content_bounds, size, self.cursor);
        // The edge margin surrounds the bubble, so it never touches the
        // window border once kept inside.
        let framed = Rectangle {
            x: corner.x - EDGE,
            y: corner.y - EDGE,
            width: size.width + EDGE * 2.0,
            height: size.height + EDGE * 2.0,
        };
        let framed = keep_inside(framed, viewport);

        layout::Node::with_children(framed.size(), vec![node.translate(Vector::new(EDGE, EDGE))])
            .translate(Vector::new(framed.x, framed.y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let Some(bubble) = layout.children().next() else {
            return;
        };
        self.bubble.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            bubble,
            cursor,
            &Rectangle::with_size(Size::INFINITE),
        );
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        let Some(bubble) = layout.children().next() else {
            return;
        };
        self.bubble
            .as_widget_mut()
            .operate(self.tree, bubble, renderer, operation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn defaults_to_top_without_delay() {
        let t: Tooltip<'_, ()> = tooltip(text("Hover"), "Hint");
        assert_eq!(t.position, Position::Top);
        assert_eq!(t.delay, Duration::ZERO);
        assert_eq!(t.max_width, None);
        assert_eq!(t.text_size, None);
    }

    #[test]
    fn builder_sets_every_option() {
        let t: Tooltip<'_, ()> = tooltip(text("Hover"), "Hint")
            .position(Position::Left)
            .delay(Duration::from_millis(300))
            .max_width(240.0)
            .text_size(13.0);
        assert_eq!(t.position, Position::Left);
        assert_eq!(t.delay, Duration::from_millis(300));
        assert_eq!(t.max_width, Some(240.0));
        assert_eq!(t.text_size, Some(13.0));
    }

    #[test]
    fn bubble_inverts_the_page_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = style(&tokens);
            assert_eq!(style.background, Some(Background::Color(tokens.primary)));
            assert_eq!(style.text_color, Some(tokens.primary_foreground));
            assert_ne!(tokens.primary, tokens.background);
        }
    }

    #[test]
    fn short_labels_are_small_and_sentences_a_size_up() {
        assert_eq!(label_size("Bold", None), text_size::XS);
        assert_eq!(label_size(&"a".repeat(LONG_LABEL), None), text_size::XS);
        assert_eq!(label_size(&"a".repeat(LONG_LABEL + 1), None), text_size::SM);
        assert_eq!(label_size("Bold", Some(16.0)), 16.0);
        assert_eq!(label_size(&"a".repeat(80), Some(11.0)), 11.0);
    }

    #[test]
    fn keyboard_focus_shows_but_pointer_focus_does_not() {
        assert_eq!(Focus::None.next(true, false), Focus::Keyboard);
        assert_eq!(Focus::None.next(true, true), Focus::Pointer);
        assert_eq!(Focus::Pointer.next(true, false), Focus::Pointer);
        assert_eq!(Focus::Keyboard.next(true, false), Focus::Keyboard);
        assert_eq!(Focus::Keyboard.next(true, true), Focus::Pointer);
        for focus in [Focus::None, Focus::Pointer, Focus::Keyboard] {
            assert_eq!(focus.next(false, false), Focus::None);
            assert_eq!(focus.next(false, true), Focus::None);
        }
    }

    #[test]
    fn shown_while_hovered_or_keyboard_focused() {
        let hovered = State {
            hover: Hover::Open {
                cursor: Point::ORIGIN,
            },
            focus: Focus::None,
        };
        assert!(hovered.shown());
        assert_eq!(hovered.cursor(), Some(Point::ORIGIN));

        let focused = State {
            hover: Hover::Idle,
            focus: Focus::Keyboard,
        };
        assert!(focused.shown());
        assert_eq!(focused.cursor(), None);

        assert!(!State::default().shown());
        assert!(
            !State {
                focus: Focus::Pointer,
                ..State::default()
            }
            .shown()
        );
    }

    #[test]
    fn bubble_sits_on_the_requested_side() {
        let content = Rectangle::new(Point::new(100.0, 100.0), Size::new(40.0, 20.0));
        let bubble = Size::new(60.0, 24.0);
        let at = |placement, cursor| place(placement, content.position(), content, bubble, cursor);
        let gap = GAP + EDGE;

        assert_eq!(
            at(Position::Top, None),
            Point::new(90.0, 100.0 - 24.0 - gap)
        );
        assert_eq!(at(Position::Bottom, None), Point::new(90.0, 120.0 + gap));
        assert_eq!(
            at(Position::Left, None),
            Point::new(100.0 - 60.0 - gap, 98.0)
        );
        assert_eq!(at(Position::Right, None), Point::new(140.0 + gap, 98.0));
        assert_eq!(
            at(Position::FollowCursor, Some(Point::new(110.0, 105.0))),
            Point::new(110.0, 81.0)
        );
        assert_eq!(
            at(Position::FollowCursor, None),
            at(Position::Top, None),
            "keyboard focus has no pointer to follow"
        );
    }

    #[test]
    fn a_bubble_with_no_room_flips_to_the_other_side() {
        let window = Rectangle::with_size(Size::new(300.0, 200.0));
        let bubble = Size::new(60.0, 24.0);
        let side = |placement, content: Rectangle| {
            side_that_fits(placement, content.position(), content, bubble, window)
        };
        let middle = Rectangle::new(Point::new(120.0, 90.0), Size::new(40.0, 20.0));
        for placement in [
            Position::Top,
            Position::Bottom,
            Position::Left,
            Position::Right,
        ] {
            assert_eq!(side(placement, middle), placement);
        }
        let top_right = Rectangle::new(Point::new(256.0, 4.0), Size::new(40.0, 20.0));
        assert_eq!(side(Position::Top, top_right), Position::Bottom);
        assert_eq!(side(Position::Right, top_right), Position::Left);
        let bottom_left = Rectangle::new(Point::new(4.0, 176.0), Size::new(40.0, 20.0));
        assert_eq!(side(Position::Bottom, bottom_left), Position::Top);
        assert_eq!(side(Position::Left, bottom_left), Position::Right);
        assert_eq!(
            side(Position::FollowCursor, top_right),
            Position::FollowCursor
        );
    }

    #[test]
    fn bubble_is_kept_inside_the_window() {
        let window = Rectangle::with_size(Size::new(200.0, 100.0));
        let inside = Rectangle::new(Point::new(10.0, 10.0), Size::new(50.0, 20.0));
        assert_eq!(keep_inside(inside, window), inside);

        let left = Rectangle::new(Point::new(-20.0, -5.0), Size::new(50.0, 20.0));
        assert_eq!(keep_inside(left, window).position(), Point::ORIGIN);

        let right = Rectangle::new(Point::new(180.0, 90.0), Size::new(50.0, 20.0));
        assert_eq!(
            keep_inside(right, window).position(),
            Point::new(150.0, 80.0)
        );
    }
}
