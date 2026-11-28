#![cfg(all(feature = "tooltip", feature = "button"))]

use std::path::PathBuf;

use iced::time::Instant;
use iced::widget::{self, container, text, text_input};
use iced::{Element, Event, Point, mouse, touch, window};
use iced_cube::button;
use iced_cube::overlay::tooltip::{Position, Tooltip, tooltip};
use iced_test::selector::Candidate;
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
/// only when the bubble is drawn. Comparing frames checks that the bubble
/// is drawn, not just laid out.
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

fn hover(ui: &mut Simulator<'_, Message>, label: &str) -> Result<(), iced_test::Error> {
    let at = ui.find(label)?.bounds().center();
    ui.point_at(at);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: at })]);
    Ok(())
}

fn leave(ui: &mut Simulator<'_, Message>) {
    let position = Point::new(1.0, 1.0);
    ui.point_at(position);
    let _ = ui.simulate([
        Event::Mouse(mouse::Event::CursorMoved { position }),
        redraw(),
    ]);
}

#[test]
fn selectors_reach_the_bubble_while_it_shows() -> Result<(), iced_test::Error> {
    let mut ui = simulator(hinted());
    hover(&mut ui, "Save")?;
    let bubble = ui.find("Save changes")?;
    let content = ui.find("Save")?.bounds();
    assert!(
        bubble.bounds().y + bubble.bounds().height < content.y,
        "the bubble sits above"
    );
    leave(&mut ui);
    assert!(ui.find("Save changes").is_err());
    Ok(())
}

const SENTENCE: &str = "Saves every change in this document and shares it with your team.";

fn hinted_with<'a>(
    configure: impl FnOnce(Tooltip<'a, Message>) -> Tooltip<'a, Message>,
) -> Element<'a, Message> {
    container(configure(tooltip(text("Save"), SENTENCE)))
        .padding(200)
        .into()
}

#[test]
fn max_width_wraps_a_sentence_onto_more_lines() -> Result<(), iced_test::Error> {
    let mut wide = simulator(hinted_with(|tooltip| tooltip));
    hover(&mut wide, "Save")?;
    let one_line = wide.find(SENTENCE)?.bounds();

    let mut narrow = simulator(hinted_with(|tooltip| tooltip.max_width(200.0)));
    hover(&mut narrow, "Save")?;
    let wrapped = narrow.find(SENTENCE)?.bounds();

    assert!(wrapped.width < 200.0, "{wrapped:?}");
    assert!(
        wrapped.height > one_line.height * 2.0,
        "{wrapped:?} vs {one_line:?}"
    );
    Ok(())
}

#[test]
fn a_sentence_reads_larger_and_the_size_can_be_set() -> Result<(), iced_test::Error> {
    let height = |element: Element<'static, Message>, label: &str| {
        let mut ui = simulator(element);
        hover(&mut ui, "Save")?;
        Ok::<_, iced_test::Error>(ui.find(label)?.bounds().height)
    };
    let automatic = height(hinted_with(|tooltip| tooltip), SENTENCE)?;
    let small = height(
        hinted_with(|tooltip| tooltip.text_size(iced_cube::theme::text_size::XS)),
        SENTENCE,
    )?;
    let large = height(hinted_with(|tooltip| tooltip.text_size(24.0)), SENTENCE)?;
    assert!(automatic > small, "{automatic} vs {small}");
    assert!(large > automatic, "{large} vs {automatic}");
    Ok(())
}

fn field() -> widget::Id {
    widget::Id::new("field")
}

#[test]
fn focus_from_a_click_does_not_keep_the_bubble_open() -> Result<(), iced_test::Error> {
    let element: Element<'_, Message> = container(tooltip(
        text_input("Name", "")
            .id(field())
            .on_input(|_| Message::Save),
        "Your full name",
    ))
    .padding(60)
    .into();
    let mut ui = simulator(element);
    ui.click(field())?;
    assert!(ui.find("Your full name").is_ok(), "hovered");
    leave(&mut ui);
    assert!(
        ui.find(|candidate: Candidate<'_>| match candidate {
            Candidate::Focusable { state, .. } if state.is_focused() => Some(()),
            _ => None,
        })
        .is_ok(),
        "the field kept focus"
    );
    assert!(ui.find("Your full name").is_err());
    Ok(())
}

#[cfg(feature = "dialog")]
#[test]
fn keyboard_focus_shows_the_bubble_and_moves_with_it() -> Result<(), iced_test::Error> {
    use iced::keyboard::key::Named;
    use iced::widget::column;
    use iced_cube::overlay::dialog::dialog;

    let body = column![
        tooltip(
            text_input("Name", "").on_input(|_| Message::Save),
            "Your full name"
        ),
        tooltip(
            text_input("Email", "").on_input(|_| Message::Save),
            "Where invites go"
        ),
    ]
    .spacing(16);
    let element: Element<'_, Message> = dialog(text("Page"))
        .open(true)
        .title("Invite")
        .body(body)
        .into();
    let mut ui = simulator(element);
    leave(&mut ui);
    assert!(
        ui.find("Your full name").is_ok(),
        "opening focused the first field"
    );
    assert!(ui.find("Where invites go").is_err());

    let _ = ui.tap_key(Named::Tab);
    let _ = ui.simulate([redraw()]);
    assert!(ui.find("Your full name").is_err());
    assert!(
        ui.find("Where invites go").is_ok(),
        "Tab moved focus and the bubble"
    );
    Ok(())
}

#[cfg(feature = "icedwright")]
#[path = "support/semantic.rs"]
mod semantic;

#[cfg(feature = "icedwright")]
#[test]
fn the_trigger_carries_its_tooltip_and_the_open_bubble_is_reported() {
    let view = || -> Element<'static, Message> {
        tooltip(button("Save").on_press(Message::Save), "Save changes").into()
    };
    let mut recorder = semantic::Recorder::default();
    let closed = "- button \"Save\" [focused=false] [focusable] \
                  [description=\"Save changes\"] [tooltip=\"Save changes\"]:\n  - text: Save\n";
    assert_eq!(recorder.yaml(view()), closed);
    let at = Point::new(20.0, 16.0);
    let _ = recorder.events(
        view(),
        mouse::Cursor::Available(at),
        &[Event::Mouse(mouse::Event::CursorMoved { position: at })],
    );
    assert_eq!(
        recorder.yaml(view()),
        format!("{closed}- tooltip \"Save changes\":\n  - text: Save changes\n")
    );
}
