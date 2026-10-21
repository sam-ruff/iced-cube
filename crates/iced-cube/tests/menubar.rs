#![cfg(feature = "menubar")]

use iced::keyboard::{Key, key::Named};
use iced::widget::{button, column, container, text};
use iced::{Element, Length, Point, event};
use iced_cube::dropdown_menu::{self, checkbox_item, item, separator, submenu};
use iced_cube::menubar::{Event, Output, State, menu, menubar};
use iced_test::simulator::{self, Simulator};

const NEW: u8 = 1;
const RECENT: u8 = 2;
const NOTES: u8 = 3;
const QUIT: u8 = 4;
const UNDO: u8 = 5;
const REDO: u8 = 6;
const GRID: u8 = 7;
const ABOUT: u8 = 8;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Menubar(Event<u8>),
    Other,
}

fn state() -> State<u8> {
    State::new([
        menu(
            "File",
            [
                item(NEW, "New file").shortcut("Ctrl+N"),
                submenu(RECENT, "Open recent", [item(NOTES, "notes.md")]),
                separator(),
                item(QUIT, "Quit"),
            ],
        ),
        menu(
            "Edit",
            [item(UNDO, "Undo").shortcut("Ctrl+Z"), item(REDO, "Redo")],
        ),
        menu("View", [checkbox_item(GRID, "Show grid", false)]),
        menu("Help", [item(ABOUT, "About")]),
    ])
}

fn view(state: &State<u8>) -> Element<'_, Message> {
    column![
        menubar(state).on_event(Message::Menubar),
        container(button(text("Other")).on_press(Message::Other))
            .padding([200, 0])
            .width(Length::Fill),
    ]
    .into()
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event<u8>> {
    ui.into_messages()
        .filter_map(|message| match message {
            Message::Menubar(event) => Some(event),
            Message::Other => None,
        })
        .collect()
}

fn apply(state: &mut State<u8>, events: Vec<Event<u8>>) -> Option<Output<u8>> {
    events
        .into_iter()
        .fold(None, |output, event| state.update(event).or(output))
}

/// Moves the pointer, sending the event mouse areas react to.
fn move_to(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
        position,
    })]);
}

/// Presses a key on the rendered bar and applies what it sends.
fn press(state: &mut State<u8>, key: Key) -> Option<Output<u8>> {
    let mut ui = simulator::simulator(view(state));
    let _ = ui.tap_key(key);
    let events = events(ui);
    apply(state, events)
}

#[test]
fn clicking_a_title_opens_its_menu_below_it() {
    let mut state = state();
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("New file").is_err());
    ui.click("File").expect("title is rendered");
    let emitted = events(ui);
    assert_eq!(emitted, vec![Event::Toggle(0)]);
    let _ = apply(&mut state, emitted);

    let mut ui = simulator::simulator(view(&state));
    let title = ui.find("File").expect("title").bounds();
    let row = ui.find("New file").expect("menu is open").bounds();
    assert!(row.y > title.y + title.height, "the menu opens below");
    assert!(ui.find("Ctrl+N").is_ok(), "shortcut hints show");
}

#[test]
fn moving_onto_another_title_switches_menus() {
    let mut state = state();
    let _ = state.update(Event::Toggle(0));
    let mut ui = simulator::simulator(view(&state));
    let edit = ui.find("Edit").expect("title").bounds();
    move_to(&mut ui, edit.center());
    let emitted = events(ui);
    assert_eq!(emitted, vec![Event::Hover(1)]);
    let _ = apply(&mut state, emitted);
    assert_eq!(state.open(), Some(1));

    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("Undo").is_ok());
    assert!(ui.find("New file").is_err());
}

#[test]
fn hovering_a_title_with_nothing_open_sends_nothing() {
    let state = state();
    let mut ui = simulator::simulator(view(&state));
    let edit = ui.find("Edit").expect("title").bounds();
    move_to(&mut ui, edit.center());
    assert!(events(ui).is_empty());
}

#[test]
fn clicking_an_item_activates_it_and_closes_the_bar() {
    let mut state = state();
    let _ = state.update(Event::Toggle(1));
    let mut ui = simulator::simulator(view(&state));
    ui.click("Redo").expect("item is rendered");
    let emitted = events(ui);
    let output = apply(&mut state, emitted);
    assert_eq!(output, Some(Output::Activated(REDO)));
    assert!(!state.is_open() && !state.is_focused());
}

