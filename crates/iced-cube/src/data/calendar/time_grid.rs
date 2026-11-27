//! The week and day views: days side by side against the hours, with
//! entries as blocks that can be dragged.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Renderer as _, Shell, renderer};
use iced::keyboard::{self, key::Named};
use iced::widget::button::Status;
use iced::widget::{
    self, Column, Row, Space, column, container, responsive, row, scrollable, text,
};
use iced::{
    Alignment, Background, Border, Element, Event as Input, Length, Padding, Point, Rectangle,
    Renderer, Size, Task, Theme, mouse, touch,
};
use jiff::civil::Date;
use jiff::tz::TimeZone;

use super::entry::{Draft, Occurrence, Tone, When, add_days};
use super::gesture::{Click, click};
use super::gesture::{DayDrag, DayGrid};
use super::layout::{self as geometry, Block, DAY_MINUTES, Grid, instant_at, minute_on, snap};
use super::month;
use super::parts::{
    CHIP_HEIGHT, Handler, LANE_GAP, day_heading, day_id, day_number, entry_icon, entry_id,
    hatch_handle, lanes, look_of, pressable, selection_of, send, time_label, weekday_label,
};
use super::state::{Allow, Event, SLOT_MINUTES, Selection, State};
use super::style;
use crate::data::ellipsis::ellipsis;
use crate::icon::opaque;
use crate::primitives::scroll_area;
use crate::theme::{self, Tokens, mix, radius, space, text_size};
use iced::advanced::svg::Renderer as _;
use iced::time::{Duration, Instant};

/// The step drags snap to, in minutes.
pub const SNAP_MINUTES: u16 = 15;
/// The step a pressed time slot starts on, in minutes.
pub const SLOT_STEP: u16 = 30;
/// The width of the hour labels.
pub const GUTTER: f32 = 52.0;
/// The narrowest a day column gets before a week shows one day at a time
/// under a strip of its seven days.
pub const MIN_DAY_WIDTH: f32 = 84.0;
/// The most lanes of whole-day entries shown above the hours.
pub const ALL_DAY_LANES: usize = 2;

/// How close to the top or lower edge of a block a press must be to
/// resize it.
const RESIZE_ZONE: f32 = 6.0;
/// The smallest resize zone. A block too short for zones this deep at
/// both ends, with as much again between them, has no edges to drag.
const MIN_RESIZE_ZONE: f32 = 4.0;
/// How far a press may move and still be a click.
const DRAG_SLOP: f32 = 4.0;
/// The width of the stripe down the leading edge of a block.
const STRIPE: f32 = 3.0;
/// The space between a block's edge and its text.
const BLOCK_PADDING: f32 = 4.0;
/// The height of a line of text in a block.
const LINE: f32 = 16.0;
/// How much of the accent today's column takes.
const TODAY_TINT: f32 = 0.3;

/// The id of the scrollable around the hours, for scrolling to a time of
/// day with iced's scrollable operations.
pub fn scroll_id() -> widget::Id {
    widget::Id::new("iced-cube-calendar-hours")
}

/// How far down the hours `hour` is when the first hour on show is
/// `first_hour` and each is `hour_height` tall.
pub fn hour_offset(hour: u8, first_hour: u8, hour_height: f32) -> f32 {
    f32::from(hour.saturating_sub(first_hour)) * hour_height.max(0.0)
}

