//! A scrolling list of rows that only builds the rows in view, and scrolls
//! a chosen row into view when asked.
//!
//! The rows are built during layout, once the height is known, so a list
//! of thousands of rows costs no more than a screenful. Rows are equally
//! tall by default. Measured rows can differ: each built row's height is
//! remembered by its key, and rows not built yet count as the estimate. An
//! optional header stays above the rows as they scroll, and gives up the
//! same room for the scrollbar, so its columns keep lining up with theirs.
//!
//! The list can follow its end, staying scrolled to the bottom as rows
//! arrive, and reports when the user scrolls away from the end or back to
//! it. Rows dropped from the front can be reported through an origin, so
//! the rows in view stay put.

#[cfg(feature = "log-viewer")]
use std::collections::HashMap;
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
/// How close to the end, in pixels, still counts as being at the end.
const END_SLOP: f32 = 1.0;

/// Keeps a scroll offset inside the content.
pub(crate) fn clamp_offset(offset: f32, content: f32, viewport: f32) -> f32 {
    offset.min((content - viewport).max(0.0)).max(0.0)
}

/// The offset that brings the span from `top` to `bottom` fully into view,
/// moving as little as possible.
fn reveal_span(offset: f32, top: f32, bottom: f32, viewport: f32) -> f32 {
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

/// The rows to build when rows differ in height. `tops` holds where each
/// row starts, followed by the content height.
fn measured_window(tops: &[f32], offset: f32, viewport: f32) -> Range<usize> {
    let count = tops.len().saturating_sub(1);
    if count == 0 {
        return 0..0;
    }
    let first = tops[..count]
        .partition_point(|&top| top <= offset)
        .saturating_sub(1);
    let last = tops[..count].partition_point(|&top| top < offset + viewport);
    first.saturating_sub(OVERSCAN)..last.saturating_add(OVERSCAN).min(count)
}

/// Where each row starts, given the rows' heights, followed by the total.
#[cfg(feature = "log-viewer")]
fn tops(heights: impl Iterator<Item = f32>) -> Vec<f32> {
    let mut tops = Vec::with_capacity(heights.size_hint().0 + 1);
    let mut top = 0.0;
    tops.push(top);
    for height in heights {
        top += height;
        tops.push(top);
    }
    tops
}

/// Whether an offset sits at the end of the content.
pub(crate) fn at_end(offset: f32, content: f32, viewport: f32) -> bool {
    offset >= (content - viewport).max(0.0) - END_SLOP
}

/// How many whole rows fit in a view, at least one.
pub(crate) fn page(viewport: f32, row_height: f32) -> usize {
    if row_height <= 0.0 {
        return 1;
    }
    ((viewport / row_height).floor() as usize).max(1)
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
type OnFollow<'a, Message> = Box<dyn Fn(bool) -> Message + 'a>;
type OnPage<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;
#[cfg(feature = "log-viewer")]
type Key<'a> = Box<dyn Fn(usize) -> u64 + 'a>;

/// A list of `count` rows, each `row_height` tall. `build` returns the
/// rows in a range, stacked in one column.
pub(crate) struct Rows<'a, Message> {
    count: usize,
    row_height: f32,
    /// Names the row at an index when rows are measured, so a measurement
    /// stays with its row. `row_height` is then the estimate for rows not
    /// built yet.
    #[cfg(feature = "log-viewer")]
    key: Option<Key<'a>>,
    reveal: Option<(usize, u64)>,
    follow: bool,
    origin: u64,
    height: Length,
    header: Option<Element<'a, Message>>,
    build: Build<'a, Message>,
    built: Option<(Range<usize>, Element<'a, Message>)>,
    on_follow: Option<OnFollow<'a, Message>>,
    on_page: Option<OnPage<'a, Message>>,
}

pub(crate) fn rows<'a, Message>(
    count: usize,
    row_height: f32,
    build: impl Fn(Range<usize>) -> Element<'a, Message> + 'a,
) -> Rows<'a, Message> {
    Rows {
        count,
        row_height,
        #[cfg(feature = "log-viewer")]
        key: None,
        reveal: None,
        follow: false,
        origin: 0,
        height: Length::Shrink,
        header: None,
        build: Box::new(build),
        built: None,
        on_follow: None,
        on_page: None,
    }
}

