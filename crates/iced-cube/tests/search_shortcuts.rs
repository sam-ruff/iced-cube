#![cfg(all(feature = "command", feature = "combobox", feature = "json-viewer"))]

mod support;

use iced::Element;
use iced::keyboard::key::Named;
use iced_cube::data::json_viewer::{self, json_viewer};
use iced_cube::forms::combobox::{self, combobox};
use iced_cube::navigation::command::{self, command, group, item};
use iced_cube::primitives::input::{Shortcut, Shortcuts};
use support::{COMMAND, Harness, jump, letter};

type Search = command::State<&'static str>;
type SearchEvent = command::Event<&'static str>;

fn palette<'a>(state: &'a Search) -> Element<'a, SearchEvent> {
    command(state).id("search").on_event(|event| event).into()
}

fn search() -> Harness<Search, SearchEvent> {
    let state = command::State::new([group("Apps", [item("calendar", "Calendar")])]);
    let mut ui = Harness::new(state, palette, |state, event| {
        let _ = state.update(event);
    });
    ui.focus("search");
    ui
}

#[test]
fn the_command_search_undoes_and_deletes_words() {
    let mut ui = search();
    ui.type_text("open cal");
    ui.press(jump(), Named::Backspace);
    assert_eq!(ui.state.query(), "open ");
    ui.press(COMMAND, letter('z'));
    assert_eq!(ui.state.query(), "open cal");
    ui.press(COMMAND, letter('z'));
    assert_eq!(ui.state.query(), "open ");
}

fn fruit<'a>(state: &'a combobox::State<&'static str>) -> Element<'a, combobox::Event> {
    combobox(state).id("fruit").on_event(|event| event).into()
}

#[test]
fn the_combobox_query_undoes() {
    let state = combobox::State::new(["Apple", "Banana"]);
    let mut ui = Harness::new(state, fruit, |state, event| {
        let _ = state.update(event);
    });
    ui.focus("fruit");
    ui.type_text("ban ap");
    ui.press(COMMAND, letter('z'));
    assert_eq!(ui.state.query(), "ban ");
}

struct Viewer {
    state: json_viewer::State,
    shortcuts: Shortcuts,
}

fn viewer(viewer: &Viewer) -> Element<'_, json_viewer::Event> {
    json_viewer(&viewer.state)
        .search_bar(true)
        .shortcuts(viewer.shortcuts)
        .on_event(|event| event)
        .into()
}

fn json(shortcuts: Shortcuts) -> Harness<Viewer, json_viewer::Event> {
    let state = json_viewer::State::new(serde_json::json!({ "name": "Ada" }));
    let id = state.search_id().clone();
    let mut ui = Harness::new(Viewer { state, shortcuts }, viewer, |viewer, event| {
        let _ = viewer.state.update(event);
    });
    ui.focus(id);
    ui
}

#[test]
fn a_component_filter_follows_its_shortcuts() {
    let mut ui = json(Shortcuts::all());
    ui.type_text("ada lovelace");
    ui.press(COMMAND, letter('z'));
    assert_eq!(ui.state.state.query(), "ada ");

    let mut ui = json(Shortcuts::none());
    ui.type_text("ada lovelace");
    assert!(!ui.press(COMMAND, letter('z')), "the app hears Ctrl+Z");
    assert_eq!(ui.state.state.query(), "ada lovelace");

    let mut ui = json(Shortcuts::all().without(Shortcut::Undo));
    ui.type_text("ada lovelace");
    assert!(!ui.press(COMMAND, letter('z')));
    ui.press(jump(), Named::Backspace);
    assert_eq!(ui.state.state.query(), "ada ");
}
