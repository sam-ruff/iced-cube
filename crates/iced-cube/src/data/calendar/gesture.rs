//! Gestures the calendar's parts share: telling a double click from a
//! single one, and dragging entries across whole days.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer as _, Shell, overlay, renderer};
use iced::keyboard::{self, key::Named};
use iced::time::{Duration, Instant};
use iced::{
    Border, Element, Event as Input, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
    mouse, touch,
};
use jiff::civil::Date;

use super::entry::Occurrence;
use super::parts::{Handler, send};
use super::state::{Event, days_between};
use super::style;
use crate::theme::{Tokens, radius};

/// How long after one press a second still makes a double click, unless
/// the state sets another interval.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// How far apart two presses may be and still make a double click.
const CLICK_SLOP: f32 = 6.0;
/// How far a press may move and still be a click.
const DRAG_SLOP: f32 = 4.0;

/// A press, for counting it towards a double click.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Click {
    pub position: Point,
    pub at: Instant,
    /// One for a single press, two for the second of a double click.
    pub count: u8,
}

/// The press at `position` and `now`, counted after `previous`. A third
/// press starts again at one, so a triple click is a double and a single.
pub(super) fn click(
    previous: Option<Click>,
    position: Point,
    now: Instant,
    interval: Duration,
) -> Click {
    let repeated = previous.is_some_and(|previous| {
        previous.count == 1
            && position.distance(previous.position) <= CLICK_SLOP
            && now.saturating_duration_since(previous.at) <= interval
    });
    Click {
        position,
        at: now,
        count: if repeated { 2 } else { 1 },
    }
}

/// Wraps something that sends a press, such as an entry's button, and
/// sends `double` instead when the press completes a double click or a
/// double tap.
pub(super) struct Pressable<'a, Message> {
    content: Element<'a, Message>,
    double: Option<Message>,
    interval: Duration,
}

impl<'a, Message> Pressable<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        double: Option<Message>,
        interval: Duration,
    ) -> Self {
        Self {
            content: content.into(),
            double,
            interval,
        }
    }
}

#[derive(Debug, Default)]
struct Clicks {
    last: Option<Click>,
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Pressable<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Clicks>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Clicks::default())
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
        event: &Input,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let pressed_at = match event {
            Input::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Input::Touch(touch::Event::FingerPressed { .. }) => {
                cursor.position_over(layout.bounds())
            }
            _ => None,
        };
        let clicks = tree.state.downcast_mut::<Clicks>();
        if let Some(position) = pressed_at {
            clicks.last = Some(click(
                clicks.last,
                position,
                crate::time::now(),
                self.interval,
            ));
        }
        let double = clicks
            .last
            .filter(|last| last.count == 2)
            .and(self.double.clone());

        let mut messages = Vec::new();
        let mut inner = Shell::new(&mut messages);
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            &mut inner,
            viewport,
        );
        let published = !inner.is_empty();
        shell.merge(inner, |message| double.clone().unwrap_or(message));
        if published && double.is_some() {
            tree.state.downcast_mut::<Clicks>().last = None;
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

/// How the days lie inside a [`DayDrag`]: rows of equal height, each a
/// run of equal columns after a gutter, with lanes of bars from `top`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DayGrid {
    pub columns: usize,
    pub rows: usize,
    /// The width before the first column.
    pub left: f32,
    /// How far below the top of a row the first lane is.
    pub top: f32,
    pub bar_height: f32,
    pub lane_gap: f32,
}

impl DayGrid {
    fn column_width(&self, bounds: Size) -> f32 {
        (bounds.width - self.left).max(0.0) / self.columns.max(1) as f32
    }

    fn row_height(&self, bounds: Size) -> f32 {
        bounds.height / self.rows.max(1) as f32
    }

    /// The rectangle of a bar from `first` over `days` columns of `row`
    /// in `lane`, from the grid's corner.
    pub fn bar(
        &self,
        bounds: Size,
        row: usize,
        first: usize,
        days: usize,
        lane: usize,
    ) -> Rectangle {
        let width = self.column_width(bounds);
        Rectangle::new(
            Point::new(
                self.left + first as f32 * width + 2.0,
                row as f32 * self.row_height(bounds)
                    + self.top
                    + lane as f32 * (self.bar_height + self.lane_gap),
            ),
            Size::new((days as f32 * width - 4.0).max(0.0), self.bar_height),
        )
    }

