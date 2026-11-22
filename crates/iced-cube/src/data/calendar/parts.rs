//! Pieces the views share: entry chips, lanes of bars, day numbers and
//! grid lines.

use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Renderer as _, renderer};
use iced::widget::button::Status;
use iced::widget::{self, Column, Row, Space, column, container, text};
use iced::{
    Alignment, Background, Border, Element, Length, Padding, Rectangle, Renderer, Size, Theme,
    mouse,
};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;

use super::entry::{Occurrence, When};
use super::layout::{Bar, Fitted};
use super::state::{Event, Selection, State};
use super::style;
use crate::data::ellipsis::ellipsis;
use crate::theme::{self, Tokens, radius, text_size};

pub(super) type Handler<'a, Message> = Option<Rc<dyn Fn(Event) -> Message + 'a>>;

/// The message for `event`, or `None` for a calendar without a handler,
/// which renders disabled.
pub(super) fn send<Message>(handler: &Handler<'_, Message>, event: Event) -> Option<Message> {
    handler.as_ref().map(|handler| handler(event))
}

/// The height of an entry's bar.
pub const CHIP_HEIGHT: f32 = 20.0;
/// The space between lanes of bars.
pub const LANE_GAP: f32 = 2.0;
/// The side of the pill around a day's number.
pub const NUMBER_SIZE: f32 = 22.0;
/// The side of the circle around the day heading a column of hours.
pub const HEADING_SIZE: f32 = 34.0;

/// A time of day, such as "09:30".
pub fn time_label(instant: Timestamp, time_zone: &TimeZone) -> String {
    instant
        .to_zoned_datetime(time_zone.clone())
        .strftime("%H:%M")
        .to_string()
}

/// When an occurrence happens within its day: "All day", or its start and
/// end times.
pub fn span_label(occurrence: &Occurrence, time_zone: &TimeZone) -> String {
    if occurrence.all_day {
        return String::from("All day");
    }
    if occurrence.end <= occurrence.start {
        return time_label(occurrence.start, time_zone);
    }
    format!(
        "{} - {}",
        time_label(occurrence.start, time_zone),
        time_label(occurrence.end, time_zone)
    )
}

/// When a span is, in words, for a dialog or a list: "Thu 19 Nov, 10:00 -
/// 11:30" for a time on one day, and the first and last day otherwise.
pub fn when_label(when: &When, time_zone: &TimeZone) -> String {
    const DAY: &str = "%a %-d %b";
    match *when {
        When::AllDay { first, last } if first == last => first.strftime(DAY).to_string(),
        When::AllDay { first, last } => {
            format!("{} - {}", first.strftime(DAY), last.strftime(DAY))
        }
        When::Timed { start, end } => {
            let from = start.to_zoned_datetime(time_zone.clone());
            let to = end.to_zoned_datetime(time_zone.clone());
            let day = from.strftime(DAY);
            let (first, last) = (from.strftime("%H:%M"), to.strftime("%H:%M"));
            if end <= start {
                return format!("{day}, {first}");
            }
            if from.date() == to.date() {
                return format!("{day}, {first} - {last}");
            }
            format!("{day} {first} - {} {last}", to.strftime(DAY))
        }
    }
}

/// What a bar says: the title, after the start time for a timed entry
/// that begins on the bar's first day.
pub fn bar_label(title: &str, bar: &Bar, time_zone: &TimeZone) -> String {
    if bar.occurrence.all_day || bar.starts_before {
        return title.to_owned();
    }
    format!("{} {title}", time_label(bar.occurrence.start, time_zone))
}

pub(super) fn selection_of(occurrence: &Occurrence) -> Selection {
    Selection::Entry {
        entry: occurrence.entry,
        start: occurrence.start,
    }
}

/// A day's short name in capitals, such as "MON", or its first letter
/// when `narrow`.
pub fn weekday_label(date: Date, narrow: bool) -> String {
    let name = date.strftime("%a").to_string().to_uppercase();
    if narrow {
        return name.chars().take(1).collect();
    }
    name
}

/// A day's number in a pill, filled for today.
pub(super) fn day_number<'a, Message: 'a>(date: Date, day: style::Day) -> Element<'a, Message> {
    number_pill(date, day, NUMBER_SIZE, text_size::XS)
}

/// A day's number as the heading of its column: larger, in a circle.
pub(super) fn day_heading<'a, Message: 'a>(date: Date, day: style::Day) -> Element<'a, Message> {
    number_pill(date, day, HEADING_SIZE, text_size::LG)
}

