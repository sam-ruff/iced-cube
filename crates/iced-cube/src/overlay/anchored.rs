//! Floats a panel next to an anchor element: the layer popovers and menus
//! are built on.
//!
//! [`anchored`] wraps an anchor, such as a trigger button, and while it has
//! content draws that content above everything else. [`place`] is the pure
//! positioning logic: it puts the panel on one [`Side`] of the anchor,
//! aligned by [`Align`], then flips and shifts it to stay inside the window.
//!
//! The panel is dismissed through one message, sent on Escape or on a press
//! outside both the panel and the anchor. Presses on the anchor are left to
//! the anchor, so a trigger can toggle the panel itself. Presses inside the
//! panel never reach the widgets underneath it.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, key::Named};
use iced::widget::container;
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Shadow, Size,
    Theme, Vector, mouse, touch,
};

use crate::theme::{Tokens, fade, radius};

/// Default space between the anchor and the panel, in logical pixels.
pub const GAP: f32 = 4.0;

/// Space kept between the panel and the edges of the window.
pub const MARGIN: f32 = 8.0;

/// The side of the anchor the panel appears on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Side {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Top, Side::Bottom, Side::Left, Side::Right];

    pub fn opposite(self) -> Side {
        match self {
            Side::Top => Side::Bottom,
            Side::Bottom => Side::Top,
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    fn is_vertical(self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }
}

/// How the panel lines up with the anchor along that side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Align {
    /// Left edges meet (or top edges, on the left and right sides).
    Start,
    #[default]
    Center,
    /// Right edges meet (or bottom edges, on the left and right sides).
    End,
}

impl Align {
    pub const ALL: [Align; 3] = [Align::Start, Align::Center, Align::End];

    fn mirrored(self) -> Align {
        match self {
            Align::Start => Align::End,
            Align::Center => Align::Center,
            Align::End => Align::Start,
        }
    }
}

/// Where a panel sits relative to its anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub side: Side,
    pub align: Align,
    /// Space between the anchor and the panel.
    pub gap: f32,
    /// Shift along the side, away from the aligned edge.
    pub offset: f32,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            side: Side::default(),
            align: Align::default(),
            gap: GAP,
            offset: 0.0,
        }
    }
}

impl Placement {
    pub fn new(side: Side, align: Align) -> Self {
        Self {
            side,
            align,
            ..Self::default()
        }
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }
}

/// Positions a panel of `size` next to `anchor`, keeping it inside
/// `viewport` less [`MARGIN`].
///
/// The panel moves to the opposite side when it does not fit on the
/// requested one and does fit there, and to the mirrored alignment when it
/// would overflow along the side. Whatever still overflows is shifted back
/// inside, pinned to the top left if the panel is larger than the window.
pub fn place(
    anchor: Rectangle,
    size: Size,
    viewport: Rectangle,
    placement: Placement,
) -> Rectangle {
    let area = viewport.shrink(MARGIN);
    let room = |side: Side| match side {
        Side::Top => anchor.y - placement.gap - area.y,
        Side::Bottom => area.y + area.height - (anchor.y + anchor.height + placement.gap),
        Side::Left => anchor.x - placement.gap - area.x,
        Side::Right => area.x + area.width - (anchor.x + anchor.width + placement.gap),
    };
    let needed = |side: Side| {
        if side.is_vertical() {
            size.height
        } else {
            size.width
        }
    };

    let requested = placement.side;
    let opposite = requested.opposite();
    let side = if room(requested) >= needed(requested) {
        requested
    } else if room(opposite) >= needed(opposite) || room(opposite) > room(requested) {
        opposite
    } else {
        requested
    };

    let main = match side {
        Side::Top => anchor.y - placement.gap - size.height,
        Side::Bottom => anchor.y + anchor.height + placement.gap,
        Side::Left => anchor.x - placement.gap - size.width,
        Side::Right => anchor.x + anchor.width + placement.gap,
    };

    let (start, length, extent, low, high) = if side.is_vertical() {
        (
            anchor.x,
            anchor.width,
            size.width,
            area.x,
            area.x + area.width,
        )
    } else {
        (
            anchor.y,
            anchor.height,
            size.height,
            area.y,
            area.y + area.height,
        )
    };
    let cross_at = |align: Align| match align {
        Align::Start => start + placement.offset,
        Align::Center => start + (length - extent) / 2.0 + placement.offset,
        Align::End => start + length - extent - placement.offset,
    };
    let fits = |value: f32| value >= low && value + extent <= high;
    let preferred = cross_at(placement.align);
    let mirrored = cross_at(placement.align.mirrored());
    let cross = if !fits(preferred) && fits(mirrored) {
        mirrored
    } else {
        preferred
    };

    let (x, y) = if side.is_vertical() {
        (cross, main)
    } else {
        (main, cross)
    };
    Rectangle::new(
        Point::new(
            clamp(x, area.x, area.x + area.width - size.width),
            clamp(y, area.y, area.y + area.height - size.height),
        ),
        size,
    )
}

