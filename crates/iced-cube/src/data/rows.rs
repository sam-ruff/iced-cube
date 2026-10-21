//! A scrolling list of equal-height rows that only builds the rows in view,
//! and scrolls the highlighted row into view when the highlight moves.
//!
//! The rows are built during layout, once the height is known, so a list
//! of thousands of rows costs no more than a screenful. An optional header
//! stays above the rows as they scroll, and gives up the same room for the
//! scrollbar, so its columns keep lining up with theirs.

use std::ops::Range;

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer as _, Shell};
use iced::mouse::ScrollDelta;
use iced::touch::{self, Finger};
use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse,
};

use crate::primitives::scroll_area::{self, Interaction, THICKNESS};
use crate::theme::{Tokens, radius};

/// Rows built beyond each edge of the view, so a scroll never shows a gap.
pub(crate) const OVERSCAN: usize = 4;
/// Space between the scrollbar and the edge, and the scrollbar and rows.
const MARGIN: f32 = 2.0;
/// Room the rows and header give up for the scrollbar when they overflow.
pub(crate) const GUTTER: f32 = THICKNESS + MARGIN * 2.0;
/// Shortest scrollbar thumb, so it stays easy to grab in a long list.
const MIN_THUMB: f32 = 24.0;
/// Pixels a wheel scrolls per line.
const LINE: f32 = 60.0;

/// Keeps a scroll offset inside the content.
pub(crate) fn clamp_offset(offset: f32, content: f32, viewport: f32) -> f32 {
    offset.min((content - viewport).max(0.0)).max(0.0)
}

/// The offset that brings row `index` fully into view, moving as little as
/// possible.
pub(crate) fn reveal(offset: f32, index: usize, row_height: f32, viewport: f32) -> f32 {
    let top = index as f32 * row_height;
    let bottom = top + row_height;
    if top < offset {
        top
    } else if bottom > offset + viewport {
        (bottom - viewport).max(0.0)
    } else {
        offset
    }
}

/// The rows to build for a view of `viewport` pixels at `offset`.
pub(crate) fn window(offset: f32, viewport: f32, row_height: f32, count: usize) -> Range<usize> {
    if count == 0 || row_height <= 0.0 {
        return 0..0;
    }
    let first = (offset / row_height).floor() as usize;
    let last = ((offset + viewport) / row_height).ceil() as usize;
    first.saturating_sub(OVERSCAN).min(count)..last.saturating_add(OVERSCAN).min(count)
}

/// The scrollbar thumb as a top and a height within a track of `track`
/// pixels, or `None` when everything fits.
pub(crate) fn thumb(offset: f32, content: f32, viewport: f32, track: f32) -> Option<(f32, f32)> {
    if content <= viewport || track <= 0.0 {
        return None;
    }
    let height = (track * viewport / content).clamp(MIN_THUMB.min(track), track);
    let travel = track - height;
    let top = travel * offset / (content - viewport);
    Some((top, height))
}

type Build<'a, Message> = Box<dyn Fn(Range<usize>) -> Element<'a, Message> + 'a>;

/// A list of `count` rows, each `row_height` tall. `build` returns the
/// rows in a range, stacked in one element.
pub(crate) struct Rows<'a, Message> {
    count: usize,
    row_height: f32,
    highlight: Option<usize>,
    height: Length,
    header: Option<Element<'a, Message>>,
    build: Build<'a, Message>,
    built: Option<(Range<usize>, Element<'a, Message>)>,
}

pub(crate) fn rows<'a, Message>(
    count: usize,
    row_height: f32,
    build: impl Fn(Range<usize>) -> Element<'a, Message> + 'a,
) -> Rows<'a, Message> {
    Rows {
        count,
        row_height,
        highlight: None,
        height: Length::Shrink,
        header: None,
        build: Box::new(build),
        built: None,
    }
}

