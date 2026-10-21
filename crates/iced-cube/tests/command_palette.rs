#![cfg(feature = "command-palette")]

use std::thread;

use futures::executor::block_on;
use futures::{SinkExt, StreamExt};
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::text;
use iced::{Element, Point, event};
use iced_cube::command_palette::{
    self, Event, Output, SEARCH_ID, State, command_palette, default_keymap, page,
};
use iced_cube::keys;
use iced_cube::navigation::command::{self, group, item, results};
use iced_test::simulator::{self, Simulator, simulator};

#[derive(Debug, Clone)]
enum Message {
    Palette(Event<&'static str>),
}

fn state() -> State<&'static str> {
    State::new([
        group(
            "Files",
            [
                item("new", "New file").shortcut("Ctrl+N"),
                item("open", "Open folder"),
            ],
        ),
        group(
            "Preferences",
            [
                item("theme", "Change theme..."),
                item("settings", "Settings"),
            ],
        ),
    ])
    .with_page(page(
        "theme",
        "Theme",
        [group("", [item("light", "Light"), item("dark", "Dark")])],
    ))
}

fn opened() -> State<&'static str> {
    let mut state = state();
    let _ = state.update(Event::Open);
    state
}

fn view<'a>(state: &'a State<&'static str>) -> Element<'a, Message> {
    command_palette(state, text("Workspace"))
        .on_event(Message::Palette)
        .into()
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event<&'static str>> {
    ui.into_messages()
        .map(|Message::Palette(event)| event)
        .collect()
}

fn apply(
    state: &mut State<&'static str>,
    events: Vec<Event<&'static str>>,
) -> Vec<Output<&'static str>> {
    events
        .into_iter()
        .filter_map(|event| state.update(event))
        .collect()
}

/// Renders the palette, lets the dialog focus the search field, then runs
/// `input` and applies everything it sent.
fn interact(
    state: &mut State<&'static str>,
    input: impl FnOnce(&mut Simulator<'_, Message>),
) -> Vec<Output<&'static str>> {
    let mut ui = simulator(view(state));
    ui.point_at(Point::new(1.0, 1.0));
    let _ = ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
        position: Point::new(1.0, 1.0),
    })]);
    input(&mut ui);
    let emitted = events(ui);
    apply(state, emitted)
}

#[test]
fn the_toggle_chord_opens_the_palette_over_the_app() {
    let mut state = state();
    let mut ui = simulator(view(&state));
    assert!(ui.find("Workspace").is_ok());
    assert!(ui.find("New file").is_err(), "closed");
    drop(ui);

    let ctrl_k = keys::Event {
        key: Key::Character("k".into()),
        modifiers: Modifiers::COMMAND,
    };
    let event = state
        .key_event(&default_keymap(), &ctrl_k)
        .expect("Ctrl+K toggles");
    let _ = state.update(event);
    assert!(state.is_open());

    let mut ui = simulator(view(&state));
    for label in [
        "Command palette",
        "Files",
        "New file",
        "Ctrl+N",
        "Preferences",
    ] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
}

#[test]
fn arrows_move_the_highlight_and_enter_opens_a_page() {
    let mut state = opened();
    let _ = interact(&mut state, |ui| {
        ui.click(SEARCH_ID).expect("search field is rendered");
        let _ = ui.tap_key(Named::ArrowDown);
        let _ = ui.tap_key(Named::ArrowDown);
    });
    let highlighted = state.command().highlighted().map(|item| item.id);
    assert_eq!(highlighted, Some("theme"));

    let outputs = interact(&mut state, |ui| {
        ui.click(SEARCH_ID).expect("search field is rendered");
        let _ = ui.tap_key(Named::Enter);
    });
    assert!(outputs.is_empty(), "{outputs:?}");
    assert_eq!(state.page_id(), Some(&"theme"));
}

#[test]
fn typing_and_enter_run_a_command() {
    let mut state = opened();
    let outputs = interact(&mut state, |ui| {
        let _ = ui.typewrite("set");
    });
    assert!(
        outputs
            .iter()
            .any(|output| matches!(output, Output::Search(query) if query == "set")),
        "{outputs:?}"
    );
    assert_eq!(state.query(), "set");

    let outputs = interact(&mut state, |ui| {
        let _ = ui.tap_key(Named::Enter);
    });
    assert!(
        matches!(outputs.as_slice(), [Output::Activated("settings")]),
        "{outputs:?}"
    );
    assert!(!state.is_open());
    assert_eq!(state.recent(), ["settings"]);
}

#[test]
fn clicking_a_row_runs_it() {
    let mut state = opened();
    let outputs = interact(&mut state, |ui| {
        ui.click("Open folder").expect("row is rendered");
    });
    assert!(
        matches!(outputs.as_slice(), [Output::Activated("open")]),
        "{outputs:?}"
    );
}

