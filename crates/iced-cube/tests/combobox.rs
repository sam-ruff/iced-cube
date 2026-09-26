#![cfg(feature = "combobox")]

use iced::keyboard::key::Named;
use iced::{Element, Point, widget};
use iced_cube::forms::combobox::{Event, State, combobox};
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Fruit(Event),
}

const FRUITS: [&str; 5] = ["Apple", "Banana", "Blueberry", "Cherry", "Grape"];

fn field_id() -> widget::Id {
    widget::Id::new("fruit")
}

fn view<'a>(state: &'a State<&'static str>) -> Element<'a, Message> {
    combobox(state)
        .placeholder("Search fruit...")
        .empty("No fruit found.")
        .id(field_id())
        .on_event(Message::Fruit)
        .into()
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event> {
    ui.into_messages()
        .map(|Message::Fruit(event)| event)
        .collect()
}

fn apply(state: &mut State<&'static str>, events: Vec<Event>) -> Option<&'static str> {
    events
        .into_iter()
        .filter_map(|event| state.update(event))
        .last()
}

#[test]
fn clicking_the_field_opens_the_list() {
    let mut state = State::new(FRUITS);
    let mut ui = simulator(view(&state));
    assert!(ui.find("Banana").is_err());
    ui.click(field_id()).expect("field is rendered");

    let events = events(ui);
    assert_eq!(events, [Event::Open]);
    let _ = apply(&mut state, events);

    let mut ui = simulator(view(&state));
    for fruit in FRUITS {
        assert!(ui.find(fruit).is_ok(), "{fruit}");
    }
}

#[test]
fn typing_filters_the_list() {
    let mut state = State::new(FRUITS);
    let mut ui = simulator(view(&state));
    ui.click(field_id()).expect("field is rendered");
    let _ = ui.typewrite("b");

    let events = events(ui);
    assert_eq!(events, [Event::Open, Event::Input("b".into())]);
    let _ = apply(&mut state, events);

    let mut ui = simulator(view(&state));
    assert!(ui.find("Banana").is_ok());
    assert!(ui.find("Blueberry").is_ok());
    assert!(ui.find("Cherry").is_err());
}

#[test]
fn arrows_and_enter_choose_a_suggestion() {
    let mut state = State::new(FRUITS);
    let _ = state.update(Event::Open);

    let mut ui = simulator(view(&state));
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);

    let events = events(ui);
    assert_eq!(events, [Event::Next, Event::Next, Event::Confirm]);
    assert_eq!(apply(&mut state, events), Some("Blueberry"));
    assert!(!state.is_open());

    let mut ui = simulator(view(&state));
    assert!(ui.find("Blueberry").is_ok());
    assert!(ui.find("Banana").is_err());
}

#[test]
fn escape_closes_the_list() {
    let mut state = State::new(FRUITS);
    let _ = state.update(Event::Input("an".into()));

    let mut ui = simulator(view(&state));
    let _ = ui.tap_key(Named::Escape);

    let events = events(ui);
    assert_eq!(events, [Event::Close]);
    let _ = apply(&mut state, events);
    assert!(!state.is_open());
    assert_eq!(state.query(), "");
}

#[test]
fn clicking_outside_closes_the_list() {
    let mut state = State::new(FRUITS);
    let _ = state.update(Event::Open);

    let mut ui = simulator(view(&state));
    ui.point_at(Point::new(600.0, 600.0));
    let _ = ui.simulate(click());

    assert_eq!(events(ui), [Event::Close]);
}

#[test]
fn clicking_a_suggestion_chooses_it() {
    let mut state = State::new(FRUITS);
    let _ = state.update(Event::Open);

    let mut ui = simulator(view(&state));
    ui.click("Cherry").expect("suggestion is rendered");

    let events = events(ui);
    assert!(events.contains(&Event::Pick(3)), "{events:?}");
    assert_eq!(apply(&mut state, events), Some("Cherry"));
}

#[test]
fn a_query_with_no_match_shows_the_empty_message() {
    let mut state = State::new(FRUITS);
    let _ = state.update(Event::Input("kiwi".into()));

    let mut ui = simulator(view(&state));
    assert!(ui.find("No fruit found.").is_ok());
    assert!(ui.find("Apple").is_err());
}

#[test]
fn keys_pass_through_while_closed_and_unfocused() {
    let state = State::new(FRUITS);
    let mut ui = simulator(view(&state));
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);
    assert!(events(ui).is_empty());
}

#[test]
fn disabled_combobox_ignores_input() {
    let state = State::new(FRUITS).with_selected(&"Grape");
    let element: Element<'_, Message> = combobox(&state).id(field_id()).into();
    let mut ui = simulator(element);
    ui.click(field_id()).expect("field is rendered");
    let _ = ui.typewrite("a");
    let _ = ui.tap_key(Named::ArrowDown);
    assert_eq!(ui.into_messages().count(), 0);
}