fn number_pill<'a, Message: 'a>(
    date: Date,
    day: style::Day,
    side: f32,
    size: f32,
) -> Element<'a, Message> {
    let font = if day.today {
        theme::semibold()
    } else {
        theme::font()
    };
    let number = text(date.day().to_string())
        .size(size)
        .font(font)
        .wrapping(text::Wrapping::None)
        .style(move |theme: &Theme| text::Style {
            color: Some(style::day_number(&Tokens::of(theme), day).foreground),
        });
    container(number)
        .center(Length::Fixed(side))
        .style(move |theme: &Theme| container::Style {
            background: style::day_number(&Tokens::of(theme), day)
                .background
                .map(Background::Color),
            border: Border {
                radius: radius::FULL.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// An entry as a bar: its label on a wash of its tone.
pub(super) fn chip<'a, Message: Clone + 'a>(
    state: &State,
    bar: &Bar,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let Some(entry) = state.entries().get(bar.occurrence.entry) else {
        return Space::new().into();
    };
    let tone = entry.tone_of();
    let selection = selection_of(&bar.occurrence);
    let selected = state.is_selected(selection);
    let label = ellipsis(bar_label(entry.title(), bar, state.time_zone()))
        .size(text_size::XS, CHIP_HEIGHT)
        .colour(move |theme| {
            style::chip(&Tokens::of(theme), tone, selected, Status::Active).foreground
        });
    widget::button(label)
        .width(Length::Fill)
        .height(Length::Fixed(CHIP_HEIGHT))
        .padding(Padding::from([0.0, 6.0]))
        .on_press_maybe(send(on_event, Event::Press(selection)))
        .style(move |theme, status| style::chip_button(&Tokens::of(theme), tone, selected, status))
        .into()
}

/// Whether a bar is a timed entry that starts and ends within one day,
/// which the month shows as a dot, a time and a title with no fill.
pub fn is_line(bar: &Bar) -> bool {
    !bar.occurrence.all_day && bar.days() == 1 && !bar.starts_before && !bar.ends_after
}

/// A timed entry within one day: a dot in its tone, its start and its
/// title, filled only while selected.
pub(super) fn line<'a, Message: Clone + 'a>(
    state: &State,
    bar: &Bar,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let Some(entry) = state.entries().get(bar.occurrence.entry) else {
        return Space::new().into();
    };
    let tone = entry.tone_of();
    let selection = selection_of(&bar.occurrence);
    let selected = state.is_selected(selection);
    let label = ellipsis(bar_label(entry.title(), bar, state.time_zone()))
        .size(text_size::XS, CHIP_HEIGHT)
        .colour(move |theme| {
            style::entry_line(&Tokens::of(theme), tone, selected, Status::Active).text_color
        });
    let mut content = Row::new().spacing(4.0).align_y(Alignment::Center);
    if !selected {
        content = content.push(dot(tone, 6.0));
    }
    widget::button(content.push(label))
        .width(Length::Fill)
        .height(Length::Fixed(CHIP_HEIGHT))
        .padding(Padding::from([0.0, 4.0]))
        .on_press_maybe(send(on_event, Event::Press(selection)))
        .style(move |theme, status| style::entry_line(&Tokens::of(theme), tone, selected, status))
        .into()
}

/// The count of entries a crowded day could not show. Pressing it opens
/// the day.
fn more<'a, Message: Clone + 'a>(
    date: Date,
    hidden: usize,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let label = text(format!("+{hidden} more"))
        .size(text_size::XS)
        .wrapping(text::Wrapping::None);
    widget::button(label)
        .width(Length::Fill)
        .height(Length::Fixed(CHIP_HEIGHT))
        .padding(Padding::from([0.0, 6.0]))
        .clip(true)
        .on_press_maybe(send(on_event, Event::OpenDay(date)))
        .style(|theme, status| {
            let tokens = Tokens::of(theme);
            let hovered = matches!(status, Status::Hovered | Status::Pressed);
            widget::button::Style {
                background: hovered.then_some(Background::Color(tokens.accent)),
                text_color: tokens.muted_foreground,
                border: Border {
                    radius: radius::SM.into(),
                    ..Border::default()
                },
                ..widget::button::Style::default()
            }
        })
        .into()
}

/// The bars of `fitted` as rows, one per lane, over `days`. The last of
/// `lanes` also holds the counts of what crowded days left out.
pub(super) fn lanes<'a, Message: Clone + 'a>(
    state: &State,
    days: &[Date],
    fitted: &Fitted,
    lanes: usize,
    on_event: &Handler<'a, Message>,
) -> Column<'a, Message> {
    let mut rows = column![].spacing(LANE_GAP);
    let used = fitted
        .bars
        .iter()
        .map(|bar| bar.lane + 1)
        .max()
        .unwrap_or(0);
    let counted = fitted.hidden.iter().any(|hidden| *hidden > 0);
    let shown = if counted { lanes } else { used.min(lanes) };

    for lane in 0..shown {
        let mut row = Row::new().height(Length::Fixed(CHIP_HEIGHT));
        let mut day = 0;
        while day < days.len() {
            let bar = fitted
                .bars
                .iter()
                .find(|bar| bar.lane == lane && bar.first == day);
            let hidden = fitted.hidden.get(day).copied().unwrap_or(0);
            let (cell, span): (Element<'a, Message>, usize) = match bar {
                Some(bar) if is_line(bar) => (line(state, bar, on_event), 1),
                Some(bar) => (chip(state, bar, on_event), bar.days()),
                None if lane + 1 == lanes && hidden > 0 => (more(days[day], hidden, on_event), 1),
                None => (Space::new().into(), 1),
            };
            let portion = u16::try_from(span).unwrap_or(1);
            row = row.push(
                container(cell)
                    .padding(Padding::from([0.0, 2.0]))
                    .width(Length::FillPortion(portion)),
            );
            day += span;
        }
        rows = rows.push(row);
    }
    rows
}

/// Up to three dots in the tones of a day's entries, for a month too
/// narrow for bars.
pub(super) fn dots<'a, Message: 'a>(state: &State, occurrences: &[Occurrence]) -> Row<'a, Message> {
    let mut row = Row::new().spacing(3.0).align_y(Alignment::Center);
    for occurrence in occurrences.iter().take(3) {
        let Some(entry) = state.entries().get(occurrence.entry) else {
            continue;
        };
        let tone = entry.tone_of();
        row = row.push(dot(tone, 5.0));
    }
    row
}

/// A filled circle in a tone.
pub(super) fn dot<'a, Message: 'a>(tone: super::entry::Tone, size: f32) -> Element<'a, Message> {
    container(Space::new())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Color(style::tone(&Tokens::of(theme), tone))),
            border: Border {
                radius: radius::FULL.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// The lines of a grid: `columns` by `rows` equal cells, drawn between
/// them and not around the outside.
pub(super) fn rules<'a, Message: 'a>(columns: usize, rows: usize) -> Element<'a, Message> {
    Element::new(Rules { columns, rows })
}