impl<'a, Message> Rows<'a, Message> {
    /// The row to keep in view. The list scrolls to it whenever it changes.
    #[cfg(any(
        feature = "tree",
        feature = "data-table",
        feature = "json-viewer",
        feature = "property-grid"
    ))]
    pub(crate) fn highlight(mut self, index: Option<usize>) -> Self {
        self.reveal = index.map(|index| (index, index as u64));
        self
    }

    /// Scrolls row `index` into view whenever `token` changes.
    #[cfg(feature = "virtual-list")]
    pub(crate) fn reveal(mut self, target: Option<(usize, u64)>) -> Self {
        self.reveal = target;
        self
    }

    /// Keeps the list scrolled to its end.
    #[cfg(feature = "virtual-list")]
    pub(crate) fn follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    /// How many rows have been dropped from the front of the list so far.
    /// When it grows, the offset moves up by their height, so the rows in
    /// view stay where they are.
    #[cfg(feature = "virtual-list")]
    pub(crate) fn origin(mut self, origin: u64) -> Self {
        self.origin = origin;
        self
    }

    /// Sent with `false` when the user scrolls away from the end while
    /// following, and with `true` when they scroll back to it.
    #[cfg(feature = "virtual-list")]
    pub(crate) fn on_follow(mut self, on_follow: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_follow = Some(Box::new(on_follow));
        self
    }

    /// Sent with how many rows fit in the view whenever that changes.
    #[cfg(feature = "virtual-list")]
    pub(crate) fn on_page(mut self, on_page: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_page = Some(Box::new(on_page));
        self
    }

    /// Lets rows take the height they lay out at, with `row_height` as the
    /// estimate for rows not built yet. `key` names the row at an index.
    #[cfg(feature = "log-viewer")]
    pub(crate) fn measured(mut self, key: impl Fn(usize) -> u64 + 'a) -> Self {
        self.key = Some(Box::new(key));
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

    #[cfg(feature = "log-viewer")]
    fn is_measured(&self) -> bool {
        self.key.is_some()
    }

    #[cfg(not(feature = "log-viewer"))]
    fn is_measured(&self) -> bool {
        false
    }

    fn geometry<'s>(&self, state: &'s State) -> Geometry<'s> {
        if self.is_measured() {
            Geometry::Measured(&state.tops)
        } else {
            Geometry::Fixed {
                row: self.row_height,
                count: self.count,
            }
        }
    }

    /// Moves the offset for rows dropped from the front, the end being
    /// followed, or a row to reveal, then keeps it inside the content.
    /// Returns the row it revealed, if any.
    fn place(&self, state: &mut State, viewport: f32, dropped_height: f32) -> Option<usize> {
        state.offset -= dropped_height;
        let geometry = self.geometry(state);
        let content = geometry.content();
        let token = self.reveal.map(|(_, token)| token);
        let mut revealed = None;
        if self.follow {
            state.offset = (content - viewport).max(0.0);
        } else if token != state.revealed
            && let Some((index, _)) = self.reveal.filter(|(index, _)| *index < self.count)
        {
            state.offset = reveal_span(
                state.offset,
                geometry.top(index),
                geometry.top(index + 1),
                viewport,
            );
            revealed = Some(index);
        }
        state.revealed = token;
        state.offset = clamp_offset(state.offset, content, viewport);
        revealed
    }

    /// Builds the rows in `range` unless they are built already.
    fn build(&mut self, tree: &mut Tree, range: &Range<usize>) {
        let stale = self.built.as_ref().is_none_or(|(built, _)| built != range);
        if !stale {
            return;
        }
        let element = (self.build)(range.clone());
        tree.children[0].diff(&element);
        self.built = Some((range.clone(), element));
    }

    /// Lays out the built rows `inner` pixels wide.
    fn layout_rows(&mut self, tree: &mut Tree, renderer: &Renderer, inner: f32) -> layout::Node {
        let measured = self.is_measured();
        let row_height = self.row_height;
        let Some((range, element)) = self.built.as_mut() else {
            return layout::Node::new(Size::ZERO);
        };
        let max = if measured {
            f32::INFINITY
        } else {
            range.len() as f32 * row_height
        };
        element.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::new(inner, 0.0), Size::new(inner, max)),
        )
    }
}

