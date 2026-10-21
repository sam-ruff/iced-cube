#![cfg(feature = "json-viewer")]

use std::time::Instant;

use iced::keyboard::{Key, Modifiers, key::Named};
use iced::widget::{column, container, text};
use iced::{Element, Point, Size, keyboard, mouse};
use iced_cube::data::json_viewer::{self, Event, MenuItem, NodeId, Output, State, json_viewer};
use iced_cube::overlay::context_menu;
use iced_test::simulator::{self, Simulator};
use serde_json::{Value, json};

#[derive(Debug, Clone)]
enum Message {
    Viewer(Event),
    Menu(context_menu::Event<MenuItem, NodeId>),
}

const WIDTH: f32 = 400.0;

fn user() -> State {
    State::new(json!({
        "name": "Ada",
        "age": 36,
        "admin": true,
        "manager": null,
        "tags": ["ops", "billing"],
        "address": { "city": "London" }
    }))
}

fn view(state: &State) -> Element<'_, Message> {
    json_viewer(state).on_event(Message::Viewer).into()
}

fn sized(element: Element<'_, Message>) -> Simulator<'_, Message> {
    Simulator::with_size(iced::Settings::default(), Size::new(WIDTH, 400.0), element)
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event> {
    ui.into_messages()
        .filter_map(|message| match message {
            Message::Viewer(event) => Some(event),
            Message::Menu(_) => None,
        })
        .collect()
}

fn apply(events: Vec<Event>, state: &mut State) -> Vec<Output> {
    events
        .into_iter()
        .filter_map(|event| state.update(event))
        .collect()
}

fn press_key(key: Key, modifiers: Modifiers) -> iced::Event {
    iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}

fn node(state: &State, path: &str) -> NodeId {
    let mut stack = state.roots().to_vec();
    while let Some(id) = stack.pop() {
        if state.path(id) == path {
            return id;
        }
        stack.extend(state.children(id));
    }
    panic!("no node at {path}")
}

#[test]
fn renders_keys_values_and_summaries() {
    let state = user();
    let mut ui = simulator::simulator(view(&state));
    for label in [
        "name", "\"Ada\"", "36", "true", "null", "[2]", "{1}", "address",
    ] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
    assert!(ui.find("\"London\"").is_err(), "collapsed");
}

#[test]
fn members_are_indented_under_their_parent() {
    let mut state = user();
    let _ = state.update(Event::ExpandAll);
    let mut ui = simulator::simulator(view(&state));
    let parent = ui.find("address").expect("parent").bounds();
    let child = ui.find("city").expect("child").bounds();
    assert_eq!(child.x - parent.x, json_viewer::INDENT);
    assert_eq!(child.y - parent.y, json_viewer::ROW_HEIGHT);
    assert!(ui.find("0:").is_ok() && ui.find("\"billing\"").is_ok());
}

#[test]
fn clicking_a_row_focuses_the_viewer_and_presses_it() {
    let state = user();
    let mut ui = simulator::simulator(view(&state));
    ui.click("age").expect("row");
    let emitted = events(ui);
    assert!(
        matches!(emitted.first(), Some(Event::Focus(true))),
        "{emitted:?}"
    );
    let age = node(&state, "$.age");
    assert!(emitted.contains(&Event::Press(age)), "{emitted:?}");
}

#[test]
fn the_chevron_and_a_double_click_toggle() {
    let state = user();
    let tags = node(&state, "$.tags");
    let mut ui = simulator::simulator(view(&state));
    let row = ui.find("tags").expect("row").bounds();
    ui.point_at(Point::new(
        row.x - json_viewer::INDENT / 2.0,
        row.center_y(),
    ));
    let _ = ui.simulate(simulator::click());
    let emitted = events(ui);
    assert!(emitted.contains(&Event::Toggle(tags)), "{emitted:?}");
    assert!(
        !emitted.contains(&Event::Press(tags)),
        "the chevron only toggles"
    );

    let mut ui = simulator::simulator(view(&state));
    let row = ui.find("tags").expect("row").bounds();
    ui.point_at(row.center());
    let _ = ui.simulate(simulator::click());
    let _ = ui.simulate(simulator::click());
    let emitted = events(ui);
    assert!(emitted.contains(&Event::Toggle(tags)), "{emitted:?}");
}

#[test]
fn keys_are_ignored_until_the_viewer_has_focus() {
    let state = user();
    let mut ui = simulator::simulator(view(&state));
    let _ = ui.tap_key(Named::ArrowDown);
    assert!(events(ui).is_empty(), "an unfocused viewer claims no keys");
}

#[test]
fn a_press_outside_takes_focus_away() {
    let state = user();
    let element: Element<'_, Message> =
        column![container(view(&state)).height(260), text("Somewhere else")].into();
    let mut ui = simulator::simulator(element);
    ui.click("age").expect("row");
    ui.click("Somewhere else").expect("text below");
    let _ = ui.tap_key(Named::ArrowDown);
    let emitted = events(ui);
    assert!(emitted.contains(&Event::Focus(false)), "{emitted:?}");
    assert!(!emitted.contains(&Event::Next));
}

#[test]
fn keyboard_navigation_and_copying_run_end_to_end() {
    let mut state = user();
    let mut ui = simulator::simulator(view(&state));
    ui.click("admin").expect("row");
    let _ = apply(events(ui), &mut state);
    assert_eq!(state.highlighted(), Some(node(&state, "$.admin")));

    // Each key goes through a fresh view of the updated state, as in an app.
    // A fresh simulator starts unfocused, so each step first clicks the
    // highlighted row again.
    let steps: [(Key, Modifiers, &str); 5] = [
        (Key::Named(Named::ArrowDown), Modifiers::empty(), "manager"),
        (Key::Named(Named::ArrowDown), Modifiers::empty(), "tags"),
        (Key::Named(Named::ArrowRight), Modifiers::empty(), "tags"),
        (Key::Named(Named::ArrowRight), Modifiers::empty(), "0:"),
        (Key::Character("c".into()), Modifiers::COMMAND, "0:"),
    ];
    let mut outputs = Vec::new();
    for (key, modifiers, label) in steps {
        let mut ui = simulator::simulator(view(&state));
        ui.click(label).expect("highlighted row");
        let _ = ui.simulate([press_key(key, modifiers)]);
        outputs.extend(apply(events(ui), &mut state));
    }
    assert!(state.is_expanded(node(&state, "$.tags")));
    assert_eq!(state.highlighted(), Some(node(&state, "$.tags[0]")));
    assert_eq!(outputs, [Output::CopyValue("\"ops\"".into())]);

    let mut ui = simulator::simulator(view(&state));
    ui.click("0:").expect("row");
    let _ = ui.simulate([press_key(
        Key::Character("C".into()),
        Modifiers::COMMAND | Modifiers::SHIFT,
    )]);
    assert_eq!(
        apply(events(ui), &mut state),
        [Output::CopyPath("$.tags[0]".into())]
    );
}

#[test]
fn the_highlighted_row_has_copy_buttons() {
    let mut state = user();
    let address = node(&state, "$.address");
    let _ = state.update(Event::Press(address));
    let mut ui = sized(view(&state));
    let row = ui.find("address").expect("row").bounds();
    // Two 32 pixel buttons at the end of the row: copy value, then path.
    let right = WIDTH - 8.0;
    ui.point_at(Point::new(right - 16.0, row.center_y()));
    let _ = ui.simulate(simulator::click());
    ui.point_at(Point::new(right - 48.0, row.center_y()));
    let _ = ui.simulate(simulator::click());
    let outputs = apply(events(ui), &mut state);
    assert_eq!(
        outputs,
        [
            Output::CopyPath("$.address".into()),
            Output::CopyValue("{\n  \"city\": \"London\"\n}".into()),
        ]
    );
}

#[test]
fn a_right_click_opens_the_menu_and_its_items_map_to_events() {
    let state = user();
    let menu = json_viewer::menu();
    let element: Element<'_, Message> = json_viewer(&state)
        .context_menu(&menu, Message::Menu)
        .on_event(Message::Viewer)
        .into();
    let mut ui = simulator::simulator(element);
    let row = ui.find("tags").expect("row").bounds();
    ui.point_at(row.center());
    let _ = ui.simulate([
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
    ]);
    let opened: Vec<NodeId> = ui
        .into_messages()
        .filter_map(|message| match message {
            Message::Menu(context_menu::Event::Open(id, _)) => Some(id),
            _ => None,
        })
        .collect();
    let tags = node(&state, "$.tags");
    assert_eq!(opened, [tags]);

    let mut state = state;
    assert_eq!(
        state.update(MenuItem::CopyPath.event(tags)),
        Some(Output::CopyPath("$.tags".into()))
    );
}

#[test]
fn the_open_menu_lists_its_entries() {
    let state = user();
    let mut menu = json_viewer::menu();
    let _ = menu.update(context_menu::Event::OpenFromKeyboard(node(
        &state, "$.name",
    )));
    let element: Element<'_, Message> = json_viewer(&state)
        .context_menu(&menu, Message::Menu)
        .on_event(Message::Viewer)
        .into();
    let mut ui = simulator::simulator(element);
    for label in ["Copy value", "Copy path", "Expand all", "Collapse all"] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
}

#[test]
fn the_search_box_searches_and_steps_through_matches() {
    let mut state = user();
    let mut ui = simulator::simulator(json_viewer(&state).search(true).on_event(Message::Viewer));
    ui.click("Search keys and values...").expect("search box");
    ui.typewrite("ad");
    let _ = apply(events(ui), &mut state);
    assert_eq!(state.query(), "ad");
    assert_eq!(
        state.matches().len(),
        3,
        "Ada and the admin and address keys"
    );

    let mut ui = simulator::simulator(json_viewer(&state).search(true).on_event(Message::Viewer));
    assert!(ui.find("1 of 3").is_ok());
    ui.click("ad").expect("search box with its text");
    let _ = ui.tap_key(Named::Enter);
    let _ = apply(events(ui), &mut state);
    assert_eq!(state.current_match(), Some(state.matches()[1]));
}

#[test]
fn the_filter_leaves_only_matches_and_says_when_nothing_matches() {
    let mut state = user().with_filter(true);
    let _ = state.update(Event::Search("london".into()));
    {
        let mut ui = simulator::simulator(view(&state));
        assert!(ui.find("city").is_ok() && ui.find("address").is_ok());
        assert!(ui.find("name").is_err(), "filtered out");
    }

    let _ = state.update(Event::Search("zebra".into()));
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("Nothing matches \"zebra\"").is_ok());
}

#[test]
fn invalid_json_renders_the_error() {
    let state = State::parse("{\"a\": [1, 2}");
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("Invalid JSON").is_ok());
    assert!(
        ui.find("expected `,` or `]` at line 1, column 12").is_ok(),
        "{:?}",
        state.error()
    );
}