/// Scrolls the week or day view so `hour` is at the top, such as to the
/// start of the working day when the calendar opens. Pass the hour height
/// the calendar was given, which is
/// [`DEFAULT_HOUR_HEIGHT`](super::DEFAULT_HOUR_HEIGHT) unless set, and run
/// the task once the view is on screen.
///
/// This is for a calendar showing the whole day. One limited with
/// `.hours(...)` starts at its first hour already.
pub fn scroll_to_hour<Message>(hour: u8, hour_height: f32) -> Task<Message> {
    widget::operation::scroll_to(
        scroll_id(),
        scrollable::AbsoluteOffset {
            x: None,
            y: Some(hour_offset(hour, 0, hour_height)),
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Options {
    pub hours: (u8, u8),
    pub hour_height: f32,
    /// The hour scrolled to when a timed view comes on show.
    pub start_hour: Option<u8>,
}

impl Options {
    /// The same hours with each tall enough for them to fill `height`
    /// between them, when they would otherwise leave room below.
    fn stretched_to(self, height: f32) -> Self {
        let hours = f32::from(self.hours.1.saturating_sub(self.hours.0).max(1));
        Self {
            hour_height: self.hour_height.max((height / hours).floor()),
            ..self
        }
    }

    fn grid(&self, width: f32, days: usize) -> Grid {
        Grid {
            width,
            gutter: GUTTER,
            days,
            hour_height: self.hour_height,
            first_hour: self.hours.0,
            last_hour: self.hours.1,
        }
    }
}

/// How many of `wanted` day columns fit in `width`: all of them, or one.
pub fn days_that_fit(width: f32, wanted: usize) -> usize {
    if width < GUTTER + wanted as f32 * MIN_DAY_WIDTH {
        1
    } else {
        wanted.max(1)
    }
}

pub(super) fn view<'a, Message: Clone + 'a>(
    state: &'a State,
    options: Options,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let on_event = on_event.clone();
    responsive(move |size| {
        // The days the view is about, and the ones there is room to show.
        let (first, last) = state.range();
        let wanted: Vec<Date> = (0..7)
            .map(|day| add_days(first, day))
            .take_while(|day| *day <= last)
            .collect();
        let fits = days_that_fit(size.width, wanted.len()) == wanted.len();
        let days: Vec<Date> = if fits {
            wanted.clone()
        } else {
            let focus = state.selected_day().filter(|day| wanted.contains(day));
            vec![focus.unwrap_or(state.cursor())]
        };

        let mut content = Column::new();
        content = content.push(if fits {
            headers(state, &days, &on_event)
        } else {
            strip(state, &wanted, days[0], &on_event)
        });
        if let Some(lane) = all_day(state, &days, &on_event) {
            content = content.push(lane);
        }
        let on_event = on_event.clone();
        content
            .push(rule())
            .push(responsive(move |area| {
                let options = options.stretched_to(area.height);
                // The scrollbar floats over the edge of the last day, so
                // the columns stay under their headers.
                let scroller = scrollable::Scrollable::with_direction(
                    hours(state, &days, options, &on_event),
                    scrollable::Direction::Vertical(scroll_area::floating_scrollbar()),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .id(scroll_id())
                .style(|theme, status| scroll_area::style(&Tokens::of(theme), status));
                let Some(hour) = options.start_hour else {
                    return scroller.into();
                };
                Element::new(StartAt {
                    content: scroller.into(),
                    offset: hour_offset(hour, options.hours.0, options.hour_height),
                    scrolls: state.scrolls(),
                })
            }))
            .into()
    })
    .into()
}

/// Scrolls the hours inside it to the start hour the first time it sees
/// an event, and again whenever [`State`] asks for it after Today or a
/// switch to a timed view.
struct StartAt<'a, Message> {
    content: Element<'a, Message>,
    offset: f32,
    scrolls: u32,
}

impl<Message> Widget<Message, Theme, Renderer> for StartAt<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Option<u32>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(None::<u32>)
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
        if *tree.state.downcast_ref::<Option<u32>>() != Some(self.scrolls) {
            *tree.state.downcast_mut::<Option<u32>>() = Some(self.scrolls);
            let mut scroll = iced::advanced::widget::operation::scrollable::scroll_to::<()>(
                scroll_id(),
                scrollable::AbsoluteOffset {
                    x: None,
                    y: Some(self.offset),
                },
            );
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                &mut scroll,
            );
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
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

fn rule<'a, Message: 'a>() -> Element<'a, Message> {
    container(Space::new())
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(|theme: &Theme| container::Style {
            background: Some(style::line(&Tokens::of(theme)).into()),
            ..container::Style::default()
        })
        .into()
}

fn day_of(state: &State, date: Date, selected: bool) -> style::Day {
    style::Day {
        today: date == state.today(),
        selected,
        in_month: true,
    }
}

/// A day's name over its number, as a button that selects the day.
fn day_button<'a, Message: Clone + 'a>(
    state: &State,
    date: Date,
    name: String,
    selected: bool,
    large: bool,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let day = day_of(state, date, selected);
    let name = text(name)
        .size(11.0)
        .wrapping(text::Wrapping::None)
        .style(move |theme: &Theme| {
            let tokens = Tokens::of(theme);
            text::Style {
                color: Some(if day.today {
                    tokens.foreground
                } else {
                    tokens.muted_foreground
                }),
            }
        });
    let number = if large {
        day_heading(date, day)
    } else {
        day_number(date, day)
    };
    let button = widget::button(
        column![name, number]
            .spacing(2.0)
            .align_x(Alignment::Center)
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding(Padding::from([space::XS, 0.0]))
    .on_press_maybe(send(on_event, Event::Press(Selection::Day(date))))
    .style(move |theme, status| style::cell(&Tokens::of(theme), day, status));
    container(pressable(state, button, Selection::Day(date), on_event))
        .id(day_id(date))
        .width(Length::FillPortion(1))
        .into()
}

/// The id of the hours of the week and day views, whose bounds
/// [`point_at`] measures from.
pub fn hours_id() -> widget::Id {
    widget::Id::new("iced-cube-calendar-hours-grid")
}

/// The point on screen of `minute` past midnight on the `day`th column
/// of hours, when the hours found by [`hours_id`] have `bounds` and show
/// `days` columns from `first_hour`, each hour `hour_height` tall. For
/// driving a drag in a test without working out the layout by hand.
/// The point is in the middle of the column.
pub fn point_at(
    bounds: Rectangle,
    days: usize,
    day: usize,
    minute: u16,
    first_hour: u8,
    hour_height: f32,
) -> Point {
    let grid = Grid {
        width: bounds.width,
        gutter: GUTTER,
        days: days.max(1),
        hour_height,
        first_hour,
        last_hour: 24,
    };
    Point::new(
        bounds.x + grid.day_x(day) + grid.day_width() / 2.0,
        bounds.y + grid.y_of(minute),
    )
}

/// The days above their columns.
fn headers<'a, Message: Clone + 'a>(
    state: &State,
    days: &[Date],
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let mut headers = row![Space::new().width(Length::Fixed(GUTTER))];
    for date in days {
        let selected = state.is_selected(Selection::Day(*date));
        let name = weekday_label(*date, false);
        headers = headers.push(day_button(state, *date, name, selected, true, on_event));
    }
    headers.into()
}

/// All seven days of the week above the one day on show, for moving
/// between them on a narrow screen.
fn strip<'a, Message: Clone + 'a>(
    state: &State,
    week: &[Date],
    shown: Date,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    Row::with_children(week.iter().map(|date| {
        let name = weekday_label(*date, true);
        day_button(state, *date, name, *date == shown, false, on_event)
    }))
    .into()
}

/// The whole-day entries of `days` as bars above the hours, or `None`
/// when there are none.
fn all_day<'a, Message: Clone + 'a>(
    state: &State,
    days: &[Date],
    on_event: &Handler<'a, Message>,
) -> Option<Element<'a, Message>> {
    let (first, last) = (*days.first()?, *days.last()?);
    let occurrences: Vec<Occurrence> = state
        .shown()
        .between(first, last)
        .into_iter()
        .filter(|occurrence| occurrence.all_day)
        .collect();
    if occurrences.is_empty() {
        return None;
    }
    let bars = geometry::bars(days, &occurrences, state.time_zone());
    // A single day shows them all, since its count of the rest would have
    // nowhere to lead.
    let room = if days.len() == 1 {
        geometry::lane_count(&bars).max(1)
    } else {
        ALL_DAY_LANES
    };
    let fitted = geometry::fit(bars, days.len(), room);
    let label = text("All day")
        .size(11.0)
        .wrapping(text::Wrapping::None)
        .style(|theme: &Theme| text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        });
    let strip = row![
        container(label)
            .width(Length::Fixed(GUTTER))
            .padding(Padding::from([0.0, space::XS]))
            .align_x(Alignment::End),
        lanes(state, days, &fitted, room, on_event).width(Length::Fill),
    ]
    .padding(Padding::from([LANE_GAP, 0.0]))
    .align_y(Alignment::Center);
    let grid = DayGrid {
        columns: days.len(),
        rows: 1,
        left: GUTTER,
        top: LANE_GAP,
        bar_height: CHIP_HEIGHT,
        lane_gap: LANE_GAP,
    };
    Some(
        DayDrag::new(
            strip,
            grid,
            days.to_vec(),
            month::targets(state, 0, &fitted),
            state.time_zone().clone(),
            on_event,
        )
        .into(),
    )
}

/// The time under a block's title. A block squeezed beside others in a
/// week has room for its start only.
fn block_label(block: &Block, week: bool, time_zone: &TimeZone) -> String {
    let start = time_label(block.occurrence.start, time_zone);
    let end = time_label(block.occurrence.end, time_zone);
    if block.starts_before {
        return format!("until {end}");
    }
    if (week && block.columns > 1) || block.occurrence.end <= block.occurrence.start {
        return start;
    }
    format!("{start}-{end}")
}

/// A block as the grid draws it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placed {
    day: usize,
    block: Block,
    tone: Tone,
    look: style::Look,
    /// Held and pressed, but never moved or resized.
    locked: bool,
}

/// The hours of `days` with their timed entries.
fn hours<'a, Message: Clone + 'a>(
    state: &State,
    days: &[Date],
    options: Options,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let time_zone = state.time_zone();
    let grid = options.grid(0.0, days.len());
    let (first_minute, last_minute) = grid.minutes();

    let mut children: Vec<Element<'a, Message>> = (first_minute / 60 + 1..last_minute.div_ceil(60))
        .map(|hour| {
            container(
                text(format!("{hour:02}:00"))
                    .size(11.0)
                    .wrapping(text::Wrapping::None)
                    .style(|theme: &Theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
            )
            .width(Length::Fixed(GUTTER))
            .padding(Padding::from([0.0, space::SM]))
            .align_x(Alignment::End)
            .into()
        })
        .collect();
    let labels = children.len();

    let mut placed = Vec::new();
    for (day, date) in days.iter().enumerate() {
        let occurrences = state.shown().on(*date);
        for block in geometry::blocks(*date, &occurrences, time_zone) {
            let Some(entry) = state.shown().get(block.occurrence.entry) else {
                continue;
            };
            let tone = entry.tone_of();
            let look = look_of(
                state,
                entry,
                state.is_selected(selection_of(&block.occurrence)),
            );
            let colour = move |theme: &Theme| {
                style::marked(&Tokens::of(theme), tone, look, Status::Active)
                    .chip
                    .foreground
            };
            let title = ellipsis(entry.title().to_owned())
                .size(text_size::XS, LINE)
                .font(theme::semibold())
                .colour(colour);
            let title: Element<'a, Message> = match entry_icon(entry, look, tone, text_size::XS) {
                Some(icon) => row![icon, title]
                    .spacing(3.0)
                    .align_y(Alignment::Center)
                    .into(),
                None => title.into(),
            };
            let mut lines = column![title];
            let height = grid.y_of(block.drawn_bottom()) - grid.y_of(block.drawn_top());
            if height >= 2.0 * LINE + BLOCK_PADDING {
                let time = block_label(&block, days.len() > 1, time_zone);
                lines = lines.push(ellipsis(time).size(11.0, LINE).colour(colour));
            }
            children.push(
                container(lines)
                    .id(entry_id(block.occurrence.entry, block.occurrence.start))
                    .into(),
            );
            placed.push(Placed {
                day,
                block,
                tone,
                look,
                locked: entry.is_locked() || look.preview,
            });
        }
    }

    let slot = match state.selection() {
        Some(Selection::Slot { date, minute }) => days
            .iter()
            .position(|day| *day == date)
            .map(|day| (day, minute)),
        _ => None,
    };
    let now = state.now().and_then(|now| {
        let date = state.shown().date_of(now);
        let day = days.iter().position(|day| *day == date)?;
        Some((day, minute_on(date, now, time_zone)))
    });

    Element::new(Hours {
        days: days.to_vec(),
        placed,
        children,
        labels,
        options,
        time_zone: time_zone.clone(),
        today: days.iter().position(|day| *day == state.today()),
        now,
        slot,
        allow: state.allow(),
        double_click: state.double_click(),
        on_event: on_event.clone(),
    })
}