#[cfg(feature = "log-viewer")]
impl<Message> Rows<'_, Message> {
    /// Recomputes where each measured row starts from the remembered
    /// heights, and forgets rows that are gone.
    fn measure_tops(&self, state: &mut State) {
        let Some(key) = &self.key else {
            return;
        };
        let estimate = self.row_height;
        state.tops = tops(
            (0..self.count)
                .map(|index| state.heights.get(&key(index)).copied().unwrap_or(estimate)),
        );
        if state.heights.len() > self.count.saturating_mul(2) + 64 {
            let keep: std::collections::HashSet<u64> = (0..self.count).map(key).collect();
            state.heights.retain(|row, _| keep.contains(row));
        }
    }

    /// Remembers the heights the built rows laid out at. Returns whether
    /// any changed.
    fn remember(&self, state: &mut State, range: &Range<usize>, node: &layout::Node) -> bool {
        let Some(key) = &self.key else {
            return false;
        };
        let mut changed = false;
        for (index, child) in range.clone().zip(node.children()) {
            let height = child.size().height;
            let previous = state.heights.insert(key(index), height);
            changed |= previous.is_none_or(|previous| (previous - height).abs() > 0.5);
        }
        changed
    }

    /// Prepares measured rows for a layout `width` wide. They always leave
    /// room for the scrollbar, so their width, and with it their heights,
    /// never flips as the scrollbar comes and goes.
    fn prepare(&self, state: &mut State, width: f32) {
        if !self.is_measured() {
            return;
        }
        let inner = (width - GUTTER).max(0.0);
        if (inner - state.measured_width).abs() > 0.5 {
            state.heights.clear();
            state.measured_width = inner;
        }
        self.measure_tops(state);
    }

    /// Measured rows may come out taller or shorter than estimated. Keeps
    /// the row just revealed, or else the first row in view, where it was,
    /// with a revealed row wholly in view, and builds again while the rows
    /// in view are not the ones built.
    fn settle(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        inner: f32,
        viewport: f32,
        revealed: Option<usize>,
        built: (Range<usize>, layout::Node),
    ) -> (Range<usize>, layout::Node) {
        /// Passes before giving up on rows that keep changing height.
        const PASSES: usize = 3;
        let (mut range, mut node) = built;
        for _ in 0..PASSES {
            let state = tree.state.downcast_mut::<State>();
            if !self.remember(state, &range, &node) {
                return (range, node);
            }
            let before = self.geometry(state);
            let anchor = revealed.unwrap_or_else(|| before.row_at(state.offset));
            let within = state.offset - before.top(anchor);
            self.measure_tops(state);
            let after = self.geometry(state);
            let content = after.content();
            let mut offset = if self.follow {
                content - viewport
            } else {
                after.top(anchor) + within
            };
            if let Some(index) = revealed.filter(|_| !self.follow) {
                offset = reveal_span(offset, after.top(index), after.top(index + 1), viewport);
            }
            let offset = clamp_offset(offset, content, viewport);
            let needed = after.window(offset, viewport);
            state.offset = offset;
            if needed.start >= range.start && needed.end <= range.end {
                return (range, node);
            }
            self.build(tree, &needed);
            node = self.layout_rows(tree, renderer, inner);
            range = needed;
        }
        let state = tree.state.downcast_mut::<State>();
        if self.remember(state, &range, &node) {
            self.measure_tops(state);
        }
        (range, node)
    }
}

