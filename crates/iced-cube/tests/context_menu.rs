#![cfg(feature = "context-menu")]

use iced::keyboard::{Key, Modifiers, key::Named};
use iced::widget::{column, container, text};
use iced::{Element, Point, mouse};
use iced_cube::keys;
use iced_cube::overlay::context_menu::{Event, State, context_menu, default_keymap, keyed};
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
    let _ = state.update(Event::Open((), position));
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
    assert_eq!(messages(ui), vec![Event::Open((), Point::new(50.0, 40.0))]);

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
    let _ = state.update(Event::OpenFromKeyboard(()));
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
            Event::Open((), Point::new(280.0, 180.0)),
        ]
    );
    for event in emitted {
        let _ = state.update(event);
    }
    assert!(state.is_open());
    assert_eq!(state.position(), Some(Point::new(280.0, 180.0)));
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
fn shift_f10_opens_below_the_area_on_the_first_item_and_navigates() {
    let mut state = state();
    let keymap = default_keymap();
    let key = |key: Key, modifiers: Modifiers| keys::Event { key, modifiers };

    let open = key(Key::Named(Named::F10), Modifiers::SHIFT);
    let event = state
        .key_event(&keymap, &open, ())
        .expect("Shift+F10 is bound");
    let _ = state.update(event);
    assert!(state.is_open());
    assert_eq!(state.position(), None);
    assert_eq!(state.menu().highlighted(), Some(BACK));

    {
        let mut ui = simulator::simulator(view(&state));
        let label = ui.find("Right click here").expect("area is rendered");
        let back = ui.find("Back").expect("menu is open");
        assert!(
            back.bounds().x + back.bounds().width <= 300.0,
            "under the area: {:?}",
            back.bounds()
        );
        assert!(
            back.bounds().y > 200.0 && back.bounds().y > label.bounds().y + label.bounds().height,
            "below the area, clear of its label"
        );
    }

    let down = key(Key::Named(Named::ArrowDown), Modifiers::empty());
    let event = state
        .key_event(&keymap, &down, ())
        .expect("ArrowDown is bound");
    let _ = state.update(event);
    assert_eq!(state.menu().highlighted(), Some(RELOAD));

    let enter = key(Key::Named(Named::Enter), Modifiers::empty());
    let event = state
        .key_event(&keymap, &enter, ())
        .expect("Enter is bound");
    assert_eq!(state.update(event), Some(Output::Activated(RELOAD)));
}

#[test]
fn the_menu_key_opens_too() {
    let mut state = state();
    let menu = keys::Event {
        key: Key::Named(Named::ContextMenu),
        modifiers: Modifiers::empty(),
    };
    let event = state.key_event(&default_keymap(), &menu, ());
    assert_eq!(event, Some(Event::OpenFromKeyboard(())));
    let _ = state.update(Event::OpenFromKeyboard(()));
    assert!(state.is_open());
}

#[derive(Debug, Clone, PartialEq)]
enum RowMessage {
    Menu(Event<u8, usize>),
}

const ROWS: [&str; 3] = ["alpha.txt", "beta.txt", "gamma.txt"];

fn rows_view(state: &State<u8, usize>) -> Element<'_, RowMessage> {
    let rows = ROWS.iter().enumerate().map(|(index, name)| {
        keyed(state, index, container(text(*name)).width(300).height(32))
            .on_event(RowMessage::Menu)
            .into()
    });
    column(rows).into()
}

fn row_messages(ui: Simulator<'_, RowMessage>) -> Vec<Event<u8, usize>> {
    ui.into_messages()
        .map(|RowMessage::Menu(event)| event)
        .collect()
}

#[test]
fn a_right_click_carries_the_row_it_came_from() {
    let state: State<u8, usize> = State::new([item(BACK, "Back"), item(RELOAD, "Reload")]);
    let mut ui = simulator::simulator(rows_view(&state));
    ui.point_at(Point::new(40.0, 48.0));
    let _ = ui.simulate([
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
    ]);
    assert_eq!(
        row_messages(ui),
        vec![Event::Open(1, Point::new(40.0, 16.0))]
    );
}

#[test]
fn a_shared_menu_shows_once_below_the_row_it_opened_on() {
    let mut state: State<u8, usize> = State::new([item(BACK, "Back"), item(RELOAD, "Reload")]);
    let _ = state.update(Event::OpenFromKeyboard(1));
    let mut ui = simulator::simulator(rows_view(&state));
    let row = ui.find("beta.txt").expect("row is rendered");
    let row_bottom = row.bounds().y + row.bounds().height;
    let back = ui.find("Back").expect("menu is open on the second row");
    assert!(back.bounds().y > row_bottom, "below the row's label");
    assert!(back.bounds().y < row_bottom + 40.0, "right under the row");

    let _ = ui.tap_key(Key::Named(Named::Escape));
    assert_eq!(
        row_messages(ui),
        vec![Event::Menu(dropdown_menu::Event::Close)],
        "only the row with the open menu answers the key"
    );
}
