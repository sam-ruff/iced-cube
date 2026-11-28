//! Double clicks are counted on the clock icedwright installs, so a test
//! that moves that clock decides whether two presses make a double click.
#![cfg(all(feature = "icedwright", feature = "virtual-list"))]

use std::cell::Cell;
use std::time::{Duration, Instant, SystemTime};

use iced::widget::text;
use iced::{Element, Event, Point, mouse};
use iced_cube::data::virtual_list::{self, State, virtual_list};

#[path = "support/semantic.rs"]
mod semantic;

thread_local! {
    static NOW: Cell<Option<Instant>> = const { Cell::new(None) };
}

fn virtual_now() -> Instant {
    NOW.with(|now| match now.get() {
        Some(at) => at,
        None => {
            let start = Instant::now();
            now.set(Some(start));
            start
        }
    })
}

fn system_now() -> SystemTime {
    SystemTime::UNIX_EPOCH
}

fn advance(by: Duration) {
    let next = virtual_now() + by;
    NOW.with(|now| now.set(Some(next)));
}

fn view(state: &State<u32>) -> Element<'_, virtual_list::Event<u32>> {
    virtual_list(state, |_, n: &u32| text(format!("Row {n}")).into())
        .height(300)
        .on_event(|event| event)
        .into()
}

/// Presses the left button on the first row twice, `gap` apart on the
/// virtual clock, and returns whether the second press activated the row.
fn activated_after(gap: Duration) -> bool {
    icedwright_probe::time::install(virtual_now, system_now);
    let state = State::new(0..10);
    let mut recorder = semantic::Recorder::default();
    let at = Point::new(20.0, 10.0);
    let cursor = mouse::Cursor::Available(at);
    let press = [
        Event::Mouse(mouse::Event::CursorMoved { position: at }),
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ];
    let mut messages = recorder.events(view(&state), cursor, &press);
    advance(gap);
    messages.extend(recorder.events(view(&state), cursor, &press));
    messages
        .iter()
        .any(|event| matches!(event, virtual_list::Event::Activate(0)))
}

#[test]
fn presses_inside_the_interval_on_the_virtual_clock_double_click() {
    assert!(activated_after(Duration::from_millis(300)));
}

#[test]
fn presses_further_apart_on_the_virtual_clock_do_not() {
    assert!(!activated_after(Duration::from_millis(301)));
}
