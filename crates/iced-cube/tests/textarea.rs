#![cfg(feature = "textarea")]

use iced::advanced::input_method;
use iced::keyboard::key::Named;
use iced::widget::Id;
use iced::widget::text_editor::{Action, Content, Edit};
use iced::{Element, Event, Point, touch};
use iced_cube::textarea;
use iced_test::{Error, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Edit(Action),
}

fn view(content: &Content, enabled: bool) -> Element<'_, Message> {
    textarea(content)
        .id("notes")
        .placeholder("Write a note")
        .on_action_maybe(enabled.then_some(Message::Edit))
        .into()
}

fn inserted(messages: &[Message]) -> String {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::Edit(Action::Edit(Edit::Insert(c))) => Some(*c),
            _ => None,
        })
        .collect()
}

#[test]
fn typing_emits_insert_actions() -> Result<(), Error> {
    let content = Content::new();
    let mut ui = simulator(view(&content, true));
    ui.click(Id::new("notes"))?;
    ui.typewrite("hi");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(inserted(&messages), "hi");
    Ok(())
}

#[test]
fn enter_inserts_a_new_line() -> Result<(), Error> {
    let content = Content::with_text("first");
    let mut ui = simulator(view(&content, true));
    ui.click(Id::new("notes"))?;
    ui.tap_key(Named::Enter);

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(messages.contains(&Message::Edit(Action::Edit(Edit::Enter))));
    Ok(())
}

#[test]
fn applying_actions_updates_the_content() -> Result<(), Error> {
    let mut content = Content::new();
    let messages: Vec<_> = {
        let mut ui = simulator(view(&content, true));
        ui.click(Id::new("notes"))?;
        ui.typewrite("ok");
        ui.into_messages().collect()
    };

    for Message::Edit(action) in messages {
        content.perform(action);
    }
    assert_eq!(content.text().trim_end(), "ok");
    Ok(())
}

fn touch(ui: &mut iced_test::Simulator<'_, Message>, events: &[(touch::Event, Point)]) {
    for (event, at) in events {
        ui.point_at(*at);
        let _ = ui.simulate([Event::Touch(*event)]);
    }
}

#[test]
fn a_tap_focuses_the_textarea_and_places_the_caret() {
    let content = Content::with_text("first\nsecond");
    let mut ui = simulator(view(&content, true));
    let id = touch::Finger(1);
    let at = Point::new(30.0, 30.0);
    touch(
        &mut ui,
        &[
            (touch::Event::FingerPressed { id, position: at }, at),
            (touch::Event::FingerLifted { id, position: at }, at),
        ],
    );
    ui.typewrite("x");
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(matches!(
        messages.first(),
        Some(Message::Edit(Action::Click(_)))
    ));
    assert_eq!(inserted(&messages), "x");
}

#[test]
fn a_vertical_drag_scrolls_the_text() {
    let lines = (1..=40).map(|n| format!("line {n}")).collect::<Vec<_>>();
    let content = Content::with_text(&lines.join("\n"));
    let view: Element<'_, Message> = textarea(&content)
        .height(120)
        .on_action(Message::Edit)
        .into();
    let mut ui = simulator(view);
    let id = touch::Finger(1);
    let mut events = vec![(
        touch::Event::FingerPressed {
            id,
            position: Point::new(30.0, 100.0),
        },
        Point::new(30.0, 100.0),
    )];
    for step in 1..=4 {
        let at = Point::new(30.0, 100.0 - 21.0 * step as f32);
        events.push((touch::Event::FingerMoved { id, position: at }, at));
    }
    let end = Point::new(30.0, 16.0);
    events.push((touch::Event::FingerLifted { id, position: end }, end));
    touch(&mut ui, &events);
    let lines: i32 = ui
        .into_messages()
        .filter_map(|message| match message {
            Message::Edit(Action::Scroll { lines }) => Some(lines),
            _ => None,
        })
        .sum();
    assert_eq!(lines, 4);
}

/// Renders an empty textarea, optionally focused and with a word being
/// composed, and returns a hash of the window.
fn empty_render(
    placeholder: bool,
    focused: bool,
    composing: Option<&str>,
    name: &str,
) -> Result<String, Error> {
    let content = Content::new();
    let editor = textarea(&content).id("notes").on_action(Message::Edit);
    let editor = if placeholder {
        editor.placeholder("Write a note")
    } else {
        editor
    };
    let mut ui = simulator(Element::from(editor));
    if focused {
        ui.click(Id::new("notes"))?;
    }
    if let Some(word) = composing {
        let _ = ui.simulate([Event::InputMethod(input_method::Event::Preedit(
            word.to_owned(),
            Some(word.len()..word.len()),
        ))]);
    }

    let directory =
        std::env::temp_dir().join(format!("iced-cube-textarea-{name}-{}", std::process::id()));
    let _ = ui
        .snapshot(&iced_cube::theme::light())?
        .matches_hash(directory.join(name))?;
    let mut hash = String::new();
    for file in std::fs::read_dir(&directory)?.filter_map(Result::ok) {
        hash = std::fs::read_to_string(file.path())?;
    }
    std::fs::remove_dir_all(&directory)?;
    Ok(hash)
}

#[test]
fn the_placeholder_hides_while_a_word_is_composed() -> Result<(), Error> {
    let composing = empty_render(true, true, Some("hel"), "composing")?;
    let no_placeholder = empty_render(false, true, Some("hel"), "no-placeholder")?;
    let not_composing = empty_render(true, true, None, "not-composing")?;
    let finished = empty_render(true, true, Some(""), "finished")?;

    assert!(!composing.is_empty());
    assert_eq!(composing, no_placeholder);
    assert_ne!(composing, not_composing);
    assert_eq!(finished, not_composing);
    Ok(())
}

#[test]
fn an_unfocused_textarea_keeps_its_placeholder() -> Result<(), Error> {
    let composing = empty_render(true, false, Some("hel"), "unfocused-composing")?;
    let idle = empty_render(true, false, None, "unfocused-idle")?;

    assert_eq!(composing, idle);
    Ok(())
}

#[test]
fn disabled_textarea_emits_nothing() -> Result<(), Error> {
    let content = Content::with_text("Read only");
    let mut ui = simulator(view(&content, false));
    ui.click(Id::new("notes"))?;
    ui.typewrite("x");

    assert_eq!(ui.into_messages().count(), 0);
    Ok(())
}
