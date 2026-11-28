#![cfg(feature = "timeline")]

use iced::{Element, Event, Point, Settings, Size, mouse, touch};
use iced_cube::data::timeline::{
    self, Action, LABEL_WIDTH, PLAYHEAD_RADIUS, Span, TRACK_HEIGHT, Tone, lane, track_bounds,
};
use iced_test::simulator::Simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Step(usize),
}

const WIDTH: f32 = 400.0;
const STEPS: [Tone; 5] = [
    Tone::Neutral,
    Tone::Success,
    Tone::Warning,
    Tone::Destructive,
    Tone::Neutral,
];

fn simulator(element: Element<'_, Message>) -> Simulator<'_, Message> {
    Simulator::with_size(Settings::default(), Size::new(WIDTH, 200.0), element)
}

/// The x of step `index` of five on a timeline with no lanes.
fn step_x(index: usize) -> f32 {
    PLAYHEAD_RADIUS + (WIDTH - PLAYHEAD_RADIUS * 2.0) * index as f32 / 4.0
}

fn moved(x: f32, y: f32) -> Event {
    Event::Mouse(mouse::Event::CursorMoved {
        position: Point::new(x, y),
    })
}

fn pressed() -> Event {
    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
}

fn released() -> Event {
    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
}

#[test]
fn pressing_the_track_selects_the_step_under_the_pointer() {
    let mut ui = simulator(timeline::timeline(STEPS, 0).on_select(Message::Step).into());
    let y = TRACK_HEIGHT / 2.0;
    ui.point_at(Point::new(step_x(3) + 3.0, y));
    ui.simulate([pressed(), released()]);
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Step(3)]
    );
}

#[test]
fn pressing_the_current_step_sends_nothing() {
    let mut ui = simulator(timeline::timeline(STEPS, 2).on_select(Message::Step).into());
    ui.point_at(Point::new(step_x(2), TRACK_HEIGHT / 2.0));
    ui.simulate([pressed(), released()]);
    assert!(ui.into_messages().next().is_none());
}

#[test]
fn dragging_sends_each_new_step_once() {
    let mut ui = simulator(timeline::timeline(STEPS, 0).on_select(Message::Step).into());
    let y = TRACK_HEIGHT / 2.0;
    ui.point_at(Point::new(step_x(1), y));
    ui.simulate([
        moved(step_x(1), y),
        pressed(),
        moved(step_x(1) + 4.0, y),
        moved(step_x(2), y),
        // Past the end, and far below the track, still follows the pointer.
        moved(WIDTH + 50.0, 180.0),
        released(),
        moved(step_x(0), y),
    ]);
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Step(1), Message::Step(2), Message::Step(4)]
    );
}

#[test]
fn a_finger_drags_the_playhead() {
    let mut ui = simulator(timeline::timeline(STEPS, 0).on_select(Message::Step).into());
    let y = TRACK_HEIGHT / 2.0;
    let finger = touch::Finger(1);
    ui.simulate([
        Event::Touch(touch::Event::FingerPressed {
            id: finger,
            position: Point::new(step_x(1), y),
        }),
        Event::Touch(touch::Event::FingerMoved {
            id: finger,
            position: Point::new(step_x(3), y),
        }),
        Event::Touch(touch::Event::FingerLifted {
            id: finger,
            position: Point::new(step_x(3), y),
        }),
    ]);
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Step(1), Message::Step(3)]
    );
}

#[test]
fn a_timeline_without_a_message_ignores_presses() {
    let mut ui = simulator(timeline::timeline(STEPS, 0).into());
    ui.point_at(Point::new(step_x(3), TRACK_HEIGHT / 2.0));
    ui.simulate([pressed(), released()]);
    assert!(ui.into_messages().next().is_none());
}

#[test]
fn lanes_move_the_track_right_of_their_labels_and_scrub_too() {
    let element = timeline::timeline(STEPS, 0)
        .lane(lane("Load", [Span::new(0, 2)]))
        .lane(lane("Save", [Span::new(3, 4).tone(Tone::Success)]))
        .on_select(Message::Step);
    let mut ui = simulator(element.into());
    let bounds = iced::Rectangle::new(Point::ORIGIN, Size::new(WIDTH, 100.0));
    let track = track_bounds(bounds, LABEL_WIDTH);
    // A press on the second lane row lands on the last step.
    ui.point_at(Point::new(track.x + track.width, TRACK_HEIGHT + 30.0));
    ui.simulate([pressed(), released()]);
    // Over the labels the pointer is left of the track, so it picks the first.
    ui.point_at(Point::new(10.0, TRACK_HEIGHT / 2.0));
    ui.simulate([pressed(), released()]);
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Step(4)]
    );
}

#[test]
fn keyboard_actions_follow_the_default_keymap() {
    let keymap = timeline::default_keymap();
    let mut current = 0;
    for (key, expected) in [
        (iced::keyboard::key::Named::ArrowRight, 1),
        (iced::keyboard::key::Named::End, 4),
        (iced::keyboard::key::Named::ArrowLeft, 3),
        (iced::keyboard::key::Named::Home, 0),
    ] {
        let Some(action) = keymap.resolve(
            &iced::keyboard::Key::Named(key),
            iced::keyboard::Modifiers::empty(),
        ) else {
            panic!("{key:?} is bound");
        };
        current = action.apply(current, STEPS.len());
        assert_eq!(current, expected, "{key:?}");
    }
    assert_eq!(Action::PageForward.apply(0, STEPS.len()), 4);
}

#[test]
fn a_narrow_timeline_keeps_its_height_and_still_scrubs() {
    let element: Element<'_, Message> = iced::widget::container(
        timeline::timeline(STEPS, 0)
            .lane(lane("A very long lane label", [Span::new(0, 4)]))
            .on_select(Message::Step),
    )
    .width(160)
    .into();
    let mut ui = Simulator::with_size(Settings::default(), Size::new(160.0, 200.0), element);
    ui.point_at(Point::new(150.0, TRACK_HEIGHT / 2.0));
    ui.simulate([pressed(), released()]);
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Step(4)]
    );
}

#[cfg(feature = "icedwright")]
#[path = "support/semantic.rs"]
mod semantic;

#[cfg(feature = "icedwright")]
#[test]
fn the_timeline_reports_itself_as_a_slider_over_its_steps() {
    let element: Element<'_, Message> = timeline::timeline(STEPS, 2)
        .label("Steps")
        .on_select(Message::Step)
        .into();
    assert_eq!(
        semantic::yaml(element),
        "- slider \"Steps\" [value=2] [min=0] [max=4]\n"
    );
    let disabled: Element<'_, Message> = timeline::timeline(STEPS, 0).into();
    assert_eq!(
        semantic::yaml(disabled),
        "- slider \"Timeline\" [disabled] [value=0] [min=0] [max=4]\n"
    );
}