impl<'a, Message> Rows<'a, Message> {
    /// The row to keep in view. The list scrolls to it whenever it changes.
    pub(crate) fn highlight(mut self, index: Option<usize>) -> Self {
        self.highlight = index;
        self
    }

    /// The whole height, header included.
    pub(crate) fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Content that stays above the rows while they scroll.
    #[cfg(feature = "data-table")]
    pub(crate) fn header(mut self, header: impl Into<Element<'a, Message>>) -> Self {
        self.header = Some(header.into());
        self
    }

    fn content_height(&self) -> f32 {
        self.count as f32 * self.row_height
    }
}

#[derive(Debug, Default)]
struct State {
    offset: f32,
    highlight: Option<usize>,
    /// Where the pointer holds the thumb, from its top, while dragging.
    drag: Option<f32>,
    /// A finger scrolling the list: which one, where it started and the
    /// offset then.
    touch: Option<(Finger, f32, f32)>,
}

/// Where the rows show: the bounds below the header.
fn viewport(layout: Layout<'_>) -> Rectangle {
    let bounds = layout.bounds();
    let header = layout
        .children()
        .nth(1)
        .map_or(0.0, |header| header.bounds().height);
    Rectangle {
        y: bounds.y + header,
        height: (bounds.height - header).max(0.0),
        ..bounds
    }
}

