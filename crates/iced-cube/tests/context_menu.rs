#![cfg(feature = "context-menu")]

use iced::keyboard::{Key, Modifiers, key::Named};
use iced::widget::{column, container, text};
use iced::{Element, Point, mouse};
use iced_cube::keys;
use iced_cube::overlay::context_menu::{Event, State, context_menu, default_keymap};
use iced_cube::overlay::dropdown_menu::{self, Output, item, separator};
use iced_test::simulator::{self, Simulator};

const BACK: u8 = 1;
const FORWARD: u8 = 2;
const RELOAD: u8 = 3;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Context(Event<u8>),
}

fn state() -> State<u8> {
    State::new([
        item(BACK, "Back"),
        item(FORWARD, "Forward").disabled(true),
        separator(),
        item(RELOAD, "Reload"),
    ])
}

fn opened_at(position: Point) -> State<u8> {
    let mut state = state();
    let _ = state.update(Event::Open(position));
    state
}

fn view(state: &State<u8>) -> Element<'_, Message> {
    column![
        context_menu(
            state,
            container(text("Right click here")).width(300).height(200)
        )
        .on_event(Message::Context),
        text("Below"),
    ]
    .into()
}

fn messages(ui: Simulator<'_, Message>) -> Vec<Event<u8>> {
    ui.into_messages()
        .map(|Message::Context(event)| event)
        .collect()
}

fn right_click(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate([
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
    ]);
}

fn left_click(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate(simulator::click());
}

#[test]
fn right_click_opens_at_the_pointer() {
    let closed = state();
    let mut ui = simulator::simulator(view(&closed));
    assert!(ui.find("Back").is_err());
    right_click(&mut ui, Point::new(50.0, 40.0));
    assert_eq!(messages(ui), vec![Event::Open(Point::new(50.0, 40.0))]);

    let open = opened_at(Point::new(50.0, 40.0));
    let mut ui = simulator::simulator(view(&open));
    let back = ui.find("Back").expect("menu is open");
    assert!(back.bounds().x > 50.0 && back.bounds().y > 40.0);
    assert!(back.bounds().x < 80.0 && back.bounds().y < 60.0);
}

#[test]
fn right_click_outside_the_area_does_nothing() {
    let state = state();
    let mut ui = simulator::simulator(view(&state));
    right_click(&mut ui, Point::new(600.0, 500.0));
    assert!(messages(ui).is_empty());
}

#[test]
fn left_click_does_not_open() {
    let state = state();
    let mut ui = simulator::simulator(view(&state));
    left_click(&mut ui, Point::new(50.0, 40.0));
    assert!(messages(ui).is_empty());
}

#[test]
fn clicking_an_item_activates_it() {
    let mut state = opened_at(Point::new(10.0, 10.0));
    let mut ui = simulator::simulator(view(&state));
    ui.click("Reload").expect("item is rendered");
    ui.click("Forward").expect("disabled item is rendered");
    let emitted = messages(ui);
    let activate = Event::Menu(dropdown_menu::Event::Activate(RELOAD));
    assert_eq!(emitted.last(), Some(&activate), "{emitted:?}");
    assert!(
        emitted
            .iter()
            .all(|event| !matches!(event, Event::Menu(dropdown_menu::Event::Activate(FORWARD))))
    );
    assert_eq!(state.update(activate), Some(Output::Activated(RELOAD)));
    assert!(!state.is_open());
}

#[test]
fn the_open_menu_resolves_its_own_keys() {
    let mut state = state();
    let _ = state.update(Event::OpenFromKeyboard);
    let mut ui = simulator::simulator(view(&state));
    let _ = ui.tap_key(Key::Named(Named::ArrowDown));
    let _ = ui.tap_key(Key::Character("b".into()));
    assert_eq!(
        messages(ui),
        vec![
            Event::Menu(dropdown_menu::Event::Next),
            Event::Menu(dropdown_menu::Event::Typeahead('b')),
        ]
    );

    let closed = self::state();
    let mut ui = simulator::simulator(view(&closed));
    let _ = ui.tap_key(Key::Named(Named::ArrowDown));
    assert!(messages(ui).is_empty(), "a closed menu claims no keys");
}

