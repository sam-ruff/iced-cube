#![cfg(feature = "dropdown-menu")]

use iced::keyboard::{Key, key::Named};
use iced::widget::{button, column, container, text};
use iced::{Element, Length, Point, event};
use iced_cube::Chord;
use iced_cube::overlay::dropdown_menu::{
    Event, Output, State, checkbox_item, default_keymap, dropdown_menu, group_label, item,
    separator, submenu,
};
use iced_test::simulator::{self, Simulator};

const COPY: u8 = 1;
const PASTE: u8 = 2;
const SHARE: u8 = 3;
const EMAIL: u8 = 4;
const LINK: u8 = 5;
const GRID: u8 = 6;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Menu(Event<u8>),
    Other,
}

fn state() -> State<u8> {
    State::new([
        group_label("Edit"),
        item(COPY, "Copy").shortcut("Ctrl+C"),
        item(PASTE, "Paste").disabled(true),
        separator(),
        submenu(
            SHARE,
            "Share",
            [item(EMAIL, "Email"), item(LINK, "Copy link")],
        ),
        checkbox_item(GRID, "Show grid", false),
    ])
}

fn opened() -> State<u8> {
    let mut state = state();
    let _ = state.update(Event::Open);
    state
}

fn view(state: &State<u8>) -> Element<'_, Message> {
    column![
        dropdown_menu(
            state,
            button(text("Actions")).on_press(Message::Menu(Event::Toggle))
        )
        .on_event(Message::Menu),
        text("Page"),
        container(button(text("Other")).on_press(Message::Other))
            .padding([240, 0])
            .width(Length::Fill),
    ]
    .into()
}

fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
    ui.into_messages().collect()
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event<u8>> {
    ui.into_messages()
        .filter_map(|message| match message {
            Message::Menu(event) => Some(event),
            Message::Other => None,
        })
        .collect()
}

/// Presses a key on the rendered menu and applies what it sends.
fn press(state: &mut State<u8>, key: Key) -> Option<Output<u8>> {
    let mut ui = simulator::simulator(view(state));
    let _ = ui.tap_key(key);
    events(ui)
        .into_iter()
        .fold(None, |output, event| state.update(event).or(output))
}

#[test]
fn the_trigger_opens_the_menu() {
    let mut state = state();
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("Copy").is_err());
    ui.click("Actions").expect("trigger is rendered");
    let emitted = events(ui);
    assert_eq!(emitted, vec![Event::Toggle]);

    let _ = state.update(Event::Toggle);
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("Edit").is_ok());
    assert!(ui.find("Copy").is_ok());
    assert!(ui.find("Ctrl+C").is_ok());
}

#[test]
fn clicking_an_item_activates_it() {
    let mut state = opened();
    let mut ui = simulator::simulator(view(&state));
    ui.click("Copy").expect("item is rendered");
    let emitted = events(ui);
    assert_eq!(emitted.last(), Some(&Event::Activate(COPY)), "{emitted:?}");
    assert_eq!(
        state.update(Event::Activate(COPY)),
        Some(Output::Activated(COPY))
    );
    assert!(!state.is_open());
}

#[test]
fn disabled_items_and_labels_emit_nothing() {
    let state = opened();
    let mut ui = simulator::simulator(view(&state));
    ui.click("Paste").expect("disabled item is rendered");
    ui.click("Edit").expect("label is rendered");
    assert!(messages(ui).is_empty());
}

#[test]
fn the_open_menu_resolves_its_own_keys() {
    let mut state = opened();
    let _ = press(&mut state, Key::Named(Named::ArrowDown));
    assert_eq!(state.highlighted(), Some(COPY));

    let _ = press(&mut state, Key::Named(Named::ArrowDown));
    assert_eq!(state.highlighted(), Some(SHARE));
    let _ = press(&mut state, Key::Named(Named::End));
    assert_eq!(state.highlighted(), Some(GRID));
    let _ = press(&mut state, Key::Named(Named::Home));
    assert_eq!(state.highlighted(), Some(COPY));
    let _ = press(&mut state, Key::Character("s".into()));
    assert_eq!(state.highlighted(), Some(SHARE));

    let _ = press(&mut state, Key::Named(Named::ArrowRight));
    assert_eq!(state.highlighted(), Some(EMAIL));
    let _ = press(&mut state, Key::Named(Named::ArrowDown));
    assert_eq!(
        press(&mut state, Key::Named(Named::Enter)),
        Some(Output::Activated(LINK))
    );
    assert!(!state.is_open());
}

#[test]
fn a_closed_menu_claims_no_keys() {
    let state = state();
    for key in [Named::ArrowDown, Named::ArrowUp, Named::Enter, Named::Space] {
        let mut ui = simulator::simulator(view(&state));
        let status = ui.tap_key(Key::Named(key));
        assert_eq!(status, event::Status::Ignored, "{key:?}");
        assert!(messages(ui).is_empty(), "{key:?}");
    }
}

