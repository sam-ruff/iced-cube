#![cfg(feature = "tree")]

use std::thread;
use std::time::{Duration, Instant};

use futures::executor::block_on;
use futures::{SinkExt, StreamExt};
use iced::keyboard::{Key, Modifiers, key::Named};
use iced::touch::{self, Finger};
use iced::widget::{column, container, text};
use iced::{Element, Point, Rectangle, Size, keyboard, mouse};
use iced_cube::data::tree::{self, Event, Mode, Output, State, loaded, node, tree};
use iced_cube::overlay::context_menu;
use iced_cube::overlay::dropdown_menu::item;
use iced_test::simulator::{self, Simulator};

#[derive(Debug, Clone)]
enum Message {
    Tree(Event<u32>),
    Menu(context_menu::Event<u8, u32>),
}

fn files() -> State<u32> {
    State::new([
        node(1, "src").folder().children([
            node(2, "main.rs").file(),
            node(3, "lib.rs").file(),
            node(4, "old.rs").file().disabled(true),
        ]),
        node(5, "docs").folder().lazy(),
        node(6, "readme.md").file(),
    ])
    .with_expanded([1])
}

fn view(state: &State<u32>) -> Element<'_, Message> {
    tree(state).on_event(Message::Tree).into()
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event<u32>> {
    ui.into_messages()
        .filter_map(|message| match message {
            Message::Tree(event) => Some(event),
            Message::Menu(_) => None,
        })
        .collect()
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate(simulator::click());
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

/// Applies the messages a simulator produced, returning the outputs.
fn apply(events: Vec<Event<u32>>, state: &mut State<u32>) -> Vec<Output<u32>> {
    events
        .into_iter()
        .filter_map(|event| state.update(event))
        .collect()
}

#[test]
fn renders_expanded_children_and_the_loading_line() {
    let mut state = files();
    {
        let mut ui = simulator::simulator(view(&state));
        for label in ["src", "main.rs", "lib.rs", "old.rs", "docs", "readme.md"] {
            assert!(ui.find(label).is_ok(), "{label}");
        }
        assert!(ui.find("Loading...").is_err());
    }

    let _ = state.update(Event::Expand(5));
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("Loading...").is_ok());
}

#[test]
fn children_are_indented_under_their_parent() {
    let state = files();
    let mut ui = simulator::simulator(view(&state));
    let parent = ui.find("src").expect("root").bounds();
    let child = ui.find("main.rs").expect("child").bounds();
    assert_eq!(child.x - parent.x, tree::INDENT);
    assert_eq!(child.y - parent.y, tree::ROW_HEIGHT);
}

#[test]
fn clicking_a_row_focuses_the_tree_and_presses_it() {
    let state = files();
    let mut ui = simulator::simulator(view(&state));
    ui.click("lib.rs").expect("row is rendered");
    let emitted = events(ui);
    assert!(
        matches!(emitted.first(), Some(Event::Focus(true))),
        "{emitted:?}"
    );
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Press(3, modifiers) if modifiers.is_empty())),
        "{emitted:?}"
    );
}

#[test]
fn modifier_clicks_carry_their_modifiers() {
    let mut state = files().with_mode(Mode::Multiple);
    let mut ui = simulator::simulator(view(&state));
    let _ = ui.simulate([iced::Event::Keyboard(keyboard::Event::ModifiersChanged(
        Modifiers::COMMAND,
    ))]);
    ui.click("main.rs").expect("row");
    ui.click("lib.rs").expect("row");
    let outputs = apply(events(ui), &mut state);
    assert!(matches!(outputs.last(), Some(Output::Selected(ids)) if ids == &[2, 3]));
}

fn finger(ui: &mut Simulator<'_, Message>, from: Point, to: Point) {
    let id = Finger(1);
    ui.point_at(from);
    let _ = ui.simulate([iced::Event::Touch(touch::Event::FingerPressed {
        id,
        position: from,
    })]);
    ui.point_at(to);
    let _ = ui.simulate([
        iced::Event::Touch(touch::Event::FingerMoved { id, position: to }),
        iced::Event::Touch(touch::Event::FingerLifted { id, position: to }),
    ]);
}

