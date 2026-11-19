//! The month view: a grid of weeks with entries as bars across the days.

use iced::widget::{self, Column, Row, Space, column, container, responsive, stack, text};
use iced::{Alignment, Element, Length, Padding, Size, Theme};
use jiff::civil::Date;

use super::agenda;
use super::layout::{self, month_weeks};
use super::parts::{
    CHIP_HEIGHT, Handler, LANE_GAP, NUMBER_SIZE, day_number, dots, lanes, rules, send,
    weekday_label,
};
use super::state::{Event, Selection, State};
use super::style;
use crate::theme::{Tokens, space};

/// The width below which days show dots for their entries, with the
/// selected day's entries listed underneath.
pub const COMPACT_WIDTH: f32 = 480.0;

/// The height of the row naming the weekdays.
const WEEKDAYS_HEIGHT: f32 = 28.0;
/// The space around a day's number.
const CELL_PADDING: f32 = 3.0;
/// The height of a week in the compact layout.
const COMPACT_WEEK_HEIGHT: f32 = 44.0;

/// How many lanes of bars fit in a week row of `height`.
pub fn lanes_that_fit(height: f32) -> usize {
    let room = height - NUMBER_SIZE - 2.0 * CELL_PADDING;
    if room < CHIP_HEIGHT {
        return 0;
    }
    ((room + LANE_GAP) / (CHIP_HEIGHT + LANE_GAP)) as usize
}

pub(super) fn view<'a, Message: Clone + 'a>(
    state: &'a State,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let on_event = on_event.clone();
    responsive(move |size| {
        if size.width < COMPACT_WIDTH {
            compact(state, &on_event)
        } else {
            wide(state, size, &on_event)
        }
    })
    .into()
}

/// The names of the weekdays above the grid, shortened to a letter when
/// `narrow`.
fn weekday_names<'a, Message: 'a>(state: &State, narrow: bool) -> Row<'a, Message> {
    let mut names = Row::new().height(Length::Fixed(WEEKDAYS_HEIGHT));
    let week = layout::week_of(state.cursor(), state.week_start());
    for date in week {
        names = names.push(
            container(
                text(weekday_label(date, narrow))
                    .size(11.0)
                    .wrapping(text::Wrapping::None)
                    .style(|theme: &Theme| text::Style {
                        color: Some(Tokens::of(theme).muted_foreground),
                    }),
            )
            .center_x(Length::FillPortion(1))
            .center_y(Length::Fill),
        );
    }
    names
}

fn day_of(state: &State, date: Date) -> style::Day {
    style::Day {
        today: date == state.today(),
        selected: state.is_selected(Selection::Day(date)),
        in_month: (date.year(), date.month()) == (state.cursor().year(), state.cursor().month()),
    }
}

/// One day of the grid: a button the size of the cell, with the number in
/// a corner and anything in `below` under it.
fn cell<'a, Message: Clone + 'a>(
    state: &State,
    date: Date,
    below: Option<Element<'a, Message>>,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let day = day_of(state, date);
    let mut content = column![day_number(date, day)]
        .spacing(2.0)
        .align_x(Alignment::Center)
        .width(Length::Fill);
    if let Some(below) = below {
        content = content.push(below);
    }
    widget::button(content)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .padding(CELL_PADDING)
        .on_press_maybe(send(on_event, Event::Press(Selection::Day(date))))
        .style(move |theme, status| style::cell(&Tokens::of(theme), day, status))
        .into()
}

/// Weeks as rows of cells with their bars laid over them.
fn wide<'a, Message: Clone + 'a>(
    state: &State,
    size: Size,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let weeks = month_weeks(state.cursor(), state.week_start());
    let week_height = (size.height - WEEKDAYS_HEIGHT) / weeks.len().max(1) as f32;
    let room = lanes_that_fit(week_height);

    let mut rows = Column::new();
    for week in &weeks {
        let occurrences = state.entries().between(week[0], week[6]);
        let bars = layout::bars(week, &occurrences, state.time_zone());
        let fitted = layout::fit(bars, week.len(), room);

        let cells = Row::with_children(week.iter().map(|date| {
            let below = (room == 0).then(|| {
                let day: Vec<_> = occurrences
                    .iter()
                    .filter(|occurrence| {
                        let (first, last) = occurrence.dates(state.time_zone());
                        (first..=last).contains(date)
                    })
                    .copied()
                    .collect();
                dots(state, &day).into()
            });
            cell(state, *date, below, on_event)
        }));
        let bars = column![
            Space::new().height(Length::Fixed(NUMBER_SIZE + 2.0 * CELL_PADDING)),
            lanes(state, week, &fitted, room, on_event),
        ];
        rows = rows.push(
            stack![cells, bars]
                .width(Length::Fill)
                .height(Length::FillPortion(1)),
        );
    }

    column![
        weekday_names(state, false),
        stack![rows, rules(7, weeks.len())]
            .width(Length::Fill)
            .height(Length::Fill),
    ]
    .into()
}

/// Weeks of dots, with the entries of the selected day listed below.
fn compact<'a, Message: Clone + 'a>(
    state: &'a State,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let weeks = month_weeks(state.cursor(), state.week_start());
    let mut rows = Column::new();
    for week in &weeks {
        let cells = Row::with_children(week.iter().map(|date| {
            let day = state.entries().on(*date);
            cell(state, *date, Some(dots(state, &day).into()), on_event)
        }))
        .height(Length::Fixed(COMPACT_WEEK_HEIGHT));
        rows = rows.push(cells);
    }
    let grid_height = weeks.len() as f32 * COMPACT_WEEK_HEIGHT;
    let listed = state.selected_day().unwrap_or(state.cursor());

    column![
        weekday_names(state, true),
        stack![rows, rules(7, weeks.len())]
            .width(Length::Fill)
            .height(Length::Fixed(grid_height)),
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fixed(1.0))
            .style(|theme: &Theme| container::Style {
                background: Some(style::line(&Tokens::of(theme)).into()),
                ..container::Style::default()
            }),
        container(agenda::day(state, listed, on_event))
            .padding(Padding::from([space::SM, space::SM]))
            .height(Length::Fill),
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lanes_follow_the_height_of_a_week() {
        assert_eq!(lanes_that_fit(30.0), 0);
        assert_eq!(lanes_that_fit(47.0), 0);
        assert_eq!(lanes_that_fit(48.0), 1);
        assert_eq!(lanes_that_fit(69.0), 1);
        assert_eq!(lanes_that_fit(70.0), 2);
        assert_eq!(lanes_that_fit(120.0), 4);
    }
}
