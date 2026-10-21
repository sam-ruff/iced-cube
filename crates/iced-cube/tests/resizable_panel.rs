#![cfg(feature = "resizable-panel")]

use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::text;
use iced::{Element, Event as Input, Point, Settings, Size, mouse, touch};
use iced_cube::layout::resizable_panel::{
    self, Axis, Event, Extent, Output, State, panel, resizable_panel,
};
use iced_test::simulator::Simulator;

const WINDOW: Size = Size::new(401.0, 200.0);
/// The panels share the window less the one pixel handle.
const TOTAL: f32 = 400.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Message {
    Panel(Event),
}

fn view(state: &State, axis: Axis) -> Element<'_, Message> {
    resizable_panel(state, [text("Left").into(), text("Right").into()])
        .axis(axis)
        .grip(true)
        .on_event(Message::Panel)
        .into()
}

/// Sends `inputs` to a fresh view of `state`, applies every message and
/// returns the events and outputs, as an app would. A view built before
/// the first measurement sends one with every input, so the events
/// returned leave measurements out.
fn run(state: &mut State, axis: Axis, inputs: &[Input]) -> (Vec<Event>, Vec<Output>) {
    let (events, outputs) = run_all(state, axis, inputs);
    let events = events
        .into_iter()
        .filter(|event| !matches!(event, Event::Measure(_)))
        .collect();
    (events, outputs)
}

fn run_all(state: &mut State, axis: Axis, inputs: &[Input]) -> (Vec<Event>, Vec<Output>) {
    let messages: Vec<Message> = {
        let mut ui = Simulator::with_size(Settings::default(), WINDOW, view(state, axis));
        for input in inputs {
            if let Input::Mouse(mouse::Event::CursorMoved { position }) = input {
                ui.point_at(*position);
            }
            let _ = ui.simulate([input.clone()]);
        }
        ui.into_messages().collect()
    };
    let events: Vec<Event> = messages
        .into_iter()
        .map(|Message::Panel(event)| event)
        .collect();
    let outputs = events
        .iter()
        .filter_map(|event| state.update(*event))
        .collect();
    (events, outputs)
}

fn moved(x: f32, y: f32) -> Input {
    Input::Mouse(mouse::Event::CursorMoved {
        position: Point::new(x, y),
    })
}

fn pressed() -> Input {
    Input::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
}

fn released() -> Input {
    Input::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
}