#[test]
fn checkbox_items_toggle_through_the_bar() {
    let mut state = state();
    let _ = state.update(Event::Toggle(2));
    let mut ui = simulator::simulator(view(&state));
    ui.click("Show grid").expect("item is rendered");
    let emitted = events(ui);
    assert_eq!(
        apply(&mut state, emitted),
        Some(Output::Toggled(GRID, true))
    );
    assert!(state.is_checked(GRID));
}

#[test]
fn arrows_switch_menus_and_open_submenus_first() {
    let mut state = state();
    let _ = state.update(Event::OpenMenu(0));
    assert_eq!(state.highlighted(), Some(NEW));

    let _ = press(&mut state, Key::Named(Named::ArrowRight));
    assert_eq!(state.open(), Some(1), "Right moves to Edit");
    assert_eq!(state.highlighted(), Some(UNDO));
    let _ = press(&mut state, Key::Named(Named::ArrowLeft));
    assert_eq!(state.open(), Some(0));

    let _ = press(&mut state, Key::Named(Named::ArrowDown));
    assert_eq!(state.highlighted(), Some(RECENT));
    let _ = press(&mut state, Key::Named(Named::ArrowRight));
    assert_eq!(state.open(), Some(0), "Right opens the submenu instead");
    assert_eq!(state.highlighted(), Some(NOTES));
    let _ = press(&mut state, Key::Named(Named::ArrowLeft));
    assert_eq!(state.highlighted(), Some(RECENT));
    let _ = press(&mut state, Key::Named(Named::ArrowLeft));
    assert_eq!(state.open(), Some(3), "Left wraps to Help");
}

#[test]
fn enter_activates_and_escape_closes() {
    let mut state = state();
    let _ = state.update(Event::OpenMenu(1));
    let _ = press(&mut state, Key::Named(Named::ArrowDown));
    assert_eq!(
        press(&mut state, Key::Named(Named::Enter)),
        Some(Output::Activated(REDO))
    );

    let _ = state.update(Event::OpenMenu(1));
    let mut ui = simulator::simulator(view(&state));
    assert_eq!(
        ui.tap_key(Key::Named(Named::Escape)),
        event::Status::Captured
    );
    let emitted = events(ui);
    let _ = apply(&mut state, emitted);
    assert!(!state.is_open());
    assert_eq!(state.focused(), Some(1), "Escape returns to the title");
}

#[test]
fn clicking_outside_dismisses_and_goes_no_further() {
    let mut state = state();
    let _ = state.update(Event::Toggle(0));
    let mut ui = simulator::simulator(view(&state));
    ui.click("Other").expect("button below is rendered");
    let messages: Vec<Message> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Menubar(Event::Dismiss)]);
    let _ = state.update(Event::Dismiss);
    assert!(!state.is_open() && !state.is_focused());
}

#[test]
fn clicking_the_open_title_closes_it() {
    let mut state = state();
    let _ = state.update(Event::Toggle(0));
    let mut ui = simulator::simulator(view(&state));
    ui.click("File").expect("title is rendered");
    let emitted = events(ui);
    let _ = apply(&mut state, emitted);
    assert!(!state.is_open());
}

#[test]
fn a_closed_bar_claims_no_keys_itself() {
    let state = state();
    for key in [
        Named::ArrowDown,
        Named::ArrowRight,
        Named::F10,
        Named::Escape,
    ] {
        let mut ui = simulator::simulator(view(&state));
        assert_eq!(
            ui.tap_key(Key::Named(key)),
            event::Status::Ignored,
            "{key:?}"
        );
        assert!(events(ui).is_empty(), "{key:?}");
    }
}

#[test]
fn keys_routed_by_the_app_focus_move_and_open() {
    let mut state = state();
    let keymap = iced_cube::menubar::default_keymap();
    let menus = dropdown_menu::default_keymap();
    let key = |key: Key| iced_cube::keys::Event {
        key,
        modifiers: iced::keyboard::Modifiers::empty(),
    };
    for named in [
        Named::F10,
        Named::ArrowRight,
        Named::ArrowRight,
        Named::ArrowDown,
    ] {
        let event = state.key_event(&keymap, &menus, &key(Key::Named(named)));
        let _ = state.update(event.expect("the bar claims the key"));
    }
    assert_eq!(state.open(), Some(2));
    assert_eq!(state.highlighted(), Some(GRID));

    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("Show grid").is_ok());
}

#[test]
fn a_disabled_bar_renders_its_titles_and_sends_nothing() {
    let state = state();
    let element: Element<'_, Message> = menubar(&state).into();
    let mut ui = simulator::simulator(element);
    ui.click("File").expect("title is rendered");
    ui.point_at(Point::new(300.0, 300.0));
    assert!(ui.into_messages().next().is_none());
}