/// What a point on the grid is over.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    /// A block, and the edge the point is on, if any.
    Block {
        index: usize,
        edge: Option<Edge>,
    },
    /// Empty space in a day, at this many minutes.
    Slot {
        day: usize,
        minutes: f32,
    },
    Nothing,
}

fn hit(grid: &Grid, placed: &[Placed], point: Point) -> Hit {
    let over = placed.iter().enumerate().rev().find_map(|(index, placed)| {
        let rect = grid.block(placed.day, &placed.block)?;
        rect.contains(point).then_some((index, rect))
    });
    if let Some((index, rect)) = over {
        let block = &placed[index].block;
        return Hit::Block {
            index,
            edge: edge_at(grid, block, rect, point.y),
        };
    }
    if point.y < 0.0 || point.y >= grid.height() {
        return Hit::Nothing;
    }
    match grid.day_at(point.x) {
        Some(day) => Hit::Slot {
            day,
            minutes: grid.minutes_at(point.y),
        },
        None => Hit::Nothing,
    }
}

/// An edge of a block, which a drag resizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Top,
    Bottom,
}

/// How deep the resize zones of a block `height` tall are, or `None` when
/// it is too short to have any.
fn resize_zone(height: f32) -> Option<f32> {
    // Each zone takes at most a quarter, so at least half the block is
    // left to move it by.
    let zone = (height / 4.0).min(RESIZE_ZONE);
    (zone >= MIN_RESIZE_ZONE).then_some(zone)
}

/// The edge of `block`, drawn at `rect`, that a point at `y` is on.
fn edge_at(grid: &Grid, block: &Block, rect: Rectangle, y: f32) -> Option<Edge> {
    let zone = resize_zone(rect.height)?;
    let (first, last) = grid.minutes();
    // A start that is yesterday or above the first hour on show, or an end
    // that is tomorrow or below the last hour, is not an edge to drag.
    let top = !block.starts_before && block.top >= first;
    let bottom = !block.ends_after && block.bottom <= last;
    if top && y < rect.y + zone {
        return Some(Edge::Top);
    }
    if bottom && y >= rect.y + rect.height - zone {
        return Some(Edge::Bottom);
    }
    None
}

/// What a press took hold of.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Grab {
    /// Empty space, from this minute of a day.
    Create { day: usize, anchor: f32 },
    /// A block, this many minutes below its top.
    Move { index: usize, offset: f32 },
    /// An edge of a block.
    Resize { index: usize, edge: Edge },
    /// A block that cannot be moved from here, because it began on an
    /// earlier day. It can still be pressed.
    Hold { index: usize },
    /// Empty space on a calendar that takes no new entries. It can still
    /// be pressed.
    Point,
}

impl Grab {
    /// The grab as far as `allow` lets it go: without leave to create,
    /// empty space is only a point to press, and without leave to
    /// reschedule a block is only held.
    fn limited_to(self, allow: Allow) -> Self {
        match self {
            Grab::Create { .. } if !allow.create => Grab::Point,
            Grab::Move { index, .. } | Grab::Resize { index, .. } if !allow.reschedule => {
                Grab::Hold { index }
            }
            grab => grab,
        }
    }
}

fn grab(placed: &[Placed], hit: Hit, first_minute: u16) -> Option<Grab> {
    match hit {
        Hit::Nothing => None,
        Hit::Slot { day, minutes } => Some(Grab::Create {
            day,
            anchor: minutes,
        }),
        Hit::Block { index, .. } if placed.get(index)?.locked => Some(Grab::Hold { index }),
        Hit::Block {
            index,
            edge: Some(edge),
        } => Some(Grab::Resize { index, edge }),
        Hit::Block { index, edge: None } => {
            let block = placed.get(index)?.block;
            // A start that is yesterday, or above the first hour on show,
            // cannot be seen, so the block is not moved from here.
            if block.starts_before || block.top < first_minute {
                return Some(Grab::Hold { index });
            }
            Some(Grab::Move { index, offset: 0.0 })
        }
    }
}

/// A span of a day in minutes, as a drag would leave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Span {
    day: usize,
    top: u16,
    bottom: u16,
}

/// Where the drag of `grab` would put things with the pointer at `point`.
fn span(grid: &Grid, placed: &[Placed], grab: Grab, point: Point) -> Option<Span> {
    let (first, last) = grid.minutes();
    let pointer = grid.minutes_at(point.y);
    match grab {
        Grab::Hold { .. } | Grab::Point => None,
        Grab::Create { day, anchor } => {
            let (from, to) = (
                snap(anchor.min(pointer), SNAP_MINUTES),
                snap(anchor.max(pointer), SNAP_MINUTES),
            );
            let from = from.min(last - SNAP_MINUTES);
            Some(Span {
                day,
                top: from,
                bottom: to.max(from + SNAP_MINUTES),
            })
        }
        Grab::Move { index, offset } => {
            let placed = placed.get(index)?;
            let length = u16::try_from(placed.block.occurrence.duration().as_mins())
                .unwrap_or(DAY_MINUTES)
                .max(SNAP_MINUTES);
            let top = snap(pointer - offset, SNAP_MINUTES).clamp(first, last - SNAP_MINUTES);
            Some(Span {
                day: grid.day_at(point.x).unwrap_or(placed.day),
                top,
                bottom: top.saturating_add(length),
            })
        }
        Grab::Resize {
            index,
            edge: Edge::Bottom,
        } => {
            let placed = placed.get(index)?;
            let top = placed.block.top;
            Some(Span {
                day: placed.day,
                top,
                bottom: snap(pointer, SNAP_MINUTES).max(top + SNAP_MINUTES),
            })
        }
        Grab::Resize {
            index,
            edge: Edge::Top,
        } => {
            let placed = placed.get(index)?;
            let bottom = placed.block.bottom;
            Some(Span {
                day: placed.day,
                top: snap(pointer, SNAP_MINUTES).min(bottom.saturating_sub(SNAP_MINUTES)),
                bottom,
            })
        }
    }
}

/// The event a drag ends in, or `None` when nothing would change.
fn dropped(
    days: &[Date],
    time_zone: &TimeZone,
    placed: &[Placed],
    grab: Grab,
    span: Span,
) -> Option<Event> {
    let date = *days.get(span.day)?;
    match grab {
        Grab::Hold { .. } | Grab::Point => None,
        Grab::Create { .. } => Some(Event::Request(Draft {
            when: When::timed(
                instant_at(date, span.top, time_zone),
                instant_at(date, span.bottom, time_zone),
            ),
        })),
        Grab::Move { index, .. } => {
            let occurrence = placed.get(index)?.block.occurrence;
            let start = instant_at(date, span.top, time_zone);
            let end = start.checked_add(occurrence.duration()).ok()?;
            (start != occurrence.start).then_some(Event::Reschedule {
                entry: occurrence.entry,
                start: occurrence.start,
                to: (start, end),
            })
        }
        Grab::Resize {
            index,
            edge: Edge::Top,
        } => {
            let occurrence = placed.get(index)?.block.occurrence;
            let start = instant_at(date, span.top, time_zone);
            (start != occurrence.start).then_some(Event::Reschedule {
                entry: occurrence.entry,
                start: occurrence.start,
                to: (start, occurrence.end),
            })
        }
        Grab::Resize {
            index,
            edge: Edge::Bottom,
        } => {
            let occurrence = placed.get(index)?.block.occurrence;
            let end = instant_at(date, span.bottom, time_zone);
            (end != occurrence.end).then_some(Event::Reschedule {
                entry: occurrence.entry,
                start: occurrence.start,
                to: (occurrence.start, end),
            })
        }
    }
}