/// The track the thumb moves along, at the right edge of the rows.
fn track(rows: Rectangle) -> Rectangle {
    Rectangle {
        x: rows.x + rows.width - THICKNESS - MARGIN,
        y: rows.y + MARGIN,
        width: THICKNESS,
        height: (rows.height - MARGIN * 2.0).max(0.0),
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Rows<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        std::iter::once(Tree::empty())
            .chain(self.header.as_ref().map(Tree::new))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let Some(header) = &self.header else {
            tree.children.truncate(1);
            return;
        };
        match tree.children.get_mut(1) {
            Some(child) => child.diff(header),
            None => tree.children.push(Tree::new(header)),
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let content = self.content_height();
        let width = limits.max().width;
        let header_height = match (&mut self.header, tree.children.get_mut(1)) {
            (Some(header), Some(header_tree)) => {
                let limits = layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY));
                header
                    .as_widget_mut()
                    .layout(header_tree, renderer, &limits)
                    .size()
                    .height
            }
            _ => 0.0,
        };
        let size = limits.resolve(
            Length::Fill,
            self.height,
            Size::new(0.0, content + header_height),
        );
        let viewport = if size.height.is_finite() {
            (size.height - header_height).max(0.0)
        } else {
            content
        };
        let size = Size::new(size.width, viewport + header_height);
        let inner = if content > viewport + 0.5 {
            (size.width - GUTTER).max(0.0)
        } else {
            size.width
        };

        let state = tree.state.downcast_mut::<State>();
        if self.highlight != state.highlight {
            if let Some(index) = self.highlight {
                state.offset = reveal(state.offset, index, self.row_height, viewport);
            }
            state.highlight = self.highlight;
        }
        state.offset = clamp_offset(state.offset, content, viewport);
        let offset = state.offset;

        let range = window(offset, viewport, self.row_height, self.count);
        let stale = self.built.as_ref().is_none_or(|(built, _)| *built != range);
        if stale {
            let element = (self.build)(range.clone());
            tree.children[0].diff(&element);
            self.built = Some((range.clone(), element));
        }

        let mut nodes = Vec::with_capacity(2);
        let height = range.len() as f32 * self.row_height;
        nodes.push(match self.built.as_mut() {
            Some((_, element)) => element
                .as_widget_mut()
                .layout(
                    &mut tree.children[0],
                    renderer,
                    &layout::Limits::new(Size::new(inner, 0.0), Size::new(inner, height)),
                )
                .move_to(Point::new(
                    0.0,
                    header_height + range.start as f32 * self.row_height - offset,
                )),
            None => layout::Node::new(Size::ZERO),
        });
        if let (Some(header), Some(header_tree)) = (&mut self.header, tree.children.get_mut(1)) {
            let limits =
                layout::Limits::new(Size::new(inner, 0.0), Size::new(inner, f32::INFINITY));
            nodes.push(
                header
                    .as_widget_mut()
                    .layout(header_tree, renderer, &limits),
            );
        }
        layout::Node::with_children(size, nodes)
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
        let rows = self::viewport(layout);
        let state = tree.state.downcast_ref::<State>();
        let dragging = state.drag.is_some();
        let mut children = layout.children();

        if let (Some((_, element)), Some(child), Some(visible)) =
            (&self.built, children.next(), rows.intersection(viewport))
        {
            let cursor = if cursor.is_over(rows) && !dragging {
                cursor
            } else {
                mouse::Cursor::Unavailable
            };
            renderer.with_layer(visible, |renderer| {
                element.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    child,
                    cursor,
                    &visible,
                );
            });
        }
        if let (Some(header), Some(child), Some(header_tree)) =
            (&self.header, children.next(), tree.children.get(1))
        {
            header
                .as_widget()
                .draw(header_tree, renderer, theme, style, child, cursor, viewport);
        }

        let track = track(rows);
        let Some((top, height)) = thumb(
            state.offset,
            self.content_height(),
            rows.height,
            track.height,
        ) else {
            return;
        };
        let interaction = if dragging {
            Interaction::Dragged
        } else if cursor.is_over(track.expand(MARGIN)) {
            Interaction::Hovered
        } else {
            Interaction::Idle
        };
        let tokens = Tokens::of(theme);
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    y: track.y + top,
                    height,
                    ..track
                },
                border: Border {
                    radius: radius::FULL.into(),
                    ..Border::default()
                },
                snap: true,
                ..Quad::default()
            },
            Background::Color(scroll_area::scroller_colour(&tokens, interaction)),
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        let children: Vec<Layout<'_>> = layout.children().collect();
        let (rows_trees, header_trees) = tree.children.split_at_mut(1);
        operation.traverse(&mut |operation| {
            if let (Some((_, element)), Some(child), Some(rows_tree)) = (
                self.built.as_mut(),
                children.first(),
                rows_trees.first_mut(),
            ) {
                element
                    .as_widget_mut()
                    .operate(rows_tree, *child, renderer, operation);
            }
            if let (Some(header), Some(child), Some(header_tree)) = (
                self.header.as_mut(),
                children.get(1),
                header_trees.first_mut(),
            ) {
                header
                    .as_widget_mut()
                    .operate(header_tree, *child, renderer, operation);
            }
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
        let rows = self::viewport(layout);
        let content = self.content_height();
        if scroll(
            tree.state.downcast_mut::<State>(),
            event,
            cursor,
            rows,
            content,
            shell,
        ) {
            shell.capture_event();
            return;
        }

        let mut children = layout.children();
        let rows_layout = children.next();
        if let (Some(header), Some(child), Some(header_tree)) =
            (&mut self.header, children.next(), tree.children.get_mut(1))
        {
            header.as_widget_mut().update(
                header_tree,
                event,
                child,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
            if shell.is_event_captured() {
                return;
            }
        }
        let (Some((_, element)), Some(child)) = (self.built.as_mut(), rows_layout) else {
            return;
        };
        // Rows scrolled out of view still sit outside the list, so they
        // only see the pointer while it is over the list.
        let cursor = if cursor.is_over(rows) {
            cursor
        } else {
            mouse::Cursor::Unavailable
        };
        element.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child,
            cursor,
            renderer,
            clipboard,
            shell,
            &rows,
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
        let state = tree.state.downcast_ref::<State>();
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        let rows = self::viewport(layout);
        let mut children = layout.children();
        let rows_layout = children.next();
        if let (Some(header), Some(child), Some(header_tree)) =
            (&self.header, children.next(), tree.children.get(1))
            && cursor.is_over(child.bounds())
        {
            return header.as_widget().mouse_interaction(
                header_tree,
                child,
                cursor,
                viewport,
                renderer,
            );
        }
        if !cursor.is_over(rows) {
            return mouse::Interaction::None;
        }
        let overflows = self.content_height() > rows.height;
        if overflows && cursor.is_over(track(rows).expand(MARGIN)) {
            return mouse::Interaction::Idle;
        }
        let (Some((_, element)), Some(child)) = (&self.built, rows_layout) else {
            return mouse::Interaction::None;
        };
        element
            .as_widget()
            .mouse_interaction(&tree.children[0], child, cursor, &rows, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let rows = self::viewport(layout);
        let visible = rows.intersection(viewport).unwrap_or(rows);
        let mut children = layout.children();
        let rows_layout = children.next()?;
        let (rows_trees, header_trees) = tree.children.split_at_mut(1);
        let header = match (
            self.header.as_mut(),
            children.next(),
            header_trees.first_mut(),
        ) {
            (Some(header), Some(child), Some(header_tree)) => {
                header
                    .as_widget_mut()
                    .overlay(header_tree, child, renderer, viewport, translation)
            }
            _ => None,
        };
        let rows_overlay = match (self.built.as_mut(), rows_trees.first_mut()) {
            (Some((_, element)), Some(rows_tree)) => element.as_widget_mut().overlay(
                rows_tree,
                rows_layout,
                renderer,
                &visible,
                translation,
            ),
            _ => None,
        };
        let overlays: Vec<_> = rows_overlay.into_iter().chain(header).collect();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

/// Handles the wheel, the scrollbar and touch drags over the rows. Returns
/// whether the event was used up, so the rows must not see it.
fn scroll<Message>(
    state: &mut State,
    event: &Event,
    cursor: mouse::Cursor,
    rows: Rectangle,
    content: f32,
    shell: &mut Shell<'_, Message>,
) -> bool {
    let track = track(rows);
    let mut scroll_to = |state: &mut State, offset: f32| {
        let offset = clamp_offset(offset, content, rows.height);
        if offset == state.offset {
            return false;
        }
        state.offset = offset;
        shell.invalidate_layout();
        shell.request_redraw();
        true
    };
    let thumb = thumb(state.offset, content, rows.height, track.height);
    let dragged_to = |grab: f32, y: f32, height: f32| {
        drag_offset(
            y - track.y - grab,
            height,
            track.height,
            content,
            rows.height,
        )
    };

    match event {
        Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(rows) => {
            let dy = match delta {
                ScrollDelta::Lines { y, .. } => y * LINE,
                ScrollDelta::Pixels { y, .. } => *y,
            };
            scroll_to(state, state.offset - dy)
        }
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
            let (Some((top, height)), Some(position)) = (thumb, cursor.position()) else {
                return false;
            };
            if !track.expand(MARGIN).contains(position) {
                return false;
            }
            let grab = position.y - track.y - top;
            let grab = if (0.0..=height).contains(&grab) {
                grab
            } else {
                height / 2.0
            };
            state.drag = Some(grab);
            let _ = scroll_to(state, dragged_to(grab, position.y, height));
            true
        }
        Event::Mouse(mouse::Event::CursorMoved { position }) => {
            let (Some(grab), Some((_, height))) = (state.drag, thumb) else {
                return false;
            };
            let _ = scroll_to(state, dragged_to(grab, position.y, height));
            true
        }
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
            state.drag.take().is_some()
        }
        Event::Touch(touch::Event::FingerPressed { id, position }) => {
            if cursor.is_over(rows) {
                state.touch = Some((*id, position.y, state.offset));
            }
            false
        }
        Event::Touch(touch::Event::FingerMoved { id, position }) => {
            if let Some((finger, start, offset)) = state.touch
                && finger == *id
            {
                let _ = scroll_to(state, offset - (position.y - start));
            }
            false
        }
        Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. }) => {
            state.touch = None;
            false
        }
        _ => false,
    }
}