#[test]
fn a_tap_presses_a_row_but_a_drag_does_not() {
    let state = files();
    let mut ui = simulator::simulator(view(&state));
    let row = ui.find("lib.rs").expect("row").bounds().center();
    finger(&mut ui, row, row);
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Press(3, _))),
        "{emitted:?}"
    );

    let mut ui = simulator::simulator(view(&state));
    let row = ui.find("lib.rs").expect("row").bounds().center();
    finger(&mut ui, row, Point::new(row.x, row.y + 40.0));
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .all(|event| !matches!(event, Event::Press(..))),
        "{emitted:?}"
    );
}

#[test]
fn a_finger_drag_scrolls_a_tall_tree() {
    let state = wide(1, 40).with_expanded([0]);
    let element: Element<'_, Message> = tree(&state).height(200).on_event(Message::Tree).into();
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(300.0, 200.0), element);
    assert!(ui.find("file 0.30").is_err());
    finger(&mut ui, Point::new(100.0, 180.0), Point::new(100.0, -800.0));
    assert!(
        ui.find("file 0.30").is_ok(),
        "rows further down are built once in view"
    );
}

#[test]
fn disabled_rows_ignore_clicks() {
    let state = files();
    let mut ui = simulator::simulator(view(&state));
    ui.click("old.rs").expect("disabled row is rendered");
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .all(|event| !matches!(event, Event::Press(..))),
        "{emitted:?}"
    );
}

#[test]
fn the_chevron_toggles_without_selecting() {
    let state = files();
    let mut ui = simulator::simulator(view(&state));
    let label = ui.find("src").expect("root").bounds();
    // The chevron sits one indent before the folder icon and its gap.
    let chevron = Point::new(label.x - 8.0 - 16.0 - tree::INDENT / 2.0, label.center_y());
    click_at(&mut ui, chevron);
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Toggle(1))),
        "{emitted:?}"
    );
    assert!(
        emitted
            .iter()
            .all(|event| !matches!(event, Event::Press(..)))
    );
}

#[test]
fn a_double_click_activates() {
    let mut state = files();
    let mut ui = simulator::simulator(view(&state));
    ui.click("readme.md").expect("row");
    ui.click("readme.md").expect("row");
    let outputs = apply(events(ui), &mut state);
    assert!(
        outputs
            .iter()
            .any(|output| matches!(output, Output::Activated(6))),
        "{outputs:?}"
    );
}

#[test]
fn keys_are_ignored_until_the_tree_has_focus() {
    let state = files();
    let mut ui = simulator::simulator(view(&state));
    let _ = ui.tap_key(Named::ArrowDown);
    assert!(events(ui).is_empty(), "an unfocused tree claims no keys");

    let mut ui = simulator::simulator(view(&state));
    ui.click("main.rs").expect("row");
    let _ = ui.tap_key(Named::ArrowDown);
    let emitted = events(ui);
    assert!(matches!(emitted.last(), Some(Event::Next)), "{emitted:?}");
}

#[test]
fn a_press_outside_takes_focus_away() {
    let state = files();
    let element: Element<'_, Message> =
        column![container(view(&state)).height(300), text("Somewhere else"),].into();
    let mut ui = simulator::simulator(element);
    ui.click("main.rs").expect("row");
    ui.click("Somewhere else").expect("text below");
    let _ = ui.tap_key(Named::ArrowDown);
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Focus(false))),
        "{emitted:?}"
    );
    assert!(!emitted.iter().any(|event| matches!(event, Event::Next)));
}