/// Keeps `value` within `low..=high`, preferring `low` when the range is empty.
fn clamp(value: f32, low: f32, high: f32) -> f32 {
    value.min(high).max(low)
}

/// The floating surface shared by popovers and menus: the page background,
/// a subtle border and the same restrained shadow as the select menu.
pub fn surface_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.background)),
        text_color: Some(tokens.foreground),
        border: Border {
            color: tokens.border,
            width: 1.0,
            radius: radius::MD.into(),
        },
        shadow: Shadow {
            color: fade(Color::BLACK, if tokens.is_dark { 0.5 } else { 0.1 }),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

/// An anchor with a panel floating next to it. Convert it into an
/// [`Element`] to render.
pub struct Anchored<'a, Message> {
    anchor: Element<'a, Message>,
    content: Option<Element<'a, Message>>,
    placement: Placement,
    point: Option<Point>,
    on_dismiss: Option<Message>,
    dismiss_on_anchor_press: bool,
}

impl<Message> std::fmt::Debug for Anchored<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Anchored")
            .field("open", &self.content.is_some())
            .field("placement", &self.placement)
            .field("point", &self.point)
            .field("dismissable", &self.on_dismiss.is_some())
            .finish_non_exhaustive()
    }
}

/// Wraps `anchor`. Nothing floats until [`content`](Anchored::content) is set.
pub fn anchored<'a, Message>(anchor: impl Into<Element<'a, Message>>) -> Anchored<'a, Message> {
    Anchored {
        anchor: anchor.into(),
        content: None,
        placement: Placement::default(),
        point: None,
        on_dismiss: None,
        dismiss_on_anchor_press: false,
    }
}