fn key(named: Named, modifiers: Modifiers) -> Input {
    Input::Keyboard(keyboard::Event::KeyPressed {
        key: Key::Named(named),
        modified_key: Key::Named(named),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}

fn sidebar_and_content() -> State {
    State::new([
        panel(Extent::Fraction(0.3))
            .min(Extent::Pixels(80.0))
            .max(Extent::Fraction(0.6))
            .collapsible(true),
        panel(Extent::Fraction(0.7)).min(Extent::Pixels(120.0)),
    ])
}

fn close(actual: &[f32], expected: &[f32]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 0.5)
}

#[test]
fn the_group_measures_itself_on_the_first_event() {
    let mut state = sidebar_and_content();
    let (events, _) = run_all(&mut state, Axis::Horizontal, &[moved(10.0, 10.0)]);
    assert_eq!(events, [Event::Measure(TOTAL)]);
    assert_eq!(state.total(), TOTAL);

    let (events, _) = run_all(&mut state, Axis::Horizontal, &[moved(20.0, 10.0)]);
    assert!(events.is_empty(), "no second measurement: {events:?}");
}

#[test]
fn dragging_a_handle_resizes_the_panels() {
    let mut state = sidebar_and_content();
    let handle = 0.3 * TOTAL;

    let (events, _) = run(
        &mut state,
        Axis::Horizontal,
        &[moved(handle + 2.0, 100.0), pressed()],
    );
    assert_eq!(
        events,
        [Event::DragStart {
            handle: 0,
            position: handle + 2.0
        }]
    );
    assert_eq!(state.dragging(), Some(0));

    let (events, outputs) = run(
        &mut state,
        Axis::Horizontal,
        &[moved(handle + 82.0, 100.0), released()],
    );
    assert_eq!(events, [Event::Drag(handle + 82.0), Event::DragEnd]);
    assert_eq!(outputs, [Output::Resized]);
    assert_eq!(state.dragging(), None);
    assert!(
        close(&state.resolve(TOTAL), &[200.0, 200.0]),
        "{:?}",
        state.resolve(TOTAL)
    );

    let mut ui = Simulator::with_size(Settings::default(), WINDOW, view(&state, Axis::Horizontal));
    let right = ui.find("Right").expect("right pane").bounds();
    assert!(
        (right.x - 201.0).abs() < 1.0,
        "right pane starts at {}",
        right.x
    );
}

#[test]
fn drags_stop_at_the_maximum() {
    let mut state = sidebar_and_content();
    let handle = 0.3 * TOTAL;
    let _ = run(
        &mut state,
        Axis::Horizontal,
        &[moved(handle, 100.0), pressed()],
    );
    let _ = run(
        &mut state,
        Axis::Horizontal,
        &[moved(390.0, 100.0), released()],
    );
    assert!(close(&state.resolve(TOTAL), &[240.0, 160.0]));
}

#[test]
fn dragging_below_the_threshold_collapses_the_panel() {
    let mut state = sidebar_and_content();
    let handle = 0.3 * TOTAL;
    let _ = run(
        &mut state,
        Axis::Horizontal,
        &[moved(handle, 100.0), pressed()],
    );
    let (_, outputs) = run(
        &mut state,
        Axis::Horizontal,
        &[moved(20.0, 100.0), released()],
    );
    assert_eq!(outputs, [Output::Collapsed(0)]);
    assert!(state.is_collapsed(0));
    assert!(close(&state.resolve(TOTAL), &[0.0, 400.0]));

    let mut ui = Simulator::with_size(Settings::default(), WINDOW, view(&state, Axis::Horizontal));
    let right = ui.find("Right").expect("right pane").bounds();
    assert!(right.x < 2.0, "the right pane fills the group");
}

#[test]
fn touch_drags_work_like_the_pointer() {
    let mut state = sidebar_and_content();
    let finger = touch::Finger(1);
    let handle = 0.3 * TOTAL;
    let _ = run(
        &mut state,
        Axis::Horizontal,
        &[Input::Touch(touch::Event::FingerPressed {
            id: finger,
            position: Point::new(handle, 50.0),
        })],
    );
    assert_eq!(state.dragging(), Some(0));
    let (_, outputs) = run(
        &mut state,
        Axis::Horizontal,
        &[
            Input::Touch(touch::Event::FingerMoved {
                id: finger,
                position: Point::new(handle + 40.0, 50.0),
            }),
            Input::Touch(touch::Event::FingerLifted {
                id: finger,
                position: Point::new(handle + 40.0, 50.0),
            }),
        ],
    );
    assert_eq!(outputs, [Output::Resized]);
    assert!(close(&state.resolve(TOTAL), &[160.0, 240.0]));
}

#[test]
fn a_focused_handle_resizes_with_the_keyboard() {
    let mut state = sidebar_and_content();
    let handle = 0.3 * TOTAL;
    let none = Modifiers::empty();
    let _ = run(&mut state, Axis::Horizontal, &[moved(10.0, 10.0)]);

    // Pressing the handle focuses it; the keys then go to the group.
    let (events, _) = run(
        &mut state,
        Axis::Horizontal,
        &[
            moved(handle, 100.0),
            pressed(),
            released(),
            key(Named::ArrowRight, none),
            key(Named::ArrowRight, Modifiers::SHIFT),
        ],
    );
    let steps: Vec<Event> = events
        .into_iter()
        .filter(|event| matches!(event, Event::Resize { .. }))
        .collect();
    assert_eq!(
        steps,
        [
            Event::Resize {
                handle: 0,
                delta: 20.0
            },
            Event::Resize {
                handle: 0,
                delta: 80.0
            }
        ]
    );
    assert!(
        close(&state.resolve(TOTAL), &[220.0, 180.0]),
        "{:?}",
        state.resolve(TOTAL)
    );

    let (_, _) = run(
        &mut state,
        Axis::Horizontal,
        &[
            moved(220.0, 100.0),
            pressed(),
            released(),
            key(Named::Home, none),
        ],
    );
    assert!(close(&state.resolve(TOTAL), &[80.0, 320.0]));

    let (_, _) = run(
        &mut state,
        Axis::Horizontal,
        &[
            moved(80.0, 100.0),
            pressed(),
            released(),
            key(Named::End, none),
        ],
    );
    assert!(close(&state.resolve(TOTAL), &[240.0, 160.0]));

    let (_, outputs) = run(
        &mut state,
        Axis::Horizontal,
        &[
            moved(240.0, 100.0),
            pressed(),
            released(),
            key(Named::Enter, none),
        ],
    );
    assert_eq!(outputs, [Output::Collapsed(0)]);
}

#[test]
fn keys_pass_through_until_a_handle_has_focus_and_after_escape() {
    let mut state = sidebar_and_content();
    let none = Modifiers::empty();
    let (events, _) = run(
        &mut state,
        Axis::Horizontal,
        &[moved(300.0, 100.0), key(Named::ArrowRight, none)],
    );
    assert!(events.is_empty(), "{events:?}");

    let handle = 0.3 * TOTAL;
    let (events, _) = run(
        &mut state,
        Axis::Horizontal,
        &[
            moved(handle, 100.0),
            pressed(),
            released(),
            key(Named::Escape, none),
            key(Named::ArrowRight, none),
        ],
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Resize { .. })),
        "{events:?}"
    );

    let (events, _) = run(
        &mut state,
        Axis::Horizontal,
        &[
            moved(handle, 100.0),
            pressed(),
            released(),
            moved(300.0, 100.0),
            pressed(),
            key(Named::ArrowRight, none),
        ],
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Resize { .. })),
        "a press elsewhere takes the focus: {events:?}"
    );
}