#[test]
fn without_a_message_nothing_responds() {
    let state = user();
    let mut ui = simulator::simulator(json_viewer::<Message>(&state));
    ui.click("age").expect("row");
    let _ = ui.tap_key(Named::ArrowDown);
    assert!(events(ui).is_empty());
}

#[test]
fn long_values_and_keys_stay_on_one_line_in_a_narrow_viewer() {
    let long = "a value much too long to fit in a narrow viewer on a phone";
    let key = "a key that is also far too long for its column";
    let state = State::new(json!({ key: long }));
    let element: Element<'_, Message> = container(view(&state)).width(240).into();
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(360.0, 200.0), element);
    let value = ui
        .find(format!("\"{long}\"").as_str())
        .expect("value is reported in full")
        .bounds();
    assert!(
        value.x + value.width <= 240.0,
        "ends at {}",
        value.x + value.width
    );
    assert!(value.height <= 20.0, "one line, got {}", value.height);
    let key = ui.find(key).expect("key is reported in full").bounds();
    assert!(key.width <= json_viewer::KEY_WIDTH, "{}", key.width);
    assert!(key.height <= 20.0);
}

#[test]
fn a_large_document_renders_only_what_is_in_view() {
    let orders: Vec<Value> = (0..3_000)
        .map(|index| json!({ "id": format!("order {index}"), "total": index, "paid": true }))
        .collect();
    let mut state = State::new(json!({ "orders": orders }));
    let _ = state.update(Event::ExpandAll);
    assert_eq!(state.visible().len(), 1 + 3_000 * 4);

    let started = Instant::now();
    let element: Element<'_, Message> = json_viewer(&state)
        .height(320)
        .on_event(Message::Viewer)
        .into();
    let mut ui = simulator::simulator(element);
    assert!(ui.find("\"order 0\"").is_ok());
    assert!(ui.find("\"order 2999\"").is_err(), "far out of view");
    assert!(started.elapsed().as_secs() < 5, "{:?}", started.elapsed());
}