#[test]
fn recent_commands_come_first_next_time() {
    let mut state = opened();
    let _ = interact(&mut state, |ui| {
        ui.click("Settings").expect("row is rendered");
    });
    let _ = state.update(Event::Open);
    let mut ui = simulator(view(&state));
    let recent = ui.find("Recent").expect("recent group").bounds();
    let files = ui.find("Files").expect("files group").bounds();
    assert!(recent.y < files.y);
}

#[test]
fn a_page_opens_and_backspace_returns() {
    let mut state = opened();
    let _ = interact(&mut state, |ui| {
        ui.click("Change theme...").expect("row is rendered");
    });
    assert_eq!(state.page_id(), Some(&"theme"));
    let mut ui = simulator(view(&state));
    for label in ["Theme", "Light", "Dark", "Back"] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
    assert!(ui.find("New file").is_err());
    drop(ui);

    let _ = interact(&mut state, |ui| {
        ui.click(SEARCH_ID).expect("search field is rendered");
        let status = ui.tap_key(Named::Backspace);
        assert_eq!(status, event::Status::Captured);
    });
    assert_eq!(state.depth(), 0, "Backspace on an empty query goes back");
    assert!(state.is_open());
}

#[test]
fn the_back_button_returns_too() {
    let mut state = opened();
    let _ = interact(&mut state, |ui| {
        ui.click("Change theme...").expect("row is rendered");
    });
    let _ = interact(&mut state, |ui| {
        ui.click("Back").expect("back button is rendered");
    });
    assert_eq!(state.depth(), 0);
}

#[test]
fn backspace_with_a_query_deletes_text() {
    let mut state = opened();
    let _ = interact(&mut state, |ui| {
        ui.click("Change theme...").expect("row is rendered");
    });
    let _ = interact(&mut state, |ui| {
        ui.click(SEARCH_ID).expect("search field is rendered");
        let _ = ui.typewrite("da");
    });
    let _ = interact(&mut state, |ui| {
        ui.click(SEARCH_ID).expect("search field is rendered");
        let _ = ui.tap_key(Named::Backspace);
    });
    assert_eq!(state.depth(), 1, "still on the theme page");
    assert_eq!(state.query(), "d");
}

#[test]
fn escape_and_the_scrim_dismiss_the_palette() {
    let mut state = opened();
    let _ = interact(&mut state, |ui| {
        ui.click(SEARCH_ID).expect("search field is rendered");
        let _ = ui.tap_key(Named::Escape);
    });
    assert!(!state.is_open(), "Escape on an empty query closes");

    let mut state = opened();
    let _ = interact(&mut state, |ui| {
        ui.point_at(Point::new(4.0, 4.0));
        let _ = ui.simulate(simulator::click());
    });
    assert!(!state.is_open(), "a click on the scrim closes");
}

#[test]
fn the_toggle_chord_closes_the_open_palette_from_the_field() {
    let mut state = opened();
    let _ = interact(&mut state, |ui| {
        ui.click(SEARCH_ID).expect("search field is rendered");
        let key = Key::Character("k".into());
        let _ = ui.simulate([iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: Modifiers::COMMAND,
            repeat: false,
            text: None,
        })]);
    });
    assert!(!state.is_open());
}

#[test]
fn async_results_arrive_through_the_command_channel() {
    let mut state = opened();
    let mut stream = Box::pin(command::stream::<&'static str>());
    let Some(ready) = block_on(stream.next()) else {
        panic!("the channel opens");
    };
    let Some(Output::Ready(sender)) = state.update(Event::Command(ready)) else {
        panic!("Ready hands the sender back");
    };
    let outputs = interact(&mut state, |ui| {
        let _ = ui.typewrite("report");
    });
    let query = outputs
        .iter()
        .rev()
        .find_map(|output| match output {
            Output::Search(query) => Some(query.clone()),
            _ => None,
        })
        .expect("the palette asks for a search");

    let producer = thread::spawn(move || {
        let mut sender = sender;
        block_on(async {
            sender
                .send(results(
                    query.as_str(),
                    "Files",
                    [item("q3", "report-q3.pdf")],
                ))
                .await
                .expect("receiver is alive");
            sender
                .send(results("old query", "Files", [item("stale", "stale.pdf")]))
                .await
                .expect("receiver is alive");
        });
    });
    producer.join().expect("producer finished");
    let Some(batch) = block_on(stream.next()) else {
        panic!("a batch arrives");
    };
    let _ = state.update(Event::Command(batch));

    let mut ui = simulator(view(&state));
    assert!(ui.find("report-q3.pdf").is_ok());
    assert!(ui.find("stale.pdf").is_err(), "stale results are dropped");
}

#[test]
fn a_palette_without_messages_renders_disabled() {
    let state = opened();
    let element: Element<'_, Message> = command_palette(&state, text("Workspace")).into();
    let mut ui = simulator(element);
    ui.click("Settings").expect("row is rendered");
    assert!(ui.into_messages().next().is_none());
    let _ = command_palette::subscription::<&'static str>();
}