#[test]
fn keyboard_navigation_runs_end_to_end() {
    let mut state = files();
    let mut ui = simulator::simulator(view(&state));
    ui.click("src").expect("row");
    let _ = apply(events(ui), &mut state);
    assert_eq!(state.highlighted(), Some(1));
    assert!(state.is_focused());

    // Each key goes through a fresh view of the updated state, as in an app.
    // A fresh simulator starts unfocused, so each step first clicks the
    // highlighted row again, which leaves the highlight where it is.
    let steps: [(Key, Modifiers); 5] = [
        (Key::Named(Named::ArrowDown), Modifiers::empty()),
        (Key::Named(Named::ArrowLeft), Modifiers::empty()),
        (Key::Named(Named::ArrowLeft), Modifiers::empty()),
        (Key::Character("*".into()), Modifiers::SHIFT),
        (Key::Character("r".into()), Modifiers::empty()),
    ];
    let mut loads = Vec::new();
    for (key, modifiers) in steps {
        let label = state
            .highlighted()
            .and_then(|id| state.node(id))
            .map(|node| node.label.clone())
            .expect("a highlighted node");
        let mut ui = simulator::simulator(view(&state));
        ui.click(label.as_str()).expect("highlighted row");
        let _ = ui.simulate([press_key(key, modifiers)]);
        for output in apply(events(ui), &mut state) {
            if let Output::Load(ids) = output {
                loads.extend(ids);
            }
        }
    }
    assert!(state.is_expanded(1), "star expanded the siblings again");
    assert!(state.is_expanded(5));
    assert_eq!(loads, [5], "the lazy sibling started loading");
    assert_eq!(state.highlighted(), Some(6), "typeahead found readme.md");
}

#[test]
fn checkboxes_check_nodes_in_checkbox_mode() {
    let mut state = files().with_mode(Mode::Checkbox);
    let mut ui = simulator::simulator(view(&state));
    let label = ui.find("src").expect("root").bounds();
    // The checkbox comes first after the chevron.
    let checkbox = Point::new(label.x - 8.0 - 16.0 - 8.0 - 8.0, label.center_y());
    click_at(&mut ui, checkbox);
    let outputs = apply(events(ui), &mut state);
    assert!(
        matches!(outputs.last(), Some(Output::Checked(ids)) if ids == &[1, 2, 3]),
        "{outputs:?}"
    );
}

#[test]
fn refreshed_children_render_and_can_be_checked_after_a_partial_selection() {
    let mut state = files().with_mode(Mode::Checkbox);
    let mut ui = simulator::simulator(view(&state));
    ui.click("main.rs").expect("old row");
    let _ = apply(events(ui), &mut state);

    let output = state.update(Event::Received(vec![loaded(1, [node(7, "new.rs").file()])]));
    assert!(matches!(output, Some(Output::Checked(ids)) if ids.is_empty()));
    assert_eq!(
        state.check_state(1),
        iced_cube::primitives::checkbox::CheckState::Unchecked
    );

    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("main.rs").is_err());
    ui.click("new.rs").expect("new row");
    let outputs = apply(events(ui), &mut state);
    assert!(matches!(outputs.last(), Some(Output::Checked(ids)) if ids == &[1, 7]));
}