    /// The cell under `point`, counted along the rows, kept inside the
    /// grid so a drag past its edge lands on the nearest day.
    pub fn cell_at(&self, bounds: Size, point: Point) -> usize {
        let width = self.column_width(bounds).max(1.0);
        let height = self.row_height(bounds).max(1.0);
        let last_column = self.columns.max(1) - 1;
        let last_row = self.rows.max(1) - 1;
        let column = (((point.x - self.left) / width).floor().max(0.0) as usize).min(last_column);
        let row = ((point.y / height).floor().max(0.0) as usize).min(last_row);
        row * self.columns.max(1) + column
    }
}

/// A bar a [`DayDrag`] can pick up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Target {
    pub row: usize,
    pub first: usize,
    pub days: usize,
    pub lane: usize,
    pub occurrence: Occurrence,
    /// Whether it moves, or is only pressed.
    pub movable: bool,
}

/// A drag in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Drag {
    target: usize,
    origin: Point,
    pointer: Point,
    dragged: bool,
    /// Cancelled with Escape: the release only lets the button go.
    cancelled: bool,
}

/// Lays drags by whole days over bars drawn as buttons, as in the month
/// and the strip of whole-day entries. A press that moves less than a few
/// pixels is left to the bar's own button.
pub(super) struct DayDrag<'a, Message> {
    content: Element<'a, Message>,
    grid: DayGrid,
    /// The date of each cell, along the rows.
    dates: Vec<Date>,
    targets: Vec<Target>,
    time_zone: jiff::tz::TimeZone,
    on_event: Handler<'a, Message>,
}

impl<'a, Message> DayDrag<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        grid: DayGrid,
        dates: Vec<Date>,
        targets: Vec<Target>,
        time_zone: jiff::tz::TimeZone,
        on_event: &Handler<'a, Message>,
    ) -> Self {
        Self {
            content: content.into(),
            grid,
            dates,
            targets,
            time_zone,
            on_event: on_event.clone(),
        }
    }
}

/// How many days a drag from cell `from` to cell `to` moves an entry.
pub(super) fn days_moved(dates: &[Date], from: usize, to: usize) -> i32 {
    match (dates.get(from), dates.get(to)) {
        (Some(from), Some(to)) => days_between(*from, *to),
        _ => 0,
    }
}

