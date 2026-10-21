#![cfg(feature = "split-pane")]

use iced::widget::text;
use iced::{Element, Event as Input, Point, Settings, Size, mouse};
use iced_cube::layout::split_pane::{Axis, Event, Extent, State, split_pane};
use iced_test::simulator::Simulator;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Message {
    Split(Event),
}

const WINDOW: Size = Size::new(401.0, 300.0);

fn view(state: &State, axis: Axis) -> Element<'_, Message> {
    split_pane(state, text("Editor"), text("Preview"))
        .axis(axis)
        .on_event(Message::Split)
        .into()
}

fn run(state: &mut State, axis: Axis, inputs: &[Input]) {
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
    for Message::Split(event) in messages {
        let _ = state.update(event);
    }
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

#[test]
fn dragging_the_handle_changes_the_ratio() {
    let mut state = State::new(0.5);
    run(
        &mut state,
        Axis::Horizontal,
        &[moved(200.0, 150.0), pressed()],
    );
    run(
        &mut state,
        Axis::Horizontal,
        &[moved(300.0, 150.0), released()],
    );
    assert!((state.ratio() - 0.75).abs() < 0.01, "{}", state.ratio());

    let mut ui = Simulator::with_size(Settings::default(), WINDOW, view(&state, Axis::Horizontal));
    let preview = ui.find("Preview").expect("end view").bounds();
    assert!((preview.x - 301.0).abs() < 1.0, "end view at {}", preview.x);
}

#[test]
fn vertical_splits_drag_along_y_within_their_limits() {
    let mut state = State::new(0.5).min(Extent::Pixels(100.0), Extent::Pixels(100.0));
    run(
        &mut state,
        Axis::Vertical,
        &[moved(200.0, 150.0), pressed()],
    );
    run(
        &mut state,
        Axis::Vertical,
        &[moved(200.0, 280.0), released()],
    );
    let ratio = state.ratio();
    assert!((ratio - 199.0 / 299.0).abs() < 0.01, "{ratio}");
}