#[test]
fn escape_closes_the_menu() {
    let state = opened_at(Point::new(10.0, 10.0));
    let mut ui = simulator::simulator(view(&state));
    let _ = ui.tap_key(Key::Named(Named::Escape));
    assert_eq!(messages(ui), vec![Event::Menu(dropdown_menu::Event::Close)]);
}

#[test]
fn clicking_outside_or_elsewhere_in_the_area_closes_the_menu() {
    for position in [Point::new(900.0, 700.0), Point::new(280.0, 180.0)] {
        let state = opened_at(Point::new(10.0, 10.0));
        let mut ui = simulator::simulator(view(&state));
        left_click(&mut ui, position);
        assert_eq!(
            messages(ui),
            vec![Event::Menu(dropdown_menu::Event::Close)],
            "{position:?}"
        );
    }
}

#[test]
fn right_clicking_elsewhere_moves_the_menu() {
    let mut state = opened_at(Point::new(10.0, 10.0));
    let mut ui = simulator::simulator(view(&state));
    right_click(&mut ui, Point::new(280.0, 180.0));
    let emitted = messages(ui);
    assert_eq!(
        emitted,
        vec![
            Event::Menu(dropdown_menu::Event::Close),
            Event::Open(Point::new(280.0, 180.0)),
        ]
    );
    for event in emitted {
        let _ = state.update(event);
    }
    assert!(state.is_open());
    assert_eq!(state.position(), Point::new(280.0, 180.0));
}

#[test]
fn clicking_inside_the_menu_does_not_reach_the_area() {
    let state = opened_at(Point::new(10.0, 10.0));
    let mut ui = simulator::simulator(view(&state));
    let back = ui.find("Back").expect("menu is open");
    right_click(&mut ui, back.bounds().center());
    assert_eq!(
        messages(ui),
        vec![Event::Menu(dropdown_menu::Event::Highlight(BACK))],
        "the row under the pointer is highlighted, and the area does not reopen the menu"
    );
}

#[test]
fn shift_f10_opens_at_the_corner_on_the_first_item_and_navigates() {
    let mut state = state();
    let keymap = default_keymap();
    let key = |key: Key, modifiers: Modifiers| keys::Event { key, modifiers };

    let open = key(Key::Named(Named::F10), Modifiers::SHIFT);
    let event = state.key_event(&keymap, &open).expect("Shift+F10 is bound");
    let _ = state.update(event);
    assert!(state.is_open());
    assert_eq!(state.position(), Point::ORIGIN);
    assert_eq!(state.menu().highlighted(), Some(BACK));

    let back = simulator::simulator(view(&state))
        .find("Back")
        .expect("menu is open");
    assert!(back.bounds().x < 30.0 && back.bounds().y < 30.0);

    let down = key(Key::Named(Named::ArrowDown), Modifiers::empty());
    let event = state.key_event(&keymap, &down).expect("ArrowDown is bound");
    let _ = state.update(event);
    assert_eq!(state.menu().highlighted(), Some(RELOAD));

    let enter = key(Key::Named(Named::Enter), Modifiers::empty());
    let event = state.key_event(&keymap, &enter).expect("Enter is bound");
    assert_eq!(state.update(event), Some(Output::Activated(RELOAD)));
}

#[test]
fn the_menu_key_opens_too() {
    let mut state = state();
    let menu = keys::Event {
        key: Key::Named(Named::ContextMenu),
        modifiers: Modifiers::empty(),
    };
    let event = state.key_event(&default_keymap(), &menu);
    assert_eq!(event, Some(Event::OpenFromKeyboard));
    let _ = state.update(Event::OpenFromKeyboard);
    assert!(state.is_open());
}