/// The cells, along the rows, that `occurrence` covers once moved by
/// `days`, as runs within each row: `(row, first column, columns)`.
pub(super) fn moved_cells(
    dates: &[Date],
    columns: usize,
    occurrence: &Occurrence,
    days: i32,
    time_zone: &jiff::tz::TimeZone,
) -> Vec<(usize, usize, usize)> {
    let (first, last) = occurrence.dates(time_zone);
    let first = super::entry::add_days(first, days);
    let last = super::entry::add_days(last, days);
    let covered: Vec<usize> = dates
        .iter()
        .enumerate()
        .filter(|(_, date)| (first..=last).contains(*date))
        .map(|(index, _)| index)
        .collect();
    let columns = columns.max(1);
    let mut runs: Vec<(usize, usize, usize)> = Vec::new();
    for index in covered {
        let (row, column) = (index / columns, index % columns);
        match runs.last_mut() {
            Some(run) if run.0 == row && run.1 + run.2 == column => run.2 += 1,
            _ => runs.push((row, column, 1)),
        }
    }
    runs
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for DayDrag<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Option<Drag>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(None::<Drag>)
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
        event: &Input,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let local = |point: Point| Point::new(point.x - bounds.x, point.y - bounds.y);
        let drag = *tree.state.downcast_ref::<Option<Drag>>();

        match (event, drag) {
            (Input::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)), _)
                if self.on_event.is_some() =>
            {
                let picked = cursor.position_over(bounds).and_then(|point| {
                    let point = local(point);
                    self.targets
                        .iter()
                        .position(|target| {
                            target.movable
                                && self
                                    .grid
                                    .bar(
                                        bounds.size(),
                                        target.row,
                                        target.first,
                                        target.days,
                                        target.lane,
                                    )
                                    .contains(point)
                        })
                        .map(|target| Drag {
                            target,
                            origin: point,
                            pointer: point,
                            dragged: false,
                            cancelled: false,
                        })
                });
                *tree.state.downcast_mut::<Option<Drag>>() = picked;
            }
            (Input::Mouse(mouse::Event::CursorMoved { .. }), Some(mut drag)) => {
                if let Some(point) = cursor.land().position() {
                    drag.pointer = local(point);
                    drag.dragged |= drag.pointer.distance(drag.origin) > DRAG_SLOP;
                }
                *tree.state.downcast_mut::<Option<Drag>>() = Some(drag);
                if drag.dragged {
                    shell.request_redraw();
                    shell.capture_event();
                    return;
                }
            }
            (Input::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)), Some(drag))
                if drag.dragged || drag.cancelled =>
            {
                *tree.state.downcast_mut::<Option<Drag>>() = None;
                if !drag.cancelled
                    && let Some(target) = self.targets.get(drag.target)
                {
                    let from = self.grid.cell_at(bounds.size(), drag.origin);
                    let to = self.grid.cell_at(bounds.size(), drag.pointer);
                    let days = days_moved(&self.dates, from, to);
                    let event = (days != 0).then_some(Event::MoveDays {
                        entry: target.occurrence.entry,
                        start: target.occurrence.start,
                        days,
                    });
                    if let Some(message) = event.and_then(|event| send(&self.on_event, event)) {
                        shell.publish(message);
                    }
                }
                // The bar's button lets go without taking this as a press.
                self.content.as_widget_mut().update(
                    &mut tree.children[0],
                    event,
                    layout,
                    mouse::Cursor::Unavailable,
                    renderer,
                    clipboard,
                    &mut Shell::new(&mut Vec::new()),
                    viewport,
                );
                shell.request_redraw();
                shell.capture_event();
                return;
            }
            (Input::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)), Some(_)) => {
                *tree.state.downcast_mut::<Option<Drag>>() = None;
            }
            (
                Input::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::Escape),
                    ..
                }),
                Some(drag),
            ) if drag.dragged && !drag.cancelled => {
                *tree.state.downcast_mut::<Option<Drag>>() = Some(Drag {
                    cancelled: true,
                    ..drag
                });
                shell.request_redraw();
                shell.capture_event();
                return;
            }
            (Input::Mouse(mouse::Event::CursorLeft), Some(drag)) if drag.dragged => {
                *tree.state.downcast_mut::<Option<Drag>>() = Some(Drag {
                    cancelled: true,
                    ..drag
                });
                shell.request_redraw();
            }
            _ => {}
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
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let drag = tree.state.downcast_ref::<Option<Drag>>();
        if drag.is_some_and(|drag| drag.dragged && !drag.cancelled) {
            return mouse::Interaction::Grabbing;
        }
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
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
        let Some(drag) = tree
            .state
            .downcast_ref::<Option<Drag>>()
            .filter(|drag| drag.dragged && !drag.cancelled)
        else {
            return;
        };
        let Some(target) = self.targets.get(drag.target) else {
            return;
        };
        let bounds = layout.bounds();
        let from = self.grid.cell_at(bounds.size(), drag.origin);
        let to = self.grid.cell_at(bounds.size(), drag.pointer);
        let days = days_moved(&self.dates, from, to);
        let colours = style::preview(&Tokens::of(theme));
        let runs = moved_cells(
            &self.dates,
            self.grid.columns,
            &target.occurrence,
            days,
            &self.time_zone,
        );
        renderer.with_layer(*viewport, |renderer| {
            for (row, first, columns) in runs {
                let rect = self
                    .grid
                    .bar(bounds.size(), row, first, columns, target.lane);
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(bounds.x + rect.x, bounds.y + rect.y),
                            rect.size(),
                        ),
                        border: Border {
                            color: colours.border,
                            width: 1.5,
                            radius: radius::SM.into(),
                        },
                        ..renderer::Quad::default()
                    },
                    colours.background,
                );
            }
        });
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

