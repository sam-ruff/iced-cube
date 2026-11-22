//! The year view: twelve small months.

use std::collections::BTreeSet;

use iced::widget::{self, Column, Row, Space, column, container, responsive, text};
use iced::{Alignment, Element, Length, Theme};
use jiff::civil::Date;

use super::entry::add_days;
use super::layout::month_weeks;
use super::parts::{Handler, dot, send, weekday_label};
use super::state::{Event, Selection, State};
use super::style;
use crate::primitives::scroll_area::scroll_area;
use crate::theme::{self, Tokens, space, text_size};

/// The side of a day in a small month.
pub const CELL: f32 = 32.0;
/// The space between small months.
pub const GAP: f32 = space::LG;
/// The most small months in a row.
pub const MAX_COLUMNS: usize = 4;

/// The width of a small month.
const MONTH_WIDTH: f32 = 7.0 * CELL;
/// The side of the dot under a day with entries.
const DOT: f32 = 3.0;

/// How many small months fit side by side in `width`, from one to
/// [`MAX_COLUMNS`], choosing a number that divides twelve so every row is
/// full.
pub fn columns_that_fit(width: f32) -> usize {
    let fit = ((width + GAP) / (MONTH_WIDTH + GAP)) as usize;
    [4, 3, 2, 1]
        .into_iter()
        .find(|columns| *columns <= fit.min(MAX_COLUMNS))
        .unwrap_or(1)
}

pub(super) fn view<'a, Message: Clone + 'a>(
    state: &'a State,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let on_event = on_event.clone();
    responsive(move |size| {
        let columns = columns_that_fit(size.width - 2.0 * space::MD);
        let (january, _) = state.range();
        let months: Vec<Date> = (1..=12)
            .filter_map(|month| january.with().month(month).build().ok())
            .collect();

        let mut rows = Column::new().spacing(GAP).align_x(Alignment::Center);
        for chunk in months.chunks(columns) {
            rows = rows.push(
                Row::with_children(chunk.iter().map(|first| month(state, *first, &on_event)))
                    .spacing(GAP),
            );
        }
        scroll_area(container(rows).center_x(Length::Fill).padding(space::MD))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    })
    .into()
}

/// The dates in the month of `first` that have an entry.
fn busy(state: &State, first: Date) -> BTreeSet<Date> {
    let last = first.last_of_month();
    let time_zone = state.time_zone();
    let mut days = BTreeSet::new();
    for occurrence in state.entries().between(first, last) {
        let (from, to) = occurrence.dates(time_zone);
        let mut date = from.max(first);
        while date <= to.min(last) {
            let _ = days.insert(date);
            let next = add_days(date, 1);
            if next == date {
                break;
            }
            date = next;
        }
    }
    days
}

/// One small month: its name, the weekdays and its days.
fn month<'a, Message: Clone + 'a>(
    state: &State,
    first: Date,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let muted = |theme: &Theme| text::Style {
        color: Some(Tokens::of(theme).muted_foreground),
    };
    let name = text(first.strftime("%B").to_string())
        .size(text_size::SM)
        .font(theme::semibold())
        .wrapping(text::Wrapping::None);
    let weeks = month_weeks(first, state.week_start());

    let mut grid = column![container(name).padding([0.0, 6.0])].spacing(2.0);
    if let Some(week) = weeks.first() {
        grid = grid.push(Row::with_children(week.iter().map(|date| {
            container(text(weekday_label(*date, true)).size(10.0).style(muted))
                .center(Length::Fixed(CELL))
                .into()
        })));
    }
    let busy = busy(state, first);
    for week in &weeks {
        grid = grid.push(Row::with_children(week.iter().map(|date| {
            if date.month() != first.month() {
                return Space::new()
                    .width(Length::Fixed(CELL))
                    .height(Length::Fixed(CELL))
                    .into();
            }
            day(state, *date, busy.contains(date), on_event)
        })));
    }
    container(grid).width(Length::Fixed(MONTH_WIDTH)).into()
}

/// One day: its number, with a dot under it when it has entries.
fn day<'a, Message: Clone + 'a>(
    state: &State,
    date: Date,
    busy: bool,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let kind = style::Day {
        today: date == state.today(),
        selected: state.is_selected(Selection::Day(date)),
        in_month: true,
    };
    let number = text(date.day().to_string())
        .size(11.0)
        .line_height(text::LineHeight::Absolute(14.0.into()))
        .wrapping(text::Wrapping::None);
    let mark: Element<'a, Message> = if busy && !kind.today {
        dot(super::entry::Tone::Primary, DOT)
    } else {
        Space::new().height(Length::Fixed(DOT)).into()
    };
    widget::button(
        column![number, mark]
            .align_x(Alignment::Center)
            .width(Length::Fill),
    )
    .width(Length::Fixed(CELL))
    .height(Length::Fixed(CELL))
    .padding([7.0, 0.0])
    .on_press_maybe(send(on_event, Event::Press(Selection::Day(date))))
    .style(move |theme, status| style::year_day(&Tokens::of(theme), kind, status))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::calendar::entry::{Entries, Entry, EntryId};
    use jiff::civil::date;

    #[test]
    fn months_sit_in_rows_that_divide_twelve() {
        let width = |columns: usize| columns as f32 * MONTH_WIDTH + (columns as f32 - 1.0) * GAP;
        assert_eq!(columns_that_fit(100.0), 1);
        assert_eq!(columns_that_fit(width(1)), 1);
        assert_eq!(columns_that_fit(width(2) - 1.0), 1);
        assert_eq!(columns_that_fit(width(2)), 2);
        assert_eq!(columns_that_fit(width(3)), 3);
        assert_eq!(columns_that_fit(width(4)), 4);
        assert_eq!(
            columns_that_fit(width(5)),
            4,
            "five would leave a short row"
        );
        assert_eq!(columns_that_fit(5000.0), MAX_COLUMNS);
    }

    #[test]
    fn busy_days_are_the_ones_entries_touch_within_the_month() {
        let state = State::new(date(2026, 6, 18)).with_entries(Entries::utc().with([
            Entry::all_day(EntryId(1), "Trip", date(2026, 5, 30), date(2026, 6, 2)),
            Entry::all_day(EntryId(2), "Day", date(2026, 6, 18), date(2026, 6, 18)),
        ]));
        let days: Vec<_> = busy(&state, date(2026, 6, 1)).into_iter().collect();
        assert_eq!(
            days,
            vec![date(2026, 6, 1), date(2026, 6, 2), date(2026, 6, 18)]
        );
        assert!(busy(&state, date(2026, 7, 1)).is_empty());
    }
}