struct Rules {
    columns: usize,
    rows: usize,
}

impl<Message> Widget<Message, Theme, Renderer> for Rules {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let colour = style::line(&Tokens::of(theme));
        let mut line = |bounds: Rectangle| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    ..renderer::Quad::default()
                },
                colour,
            );
        };
        for column in 1..self.columns {
            let x = bounds.x + bounds.width * column as f32 / self.columns as f32;
            line(Rectangle::new(
                iced::Point::new(x.round(), bounds.y),
                Size::new(1.0, bounds.height),
            ));
        }
        for row in 1..self.rows {
            let y = bounds.y + bounds.height * row as f32 / self.rows as f32;
            line(Rectangle::new(
                iced::Point::new(bounds.x, y.round()),
                Size::new(bounds.width, 1.0),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::calendar::entry::{Entries, Entry, EntryId};
    use crate::data::calendar::layout;
    use jiff::civil::date;

    fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
        date(2026, 11, day)
            .at(hour, minute, 0, 0)
            .to_zoned_datetime(TimeZone::UTC)
            .map(|zoned| zoned.timestamp())
            .unwrap_or_default()
    }

    #[test]
    fn times_are_shown_on_the_24_hour_clock_in_the_calendar_time_zone() {
        assert_eq!(time_label(at(3, 9, 5), &TimeZone::UTC), "09:05");
        assert_eq!(time_label(at(3, 17, 30), &TimeZone::UTC), "17:30");
        let Ok(tokyo) = TimeZone::get("Asia/Tokyo") else {
            panic!("the time zone database is bundled for tests");
        };
        assert_eq!(time_label(at(3, 9, 5), &tokyo), "18:05");
    }

    #[test]
    fn spans_say_all_day_a_range_or_a_single_time() {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Meeting", at(3, 9, 0), at(3, 10, 30)),
            Entry::new(EntryId(2), "Deadline", at(3, 17, 0), at(3, 17, 0)),
            Entry::all_day(EntryId(3), "Trip", date(2026, 11, 3), date(2026, 11, 3)),
        ]);
        let labels: Vec<_> = entries
            .on(date(2026, 11, 3))
            .iter()
            .map(|occurrence| span_label(occurrence, entries.time_zone()))
            .collect();
        assert_eq!(labels, vec!["All day", "09:00 - 10:30", "17:00"]);
    }

    #[test]
    fn weekdays_are_capitals_or_a_single_letter() {
        let thursday = date(2026, 11, 19);
        assert_eq!(weekday_label(thursday, false), "THU");
        assert_eq!(weekday_label(thursday, true), "T");
        assert_eq!(weekday_label(date(2026, 11, 22), false), "SUN");
    }

    #[test]
    fn only_timed_entries_within_a_day_are_lines() {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Meeting", at(3, 9, 0), at(3, 10, 0)),
            Entry::new(EntryId(2), "Overnight", at(4, 22, 0), at(5, 2, 0)),
            Entry::all_day(EntryId(3), "Trip", date(2026, 11, 6), date(2026, 11, 6)),
            Entry::new(EntryId(4), "Into next week", at(8, 22, 0), at(9, 2, 0)),
        ]);
        let week = layout::week_of(date(2026, 11, 3), jiff::civil::Weekday::Monday);
        let bars = layout::bars(
            &week,
            &entries.between(week[0], week[6]),
            entries.time_zone(),
        );
        let lines: Vec<_> = bars
            .iter()
            .filter(|bar| is_line(bar))
            .map(|bar| bar.occurrence.entry.0)
            .collect();
        assert_eq!(lines, vec![1]);
    }

    #[test]
    fn bars_lead_with_the_time_only_where_a_timed_entry_starts() {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Flight", at(8, 22, 0), at(9, 6, 0)),
            Entry::all_day(EntryId(2), "Trip", date(2026, 11, 3), date(2026, 11, 3)),
        ]);
        let label = |week_of: Date, id: u64| {
            let week = layout::week_of(week_of, jiff::civil::Weekday::Monday);
            let bars = layout::bars(
                &week,
                &entries.between(week[0], week[6]),
                entries.time_zone(),
            );
            let bar = bars.iter().find(|bar| bar.occurrence.entry == EntryId(id));
            let title = entries.get(EntryId(id)).map(Entry::title);
            match (bar, title) {
                (Some(bar), Some(title)) => bar_label(title, bar, entries.time_zone()),
                _ => String::new(),
            }
        };
        assert_eq!(label(date(2026, 11, 8), 1), "22:00 Flight");
        assert_eq!(label(date(2026, 11, 9), 1), "Flight", "carried over");
        assert_eq!(label(date(2026, 11, 3), 2), "Trip");
    }

    #[test]
    fn spans_are_described_by_their_days_and_times() {
        let utc = &TimeZone::UTC;
        let day = date(2026, 11, 19);
        assert_eq!(when_label(&When::day(day), utc), "Thu 19 Nov");
        assert_eq!(
            when_label(&When::all_day(day, date(2026, 11, 21)), utc),
            "Thu 19 Nov - Sat 21 Nov"
        );
        assert_eq!(
            when_label(&When::timed(at(19, 10, 0), at(19, 11, 30)), utc),
            "Thu 19 Nov, 10:00 - 11:30"
        );
        assert_eq!(
            when_label(&When::timed(at(19, 17, 0), at(19, 17, 0)), utc),
            "Thu 19 Nov, 17:00"
        );
        assert_eq!(
            when_label(&When::timed(at(19, 22, 0), at(20, 2, 0)), utc),
            "Thu 19 Nov 22:00 - Fri 20 Nov 02:00"
        );
    }

    #[test]
    fn without_a_handler_nothing_is_sent() {
        let none: Handler<'_, Event> = None;
        assert_eq!(send(&none, Event::Next), None);
        let some: Handler<'_, Event> = Some(Rc::new(|event| event));
        assert_eq!(send(&some, Event::Next), Some(Event::Next));
    }
}