/// The event a press without a drag sends: an entry or a time slot is
/// pressed, or double pressed when the press completes a double click.
fn pressed(days: &[Date], placed: &[Placed], hit: Hit, double: bool) -> Option<Event> {
    let selection = match hit {
        Hit::Nothing => return None,
        Hit::Block { index, .. } => selection_of(&placed.get(index)?.block.occurrence),
        Hit::Slot { day, minutes } => {
            let minute = (minutes as u16 / SLOT_STEP) * SLOT_STEP;
            Selection::Slot {
                date: *days.get(day)?,
                minute: minute.min(DAY_MINUTES - SLOT_STEP),
            }
        }
    };
    Some(if double {
        Event::DoublePress(selection)
    } else {
        Event::Press(selection)
    })
}

/// A press in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Press {
    grab: Grab,
    /// Where it started and where the pointer is now, from the grid's
    /// corner.
    origin: Point,
    pointer: Point,
    dragged: bool,
    /// Whether the press completed a double click.
    double: bool,
}

/// What the grid remembers between events.
#[derive(Debug, Default)]
struct Memory {
    press: Option<Press>,
    /// A finger that is down, and where it landed.
    finger: Option<(touch::Finger, Point)>,
    hover: Option<Hit>,
    /// The last press or tap, for telling a double one.
    click: Option<Click>,
}

impl Memory {
    /// Counts a press at `point`, and whether it completes a double one.
    fn count(&mut self, point: Point, interval: Duration) -> bool {
        let click = click(self.click, point, Instant::now(), interval);
        // A double press starts the count again.
        self.click = (click.count == 1).then_some(click);
        click.count == 2
    }
}

struct Hours<'a, Message> {
    days: Vec<Date>,
    placed: Vec<Placed>,
    /// The hour labels, then the text of each block in `placed`.
    children: Vec<Element<'a, Message>>,
    labels: usize,
    options: Options,
    time_zone: TimeZone,
    today: Option<usize>,
    now: Option<(usize, u16)>,
    slot: Option<(usize, u16)>,
    allow: Allow,
    double_click: Duration,
    on_event: Handler<'a, Message>,
}

impl<Message> Hours<'_, Message> {
    fn grid(&self, width: f32) -> Grid {
        self.options.grid(width, self.days.len())
    }

    /// The part of a block its text is laid out in.
    fn text_area(rect: Rectangle) -> Rectangle {
        // A block barely taller than a line centres its title instead of
        // keeping the usual space above it, which would cut the text off.
        let inset = ((rect.height - LINE) / 2.0).clamp(0.0, 2.0);
        Rectangle::new(
            Point::new(rect.x + STRIPE + BLOCK_PADDING, rect.y + inset),
            Size::new(
                (rect.width - STRIPE - BLOCK_PADDING - 1.0).max(0.0),
                (rect.height - 2.0 * inset).max(0.0),
            ),
        )
    }
}

fn local(bounds: Rectangle, point: Point) -> Point {
    Point::new(point.x - bounds.x, point.y - bounds.y)
}