/// The offset for a thumb dragged to `top` within its track.
fn drag_offset(top: f32, thumb: f32, track: f32, content: f32, viewport: f32) -> f32 {
    let travel = (track - thumb).max(1.0);
    (top / travel).clamp(0.0, 1.0) * (content - viewport).max(0.0)
}

impl<'a, Message: 'a> From<Rows<'a, Message>> for Element<'a, Message> {
    fn from(rows: Rows<'a, Message>) -> Self {
        Element::new(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_stay_inside_the_content() {
        assert_eq!(clamp_offset(-10.0, 500.0, 100.0), 0.0);
        assert_eq!(clamp_offset(450.0, 500.0, 100.0), 400.0);
        assert_eq!(
            clamp_offset(50.0, 80.0, 100.0),
            0.0,
            "short content never scrolls"
        );
    }

    #[test]
    fn reveal_moves_as_little_as_possible() {
        assert_eq!(reveal(0.0, 2, 32.0, 100.0), 0.0, "already in view");
        assert_eq!(
            reveal(0.0, 5, 32.0, 100.0),
            92.0,
            "below: its bottom edge meets the view's"
        );
        assert_eq!(
            reveal(200.0, 3, 32.0, 100.0),
            96.0,
            "above: its top meets the view's"
        );
    }

    #[test]
    fn the_window_covers_the_view_plus_overscan() {
        assert_eq!(window(0.0, 100.0, 32.0, 1000), 0..4 + OVERSCAN);
        assert_eq!(
            window(3200.0, 100.0, 32.0, 1000),
            100 - OVERSCAN..104 + OVERSCAN
        );
        assert_eq!(window(0.0, 100.0, 32.0, 2), 0..2);
        assert_eq!(window(0.0, 100.0, 32.0, 0), 0..0);
        assert_eq!(window(0.0, 100.0, 0.0, 10), 0..0);
    }

    #[test]
    fn the_thumb_tracks_the_offset_and_keeps_a_minimum_size() {
        assert_eq!(
            thumb(0.0, 100.0, 200.0, 196.0),
            None,
            "no thumb when it fits"
        );
        let (top, height) = thumb(0.0, 1000.0, 200.0, 196.0).expect("overflows");
        assert_eq!(top, 0.0);
        assert!((height - 39.2).abs() < 1e-3);
        let (top, _) = thumb(800.0, 1000.0, 200.0, 196.0).expect("overflows");
        assert!(
            (top - (196.0 - 39.2)).abs() < 1e-3,
            "at the end of the track"
        );
        let (_, tiny) = thumb(0.0, 1_000_000.0, 200.0, 196.0).expect("overflows");
        assert_eq!(tiny, MIN_THUMB);
    }

    #[test]
    fn dragging_the_thumb_maps_back_to_an_offset() {
        assert_eq!(drag_offset(0.0, 40.0, 200.0, 1000.0, 200.0), 0.0);
        assert_eq!(drag_offset(160.0, 40.0, 200.0, 1000.0, 200.0), 800.0);
        assert_eq!(drag_offset(80.0, 40.0, 200.0, 1000.0, 200.0), 400.0);
        assert_eq!(drag_offset(999.0, 40.0, 200.0, 1000.0, 200.0), 800.0);
    }

    #[test]
    fn the_track_runs_down_the_right_edge_inside_a_margin() {
        let rows = Rectangle::new(Point::new(10.0, 20.0), Size::new(200.0, 100.0));
        let track = track(rows);
        assert_eq!(track.x, 10.0 + 200.0 - THICKNESS - MARGIN);
        assert_eq!(track.y, 20.0 + MARGIN);
        assert_eq!(track.height, 100.0 - MARGIN * 2.0);
        assert_eq!(GUTTER, THICKNESS + MARGIN * 2.0);
    }
}
