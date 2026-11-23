#![cfg(feature = "multi-combobox")]

use iced::keyboard::key::Named;
use iced::widget::column;
use iced::{Element, Point, widget};
use iced_cube::forms::multi_combobox::{Event, Output, State, multi_combobox, remove_id};
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Guests(Event),
}

const PEOPLE: [&str; 6] = ["Ada", "Alan", "Grace", "Katherine", "Linus", "Margaret"];
const KEY: &str = "guests";

fn field_id() -> widget::Id {
    widget::Id::new(KEY)
}

fn view<'a>(state: &'a State<&'static str>) -> Element<'a, Message> {
    multi_combobox(state)
        .placeholder("Add guests...")
        .empty("Nobody found.")
        .id(KEY)
        .on_event(Message::Guests)
        .into()
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event> {
    ui.into_messages()
        .map(|Message::Guests(event)| event)
        .collect()
}

fn apply(state: &mut State<&'static str>, events: Vec<Event>) -> Vec<Output<&'static str>> {
    events
        .into_iter()
        .filter_map(|event| state.update(event))
        .collect()
}

fn chosen(state: &State<&'static str>) -> Vec<&'static str> {
    state.chosen().copied().collect()
}

#[test]
fn clicking_the_field_opens_the_list() {
    let mut state = State::new(PEOPLE);
    let mut ui = simulator(view(&state));
    assert!(ui.find("Grace").is_err());
    ui.click(field_id()).expect("field is rendered");

    let events = events(ui);
    assert_eq!(events, [Event::Open]);
    let _ = apply(&mut state, events);

    let mut ui = simulator(view(&state));
    for person in PEOPLE {
        assert!(ui.find(person).is_ok(), "{person}");
    }
}

#[test]
fn typing_filters_the_list() {
    let mut state = State::new(PEOPLE);
    let mut ui = simulator(view(&state));
    ui.click(field_id()).expect("field is rendered");
    let _ = ui.typewrite("al");

    let events = events(ui);
    assert_eq!(events.first(), Some(&Event::Open));
    assert_eq!(events.last(), Some(&Event::Input("al".into())));
    let _ = apply(&mut state, events);

    let mut ui = simulator(view(&state));
    assert!(ui.find("Alan").is_ok());
    assert!(ui.find("Grace").is_err());
}

#[test]
fn clicking_a_suggestion_adds_a_chip_and_keeps_the_list_open() {
    let mut state = State::new(PEOPLE);
    let _ = state.update(Event::Open);

    let mut ui = simulator(view(&state));
    ui.click("Grace").expect("suggestion is rendered");
    let events = events(ui);
    assert!(events.contains(&Event::Activate(2)), "{events:?}");
    assert!(!events.contains(&Event::Close), "{events:?}");
    assert_eq!(apply(&mut state, events), [Output::Added("Grace")]);
    assert!(state.is_open());

    let mut ui = simulator(view(&state));
    assert!(ui.find(remove_id(KEY, 2)).is_ok(), "Grace has a chip");
    assert!(ui.find("Linus").is_ok(), "the list is still open");
}

#[test]
fn chips_sit_above_the_field_and_the_list_below_it() {
    let mut state = State::new(PEOPLE).with_chosen(&["Ada"]);
    let _ = state.update(Event::Open);
    let mut ui = simulator(view(&state));
    let field = ui.find(field_id()).expect("field").bounds();
    let chip = ui.find(remove_id(KEY, 0)).expect("chip").bounds();
    let row = ui.find("Linus").expect("list").bounds();
    assert!(chip.y + chip.height <= field.y);
    assert!(row.y >= field.y + field.height);
}

#[test]
fn arrows_and_enter_add_and_remove() {
    let mut state = State::new(PEOPLE);
    let _ = state.update(Event::Open);

    let mut ui = simulator(view(&state));
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);
    let added = events(ui);
    assert_eq!(added, [Event::Next, Event::ActivateHighlighted]);
    assert_eq!(apply(&mut state, added), [Output::Added("Alan")]);
    assert!(state.is_open());

    let mut ui = simulator(view(&state));
    let _ = ui.tap_key(Named::Enter);
    let removed = events(ui);
    assert_eq!(apply(&mut state, removed), [Output::Removed("Alan")]);
    assert!(chosen(&state).is_empty());
}

#[test]
fn backspace_in_an_empty_field_removes_the_last_chip() {
    let mut state = State::new(PEOPLE).with_chosen(&["Linus", "Ada"]);
    let mut ui = simulator(view(&state));
    ui.click(field_id()).expect("field is rendered");
    let _ = ui.tap_key(Named::Backspace);

    let events = events(ui);
    assert_eq!(events, [Event::Open, Event::RemoveLast]);
    assert_eq!(apply(&mut state, events), [Output::Removed("Ada")]);
    assert_eq!(chosen(&state), ["Linus"]);
}

#[test]
fn backspace_with_a_query_edits_the_query() {
    let mut state = State::new(PEOPLE).with_chosen(&["Ada"]);
    let _ = state.update(Event::Input("Li".into()));
    let mut ui = simulator(view(&state));
    ui.click(field_id()).expect("field is rendered");
    let _ = ui.tap_key(Named::Backspace);

    let events = events(ui);
    assert!(!events.contains(&Event::RemoveLast), "{events:?}");
    assert!(events.contains(&Event::Input("L".into())), "{events:?}");
}

#[test]
fn clicking_a_chips_x_removes_it_without_opening_the_list() {
    let mut state = State::new(PEOPLE).with_chosen(&["Ada", "Grace"]);
    let mut ui = simulator(view(&state));
    ui.click(remove_id(KEY, 0)).expect("x is rendered");

    let events = events(ui);
    assert_eq!(events, [Event::Remove(0)]);
    assert_eq!(apply(&mut state, events), [Output::Removed("Ada")]);

    let mut ui = simulator(view(&state));
    assert!(ui.find(remove_id(KEY, 0)).is_err());
    assert!(ui.find(remove_id(KEY, 2)).is_ok());
}

#[test]
fn clicking_a_chips_x_while_open_removes_it() {
    let mut state = State::new(PEOPLE).with_chosen(&["Ada", "Grace"]);
    let _ = state.update(Event::Open);
    let mut ui = simulator(view(&state));
    ui.click(remove_id(KEY, 2)).expect("x is rendered");

    let events = events(ui);
    assert!(events.contains(&Event::Remove(2)), "{events:?}");
    let _ = apply(&mut state, events);
    assert_eq!(chosen(&state), ["Ada"]);
}

#[test]
fn escape_closes_the_list() {
    let mut state = State::new(PEOPLE).with_chosen(&["Ada"]);
    let _ = state.update(Event::Input("gr".into()));

    let mut ui = simulator(view(&state));
    let _ = ui.tap_key(Named::Escape);

    let events = events(ui);
    assert_eq!(events, [Event::Close]);
    let _ = apply(&mut state, events);
    assert!(!state.is_open());
    assert_eq!(state.query(), "");
    assert_eq!(chosen(&state), ["Ada"]);
}

#[test]
fn clicking_outside_closes_the_list() {
    let mut state = State::new(PEOPLE);
    let _ = state.update(Event::Open);

    let mut ui = simulator(view(&state));
    ui.point_at(Point::new(700.0, 580.0));
    let _ = ui.simulate(click());

    assert_eq!(events(ui), [Event::Close]);
}

#[test]
fn keys_pass_through_while_closed_and_unfocused() {
    let state = State::new(PEOPLE).with_chosen(&["Ada"]);
    let mut ui = simulator(view(&state));
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Backspace);
    assert!(events(ui).is_empty());
}

#[test]
fn a_query_with_no_match_shows_the_empty_message() {
    let mut state = State::new(PEOPLE);
    let _ = state.update(Event::Input("zz".into()));
    let mut ui = simulator(view(&state));
    assert!(ui.find("Nobody found.").is_ok());
}

#[test]
fn disabled_combobox_shows_chips_without_x_and_ignores_input() {
    let state = State::new(PEOPLE).with_chosen(&["Ada"]);
    let element: Element<'_, Message> = multi_combobox(&state).id(KEY).into();
    let mut ui = simulator(element);
    assert!(ui.find("Ada").is_ok());
    assert!(ui.find(remove_id(KEY, 0)).is_err());
    ui.click(field_id()).expect("field is rendered");
    let _ = ui.typewrite("a");
    let _ = ui.tap_key(Named::Backspace);
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn chips_wrap_in_a_narrow_container() {
    let state = State::new(PEOPLE).with_chosen(&PEOPLE);
    let element: Element<'_, Message> = multi_combobox(&state)
        .id(KEY)
        .width(iced::Length::Fill)
        .on_event(Message::Guests)
        .into();
    let mut ui = simulator(column![element].width(200));
    let field = ui.find(field_id()).expect("field").bounds();
    let mut lines: Vec<f32> = Vec::new();
    for index in 0..PEOPLE.len() {
        let x = ui.find(remove_id(KEY, index)).expect("chip").bounds();
        assert!(x.x + x.width <= 200.0, "chip {index} fits across");
        assert!(x.y + x.height <= field.y, "chip {index} above the field");
        if !lines.iter().any(|y| (y - x.y).abs() < 0.5) {
            lines.push(x.y);
        }
    }
    assert!(lines.len() > 1, "wrapped onto {} lines", lines.len());
    assert_eq!(field.width, 200.0);
}