fn fill(renderer: &mut Renderer, bounds: Rectangle, background: impl Into<Background>) {
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            ..renderer::Quad::default()
        },
        background,
    );
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Hours<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Memory>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Memory::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let grid = self.grid(limits.max().width);
        let (first, _) = grid.minutes();
        let labels = self.labels;
        let placed = &self.placed;

        let nodes = self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .enumerate()
            .map(|(index, (child, tree))| {
                let Some(block) = index
                    .checked_sub(labels)
                    .and_then(|block| placed.get(block))
                else {
                    // Hour labels sit across their line, in the gutter.
                    let hour = first / 60 + 1 + u16::try_from(index).unwrap_or(0);
                    let node = child.as_widget_mut().layout(
                        tree,
                        renderer,
                        &layout::Limits::new(Size::ZERO, Size::new(GUTTER, LINE)),
                    );
                    let y = grid.y_of(hour * 60) - node.size().height / 2.0;
                    return node.move_to(Point::new(0.0, y));
                };
                let Some(rect) = grid.block(block.day, &block.block) else {
                    return layout::Node::new(Size::ZERO);
                };
                let area = Self::text_area(rect);
                child
                    .as_widget_mut()
                    .layout(
                        tree,
                        renderer,
                        &layout::Limits::new(Size::ZERO, area.size()),
                    )
                    .move_to(area.position())
            })
            .collect();
        layout::Node::with_children(grid.size(), nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(Some(&hours_id()), layout.bounds());
        operation.traverse(&mut |operation| {
            for ((child, tree), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child
                    .as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            }
        });
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
        // The text of the blocks only watches the pointer, for the
        // tooltips on their icons.
        if matches!(event, Input::Mouse(mouse::Event::CursorMoved { .. })) {
            for ((child, tree), layout) in self
                .children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                child.as_widget_mut().update(
                    tree, event, layout, cursor, renderer, clipboard, shell, viewport,
                );
            }
        }
        if self.on_event.is_none() {
            return;
        }
        let bounds = layout.bounds();
        let grid = self.grid(bounds.width);
        let memory = tree.state.downcast_mut::<Memory>();
        let over = cursor
            .position_over(bounds)
            .map(|point| local(bounds, point));

        match event {
            Input::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(point) = over else {
                    return;
                };
                let target = hit(&grid, &self.placed, point);
                let Some(grab) = grab(&self.placed, target, grid.minutes().0) else {
                    return;
                };
                let mut grab = grab.limited_to(self.allow);
                if let Grab::Move { index, offset } = &mut grab
                    && let Some(placed) = self.placed.get(*index)
                {
                    *offset = grid.minutes_at(point.y) - f32::from(placed.block.top);
                }
                let double = memory.count(point, self.double_click);
                memory.press = Some(Press {
                    grab,
                    origin: point,
                    pointer: point,
                    dragged: false,
                    double,
                });
                shell.capture_event();
            }
            Input::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(press) = &mut memory.press {
                    // A drag carries on with the pointer outside the grid.
                    let Some(point) = cursor.land().position() else {
                        return;
                    };
                    press.pointer = local(bounds, point);
                    press.dragged |= press.pointer.distance(press.origin) > DRAG_SLOP;
                    shell.request_redraw();
                    shell.capture_event();
                    return;
                }
                let hover = over.map(|point| hit(&grid, &self.placed, point));
                if hover != memory.hover {
                    memory.hover = hover;
                    shell.request_redraw();
                }
            }
            Input::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let Some(press) = memory.press.take() else {
                    return;
                };
                let event = if press.dragged {
                    span(&grid, &self.placed, press.grab, press.pointer).and_then(|span| {
                        dropped(&self.days, &self.time_zone, &self.placed, press.grab, span)
                    })
                } else {
                    pressed(
                        &self.days,
                        &self.placed,
                        hit(&grid, &self.placed, press.origin),
                        press.double,
                    )
                };
                if press.dragged {
                    memory.click = None;
                }
                if let Some(message) = event.and_then(|event| send(&self.on_event, event)) {
                    shell.publish(message);
                }
                shell.request_redraw();
                shell.capture_event();
            }
            Input::Mouse(mouse::Event::CursorLeft) => {
                if memory.press.take().is_some() || memory.hover.take().is_some() {
                    shell.request_redraw();
                }
            }
            Input::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Escape),
                ..
            }) => {
                if memory.press.take().is_some_and(|press| press.dragged) {
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            // A finger is never captured, so it can still scroll the hours.
            // One that lands and lifts in the same place is a press.
            Input::Touch(touch::Event::FingerPressed { id, .. }) => {
                memory.finger = over.map(|point| (*id, point));
            }
            // Only a tap that lifts where it landed counts towards a
            // double tap, so scrolling with a finger never makes one.
            Input::Touch(touch::Event::FingerMoved { id, .. }) => {
                let Some((finger, origin)) = memory.finger else {
                    return;
                };
                let moved = cursor
                    .land()
                    .position()
                    .map(|point| local(bounds, point).distance(origin));
                if finger == *id && moved.is_none_or(|moved| moved > DRAG_SLOP) {
                    memory.finger = None;
                }
            }
            Input::Touch(touch::Event::FingerLifted { id, .. }) => {
                let Some((_, origin)) = memory.finger.take_if(|(finger, _)| finger == id) else {
                    return;
                };
                let double = memory.count(origin, self.double_click);
                let event = pressed(
                    &self.days,
                    &self.placed,
                    hit(&grid, &self.placed, origin),
                    double,
                );
                if let Some(message) = event.and_then(|event| send(&self.on_event, event)) {
                    shell.publish(message);
                }
            }
            Input::Touch(touch::Event::FingerLost { .. }) => {
                memory.finger = None;
            }
            _ => {}
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        iced::advanced::overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if self.on_event.is_none() {
            return mouse::Interaction::None;
        }
        let memory = tree.state.downcast_ref::<Memory>();
        if let Some(press) = memory.press.filter(|press| press.dragged) {
            return match press.grab {
                Grab::Move { .. } => mouse::Interaction::Grabbing,
                Grab::Resize { .. } | Grab::Create { .. } => mouse::Interaction::ResizingVertically,
                Grab::Hold { .. } => mouse::Interaction::Pointer,
                Grab::Point => mouse::Interaction::None,
            };
        }
        let bounds = layout.bounds();
        let Some(point) = cursor.position_over(bounds) else {
            return mouse::Interaction::None;
        };
        match hit(&self.grid(bounds.width), &self.placed, local(bounds, point)) {
            Hit::Block {
                index,
                edge: Some(_),
            } if self.allow.reschedule
                && self.placed.get(index).is_some_and(|placed| !placed.locked) =>
            {
                mouse::Interaction::ResizingVertically
            }
            Hit::Block { .. } => mouse::Interaction::Pointer,
            Hit::Slot { .. } | Hit::Nothing => mouse::Interaction::None,
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let tokens = Tokens::of(theme);
        let bounds = layout.bounds();
        let grid = self.grid(bounds.width);
        let memory = tree.state.downcast_ref::<Memory>();
        let (first, last) = grid.minutes();
        let origin = bounds.position();
        let at = |x: f32, y: f32, width: f32, height: f32| {
            Rectangle::new(
                Point::new(origin.x + x, origin.y + y),
                Size::new(width, height),
            )
        };
        let day_width = grid.day_width();
        let line = style::line(&tokens);

        // Today stands out among the days of a week. On its own it has
        // nothing to stand out from.
        if let Some(today) = self.today
            && self.days.len() > 1
        {
            fill(
                renderer,
                at(grid.day_x(today), 0.0, day_width, bounds.height),
                mix(tokens.background, tokens.accent, TODAY_TINT),
            );
        }
        if let Some((day, minute)) = self.slot
            && minute < last
            && minute + SLOT_MINUTES > first
        {
            let top = grid.y_of(minute.max(first));
            let bottom = grid.y_of((minute + SLOT_MINUTES).min(last));
            renderer.fill_quad(
                renderer::Quad {
                    bounds: at(
                        grid.day_x(day) + 1.0,
                        top + 1.0,
                        day_width - 1.0,
                        bottom - top - 1.0,
                    ),
                    border: Border {
                        color: tokens.ring,
                        width: 1.0,
                        radius: radius::SM.into(),
                    },
                    ..renderer::Quad::default()
                },
                tokens.accent,
            );
        }

        for hour in first / 60 + 1..last.div_ceil(60) {
            let y = grid.y_of(hour * 60).round();
            fill(renderer, at(GUTTER, y, bounds.width - GUTTER, 1.0), line);
        }
        for day in 0..self.days.len() {
            fill(
                renderer,
                at(grid.day_x(day).round(), 0.0, 1.0, bounds.height),
                line,
            );
        }

        let mut children = self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children());
        for ((label, tree), layout) in children.by_ref().take(self.labels) {
            label
                .as_widget()
                .draw(tree, renderer, theme, defaults, layout, cursor, viewport);
        }

        let hovered = match (memory.press, memory.hover) {
            (None, Some(Hit::Block { index, .. })) => Some(index),
            _ => None,
        };
        for (index, (placed, ((content, tree), layout))) in
            self.placed.iter().zip(children).enumerate()
        {
            let Some(rect) = grid.block(placed.day, &placed.block) else {
                continue;
            };
            let rect = at(rect.x, rect.y, rect.width, rect.height);
            let status = if hovered == Some(index) {
                Status::Hovered
            } else {
                Status::Active
            };
            let marked = style::marked(&tokens, placed.tone, placed.look, status);
            let chip = marked.chip;
            renderer.fill_quad(
                renderer::Quad {
                    bounds: rect,
                    border: Border {
                        color: marked.border,
                        width: if marked.border.a > 0.0 { 1.0 } else { 0.0 },
                        radius: radius::SM.into(),
                    },
                    ..renderer::Quad::default()
                },
                chip.background,
            );
            if let Some(colour) = marked.hatch
                && let Some(clip) = rect.intersection(viewport)
            {
                let svg = iced::advanced::svg::Svg::new(hatch_handle(rect.size()))
                    .color(opaque(colour))
                    .opacity(colour.a);
                renderer.draw_svg(svg, rect, clip);
            }
            if let Some(colour) = marked.strike {
                let area = Self::text_area(rect);
                fill(
                    renderer,
                    Rectangle::new(
                        Point::new(area.x, (area.y + LINE / 2.0).round()),
                        Size::new(area.width, 1.0),
                    ),
                    colour,
                );
            }
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle::new(rect.position(), Size::new(STRIPE, rect.height)),
                    border: Border {
                        radius: iced::border::Radius::default().left(radius::SM),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                chip.stripe,
            );
            let Some(clip) = rect.intersection(viewport) else {
                continue;
            };
            renderer.with_layer(clip, |renderer| {
                content
                    .as_widget()
                    .draw(tree, renderer, theme, defaults, layout, cursor, viewport);
            });
        }

        if let Some((day, minute)) = self.now
            && (first..=last).contains(&minute)
        {
            let y = grid.y_of(minute).round();
            let colour = style::now_line(&tokens);
            fill(
                renderer,
                at(grid.day_x(day), y - 1.0, day_width, 2.0),
                colour,
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: at(grid.day_x(day) - 4.0, y - 4.0, 8.0, 8.0),
                    border: Border {
                        radius: radius::FULL.into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                colour,
            );
        }

        let preview = memory
            .press
            .filter(|press| press.dragged)
            .and_then(|press| span(&grid, &self.placed, press.grab, press.pointer));
        if let Some(span) = preview {
            let top = grid.y_of(span.top.max(first));
            let bottom = grid.y_of(span.bottom.min(last));
            let colours = style::preview(&tokens);
            renderer.with_layer(*viewport, |renderer| {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: at(
                            grid.day_x(span.day) + 1.0,
                            top,
                            (day_width - geometry::BLOCK_GUTTER).max(0.0),
                            (bottom - top).max(2.0),
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
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::calendar::entry::{Entries, Entry, EntryId};
    use jiff::Timestamp;
    use jiff::civil::date;

    const OPTIONS: Options = Options {
        hours: (0, 24),
        hour_height: 60.0,
        start_hour: None,
    };

    /// Monday 2 to Sunday 8 November 2026.
    fn week() -> Vec<Date> {
        (2..=8).map(|day| date(2026, 11, day)).collect()
    }

    fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
        instant_at(
            date(2026, 11, day),
            u16::try_from(i32::from(hour) * 60 + i32::from(minute)).unwrap_or(0),
            &TimeZone::UTC,
        )
    }

    /// A week grid 752 wide: 100 per day, a minute per pixel.
    fn grid() -> Grid {
        OPTIONS.grid(GUTTER + 700.0, 7)
    }

    fn entries() -> Entries {
        Entries::utc().with([
            // Tuesday 09:00 to 10:00.
            Entry::new(EntryId(1), "Stand-up", at(3, 9, 0), at(3, 10, 0)),
            // Wednesday 23:00 to Thursday 01:00.
            Entry::new(EntryId(2), "Deploy", at(4, 23, 0), at(5, 1, 0)),
            // Friday, ten minutes.
            Entry::new(EntryId(3), "Call", at(6, 12, 0), at(6, 12, 10)),
        ])
    }

    fn placed(entries: &Entries) -> Vec<Placed> {
        let mut placed = Vec::new();
        for (day, date) in week().iter().enumerate() {
            for block in geometry::blocks(*date, &entries.on(*date), entries.time_zone()) {
                placed.push(Placed {
                    day,
                    block,
                    tone: Tone::Primary,
                    look: style::Look::default(),
                    locked: false,
                });
            }
        }
        placed
    }

    /// A point in a day column, by minutes from midnight.
    fn point(day: usize, minute: f32) -> Point {
        Point::new(GUTTER + day as f32 * 100.0 + 20.0, minute)
    }

    fn index_of(placed: &[Placed], id: u64, day: usize) -> usize {
        placed
            .iter()
            .position(|placed| placed.block.occurrence.entry == EntryId(id) && placed.day == day)
            .unwrap_or(usize::MAX)
    }

    #[test]
    fn an_hour_is_that_many_hour_heights_down() {
        assert_eq!(hour_offset(8, 0, 48.0), 384.0);
        assert_eq!(hour_offset(0, 0, 48.0), 0.0);
        assert_eq!(hour_offset(10, 8, 40.0), 80.0);
        assert_eq!(hour_offset(6, 8, 40.0), 0.0, "before the first hour");
        assert_eq!(hour_offset(8, 0, -5.0), 0.0);
    }

    #[test]
    fn hours_stretch_to_fill_but_never_shrink() {
        let working_day = Options {
            hours: (8, 18),
            hour_height: 40.0,
            start_hour: None,
        };
        assert_eq!(working_day.stretched_to(555.0).hour_height, 55.0);
        assert_eq!(working_day.stretched_to(300.0).hour_height, 40.0);
        assert_eq!(working_day.stretched_to(0.0).hour_height, 40.0);
        assert_eq!(working_day.stretched_to(555.0).hours, (8, 18));
    }

    #[test]
    fn a_view_shows_all_its_days_or_one() {
        assert_eq!(days_that_fit(GUTTER + 7.0 * MIN_DAY_WIDTH, 7), 7);
        assert_eq!(days_that_fit(GUTTER + 7.0 * MIN_DAY_WIDTH - 1.0, 7), 1);
        assert_eq!(days_that_fit(328.0, 7), 1);
        assert_eq!(days_that_fit(720.0, 7), 7);
        assert_eq!(days_that_fit(420.0, 4), 4);
        assert_eq!(days_that_fit(328.0, 4), 1);
        assert_eq!(days_that_fit(328.0, 1), 1);
        assert_eq!(days_that_fit(10.0, 0), 1);
    }

    #[test]
    fn points_hit_blocks_their_edges_slots_or_nothing() {
        let entries = entries();
        let placed = placed(&entries);
        let grid = grid();
        let stand_up = index_of(&placed, 1, 1);

        assert_eq!(
            hit(&grid, &placed, point(1, 570.0)),
            Hit::Block {
                index: stand_up,
                edge: None
            }
        );
        assert_eq!(
            hit(&grid, &placed, point(1, 595.0)),
            Hit::Block {
                index: stand_up,
                edge: Some(Edge::Bottom)
            }
        );
        assert_eq!(
            hit(&grid, &placed, point(1, 700.0)),
            Hit::Slot {
                day: 1,
                minutes: 700.0
            }
        );
        assert_eq!(hit(&grid, &placed, Point::new(10.0, 570.0)), Hit::Nothing);
        assert_eq!(hit(&grid, &placed, point(1, 2000.0)), Hit::Nothing);
        assert_eq!(hit(&grid, &placed, point(1, -5.0)), Hit::Nothing);
    }

    #[test]
    fn the_sliver_beside_a_block_is_free_for_a_new_entry() {
        let entries = entries();
        let placed = placed(&entries);
        let beside = Point::new(GUTTER + 2.0 * 100.0 - 3.0, 570.0);
        assert!(matches!(
            hit(&grid(), &placed, beside),
            Hit::Slot { day: 1, .. }
        ));
    }

    #[test]
    fn a_block_that_carries_on_tomorrow_has_no_edge_to_resize() {
        let entries = entries();
        let placed = placed(&entries);
        let tonight = index_of(&placed, 2, 2);
        assert_eq!(
            hit(&grid(), &placed, point(2, 1437.0)),
            Hit::Block {
                index: tonight,
                edge: None
            }
        );
        // The part after midnight began yesterday, so it is held, not moved.
        let morning = index_of(&placed, 2, 3);
        let target = hit(&grid(), &placed, point(3, 20.0));
        assert_eq!(
            grab(&placed, target, 0),
            Some(Grab::Hold { index: morning })
        );
        assert_eq!(
            span(
                &grid(),
                &placed,
                Grab::Hold { index: morning },
                point(3, 300.0)
            ),
            None
        );
    }

    #[test]
    fn a_short_block_is_too_small_to_have_an_edge() {
        let entries =
            Entries::utc().with([Entry::new(EntryId(1), "Tiny", at(3, 9, 0), at(3, 9, 10))]);
        let placed = placed(&entries);
        let small = Grid {
            hour_height: 30.0,
            ..grid()
        };
        // Half an hour is 15 pixels here, under three resize zones.
        assert_eq!(
            hit(&small, &placed, point(1, 281.0)),
            Hit::Block {
                index: 0,
                edge: None
            }
        );
    }

    #[test]
    fn presses_grab_what_they_hit() {
        let entries = entries();
        let placed = placed(&entries);
        let stand_up = index_of(&placed, 1, 1);
        assert_eq!(grab(&placed, Hit::Nothing, 0), None);
        assert_eq!(
            grab(
                &placed,
                Hit::Slot {
                    day: 4,
                    minutes: 600.0
                },
                0
            ),
            Some(Grab::Create {
                day: 4,
                anchor: 600.0
            })
        );
        assert_eq!(
            grab(
                &placed,
                Hit::Block {
                    index: stand_up,
                    edge: Some(Edge::Bottom)
                },
                0
            ),
            Some(Grab::Resize {
                index: stand_up,
                edge: Edge::Bottom
            })
        );
        assert!(matches!(
            grab(
                &placed,
                Hit::Block {
                    index: stand_up,
                    edge: None
                },
                0
            ),
            Some(Grab::Move { index, .. }) if index == stand_up
        ));

        // With the grid starting at 09:30 its start is out of sight, so it
        // is held where it is; and one running past the last hour has no
        // edge to drag.
        let held = grab(
            &placed,
            Hit::Block {
                index: stand_up,
                edge: None,
            },
            570,
        );
        assert_eq!(held, Some(Grab::Hold { index: stand_up }));
        let until_ten = Grid {
            last_hour: 10,
            ..grid()
        };
        assert_eq!(
            hit(&until_ten, &placed, point(1, 597.0)),
            Hit::Block {
                index: stand_up,
                edge: Some(Edge::Bottom)
            },
            "ends at the last hour, so its edge is there"
        );
        let long = Entries::utc().with([Entry::new(EntryId(1), "Long", at(3, 9, 0), at(3, 11, 0))]);
        assert_eq!(
            hit(&until_ten, &self::placed(&long), point(1, 597.0)),
            Hit::Block {
                index: 0,
                edge: None
            },
            "ends below the last hour"
        );
    }

    #[test]
    fn grabs_go_only_as_far_as_they_are_allowed() {
        let create = Grab::Create {
            day: 1,
            anchor: 600.0,
        };
        let moving = Grab::Move {
            index: 2,
            offset: 5.0,
        };
        let resizing = Grab::Resize {
            index: 2,
            edge: Edge::Top,
        };

        for grab in [create, moving, resizing, Grab::Hold { index: 2 }] {
            assert_eq!(grab.limited_to(Allow::ALL), grab);
        }
        assert_eq!(create.limited_to(Allow::NONE), Grab::Point);
        assert_eq!(moving.limited_to(Allow::NONE), Grab::Hold { index: 2 });
        assert_eq!(resizing.limited_to(Allow::NONE), Grab::Hold { index: 2 });

        let no_new = Allow {
            create: false,
            ..Allow::ALL
        };
        assert_eq!(create.limited_to(no_new), Grab::Point);
        assert_eq!(moving.limited_to(no_new), moving);

        // A press that cannot drag leaves nothing behind when it moves.
        assert_eq!(span(&grid(), &[], Grab::Point, point(1, 700.0)), None);
    }

    #[test]
    fn dragging_empty_space_asks_for_that_time_either_way_up() {
        let entries = entries();
        let placed = placed(&entries);
        let create = Grab::Create {
            day: 4,
            anchor: 602.0,
        };
        let down = span(&grid(), &placed, create, point(4, 688.0));
        assert_eq!(
            down,
            Some(Span {
                day: 4,
                top: 600,
                bottom: 690
            })
        );
        let up = span(&grid(), &placed, create, point(4, 541.0));
        assert_eq!(
            up,
            Some(Span {
                day: 4,
                top: 540,
                bottom: 600
            })
        );
        // Wandering into another day keeps the day it started on.
        let across = span(&grid(), &placed, create, point(6, 688.0));
        assert_eq!(across.map(|span| span.day), Some(4));

        let Some(span) = down else {
            panic!("a span");
        };
        assert_eq!(
            dropped(&week(), &TimeZone::UTC, &placed, create, span),
            Some(Event::Request(Draft {
                when: When::timed(at(6, 10, 0), at(6, 11, 30))
            }))
        );
    }

    #[test]
    fn a_drag_that_ends_where_it_began_still_has_a_length() {
        let create = Grab::Create {
            day: 0,
            anchor: 600.0,
        };
        assert_eq!(
            span(&grid(), &[], create, point(0, 603.0)),
            Some(Span {
                day: 0,
                top: 600,
                bottom: 615
            })
        );
        let at_the_end = Grab::Create {
            day: 0,
            anchor: 1439.0,
        };
        assert_eq!(
            span(&grid(), &[], at_the_end, point(0, 1440.0)),
            Some(Span {
                day: 0,
                top: 1425,
                bottom: 1440
            })
        );
    }

    #[test]
    fn dragging_a_block_moves_it_in_time_and_across_days() {
        let entries = entries();
        let placed = placed(&entries);
        let stand_up = index_of(&placed, 1, 1);
        // Grabbed 20 minutes below its top, dropped on Thursday at 14:27.
        let moving = Grab::Move {
            index: stand_up,
            offset: 20.0,
        };
        let to = span(&grid(), &placed, moving, point(3, 867.0));
        assert_eq!(
            to,
            Some(Span {
                day: 3,
                top: 840,
                bottom: 900
            })
        );
        let Some(to) = to else {
            panic!("a span");
        };
        assert_eq!(
            dropped(&week(), &TimeZone::UTC, &placed, moving, to),
            Some(Event::Reschedule {
                entry: EntryId(1),
                start: at(3, 9, 0),
                to: (at(5, 14, 0), at(5, 15, 0)),
            })
        );

        // Dragged off the side of the grid, it stays on its own day.
        let off = span(&grid(), &placed, moving, Point::new(5.0, 867.0));
        assert_eq!(off.map(|span| span.day), Some(1));
    }

    #[test]
    fn dropping_a_block_where_it_was_changes_nothing() {
        let entries = entries();
        let placed = placed(&entries);
        let stand_up = index_of(&placed, 1, 1);
        let moving = Grab::Move {
            index: stand_up,
            offset: 20.0,
        };
        let Some(same) = span(&grid(), &placed, moving, point(1, 563.0)) else {
            panic!("a span");
        };
        assert_eq!(
            dropped(&week(), &TimeZone::UTC, &placed, moving, same),
            None
        );

        for (edge, y) in [(Edge::Bottom, 598.0), (Edge::Top, 542.0)] {
            let resizing = Grab::Resize {
                index: stand_up,
                edge,
            };
            let Some(same) = span(&grid(), &placed, resizing, point(1, y)) else {
                panic!("a span");
            };
            assert_eq!(
                dropped(&week(), &TimeZone::UTC, &placed, resizing, same),
                None
            );
        }
    }

    #[test]
    fn a_moved_block_keeps_its_real_length_and_stays_in_the_day() {
        let entries = entries();
        let placed = placed(&entries);
        // The ten minute call, drawn as half an hour.
        let call = index_of(&placed, 3, 4);
        let moving = Grab::Move {
            index: call,
            offset: 0.0,
        };
        let Some(to) = span(&grid(), &placed, moving, point(4, 5000.0)) else {
            panic!("a span");
        };
        assert_eq!(to.top, 1425, "kept inside the day");
        assert_eq!(
            dropped(&week(), &TimeZone::UTC, &placed, moving, to),
            Some(Event::Reschedule {
                entry: EntryId(3),
                start: at(6, 12, 0),
                to: (at(6, 23, 45), at(6, 23, 55)),
            })
        );
    }

    #[test]
    fn dragging_the_edge_changes_the_end_and_keeps_a_minimum() {
        let entries = entries();
        let placed = placed(&entries);
        let stand_up = index_of(&placed, 1, 1);
        let resizing = Grab::Resize {
            index: stand_up,
            edge: Edge::Bottom,
        };

        let Some(longer) = span(&grid(), &placed, resizing, point(1, 668.0)) else {
            panic!("a span");
        };
        assert_eq!((longer.top, longer.bottom), (540, 675));
        assert_eq!(
            dropped(&week(), &TimeZone::UTC, &placed, resizing, longer),
            Some(Event::Reschedule {
                entry: EntryId(1),
                start: at(3, 9, 0),
                to: (at(3, 9, 0), at(3, 11, 15)),
            })
        );

        let above = span(&grid(), &placed, resizing, point(1, 300.0));
        assert_eq!(above.map(|span| span.bottom), Some(555));
        let elsewhere = span(&grid(), &placed, resizing, point(5, 668.0));
        assert_eq!(elsewhere.map(|span| span.day), Some(1));
    }

    #[test]
    fn dragging_the_top_edge_changes_the_start_and_keeps_the_end() {
        let entries = entries();
        let placed = placed(&entries);
        let stand_up = index_of(&placed, 1, 1);
        let resizing = Grab::Resize {
            index: stand_up,
            edge: Edge::Top,
        };

        let Some(earlier) = span(&grid(), &placed, resizing, point(1, 452.0)) else {
            panic!("a span");
        };
        assert_eq!((earlier.top, earlier.bottom), (450, 600));
        assert_eq!(
            dropped(&week(), &TimeZone::UTC, &placed, resizing, earlier),
            Some(Event::Reschedule {
                entry: EntryId(1),
                start: at(3, 9, 0),
                to: (at(3, 7, 30), at(3, 10, 0)),
            })
        );

        // Dragged below the end, it keeps the shortest length.
        let Some(below) = span(&grid(), &placed, resizing, point(1, 800.0)) else {
            panic!("a span");
        };
        assert_eq!((below.top, below.bottom), (585, 600));
        assert_eq!(
            dropped(&week(), &TimeZone::UTC, &placed, resizing, below),
            Some(Event::Reschedule {
                entry: EntryId(1),
                start: at(3, 9, 0),
                to: (at(3, 9, 45), at(3, 10, 0)),
            })
        );

        // Above the first hour on show, it stops at that hour.
        let from_eight = Grid {
            first_hour: 8,
            ..grid()
        };
        let above = span(&from_eight, &placed, resizing, point(1, -100.0));
        assert_eq!(above.map(|span| span.top), Some(480));
        let elsewhere = span(&grid(), &placed, resizing, point(5, 452.0));
        assert_eq!(elsewhere.map(|span| span.day), Some(1));
    }

    #[test]
    fn a_block_has_a_top_edge_only_where_its_start_is_on_show() {
        let entries = entries();
        let placed = placed(&entries);
        let stand_up = index_of(&placed, 1, 1);
        assert_eq!(
            hit(&grid(), &placed, point(1, 542.0)),
            Hit::Block {
                index: stand_up,
                edge: Some(Edge::Top)
            }
        );
        assert_eq!(
            grab(
                &placed,
                Hit::Block {
                    index: stand_up,
                    edge: Some(Edge::Top)
                },
                0
            ),
            Some(Grab::Resize {
                index: stand_up,
                edge: Edge::Top
            })
        );

        // The part of the deploy after midnight began yesterday.
        let morning = index_of(&placed, 2, 3);
        assert_eq!(
            hit(&grid(), &placed, point(3, 2.0)),
            Hit::Block {
                index: morning,
                edge: None
            }
        );
        // Its evening part has a top edge but no lower one.
        let tonight = index_of(&placed, 2, 2);
        assert_eq!(
            hit(&grid(), &placed, point(2, 1382.0)),
            Hit::Block {
                index: tonight,
                edge: Some(Edge::Top)
            }
        );

        // With the grid starting at 09:00 an 08:00 start is out of sight.
        let from_nine = Grid {
            first_hour: 9,
            ..grid()
        };
        let long = Entries::utc().with([Entry::new(EntryId(1), "Long", at(3, 8, 0), at(3, 11, 0))]);
        let long = self::placed(&long);
        assert_eq!(
            hit(&from_nine, &long, point(1, 2.0)),
            Hit::Block {
                index: 0,
                edge: None
            }
        );
        assert_eq!(
            hit(&from_nine, &long, point(1, 115.0)),
            Hit::Block {
                index: 0,
                edge: Some(Edge::Bottom)
            }
        );
    }

    #[test]
    fn short_blocks_have_shallower_zones_and_room_to_move() {
        assert_eq!(resize_zone(60.0), Some(RESIZE_ZONE));
        assert_eq!(resize_zone(20.0), Some(5.0));
        assert_eq!(resize_zone(16.0), Some(MIN_RESIZE_ZONE));
        assert_eq!(resize_zone(15.0), None);
        for height in [16.0, 18.0, 20.0, 24.0, 40.0] {
            let Some(zone) = resize_zone(height) else {
                panic!("zones at {height}");
            };
            assert!(height - 2.0 * zone >= height / 2.0, "middle at {height}");
        }

        // Half an hour is 20 pixels at 40 an hour, less the gap between
        // blocks: zones under 5 pixels at each end and 9 between them.
        let entries =
            Entries::utc().with([Entry::new(EntryId(1), "Short", at(3, 9, 0), at(3, 9, 30))]);
        let placed = placed(&entries);
        let small = Grid {
            hour_height: 40.0,
            ..grid()
        };
        let edge = |y: f32| match hit(&small, &placed, point(1, y)) {
            Hit::Block { edge, .. } => edge,
            other => panic!("{other:?}"),
        };
        assert_eq!(edge(361.0), Some(Edge::Top));
        assert_eq!(edge(366.0), None);
        assert_eq!(edge(372.0), None);
        assert_eq!(edge(375.0), Some(Edge::Bottom));
    }

    #[test]
    fn a_press_without_a_drag_selects_the_entry_or_the_slot() {
        let entries = entries();
        let placed = placed(&entries);
        let stand_up = index_of(&placed, 1, 1);
        assert_eq!(
            pressed(
                &week(),
                &placed,
                Hit::Block {
                    index: stand_up,
                    edge: Some(Edge::Bottom)
                },
                false
            ),
            Some(Event::Press(Selection::Entry {
                entry: EntryId(1),
                start: at(3, 9, 0)
            }))
        );
        assert_eq!(
            pressed(
                &week(),
                &placed,
                Hit::Slot {
                    day: 4,
                    minutes: 644.0
                },
                false
            ),
            Some(Event::Press(Selection::Slot {
                date: date(2026, 11, 6),
                minute: 630
            }))
        );
        assert_eq!(
            pressed(
                &week(),
                &placed,
                Hit::Slot {
                    day: 0,
                    minutes: 1440.0
                },
                false
            ),
            Some(Event::Press(Selection::Slot {
                date: date(2026, 11, 2),
                minute: 1410
            }))
        );
        assert_eq!(pressed(&week(), &placed, Hit::Nothing, false), None);
    }

    #[test]
    fn a_double_press_on_a_block_or_a_slot_says_so() {
        let entries = entries();
        let placed = placed(&entries);
        let stand_up = index_of(&placed, 1, 1);
        let block = Hit::Block {
            index: stand_up,
            edge: None,
        };
        assert_eq!(
            pressed(&week(), &placed, block, true),
            Some(Event::DoublePress(Selection::Entry {
                entry: EntryId(1),
                start: at(3, 9, 0)
            }))
        );
        let slot = Hit::Slot {
            day: 4,
            minutes: 644.0,
        };
        assert_eq!(
            pressed(&week(), &placed, slot, true),
            Some(Event::DoublePress(Selection::Slot {
                date: date(2026, 11, 6),
                minute: 630
            }))
        );
    }

    #[test]
    fn a_locked_block_is_only_held() {
        let entries = entries();
        let mut placed = placed(&entries);
        let stand_up = index_of(&placed, 1, 1);
        placed[stand_up].locked = true;
        for edge in [None, Some(Edge::Top), Some(Edge::Bottom)] {
            assert_eq!(
                grab(
                    &placed,
                    Hit::Block {
                        index: stand_up,
                        edge
                    },
                    0
                ),
                Some(Grab::Hold { index: stand_up })
            );
        }
    }

    #[test]
    fn blocks_say_as_much_about_their_time_as_fits() {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Alone", at(3, 9, 0), at(3, 10, 0)),
            Entry::new(EntryId(2), "Shared", at(4, 9, 0), at(4, 10, 0)),
            Entry::new(EntryId(3), "Shared", at(4, 9, 30), at(4, 10, 30)),
            Entry::new(EntryId(4), "Deadline", at(5, 17, 0), at(5, 17, 0)),
            Entry::new(EntryId(5), "Overnight", at(6, 23, 0), at(7, 1, 0)),
        ]);
        let placed = placed(&entries);
        let label = |id: u64, day: usize, week: bool| {
            let block = placed[index_of(&placed, id, day)].block;
            block_label(&block, week, &TimeZone::UTC)
        };
        assert_eq!(label(1, 1, true), "09:00-10:00");
        assert_eq!(label(2, 2, true), "09:00", "beside another in a week");
        assert_eq!(label(2, 2, false), "09:00-10:00", "a day has the room");
        assert_eq!(label(4, 3, true), "17:00");
        assert_eq!(label(5, 4, true), "23:00-01:00");
        assert_eq!(label(5, 5, true), "until 01:00");
    }

    #[test]
    fn the_text_of_a_block_keeps_clear_of_its_stripe() {
        let rect = Rectangle::new(Point::new(100.0, 50.0), Size::new(90.0, 58.0));
        let area = Hours::<()>::text_area(rect);
        assert_eq!(area.x, 100.0 + STRIPE + BLOCK_PADDING);
        assert_eq!(area.width, 90.0 - STRIPE - BLOCK_PADDING - 1.0);
        assert_eq!((area.y, area.height), (52.0, 54.0));
        // A block one line tall gives the line all its height.
        let short = Hours::<()>::text_area(Rectangle::new(Point::ORIGIN, Size::new(90.0, 17.0)));
        assert_eq!((short.y, short.height), (0.5, LINE));
        let tiny = Hours::<()>::text_area(Rectangle::new(Point::ORIGIN, Size::new(4.0, 2.0)));
        assert_eq!(tiny.size(), Size::new(0.0, 2.0));
    }
}