impl<'a, Message: Clone + 'a> From<DayDrag<'a, Message>> for Element<'a, Message> {
    fn from(drag: DayDrag<'a, Message>) -> Self {
        Element::new(drag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    fn month() -> Vec<Date> {
        // Monday 1 June 2026 onwards, five weeks.
        (0..35)
            .map(|day| super::super::entry::add_days(date(2026, 6, 1), day))
            .collect()
    }

    const GRID: DayGrid = DayGrid {
        columns: 7,
        rows: 5,
        left: 0.0,
        top: 28.0,
        bar_height: 20.0,
        lane_gap: 2.0,
    };

    #[test]
    fn two_quick_presses_close_together_make_a_double_click() {
        let start = Instant::now();
        let later = |ms| start + Duration::from_millis(ms);
        let first = click(None, Point::new(10.0, 10.0), start, DOUBLE_CLICK);
        assert_eq!(first.count, 1);
        let second = click(
            Some(first),
            Point::new(12.0, 11.0),
            later(200),
            DOUBLE_CLICK,
        );
        assert_eq!(second.count, 2);
        // A third press starts counting again.
        assert_eq!(
            click(
                Some(second),
                Point::new(12.0, 11.0),
                later(300),
                DOUBLE_CLICK
            )
            .count,
            1
        );
        // Too slow, or too far away.
        assert_eq!(
            click(
                Some(first),
                Point::new(10.0, 10.0),
                later(401),
                DOUBLE_CLICK
            )
            .count,
            1
        );
        assert_eq!(
            click(
                Some(first),
                Point::new(30.0, 10.0),
                later(100),
                DOUBLE_CLICK
            )
            .count,
            1
        );
    }

    #[test]
    fn bars_and_cells_follow_the_grid() {
        let size = Size::new(700.0, 500.0);
        let bar = GRID.bar(size, 1, 2, 3, 1);
        assert_eq!(
            bar,
            Rectangle::new(Point::new(202.0, 150.0), Size::new(296.0, 20.0))
        );
        assert_eq!(GRID.cell_at(size, Point::new(250.0, 150.0)), 9);
        // Past the edges the nearest cell is taken.
        assert_eq!(GRID.cell_at(size, Point::new(-40.0, -10.0)), 0);
        assert_eq!(GRID.cell_at(size, Point::new(900.0, 900.0)), 34);

        let lane = DayGrid {
            columns: 7,
            rows: 1,
            left: 52.0,
            ..GRID
        };
        assert_eq!(
            lane.cell_at(Size::new(752.0, 50.0), Point::new(60.0, 10.0)),
            0
        );
        assert_eq!(
            lane.cell_at(Size::new(752.0, 50.0), Point::new(160.0, 10.0)),
            1
        );
    }

    #[test]
    fn a_drag_moves_by_the_days_between_its_cells() {
        let dates = month();
        assert_eq!(days_moved(&dates, 3, 3), 0);
        assert_eq!(days_moved(&dates, 3, 5), 2);
        assert_eq!(days_moved(&dates, 10, 2), -8);
        assert_eq!(days_moved(&dates, 3, 99), 0);
    }

    #[test]
    fn a_moved_entry_covers_runs_of_cells_row_by_row() {
        let dates = month();
        let utc = TimeZone::UTC;
        let midnight = |day| super::super::entry::midnight(date(2026, 6, day), &utc);
        // Saturday 6 to Monday 8 June, moved on by a day: Sunday to Tuesday.
        let occurrence = Occurrence {
            entry: super::super::entry::EntryId(1),
            index: 0,
            start: midnight(6),
            end: midnight(9),
            all_day: true,
        };
        assert_eq!(
            moved_cells(&dates, 7, &occurrence, 0, &utc),
            vec![(0, 5, 2), (1, 0, 1)]
        );
        assert_eq!(
            moved_cells(&dates, 7, &occurrence, 1, &utc),
            vec![(0, 6, 1), (1, 0, 2)]
        );
        // Moved off the grid, nothing is left to draw.
        assert!(moved_cells(&dates, 7, &occurrence, 60, &utc).is_empty());
    }
}
