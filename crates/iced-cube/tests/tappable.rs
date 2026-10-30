#![cfg(feature = "tappable")]

use std::cell::RefCell;
use std::rc::Rc;

use iced::time::{Duration, Instant};
use iced::widget::{self, column, row, scrollable, text};
use iced::{Element, Event, Length, Point, mouse, touch, window};
use iced_cube::primitives::tappable::{DEFAULT_HOLD, Status, style};
use iced_cube::tappable;
use iced_cube::theme::Tokens;
use iced_test::simulator::{Simulator, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Tap(usize),
    Hold(usize),
    Nested,
    Scrolled(f32),
}

fn rows() -> Element<'static, Message> {
    let rows = (0..30).map(|index| {
        tappable(text(format!("Row {index}")))
            .on_tap(Message::Tap(index))
            .on_long_press(Message::Hold(index))
            .width(Length::Fill)
            .padding(12)
            .into()
    });
    scrollable(column(rows))
        .height(300)
        .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
        .into()
}

fn finger(event: fn(touch::Finger, Point) -> touch::Event, position: Point) -> Event {
    Event::Touch(event(touch::Finger(1), position))
}

fn pressed(id: touch::Finger, position: Point) -> touch::Event {
    touch::Event::FingerPressed { id, position }
}

fn moved(id: touch::Finger, position: Point) -> touch::Event {
    touch::Event::FingerMoved { id, position }
}

fn lifted(id: touch::Finger, position: Point) -> touch::Event {
    touch::Event::FingerLifted { id, position }
}

/// Puts the cursor where the finger is, as the windowing shell does.
fn touch_at(
    ui: &mut Simulator<'_, Message>,
    event: fn(touch::Finger, Point) -> touch::Event,
    at: Point,
) {
    ui.point_at(at);
    let _ = ui.simulate([finger(event, at)]);
}

fn centre(ui: &mut Simulator<'_, Message>, label: &str) -> Point {
    match ui.find(label) {
        Ok(target) => target.bounds().center(),
        Err(error) => panic!("{label} is not rendered: {error:?}"),
    }
}

fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
    ui.into_messages().collect()
}

/// The messages without the scrollable's own, which it also sends on a
/// redraw.
fn gestures(ui: Simulator<'_, Message>) -> Vec<Message> {
    ui.into_messages()
        .filter(|message| !matches!(message, Message::Scrolled(_)))
        .collect()
}

fn later() -> Event {
    Event::Window(window::Event::RedrawRequested(
        Instant::now() + DEFAULT_HOLD + Duration::from_millis(50),
    ))
}

#[test]
fn a_tap_on_a_row_publishes_its_message() {
    let mut ui = simulator(rows());
    let at = centre(&mut ui, "Row 2");
    touch_at(&mut ui, pressed, at);
    touch_at(&mut ui, lifted, at);
    assert_eq!(messages(ui), vec![Message::Tap(2)]);
}

#[test]
fn holding_a_row_publishes_one_long_press_and_no_tap() {
    let mut ui = simulator(rows());
    let at = centre(&mut ui, "Row 1");
    touch_at(&mut ui, pressed, at);
    let _ = ui.simulate([later(), later()]);
    touch_at(&mut ui, lifted, at);
    assert_eq!(gestures(ui), vec![Message::Hold(1)]);
}

#[test]
fn dragging_from_a_row_scrolls_the_list_and_taps_nothing() {
    let mut ui = simulator(rows());
    let start = centre(&mut ui, "Row 5");
    touch_at(&mut ui, pressed, start);
    for step in 1..=6 {
        touch_at(
            &mut ui,
            moved,
            Point::new(start.x, start.y - 20.0 * step as f32),
        );
    }
    touch_at(&mut ui, lifted, Point::new(start.x, start.y - 120.0));
    let _ = ui.simulate([later()]);
    let messages = messages(ui);
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Scrolled(y) if *y > 100.0)),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .all(|message| matches!(message, Message::Scrolled(_))),
        "{messages:?}"
    );
}

#[test]
fn a_small_wobble_is_still_a_tap() {
    let mut ui = simulator(rows());
    let at = centre(&mut ui, "Row 3");
    touch_at(&mut ui, pressed, at);
    touch_at(&mut ui, moved, Point::new(at.x + 3.0, at.y + 3.0));
    touch_at(&mut ui, lifted, Point::new(at.x + 3.0, at.y + 3.0));
    assert!(messages(ui).contains(&Message::Tap(3)));
}

#[test]
fn a_lost_finger_cancels() {
    let mut ui = simulator(rows());
    let at = centre(&mut ui, "Row 0");
    touch_at(&mut ui, pressed, at);
    let _ = ui.simulate([Event::Touch(touch::Event::FingerLost {
        id: touch::Finger(1),
        position: at,
    })]);
    touch_at(&mut ui, lifted, at);
    assert!(messages(ui).is_empty());
}

fn mouse_at(ui: &mut Simulator<'_, Message>, at: Point, events: &[mouse::Event]) {
    ui.point_at(at);
    let _ = ui.simulate(
        std::iter::once(Event::Mouse(mouse::Event::CursorMoved { position: at }))
            .chain(events.iter().copied().map(Event::Mouse)),
    );
}

#[test]
fn a_left_click_taps_and_a_right_click_long_presses() {
    let mut ui = simulator(rows());
    let at = centre(&mut ui, "Row 4");
    mouse_at(
        &mut ui,
        at,
        &[
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
            mouse::Event::ButtonPressed(mouse::Button::Right),
            mouse::Event::ButtonReleased(mouse::Button::Right),
        ],
    );
    assert_eq!(messages(ui), vec![Message::Tap(4), Message::Hold(4)]);
}