#[test]
fn vertical_groups_drag_along_y() {
    let mut state = State::new([panel(Extent::Fraction(0.5)), panel(Extent::Fraction(0.5))]);
    // 200 tall less the handle leaves 199 to share.
    let handle = 99.5;
    let _ = run(
        &mut state,
        Axis::Vertical,
        &[moved(50.0, handle), pressed()],
    );
    assert_eq!(state.total(), 199.0);
    let (_, outputs) = run(
        &mut state,
        Axis::Vertical,
        &[moved(50.0, handle + 40.0), released()],
    );
    assert_eq!(outputs, [Output::Resized]);
    assert!(close(&state.resolve(199.0), &[139.5, 59.5]));
}

#[test]
fn a_group_without_a_message_ignores_the_pointer() {
    let state = sidebar_and_content();
    let group: Element<'_, Message> =
        resizable_panel(&state, [text("Left").into(), text("Right").into()]).into();
    let mut ui = Simulator::with_size(Settings::default(), WINDOW, group);
    ui.point_at(Point::new(120.0, 100.0));
    let _ = ui.simulate([moved(120.0, 100.0), pressed(), released()]);
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn narrow_groups_stack_below_the_breakpoint() {
    let state = State::new([panel(Extent::Fraction(0.5)), panel(Extent::Fraction(0.5))]);
    let group: Element<'_, Message> =
        resizable_panel(&state, [text("Left").into(), text("Right").into()])
            .stack_below(480.0)
            .on_event(Message::Panel)
            .into();
    let mut ui = Simulator::with_size(Settings::default(), Size::new(360.0, 400.0), group);
    let left = ui.find("Left").expect("left pane").bounds();
    let right = ui.find("Right").expect("right pane").bounds();
    assert_eq!(left.x, right.x, "stacked in one column");
    assert!(
        right.y > 190.0,
        "right pane below the left one at {}",
        right.y
    );
}

#[test]
fn the_default_keymap_is_exported() {
    assert!(!resizable_panel::default_keymap().is_empty());
}