#[test]
fn a_custom_keymap_replaces_the_defaults() {
    let state = opened();
    let keymap = default_keymap().unbind(&Chord::named(Named::Escape)).bind(
        Chord::character('q'),
        iced_cube::dropdown_menu::Action::Close,
    );
    let view = || -> Element<'_, Message> {
        dropdown_menu(&state, text("Actions"))
            .keymap(keymap.clone())
            .on_event(Message::Menu)
            .into()
    };

    let mut ui = simulator::simulator(view());
    let status = ui.tap_key(Key::Named(Named::Escape));
    assert_eq!(status, event::Status::Ignored, "Escape is unbound");
    assert!(messages(ui).is_empty());

    let mut ui = simulator::simulator(view());
    let _ = ui.tap_key(Key::Character("q".into()));
    assert_eq!(events(ui), vec![Event::Close]);
}

#[test]
fn escape_closes_the_menu() {
    let mut state = opened();
    let mut ui = simulator::simulator(view(&state));
    let status = ui.tap_key(Key::Named(Named::Escape));
    assert_eq!(status, event::Status::Captured);
    assert_eq!(events(ui), vec![Event::Close]);

    let _ = press(&mut state, Key::Named(Named::Escape));
    assert!(!state.is_open());
}

#[test]
fn escape_in_a_submenu_closes_only_the_submenu() {
    let mut state = opened();
    let _ = state.update(Event::Highlight(SHARE));
    let _ = press(&mut state, Key::Named(Named::Escape));
    assert!(state.is_open());
    assert_eq!(state.depth(), 1);
    assert_eq!(state.highlighted(), Some(SHARE));
}

#[test]
fn clicking_outside_closes_the_menu() {
    let state = opened();
    let mut ui = simulator::simulator(view(&state));
    ui.point_at(Point::new(900.0, 700.0));
    let _ = ui.simulate(simulator::click());
    assert_eq!(messages(ui), vec![Message::Menu(Event::Close)]);
}

#[test]
fn the_outside_click_that_closes_the_menu_goes_no_further() {
    let state = opened();
    let mut ui = simulator::simulator(view(&state));
    ui.click("Other").expect("button below is rendered");
    assert_eq!(messages(ui), vec![Message::Menu(Event::Close)]);

    let closed = self::state();
    let mut ui = simulator::simulator(view(&closed));
    ui.click("Other").expect("button below is rendered");
    assert_eq!(messages(ui), vec![Message::Other]);
}

#[test]
fn clicking_the_trigger_while_open_only_toggles() {
    let state = opened();
    let mut ui = simulator::simulator(view(&state));
    ui.click("Actions").expect("trigger is rendered");
    assert_eq!(events(ui), vec![Event::Toggle]);
}

#[test]
fn submenu_items_are_rendered_beside_their_item_and_activate() {
    let mut state = opened();
    let _ = state.update(Event::Highlight(SHARE));
    let mut ui = simulator::simulator(view(&state));

    let share = ui.find("Share").expect("submenu item is rendered");
    let email = ui.find("Email").expect("submenu is open");
    assert!(email.bounds().x > share.bounds().x + share.bounds().width);

    ui.click("Copy link").expect("submenu item is rendered");
    let emitted = events(ui);
    assert_eq!(emitted.last(), Some(&Event::Activate(LINK)), "{emitted:?}");
    assert_eq!(
        state.update(Event::Activate(LINK)),
        Some(Output::Activated(LINK))
    );
}

#[test]
fn clicking_outside_with_a_submenu_open_closes_everything_once() {
    let mut state = opened();
    let _ = state.update(Event::Highlight(SHARE));
    let mut ui = simulator::simulator(view(&state));
    ui.point_at(Point::new(900.0, 700.0));
    let _ = ui.simulate(simulator::click());
    let emitted = events(ui);
    assert_eq!(emitted, vec![Event::Close]);
    for event in emitted {
        let _ = state.update(event);
    }
    assert!(!state.is_open());
}

#[test]
fn checkbox_items_toggle_through_the_state() {
    let mut state = opened();
    let mut ui = simulator::simulator(view(&state));
    ui.click("Show grid").expect("checkbox item is rendered");
    let emitted = events(ui);
    assert_eq!(emitted.last(), Some(&Event::Activate(GRID)), "{emitted:?}");
    assert_eq!(
        state.update(Event::Activate(GRID)),
        Some(Output::Toggled(GRID, true))
    );
}

#[test]
fn a_menu_without_on_event_is_inert() {
    let mut state = opened();
    let _ = state.update(Event::Next);
    let mut ui = simulator::simulator(Element::from(dropdown_menu::<u8, Message>(
        &state,
        text("Actions"),
    )));
    ui.click("Copy").expect("item is rendered");
    let _ = ui.tap_key(Key::Named(Named::Escape));
    assert!(messages(ui).is_empty());
}