#[test]
fn holding_the_left_button_long_presses_and_dragging_cancels() {
    let mut ui = simulator(rows());
    let at = centre(&mut ui, "Row 6");
    mouse_at(
        &mut ui,
        at,
        &[mouse::Event::ButtonPressed(mouse::Button::Left)],
    );
    let _ = ui.simulate([later()]);
    mouse_at(
        &mut ui,
        at,
        &[mouse::Event::ButtonReleased(mouse::Button::Left)],
    );

    let other = centre(&mut ui, "Row 7");
    mouse_at(
        &mut ui,
        other,
        &[mouse::Event::ButtonPressed(mouse::Button::Left)],
    );
    let away = Point::new(other.x, other.y + 30.0);
    mouse_at(
        &mut ui,
        away,
        &[mouse::Event::ButtonReleased(mouse::Button::Left)],
    );
    assert_eq!(gestures(ui), vec![Message::Hold(6)]);
}

#[test]
fn a_nested_button_takes_its_own_tap_and_the_row_stays_quiet() {
    let view: Element<'_, Message> = tappable(
        row![
            text("Row"),
            widget::button(text("Star")).on_press(Message::Nested)
        ]
        .spacing(40),
    )
    .on_tap(Message::Tap(0))
    .padding(12)
    .into();
    let mut ui = simulator(view);
    let star = centre(&mut ui, "Star");
    touch_at(&mut ui, pressed, star);
    touch_at(&mut ui, lifted, star);
    let label = centre(&mut ui, "Row");
    touch_at(&mut ui, pressed, label);
    touch_at(&mut ui, lifted, label);
    assert_eq!(messages(ui), vec![Message::Nested, Message::Tap(0)]);
}

fn nested() -> Element<'static, Message> {
    tappable(
        row![
            text("Outer"),
            tappable(text("Inner"))
                .on_tap(Message::Tap(1))
                .on_long_press(Message::Hold(1))
                .padding(8),
        ]
        .spacing(40),
    )
    .on_tap(Message::Tap(0))
    .on_long_press(Message::Hold(0))
    .padding(12)
    .into()
}

#[test]
fn a_nested_tappable_takes_its_own_taps_and_long_presses() {
    let mut ui = simulator(nested());
    let inner = centre(&mut ui, "Inner");
    touch_at(&mut ui, pressed, inner);
    touch_at(&mut ui, lifted, inner);
    touch_at(&mut ui, pressed, inner);
    let _ = ui.simulate([later()]);
    touch_at(&mut ui, lifted, inner);
    mouse_at(
        &mut ui,
        inner,
        &[
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ],
    );
    let outer = centre(&mut ui, "Outer");
    touch_at(&mut ui, pressed, outer);
    touch_at(&mut ui, lifted, outer);
    assert_eq!(
        messages(ui),
        vec![
            Message::Tap(1),
            Message::Hold(1),
            Message::Tap(1),
            Message::Tap(0)
        ]
    );
}

#[test]
fn a_second_finger_cancels_the_tap() {
    let mut ui = simulator(rows());
    let at = centre(&mut ui, "Row 2");
    let second = Point::new(at.x + 40.0, at.y);
    ui.point_at(at);
    let _ = ui.simulate([
        finger(pressed, at),
        Event::Touch(pressed(touch::Finger(2), second)),
        Event::Touch(lifted(touch::Finger(2), second)),
        finger(lifted, at),
    ]);
    assert!(gestures(ui).is_empty());
}

/// A row that records every status it is drawn in.
fn recorded(statuses: Rc<RefCell<Vec<Status>>>) -> Element<'static, Message> {
    let row = tappable(text("Recorded"))
        .on_tap(Message::Tap(0))
        .on_long_press(Message::Hold(0))
        .width(Length::Fill)
        .padding(12)
        .style(move |theme, status| {
            statuses.borrow_mut().push(status);
            style(&Tokens::of(theme), status)
        });
    column![row, text("Below")].into()
}

fn drawn(ui: &mut Simulator<'_, Message>, statuses: &RefCell<Vec<Status>>) -> Option<Status> {
    statuses.borrow_mut().clear();
    if let Err(error) = ui.snapshot(&iced_cube::theme::light()) {
        panic!("snapshot failed: {error:?}");
    }
    statuses.borrow().last().copied()
}

#[test]
fn a_touch_leaves_no_pressed_or_hovered_look_behind() {
    let statuses = Rc::new(RefCell::new(Vec::new()));
    let mut ui = simulator(recorded(statuses.clone()));
    let at = centre(&mut ui, "Recorded");

    touch_at(&mut ui, pressed, at);
    assert_eq!(drawn(&mut ui, &statuses), Some(Status::Pressed));
    touch_at(&mut ui, lifted, at);
    assert_eq!(drawn(&mut ui, &statuses), Some(Status::Active));

    touch_at(&mut ui, pressed, at);
    touch_at(&mut ui, moved, Point::new(at.x, at.y + 30.0));
    assert_eq!(drawn(&mut ui, &statuses), Some(Status::Active));
    touch_at(&mut ui, lifted, Point::new(at.x, at.y + 30.0));

    // A real mouse over the row brings the hover back.
    mouse_at(&mut ui, at, &[]);
    assert_eq!(drawn(&mut ui, &statuses), Some(Status::Hovered));
    assert_eq!(messages(ui), vec![Message::Tap(0)]);
}

#[test]
fn without_messages_nothing_is_published() {
    let view: Element<'_, Message> = tappable(text("Plain")).into();
    let mut ui = simulator(view);
    let at = centre(&mut ui, "Plain");
    touch_at(&mut ui, pressed, at);
    touch_at(&mut ui, lifted, at);
    assert!(messages(ui).is_empty());
}