impl<'a, Message> Anchored<'a, Message> {
    /// The floating panel. `None` keeps it closed.
    pub fn content(mut self, content: Option<Element<'a, Message>>) -> Self {
        self.content = content;
        self
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Anchors the panel to a point, relative to the anchor's top left
    /// corner, instead of to the anchor's edges.
    pub fn at(mut self, point: Point) -> Self {
        self.point = Some(point);
        self
    }

    /// Sent on Escape and on a press outside the panel and the anchor.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Also dismiss on presses on the anchor itself, for anchors that do
    /// not toggle the panel, such as a context menu area.
    pub fn dismiss_on_anchor_press(mut self, dismiss: bool) -> Self {
        self.dismiss_on_anchor_press = dismiss;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Anchored<'a, Message>> for Element<'a, Message> {
    fn from(anchored: Anchored<'a, Message>) -> Self {
        Element::new(anchored)
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Anchored<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.anchor.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.anchor.as_widget().size_hint()
    }

    fn children(&self) -> Vec<Tree> {
        std::iter::once(&self.anchor)
            .chain(&self.content)
            .map(Tree::new)
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        match &self.content {
            Some(content) => tree.diff_children(&[&self.anchor, content]),
            None => tree.diff_children(&[&self.anchor]),
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.anchor
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
        self.anchor.as_widget().draw(
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
        self.anchor
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
        self.anchor.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
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
        self.anchor.as_widget().mouse_interaction(
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
        let Self {
            anchor,
            content,
            placement,
            point,
            on_dismiss,
            dismiss_on_anchor_press,
        } = self;
        let mut trees = tree.children.iter_mut();
        let anchor_tree = trees.next()?;
        let trigger = layout.bounds() + translation;
        let own =
            anchor
                .as_widget_mut()
                .overlay(anchor_tree, layout, renderer, viewport, translation);

        let target = match point {
            Some(point) => Rectangle::new(
                trigger.position() + Vector::new(point.x, point.y),
                Size::ZERO,
            ),
            None => trigger,
        };
        let floating = content.as_mut().zip(trees.next()).map(|(content, tree)| {
            overlay::Element::new(Box::new(Floating {
                content,
                tree,
                target,
                trigger,
                placement: *placement,
                on_dismiss: on_dismiss.as_ref(),
                dismiss_on_anchor_press: *dismiss_on_anchor_press,
            }))
        });

        match (own, floating) {
            (Some(own), Some(floating)) => {
                Some(overlay::Group::with_children(vec![own, floating]).overlay())
            }
            (own, floating) => own.or(floating),
        }
    }
}

/// The overlay drawing the panel. Its node covers the whole window so the
/// panel's shadow is never clipped; the panel is its only child.
struct Floating<'a, 'b, Message> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    target: Rectangle,
    trigger: Rectangle,
    placement: Placement,
    on_dismiss: Option<&'b Message>,
    dismiss_on_anchor_press: bool,
}

impl<Message: Clone> Floating<'_, '_, Message> {
    fn dismiss(&self, shell: &mut Shell<'_, Message>) -> bool {
        let Some(message) = self.on_dismiss else {
            return false;
        };
        shell.publish(message.clone());
        true
    }
}

/// Where a mouse or touch press landed, if the event is one.
fn press_position(event: &Event, cursor: mouse::Cursor) -> Option<Point> {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => cursor.position(),
        Event::Touch(touch::Event::FingerPressed { position, .. }) => Some(*position),
        _ => None,
    }
}

fn is_pointer_input(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(
            mouse::Event::ButtonPressed(_)
                | mouse::Event::ButtonReleased(_)
                | mouse::Event::WheelScrolled { .. }
        ) | Event::Touch(_)
    )
}

impl<Message: Clone> overlay::Overlay<Message, Theme, Renderer> for Floating<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let viewport = Rectangle::with_size(bounds);
        let room = viewport.shrink(MARGIN).size();
        let limits = layout::Limits::new(
            Size::ZERO,
            Size::new(room.width.max(0.0), room.height.max(0.0)),
        );
        let node = self
            .content
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);
        let panel = place(self.target, node.size(), viewport, self.placement);
        layout::Node::with_children(bounds, vec![node.move_to(panel.position())])
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let Some(panel) = layout.children().next() else {
            return;
        };
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            panel,
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        let Some(panel) = layout.children().next() else {
            return;
        };
        self.content
            .as_widget_mut()
            .operate(self.tree, panel, renderer, operation);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(panel) = layout.children().next() else {
            return;
        };

        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(Named::Escape),
            ..
        }) = event
            && self.dismiss(shell)
        {
            shell.capture_event();
            return;
        }

        self.content.as_widget_mut().update(
            self.tree,
            event,
            panel,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );
        if shell.is_event_captured() || !is_pointer_input(event) {
            return;
        }

        let bounds = panel.bounds();
        if cursor.is_over(bounds) {
            shell.capture_event();
            return;
        }
        let Some(position) = press_position(event, cursor) else {
            return;
        };
        if bounds.contains(position) {
            shell.capture_event();
            return;
        }
        if self.dismiss_on_anchor_press || !self.trigger.contains(position) {
            let _ = self.dismiss(shell);
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(panel) = layout.children().next() else {
            return mouse::Interaction::None;
        };
        if !cursor.is_over(panel.bounds()) {
            return mouse::Interaction::None;
        }
        self.content
            .as_widget()
            .mouse_interaction(self.tree, panel, cursor, &layout.bounds(), renderer)
            .max(mouse::Interaction::Idle)
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'c>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        let panel = layout.children().next()?;
        let viewport = layout.bounds();
        self.content
            .as_widget_mut()
            .overlay(self.tree, panel, renderer, &viewport, Vector::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const VIEWPORT: Rectangle = Rectangle {
        x: 0.0,
        y: 0.0,
        width: 400.0,
        height: 300.0,
    };

    fn anchor() -> Rectangle {
        Rectangle::new(Point::new(150.0, 100.0), Size::new(100.0, 40.0))
    }

    const PANEL: Size = Size::new(60.0, 50.0);

    fn at(side: Side, align: Align) -> Point {
        place(anchor(), PANEL, VIEWPORT, Placement::new(side, align)).position()
    }

    #[test]
    fn defaults_to_bottom_centre_with_a_small_gap() {
        let placement = Placement::default();
        assert_eq!(placement.side, Side::Bottom);
        assert_eq!(placement.align, Align::Center);
        assert_eq!(placement.gap, GAP);
        assert_eq!(placement.offset, 0.0);
    }

    #[test]
    fn each_side_sits_outside_the_anchor_with_the_gap() {
        assert_eq!(at(Side::Bottom, Align::Start), Point::new(150.0, 144.0));
        assert_eq!(at(Side::Top, Align::Start), Point::new(150.0, 46.0));
        assert_eq!(at(Side::Left, Align::Start), Point::new(86.0, 100.0));
        assert_eq!(at(Side::Right, Align::Start), Point::new(254.0, 100.0));
    }

    #[test]
    fn alignment_moves_along_the_side() {
        assert_eq!(at(Side::Bottom, Align::Center).x, 170.0);
        assert_eq!(at(Side::Bottom, Align::End).x, 190.0);
        assert_eq!(at(Side::Right, Align::Center).y, 95.0);
        assert_eq!(at(Side::Right, Align::End).y, 90.0);
    }

    #[test]
    fn offset_shifts_away_from_the_aligned_edge() {
        let placement = Placement::new(Side::Bottom, Align::Start).offset(5.0);
        let start = place(anchor(), PANEL, VIEWPORT, placement);
        assert_eq!(start.x, 155.0);
        let placement = Placement::new(Side::Bottom, Align::End).offset(5.0);
        let end = place(anchor(), PANEL, VIEWPORT, placement);
        assert_eq!(end.x, 185.0);
    }

    #[test]
    fn flips_to_the_opposite_side_when_there_is_no_room() {
        let low = Rectangle::new(Point::new(150.0, 230.0), Size::new(100.0, 40.0));
        let panel = place(
            low,
            PANEL,
            VIEWPORT,
            Placement::new(Side::Bottom, Align::Start),
        );
        assert_eq!(panel.y, 230.0 - GAP - PANEL.height);

        let left_edge = Rectangle::new(Point::new(20.0, 100.0), Size::new(40.0, 40.0));
        let panel = place(
            left_edge,
            PANEL,
            VIEWPORT,
            Placement::new(Side::Left, Align::Start),
        );
        assert_eq!(panel.x, 60.0 + GAP);
    }

    #[test]
    fn stays_put_when_neither_side_fits_and_the_requested_one_is_larger() {
        let tall = Size::new(60.0, 290.0);
        let panel = place(
            anchor(),
            tall,
            VIEWPORT,
            Placement::new(Side::Bottom, Align::Start),
        );
        assert_eq!(panel.y, MARGIN, "shifted up to fit, pinned at the margin");
    }

    #[test]
    fn mirrors_the_alignment_when_it_would_overflow() {
        let right_edge = Rectangle::new(Point::new(360.0, 100.0), Size::new(30.0, 30.0));
        let panel = place(
            right_edge,
            PANEL,
            VIEWPORT,
            Placement::new(Side::Bottom, Align::Start),
        );
        assert_eq!(panel.x, 390.0 - PANEL.width);
    }

    #[test]
    fn shifts_inside_the_margin_when_nothing_else_fits() {
        let corner = Rectangle::new(Point::new(0.0, 0.0), Size::new(10.0, 10.0));
        let panel = place(
            corner,
            PANEL,
            VIEWPORT,
            Placement::new(Side::Left, Align::Center),
        );
        assert_eq!(panel.x, 10.0 + GAP, "flipped to the right");
        assert_eq!(panel.y, MARGIN, "shifted down to the margin");
    }

    #[test]
    fn a_point_anchor_opens_below_and_right_then_flips_near_edges() {
        let point = |x, y| Rectangle::new(Point::new(x, y), Size::ZERO);
        let placement = Placement::new(Side::Bottom, Align::Start).gap(0.0);
        assert_eq!(
            place(point(100.0, 100.0), PANEL, VIEWPORT, placement).position(),
            Point::new(100.0, 100.0)
        );
        assert_eq!(
            place(point(380.0, 280.0), PANEL, VIEWPORT, placement).position(),
            Point::new(320.0, 230.0)
        );
    }

    #[test]
    fn a_panel_larger_than_the_window_is_pinned_top_left() {
        let huge = Size::new(1000.0, 1000.0);
        let panel = place(anchor(), huge, VIEWPORT, Placement::default());
        assert_eq!(panel.position(), Point::new(MARGIN, MARGIN));
        assert_eq!(panel.size(), huge);
    }

    #[test]
    fn sides_and_alignments_mirror() {
        for side in Side::ALL {
            assert_eq!(side.opposite().opposite(), side);
            assert_ne!(side.opposite(), side);
        }
        assert_eq!(Align::Center.mirrored(), Align::Center);
        assert_eq!(Align::Start.mirrored(), Align::End);
    }

    #[test]
    fn surface_uses_background_border_and_a_shadow_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = surface_style(&tokens);
            assert_eq!(style.background, Some(Background::Color(tokens.background)));
            assert_eq!(style.text_color, Some(tokens.foreground));
            assert_eq!(style.border.color, tokens.border);
            assert_eq!(style.border.width, 1.0);
            assert!(style.shadow.color.a > 0.0);
        }
        let light = surface_style(&Tokens::of(&light())).shadow.color.a;
        let dark = surface_style(&Tokens::of(&dark())).shadow.color.a;
        assert!(dark > light, "shadows are stronger on a dark page");
    }

    #[test]
    fn builder_starts_closed_and_undismissable() {
        let anchored: Anchored<'_, ()> = anchored(iced::widget::text("Anchor"));
        assert!(anchored.content.is_none());
        assert!(anchored.on_dismiss.is_none());
        assert!(anchored.point.is_none());
        assert!(!anchored.dismiss_on_anchor_press);
    }

    #[test]
    fn press_position_only_reports_presses() {
        let cursor = mouse::Cursor::Available(Point::new(3.0, 4.0));
        let pressed = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
        let released = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        assert_eq!(press_position(&pressed, cursor), Some(Point::new(3.0, 4.0)));
        assert_eq!(press_position(&released, cursor), None);
        assert_eq!(press_position(&pressed, mouse::Cursor::Unavailable), None);
        assert!(is_pointer_input(&released));
        assert!(!is_pointer_input(&Event::Mouse(mouse::Event::CursorLeft)));
    }
}
