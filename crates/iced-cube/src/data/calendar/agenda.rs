//! The schedule: the entries of a period as a list, day by day.

use iced::widget::{Column, Space, column, container, responsive, row, text};
use iced::{Alignment, Element, Length, Padding, Theme};
use jiff::civil::Date;

use super::entry::{Emphasis, Occurrence, add_days};
use super::parts::{
    Handler, day_heading, dot, entry_icon, entry_id, labelled, look_of, marked, pressable, ring,
    selection_of, send, span_label, weekday_label,
};
use super::state::{Event, State};
use super::style;
use crate::primitives::scroll_area::scroll_area;
use crate::primitives::tappable::{self, tappable};
use crate::theme::{self, Tokens, space, text_size};

/// The width of the column of times beside the titles.
const TIME_WIDTH: f32 = 96.0;
/// The height of a row, tall enough to tap.
const ROW_HEIGHT: f32 = 36.0;

/// The width below which a day's date sits above its entries instead of
/// beside them.
pub const STACKED_WIDTH: f32 = 480.0;
/// The width of the column of dates beside the entries.
const DATE_WIDTH: f32 = 112.0;

/// Every day of the period on show that has entries: the date on the
/// left and that day's entries beside it, with a line between days.
pub(super) fn view<'a, Message: Clone + 'a>(
    state: &'a State,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let on_event = on_event.clone();
    responsive(move |size| {
        let stacked = size.width < STACKED_WIDTH;
        let (first, last) = state.range();
        let mut days = Column::new().width(Length::Fill);
        let mut date = first;
        let mut any = false;
        while date <= last {
            let occurrences = occurrences_on(state, date);
            if !occurrences.is_empty() {
                if any {
                    days = days.push(rule());
                }
                any = true;
                let entries = rows(state, &occurrences, &on_event);
                let day: Element<'a, Message> = if stacked {
                    column![heading(state, date), entries].into()
                } else {
                    row![date_beside(state, date), entries].into()
                };
                days = days.push(container(day).padding(Padding::from([space::XS, 0.0])));
            }
            let next = add_days(date, 1);
            if next == date {
                break;
            }
            date = next;
        }
        if !any {
            return container(note("No entries in this period"))
                .center(Length::Fill)
                .into();
        }
        scroll_area(container(days).padding(Padding::from([space::XS, space::SM])))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    })
    .into()
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

/// A day's number with its month and weekday after it, such as
/// "18 JUN, THU", level with the day's first entry.
fn date_beside<'a, Message: 'a>(state: &State, date: Date) -> Element<'a, Message> {
    let today = date == state.today();
    let day = style::Day {
        today,
        selected: false,
        in_month: true,
    };
    let label = format!(
        "{}, {}",
        date.strftime("%b").to_string().to_uppercase(),
        weekday_label(date, false)
    );
    let label =
        text(label)
            .size(11.0)
            .wrapping(text::Wrapping::None)
            .style(move |theme: &Theme| {
                let tokens = Tokens::of(theme);
                text::Style {
                    color: Some(if today {
                        tokens.foreground
                    } else {
                        tokens.muted_foreground
                    }),
                }
            });
    row![day_heading(date, day), label]
        .spacing(space::SM)
        .align_y(Alignment::Center)
        .width(Length::Fixed(DATE_WIDTH))
        .height(Length::Fixed(ROW_HEIGHT))
        .into()
}

/// The entries of one day under its heading, for the list below a narrow
/// month.
pub(super) fn day<'a, Message: Clone + 'a>(
    state: &'a State,
    date: Date,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let occurrences = occurrences_on(state, date);
    let body: Element<'a, Message> = if occurrences.is_empty() {
        container(note("No entries"))
            .padding(Padding::from([space::SM, space::SM]))
            .into()
    } else {
        rows(state, &occurrences, on_event).into()
    };
    scroll_area(column![heading(state, date), body])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// The occurrences that touch `date`, whole days first.
fn occurrences_on(state: &State, date: Date) -> Vec<Occurrence> {
    let mut occurrences = state.shown().on(date);
    occurrences.sort_by_key(|occurrence| (!occurrence.all_day, occurrence.start, occurrence.entry));
    occurrences
}

fn note<'a, Message: 'a>(content: &'a str) -> Element<'a, Message> {
    text(content)
        .size(text_size::SM)
        .style(|theme: &Theme| text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        })
        .into()
}

/// A day's name and date, with today marked.
fn heading<'a, Message: 'a>(state: &State, date: Date) -> Element<'a, Message> {
    let today = date == state.today();
    let mut label = date.strftime("%A %-d %B").to_string();
    if today {
        label.push_str(" (today)");
    }
    container(
        text(label)
            .size(text_size::XS)
            .font(theme::semibold())
            .wrapping(text::Wrapping::None)
            .style(move |theme: &Theme| {
                let tokens = Tokens::of(theme);
                text::Style {
                    color: Some(if today {
                        tokens.foreground
                    } else {
                        tokens.muted_foreground
                    }),
                }
            }),
    )
    .padding(Padding::from([space::XS, space::SM]))
    .into()
}

fn rows<'a, Message: Clone + 'a>(
    state: &State,
    occurrences: &[Occurrence],
    on_event: &Handler<'a, Message>,
) -> Column<'a, Message> {
    let mut rows = Column::new().width(Length::Fill);
    for occurrence in occurrences {
        let Some(entry) = state.shown().get(occurrence.entry) else {
            continue;
        };
        let selection = selection_of(occurrence);
        let selected = state.is_selected(selection);
        // The list keeps its own row look; the emphasis shows in the
        // marker and the title.
        let look = style::Look {
            selected: false,
            ..look_of(state, entry, false)
        };
        let tone = entry.tone_of();
        let time = text(span_label(occurrence, state.time_zone()))
            .size(text_size::XS)
            .wrapping(text::Wrapping::None)
            .style(|theme: &Theme| text::Style {
                color: Some(Tokens::of(theme).muted_foreground),
            });
        let marker = if look.emphasis == Emphasis::Outlined || look.preview {
            ring(tone, 8.0)
        } else {
            dot(tone, 8.0)
        };
        let title = labelled(
            entry_icon(entry, look, tone, text_size::SM),
            entry.title().to_owned(),
            text_size::SM,
            20.0,
            move |theme| {
                let tokens = Tokens::of(theme);
                if look.preview || matches!(look.emphasis, Emphasis::Struck | Emphasis::Faded) {
                    tokens.muted_foreground
                } else {
                    tokens.foreground
                }
            },
        );
        let title = container(marked(title, tone, look)).height(Length::Fixed(20.0));
        let content = row![
            marker,
            container(time).width(Length::Fixed(TIME_WIDTH)),
            title,
        ]
        .spacing(space::SM)
        .align_y(Alignment::Center)
        .height(Length::Fixed(ROW_HEIGHT));

        let mut row = tappable(content)
            .width(Length::Fill)
            .padding(Padding::from([0.0, space::SM]))
            .style(move |theme, status| {
                let hovered = matches!(
                    status,
                    tappable::Status::Hovered | tappable::Status::Pressed
                );
                style::row(&Tokens::of(theme), selected, hovered)
            });
        if let Some(message) = send(on_event, Event::Press(selection)) {
            row = row.on_tap(message);
        }
        let row = container(pressable(state, row, selection, on_event))
            .id(entry_id(occurrence.entry, occurrence.start))
            .width(Length::Fill);
        #[cfg(feature = "icedwright")]
        let row = super::parts::annotate_entry(row, entry.title(), selected, on_event.is_some());
        rows = rows.push(row);
    }
    rows
}
