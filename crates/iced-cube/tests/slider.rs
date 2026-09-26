use iced::{Element, Point, Settings, Size};
use iced_cube::primitives::slider;
use iced_cube::primitives::slider::HANDLE_RADIUS;
use iced_test::simulator::{Simulator, click};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Volume(u8),
    Released,
}

const WIDTH: f32 = 400.0;

fn run(element: Element<'_, Message>, x: f32) -> Vec<Message> {
    let mut ui = Simulator::with_size(Settings::default(), Size::new(WIDTH, 100.0), element);
    ui.point_at(Point::new(x, HANDLE_RADIUS));
    ui.simulate(click());
    ui.into_messages().collect()
}

#[test]
fn clicking_the_track_emits_the_value_under_the_cursor() {
    let element = slider(0..=100, 10)
        .on_change(Message::Volume)
        .on_release(Message::Released)
        .into();

    let messages = run(element, WIDTH / 2.0);
    assert_eq!(messages, vec![Message::Volume(50), Message::Released]);
}

#[test]
fn step_snaps_the_emitted_value() {
    let element = slider(0..=100, 0)
        .step(25)
        .on_change(Message::Volume)
        .into();

    let messages = run(element, WIDTH * 0.4);
    assert_eq!(messages, vec![Message::Volume(50)]);
}

#[test]
fn disabled_slider_emits_nothing() {
    let element = slider(0..=100, 10).into();

    assert!(run(element, WIDTH / 2.0).is_empty());
}

#[test]
fn value_label_is_rendered() {
    let element: Element<'_, Message> = slider(0..=100, 42_u8)
        .label("Volume")
        .show_value()
        .on_change(Message::Volume)
        .into();
    let mut ui = Simulator::with_size(Settings::default(), Size::new(WIDTH, 100.0), element);

    assert!(ui.find("Volume").is_ok());
    assert!(ui.find("42").is_ok());
}