/// Where rows sit: equally tall, or measured.
#[derive(Debug, Clone, Copy)]
enum Geometry<'a> {
    Fixed { row: f32, count: usize },
    Measured(&'a [f32]),
}

impl Geometry<'_> {
    fn content(self) -> f32 {
        match self {
            Geometry::Fixed { row, count } => count as f32 * row,
            Geometry::Measured(tops) => tops.last().copied().unwrap_or(0.0),
        }
    }

    /// Where row `index` starts. The row after the last starts at the end.
    fn top(self, index: usize) -> f32 {
        match self {
            Geometry::Fixed { row, count } => index.min(count) as f32 * row,
            Geometry::Measured(tops) => tops.get(index).or(tops.last()).copied().unwrap_or(0.0),
        }
    }

    fn window(self, offset: f32, viewport: f32) -> Range<usize> {
        match self {
            Geometry::Fixed { row, count } => window(offset, viewport, row, count),
            Geometry::Measured(tops) => measured_window(tops, offset, viewport),
        }
    }

    /// The row the offset falls in.
    #[cfg(feature = "log-viewer")]
    fn row_at(self, offset: f32) -> usize {
        match self {
            Geometry::Fixed { row, .. } if row > 0.0 => (offset / row).floor() as usize,
            Geometry::Fixed { .. } => 0,
            Geometry::Measured(tops) => {
                tops.partition_point(|&top| top <= offset).saturating_sub(1)
            }
        }
    }
}

#[derive(Debug, Default)]
struct State {
    offset: f32,
    /// The token of the row last revealed.
    revealed: Option<u64>,
    /// Where the pointer holds the thumb, from its top, while dragging.
    drag: Option<f32>,
    /// A finger scrolling the list: which one, where it started and the
    /// offset then.
    touch: Option<(Finger, f32, f32)>,
    /// Rows dropped from the front, as of the last layout.
    origin: u64,
    /// The content height, as of the last layout.
    content: f32,
    /// Rows that fit in the view, and the count last reported.
    page: usize,
    reported_page: usize,
    /// Where each measured row starts, followed by the content height.
    tops: Vec<f32>,
    /// Measured heights by row key, at the width they were measured.
    #[cfg(feature = "log-viewer")]
    heights: HashMap<u64, f32>,
    #[cfg(feature = "log-viewer")]
    measured_width: f32,
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

        let state = tree.state.downcast_mut::<State>();
        // The height of rows dropped from the front since the last layout,
        // from the heights they had then.
        let dropped = self.origin.saturating_sub(state.origin);
        let dropped_height = if dropped == 0 {
            0.0
        } else if self.is_measured() {
            let index = usize::try_from(dropped).unwrap_or(usize::MAX);
            Geometry::Measured(&state.tops).top(index)
        } else {
            dropped as f32 * self.row_height
        };
        state.origin = self.origin;
        #[cfg(feature = "log-viewer")]
        self.prepare(state, width);
        let content = self.geometry(state).content();

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
        let inner = if self.is_measured() || content > viewport + 0.5 {
            (size.width - GUTTER).max(0.0)
        } else {
            size.width
        };
        state.page = page(viewport, self.row_height);

        let revealed = self.place(state, viewport, dropped_height);
        let range = self.geometry(state).window(state.offset, viewport);
        self.build(tree, &range);
        let node = self.layout_rows(tree, renderer, inner);
        #[cfg(feature = "log-viewer")]
        let (range, node) = self.settle(tree, renderer, inner, viewport, revealed, (range, node));
        #[cfg(not(feature = "log-viewer"))]
        let _ = revealed;

        let state = tree.state.downcast_mut::<State>();
        let geometry = self.geometry(state);
        let (content, first_top) = (geometry.content(), geometry.top(range.start));
        state.content = content;
        state.offset = clamp_offset(state.offset, content, viewport);
        let top = header_height + first_top - state.offset;

