#![cfg(all(feature = "tooltip", feature = "button"))]

use std::path::PathBuf;

use iced::time::Instant;
use iced::widget::{container, text};
use iced::{Element, Event, mouse, touch, window};
use iced_cube::button;
use iced_cube::overlay::tooltip::{Position, tooltip};
use iced_test::simulator::{Simulator, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Save,
}

#[test]
fn wrapped_content_still_receives_clicks() {
    for position in Position::ALL {
        let element: Element<'_, Message> =
            tooltip(button("Save").on_press(Message::Save), "Save changes")
                .position(position)
                .into();
        let mut ui = simulator(element);
        ui.click("Save").expect("content is rendered");
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![Message::Save],
            "{position:?}"
        );
    }
}

fn redraw() -> Event {
    Event::Window(window::Event::RedrawRequested(Instant::now()))
}

/// Plain text has no hover look, so a frame differs from the closed one
/// only when the bubble is drawn. The bubble is an overlay that selectors
/// do not reach, so frames are compared instead.
struct Frames {
    directory: PathBuf,
    closed: PathBuf,
}

impl Drop for Frames {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

impl Frames {
    fn new(ui: &mut Simulator<'_, Message>, name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("iced-cube-tooltip-{name}-{}", std::process::id()));
        let closed = directory.join("closed");
        let frames = Self { directory, closed };
        assert!(!frames.bubble(ui), "the bubble starts closed");
        frames
    }

    fn bubble(&self, ui: &mut Simulator<'_, Message>) -> bool {
        let _ = ui.simulate([redraw()]);
        match ui
            .snapshot(&iced_cube::theme::light())
            .and_then(|frame| frame.matches_hash(&self.closed))
        {
            Ok(same) => !same,
            Err(error) => panic!("snapshot failed: {error:?}"),
        }
    }
}

fn hinted() -> Element<'static, Message> {
    container(tooltip(text("Save"), "Save changes"))
        .padding(60)
        .into()
}

#[test]
fn bubble_opens_when_the_mouse_rests_on_the_content() -> Result<(), iced_test::Error> {
    let mut ui = simulator(hinted());
    let frames = Frames::new(&mut ui, "mouse");
    let at = ui.find("Save")?.bounds().center();
    ui.point_at(at);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: at })]);
    assert!(frames.bubble(&mut ui));
    Ok(())
}

#[test]
fn a_tap_does_not_leave_the_bubble_open_until_the_mouse_moves() -> Result<(), iced_test::Error> {
    let mut ui = simulator(hinted());
    let frames = Frames::new(&mut ui, "touch");
    let at = ui.find("Save")?.bounds().center();
    let id = touch::Finger(1);
    ui.point_at(at);
    let _ = ui.simulate([Event::Touch(touch::Event::FingerPressed {
        id,
        position: at,
    })]);
    assert!(frames.bubble(&mut ui), "a held finger shows the bubble");
    let _ = ui.simulate([Event::Touch(touch::Event::FingerLifted {
        id,
        position: at,
    })]);
    assert!(!frames.bubble(&mut ui));
    assert!(!frames.bubble(&mut ui));

    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: at })]);
    assert!(frames.bubble(&mut ui));
    Ok(())
}

#[test]
fn a_tap_on_a_hinted_button_presses_it() -> Result<(), iced_test::Error> {
    let element: Element<'_, Message> =
        tooltip(button("Save").on_press(Message::Save), "Save changes").into();
    let mut ui = simulator(element);
    let at = ui.find("Save")?.bounds().center();
    let id = touch::Finger(1);
    ui.point_at(at);
    let _ = ui.simulate([
        Event::Touch(touch::Event::FingerPressed { id, position: at }),
        Event::Touch(touch::Event::FingerLifted { id, position: at }),
        redraw(),
    ]);
    assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![Message::Save]);
    Ok(())
}

#[test]
fn bubble_is_hidden_until_hovered() {
    let element: Element<'_, Message> =
        tooltip(button("Save").on_press(Message::Save), "Save changes").into();
    let mut ui = simulator(element);
    assert!(ui.find("Save").is_ok());
    assert!(ui.find("Save changes").is_err());
}
