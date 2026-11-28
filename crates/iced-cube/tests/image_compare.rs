#![cfg(feature = "image-compare")]

use iced::widget::image::Handle;
use iced::{Element, Event, Point, Rectangle, Settings, Size, mouse, touch};
use iced_cube::data::image_compare::{self, Mode, Outline};
use iced_test::simulator::Simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Divider(f32),
    Hovered(Option<Point>),
    Pressed(Point),
}

/// The window is twice the image's size, so one screen point is half a pixel.
const IMAGE: Size = Size::new(200.0, 100.0);
const WINDOW: Size = Size::new(400.0, 300.0);

fn picture() -> Handle {
    Handle::from_rgba(200, 100, vec![128; 200 * 100 * 4])
}

fn simulator(element: Element<'_, Message>) -> Simulator<'_, Message> {
    Simulator::with_size(Settings::default(), WINDOW, element)
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

fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
    ui.into_messages().collect()
}

#[test]
fn dragging_the_divider_sends_its_new_place() {
    let element = image_compare::image_compare(picture(), picture())
        .divider(0.5)
        .on_divider(Message::Divider)
        .into();
    let mut ui = simulator(element);
    // The image fills the width and keeps its shape: 400 by 200.
    ui.point_at(Point::new(203.0, 100.0));
    ui.simulate([
        moved(203.0, 100.0),
        pressed(),
        moved(100.0, 100.0),
        moved(-50.0, 100.0),
        released(),
    ]);
    assert_eq!(
        messages(ui),
        vec![Message::Divider(0.25), Message::Divider(0.0)]
    );
}

#[test]
fn a_press_away_from_the_divider_reports_the_image_pixel() {
    let element = image_compare::image_compare(picture(), picture())
        .on_divider(Message::Divider)
        .on_press(Message::Pressed)
        .into();
    let mut ui = simulator(element);
    ui.point_at(Point::new(100.0, 50.0));
    ui.simulate([pressed(), released()]);
    assert_eq!(messages(ui), vec![Message::Pressed(Point::new(50.0, 25.0))]);
}

#[test]
fn a_divider_without_a_message_stays_put() {
    let element = image_compare::image_compare(picture(), picture())
        .on_press(Message::Pressed)
        .into();
    let mut ui = simulator(element);
    ui.point_at(Point::new(200.0, 100.0));
    ui.simulate([pressed(), moved(300.0, 100.0), released()]);
    // The press falls through to the picture instead.
    assert_eq!(
        messages(ui),
        vec![Message::Pressed(Point::new(100.0, 50.0))]
    );
}

#[test]
fn hovering_reports_each_new_pixel_and_leaving() {
    let element = image_compare::single(picture())
        .on_hover(Message::Hovered)
        .into();
    let mut ui = simulator(element);
    ui.simulate([
        moved(20.0, 20.0),
        moved(20.0, 20.0),
        moved(40.0, 60.0),
        moved(20.0, 250.0),
        Event::Mouse(mouse::Event::CursorLeft),
    ]);
    assert_eq!(
        messages(ui),
        vec![
            Message::Hovered(Some(Point::new(10.0, 10.0))),
            Message::Hovered(Some(Point::new(20.0, 30.0))),
            Message::Hovered(None),
        ]
    );
}

#[test]
fn side_by_side_reports_pixels_in_either_image() {
    let element = image_compare::image_compare(picture(), picture())
        .mode(Mode::SideBySide)
        .on_press(Message::Pressed)
        .into();
    let mut ui = simulator(element);
    // Each half is 196 wide with an 8 point gap.
    ui.point_at(Point::new(98.0, 49.0));
    ui.simulate([pressed(), released()]);
    ui.point_at(Point::new(204.0 + 98.0, 49.0));
    ui.simulate([pressed(), released()]);
    ui.point_at(Point::new(200.0, 49.0));
    ui.simulate([pressed(), released()]);
    assert_eq!(
        messages(ui),
        vec![
            Message::Pressed(Point::new(100.0, 50.0)),
            Message::Pressed(Point::new(100.0, 50.0)),
        ]
    );
}

#[test]
fn a_finger_drags_the_divider_from_further_away() {
    let element = image_compare::image_compare(picture(), picture())
        .on_divider(Message::Divider)
        .into();
    let mut ui = simulator(element);
    let finger = touch::Finger(1);
    ui.simulate([
        Event::Touch(touch::Event::FingerPressed {
            id: finger,
            position: Point::new(215.0, 100.0),
        }),
        Event::Touch(touch::Event::FingerMoved {
            id: finger,
            position: Point::new(300.0, 100.0),
        }),
        Event::Touch(touch::Event::FingerLifted {
            id: finger,
            position: Point::new(300.0, 100.0),
        }),
    ]);
    assert_eq!(messages(ui), vec![Message::Divider(0.75)]);
}

#[test]
fn a_fixed_height_letterboxes_the_image() {
    let element: Element<'_, Message> = iced::widget::container(
        image_compare::single(picture())
            .height(300)
            .outline(Outline::new(Rectangle::new(Point::ORIGIN, IMAGE)))
            .on_press(Message::Pressed),
    )
    .width(160)
    .into();
    let mut ui = Simulator::with_size(Settings::default(), Size::new(160.0, 400.0), element);
    // 160 wide keeps the shape at 160 by 80, centred in 300.
    ui.point_at(Point::new(80.0, 20.0));
    ui.simulate([pressed(), released()]);
    ui.point_at(Point::new(80.0, 150.0));
    ui.simulate([pressed(), released()]);
    assert_eq!(
        messages(ui),
        vec![Message::Pressed(Point::new(100.0, 50.0))]
    );
}

#[cfg(feature = "icedwright")]
#[path = "support/semantic.rs"]
mod semantic;

#[cfg(feature = "icedwright")]
#[test]
fn the_compare_reports_its_images_and_divider() {
    let element: Element<'_, Message> = image_compare::image_compare(picture(), picture())
        .divider(0.25)
        .on_divider(Message::Divider)
        .into();
    assert_eq!(
        semantic::yaml(element),
        "- group \"Image compare\" [prop:mode=\"slider\"]:\n  - img \"Before\"\n  - img \"After\"\n  \
         - separator \"Divider\" [value=25] [min=0] [max=100]\n"
    );
    let single: Element<'_, Message> = image_compare::single(picture()).label("Frame").into();
    assert_eq!(
        semantic::yaml(single),
        "- group \"Frame\" [prop:mode=\"single\"]:\n  - img \"After\"\n"
    );
    let side: Element<'_, Message> = image_compare::image_compare(picture(), picture())
        .mode(Mode::SideBySide)
        .into();
    assert_eq!(
        semantic::yaml(side),
        "- group \"Image compare\" [prop:mode=\"side-by-side\"]:\n  - img \"Before\"\n  - img \"After\"\n"
    );
}