        let mut nodes = Vec::with_capacity(2);
        nodes.push(node.move_to(Point::new(0.0, top)));
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
        let Some((top, height)) = thumb(state.offset, state.content, rows.height, track.height)
        else {
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
        let state = tree.state.downcast_mut::<State>();
        if state.page != state.reported_page
            && let Some(on_page) = &self.on_page
        {
            state.reported_page = state.page;
            shell.publish(on_page(state.page));
        }
        let content = state.content;
        if scroll(state, event, cursor, rows, content, shell) {
            let at_end = at_end(state.offset, content, rows.height);
            if at_end != self.follow
                && let Some(on_follow) = &self.on_follow
            {
                shell.publish(on_follow(at_end));
            }
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
        let overflows = state.content > rows.height;
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
/// whether the offset moved or the event was used up, so the rows must
/// not see it.
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
        assert_eq!(reveal_span(0.0, 64.0, 96.0, 100.0), 0.0, "already in view");
        assert_eq!(
            reveal_span(0.0, 160.0, 192.0, 100.0),
            92.0,
            "below: its bottom edge meets the view's"
        );
        assert_eq!(
            reveal_span(200.0, 96.0, 128.0, 100.0),
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

    #[cfg(feature = "log-viewer")]
    #[test]
    fn tops_accumulate_the_heights() {
        let tops = tops([20.0, 60.0, 20.0].into_iter());
        assert_eq!(tops, [0.0, 20.0, 80.0, 100.0]);
        assert_eq!(super::tops(std::iter::empty()), [0.0]);
    }

    #[test]
    fn measured_rows_find_their_window_from_their_tops() {
        let tops = [0.0, 20.0, 80.0, 100.0, 120.0, 220.0, 240.0];
        assert_eq!(measured_window(&tops, 0.0, 50.0), 0..6, "overscan below");
        let deep: Vec<f32> = (0..=100).map(|row| row as f32 * 10.0).collect();
        assert_eq!(
            measured_window(&deep, 500.0, 50.0),
            50 - OVERSCAN..55 + OVERSCAN
        );
        assert_eq!(measured_window(&[0.0], 0.0, 50.0), 0..0, "no rows");
    }

    #[test]
    fn geometry_reads_where_measured_and_fixed_rows_start() {
        let tops = [0.0, 20.0, 80.0, 100.0];
        let geometry = Geometry::Measured(&tops);
        assert_eq!(geometry.content(), 100.0);
        assert_eq!(geometry.top(1), 20.0);
        assert_eq!(geometry.top(9), 100.0, "past the end is the end");
        let fixed = Geometry::Fixed {
            row: 10.0,
            count: 5,
        };
        assert_eq!(fixed.content(), 50.0);
        assert_eq!(fixed.top(9), 50.0);
    }

    #[cfg(feature = "log-viewer")]
    #[test]
    fn the_row_at_an_offset_is_the_one_containing_it() {
        let tops = [0.0, 20.0, 80.0, 100.0];
        let geometry = Geometry::Measured(&tops);
        assert_eq!(geometry.row_at(0.0), 0);
        assert_eq!(geometry.row_at(50.0), 1);
        assert_eq!(geometry.row_at(80.0), 2);
        assert_eq!(
            Geometry::Fixed {
                row: 10.0,
                count: 5
            }
            .row_at(25.0),
            2
        );
    }

    #[test]
    fn the_end_counts_within_a_pixel() {
        assert!(at_end(400.0, 500.0, 100.0));
        assert!(at_end(399.5, 500.0, 100.0));
        assert!(!at_end(390.0, 500.0, 100.0));
        assert!(at_end(0.0, 50.0, 100.0), "content that fits is at its end");
    }

    #[test]
    fn a_page_is_the_whole_rows_in_view() {
        assert_eq!(page(100.0, 32.0), 3);
        assert_eq!(page(10.0, 32.0), 1, "at least one");
        assert_eq!(page(100.0, 0.0), 1);
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