#[test]
fn a_right_click_opens_the_context_menu_on_that_node() {
    let files = files();
    let menu = context_menu::State::<u8, u32>::new([item(1, "Rename")]);
    let element: Element<'_, Message> = tree(&files)
        .context_menu(&menu, Message::Menu)
        .on_event(Message::Tree)
        .into();
    let mut ui = simulator::simulator(element);
    let row = ui.find("lib.rs").expect("row").bounds();
    ui.point_at(row.center());
    let _ = ui.simulate([
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
    ]);
    let opened: Vec<_> = ui
        .into_messages()
        .filter_map(|message| match message {
            Message::Menu(context_menu::Event::Open(id, _)) => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(opened, [3]);
}

#[test]
fn shift_f10_opens_the_menu_on_the_highlighted_node() {
    let mut files = files();
    let _ = files.update(Event::Press(2, Modifiers::empty()));
    let menu = context_menu::State::<u8, u32>::new([item(1, "Rename")]);
    let element: Element<'_, Message> = tree(&files)
        .context_menu(&menu, Message::Menu)
        .on_event(Message::Tree)
        .into();
    let mut ui = simulator::simulator(element);
    ui.click("main.rs").expect("row");
    let _ = ui.simulate([press_key(Key::Named(Named::F10), Modifiers::SHIFT)]);
    let opened = ui.into_messages().any(|message| {
        matches!(
            message,
            Message::Menu(context_menu::Event::OpenFromKeyboard(2))
        )
    });
    assert!(opened);
}

#[test]
fn long_labels_stay_on_one_line_in_a_narrow_tree() {
    let state = State::new([node(1, "A label much too long to fit in a narrow sidebar").file()]);
    let element: Element<'_, Message> = container(view(&state)).width(160).into();
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(360.0, 200.0), element);
    let label = ui
        .find("A label much too long to fit in a narrow sidebar")
        .expect("label is reported in full")
        .bounds();
    assert!(
        label.x + label.width <= 160.0,
        "ends at {}",
        label.x + label.width
    );
    assert!(label.height <= 20.0, "one line, got {}", label.height);
}

/// A tree of `roots` folders with `per` files each.
fn wide(roots: u32, per: u32) -> State<u32> {
    State::new((0..roots).map(|folder| {
        node(folder * 1000, format!("folder {folder}"))
            .folder()
            .children((1..=per).map(move |file| {
                node(folder * 1000 + file, format!("file {folder}.{file}")).file()
            }))
    }))
}

#[test]
fn five_thousand_nodes_only_build_the_rows_in_view() {
    let started = Instant::now();
    let collapsed = wide(50, 99);
    assert_eq!(collapsed.len(), 5000);
    let element: Element<'_, Message> = tree(&collapsed).height(400).on_event(Message::Tree).into();
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(400.0, 400.0), element);
    assert!(ui.find("folder 0").is_ok());
    assert!(ui.find("folder 49").is_err(), "beyond the view");

    let expanded = wide(50, 99).with_expanded((0..50).map(|folder| folder * 1000));
    assert_eq!(expanded.visible().len(), 5000);
    let element: Element<'_, Message> = tree(&expanded).height(400).on_event(Message::Tree).into();
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(400.0, 400.0), element);
    assert!(ui.find("file 0.3").is_ok());
    assert!(ui.find("file 3.1").is_err(), "rows far below are not built");
    let _ = ui.simulate([iced::Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -3200.0 },
    })]);

    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "building and scrolling took {elapsed:?}"
    );
}

#[test]
fn the_highlight_scrolls_into_view() {
    let mut state = wide(1, 60).with_expanded([0]);
    let _ = state.update(Event::Focus(true));
    let _ = state.update(Event::Press(59, Modifiers::empty()));
    let element: Element<'_, Message> = tree(&state).height(200).on_event(Message::Tree).into();
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(300.0, 200.0), element);
    let row = ui
        .find("file 0.59")
        .expect("the highlighted row is built")
        .bounds();
    let view = Rectangle::new(Point::ORIGIN, Size::new(300.0, 200.0));
    assert!(view.contains(row.center()), "row at {row:?}");
}

#[test]
fn a_producer_thread_sends_children_through_the_channel() {
    let mut events = Box::pin(tree::stream::<u32>());
    let Some(Event::Ready(sender)) = block_on(events.next()) else {
        panic!("the first event must be Ready");
    };
    let mut state = files();
    let loads = match state.update(Event::Expand(5)) {
        Some(Output::Load(ids)) => ids,
        other => panic!("expected a load, got {other:?}"),
    };

    let producer = thread::spawn(move || {
        let mut sender = sender;
        block_on(async {
            for parent in loads {
                sender
                    .send(loaded(parent, [node(50, "guide.md"), node(51, "api.md")]))
                    .await
                    .expect("receiver is alive");
            }
        });
    });
    producer.join().expect("producer finished");

    let Some(event) = block_on(events.next()) else {
        panic!("expected a batch");
    };
    assert!(matches!(&event, Event::Received(batch) if batch.len() == 1));
    let _ = state.update(event);
    assert!(!state.is_loading(5));
    assert_eq!(state.children(5), &[50, 51]);

    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("guide.md").is_ok());
}
