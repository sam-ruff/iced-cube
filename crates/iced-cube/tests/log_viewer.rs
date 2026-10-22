#![cfg(feature = "log-viewer")]

use std::thread;

use futures::executor::block_on;
use futures::{SinkExt, StreamExt};
use iced::keyboard::{Key, Modifiers, key::Named};
use iced::widget::container;
use iced::{Element, Point, Size, keyboard, mouse, touch};
use iced_cube::data::log_viewer::{
    self, Entry, Event, Level, LineId, MenuItem, Output, ROW_HEIGHT, State, entry, log_viewer,
};
use iced_cube::overlay::context_menu;
use iced_cube::theme::space;
use iced_test::simulator::Simulator;

#[derive(Debug, Clone)]
enum Message {
    Logs(Event),
    Menu(context_menu::Event<MenuItem, LineId>),
}

fn sample() -> State {
    let mut state = State::new();
    state.extend([
        entry(Level::Info, "server started")
            .timestamp("09:00:00.000")
            .target("app"),
        entry(Level::Debug, "loaded config"),
        entry(Level::Warn, "disk almost full"),
        entry(Level::Error, "request failed: timeout"),
        entry(Level::Info, "request served"),
    ]);
    state
}

fn view(state: &State) -> Element<'_, Message> {
    log_viewer(state).height(360).on_event(Message::Logs).into()
}

fn sized(element: Element<'_, Message>, width: f32) -> Simulator<'_, Message> {
    Simulator::with_size(iced::Settings::default(), Size::new(width, 400.0), element)
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event> {
    ui.into_messages()
        .filter_map(|message| match message {
            Message::Logs(event) => Some(event),
            Message::Menu(_) => None,
        })
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

fn has(events: &[Event], expected: &str) -> bool {
    events.iter().any(|event| format!("{event:?}") == expected)
}

#[test]
fn renders_lines_with_their_parts_and_the_toolbar() {
    let state = sample();
    let mut ui = sized(view(&state), 720.0);
    for label in [
        "server started",
        "09:00:00.000",
        "app",
        "WARN",
        "ERROR",
        "Info 2",
        "Trace 0",
    ] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
}

#[test]
fn lines_that_do_not_wrap_are_equally_tall() {
    let state = sample();
    let mut ui = sized(view(&state), 720.0);
    let first = ui.find("loaded config").expect("line").bounds();
    let second = ui.find("disk almost full").expect("line").bounds();
    assert!((second.y - first.y - ROW_HEIGHT).abs() < 0.01);
}

#[test]
fn a_long_line_wraps_only_when_asked() {
    let long = "a long message that keeps going ".repeat(12);
    let mut state = State::new();
    state.extend([
        entry(Level::Info, long.clone()),
        entry(Level::Info, "after"),
    ]);
    {
        let mut ui = sized(view(&state), 480.0);
        let line = ui.find(long.as_str()).expect("line").bounds();
        assert!(line.height < 20.0, "one line: {line:?}");
    }
    let _ = state.update(Event::ToggleWrap);
    let mut ui = sized(view(&state), 480.0);
    let line = ui.find(long.as_str()).expect("line").bounds();
    let after = ui.find("after").expect("next line").bounds();
    assert!(line.height > 40.0, "wrapped: {line:?}");
    assert!(after.y >= line.y + line.height, "the next line sits below");
}

#[test]
fn a_match_among_wrapped_lines_scrolls_wholly_into_view() {
    let long = "a wrapped line that runs on and on ".repeat(6);
    let mut state = State::new().with_wrap(true);
    state.extend((0..300).map(|n| {
        let message = if n == 150 {
            format!("needle {long}")
        } else {
            format!("{n} {long}")
        };
        entry(Level::Info, message)
    }));
    state.extend([entry(Level::Info, "the newest line")]);
    let _ = state.update(Event::Search("needle".into()));
    let _ = state.update(Event::NextMatch);

    let mut ui = sized(view(&state), 480.0);
    let needle = format!("needle {long}");
    let line = ui
        .find(needle.as_str())
        .expect("the match is built")
        .bounds();
    assert!(line.height > 40.0, "it wraps: {line:?}");
    // The toolbar stacks below 560 pixels, and the lines start below it.
    let list = ui.find("1 of 1").expect("counter").bounds();
    assert!(line.y > list.y + list.height, "below the toolbar: {line:?}");
    assert!(line.y + line.height <= 400.0, "above the bottom: {line:?}");
}

#[test]
fn typing_in_the_search_field_searches() {
    let state = sample();
    let mut ui = sized(view(&state), 720.0);
    ui.click("Search messages").expect("search field");
    let _ = ui.typewrite("disk");
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Search(query) if query == "d")),
        "{emitted:?}"
    );
}

#[test]
fn enter_and_shift_enter_in_the_search_field_step_through_matches() {
    let mut state = sample();
    let _ = state.update(Event::Search("request".into()));
    for (modifiers, expected, other) in [
        (Modifiers::empty(), "NextMatch", "PreviousMatch"),
        (Modifiers::SHIFT, "PreviousMatch", "NextMatch"),
    ] {
        let mut ui = sized(view(&state), 720.0);
        ui.click("request").expect("search field shows the query");
        let _ = ui.simulate([press_key(Key::Named(Named::Enter), modifiers)]);
        let emitted = events(ui);
        assert!(has(&emitted, expected), "{emitted:?}");
        assert!(!has(&emitted, other), "{emitted:?}");
    }
}

#[test]
fn enter_on_the_lines_leaves_the_matches_alone() {
    let mut state = sample();
    let _ = state.update(Event::Search("request".into()));
    let mut ui = sized(view(&state), 720.0);
    ui.click("loaded config").expect("line");
    let _ = ui.simulate([
        press_key(Key::Named(Named::Enter), Modifiers::empty()),
        press_key(Key::Named(Named::F3), Modifiers::empty()),
    ]);
    let emitted = events(ui);
    let steps = emitted
        .iter()
        .filter(|event| matches!(event, Event::NextMatch))
        .count();
    assert_eq!(steps, 1, "only F3 steps: {emitted:?}");
}

#[test]
fn escape_in_the_search_field_clears_the_text_first() {
    let mut state = sample();
    let _ = state.update(Event::Search("request".into()));
    let mut ui = sized(view(&state), 720.0);
    ui.click("request").expect("search field");
    let _ = ui.simulate([press_key(Key::Named(Named::Escape), Modifiers::empty())]);
    let _ = ui.typewrite("x");
    let emitted = events(ui);
    assert!(has(&emitted, "ClearSearch"), "{emitted:?}");
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Search(query) if query.ends_with('x'))),
        "the field keeps focus: {emitted:?}"
    );
}

#[test]
fn escape_in_an_empty_search_field_leaves_it() {
    let state = sample();
    let mut ui = sized(view(&state), 720.0);
    ui.click("Search messages").expect("search field");
    let _ = ui.simulate([press_key(Key::Named(Named::Escape), Modifiers::empty())]);
    let _ = ui.typewrite("x");
    let emitted = events(ui);
    assert!(!has(&emitted, "ClearSearch"), "{emitted:?}");
    assert!(
        !emitted
            .iter()
            .any(|event| matches!(event, Event::Search(_))),
        "the field lost focus: {emitted:?}"
    );
}

#[test]
fn the_match_counter_shows_the_current_match() {
    let mut state = sample();
    let _ = state.update(Event::Search("request".into()));
    {
        let mut ui = sized(view(&state), 720.0);
        assert!(ui.find("2 found").is_ok());
    }
    let _ = state.update(Event::NextMatch);
    {
        let mut ui = sized(view(&state), 720.0);
        assert!(ui.find("1 of 2").is_ok());
    }
    let _ = state.update(Event::Search("nothing like this".into()));
    let mut ui = sized(view(&state), 720.0);
    assert!(ui.find("No matches").is_ok());
}

#[test]
fn the_toolbar_fits_a_phone_while_searching() {
    let mut state = sample();
    let _ = state.update(Event::Search("nothing like this".into()));
    let element: Element<'_, Message> = container(view(&state)).width(280).into();
    let mut ui = sized(element, 280.0);
    let field = ui.find("nothing like this").expect("field").bounds();
    assert!(field.width > 240.0, "the field takes the row: {field:?}");
    let counter = ui.find("No matches").expect("counter").bounds();
    assert!(counter.width <= 64.0, "{counter:?}");
    // The two step buttons, 32 pixels each, follow the counter.
    let right = counter.x + counter.width + 2.0 * (space::XS + 32.0);
    assert!(right <= 280.0, "the stepper runs off the edge: {right}");
}

#[test]
fn the_search_field_keeps_its_width_when_the_counter_appears() {
    let idle = sample();
    let mut ui = sized(view(&idle), 400.0);
    let before = ui.find("Search messages").expect("field").bounds();
    let mut searching = sample();
    let _ = searching.update(Event::Search("request".into()));
    let mut ui = sized(view(&searching), 400.0);
    let after = ui.find("request").expect("field").bounds();
    assert_eq!(before.width, after.width);
}

#[test]
fn level_filters_toggle_levels() {
    let state = sample();
    let mut ui = sized(view(&state), 720.0);
    ui.click("Debug 1").expect("level filter");
    let emitted = events(ui);
    assert!(has(&emitted, "Levels(Toggle(Debug))"), "{emitted:?}");
}

#[test]
fn a_right_click_opens_the_context_menu_on_that_line() {
    let state = sample();
    let menu = log_viewer::menu();
    let element: Element<'_, Message> = log_viewer(&state)
        .height(360)
        .context_menu(&menu, Message::Menu)
        .on_event(Message::Logs)
        .into();
    let mut ui = sized(element, 720.0);
    let line = ui.find("disk almost full").expect("line").bounds();
    ui.point_at(line.center());
    let _ = ui.simulate([
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
    ]);
    let opened: Vec<LineId> = ui
        .into_messages()
        .filter_map(|message| match message {
            Message::Menu(context_menu::Event::Open(id, _)) => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(opened, [2]);
}

#[test]
fn shift_f10_opens_the_menu_on_the_highlighted_line() {
    let mut state = sample();
    let _ = state.update(Event::Press(3, Modifiers::empty()));
    let _ = state.update(Event::Focus(true));
    let menu = log_viewer::menu();
    let element: Element<'_, Message> = log_viewer(&state)
        .height(360)
        .context_menu(&menu, Message::Menu)
        .on_event(Message::Logs)
        .into();
    let mut ui = sized(element, 720.0);
    ui.click("request failed: timeout").expect("line");
    let _ = ui.simulate([press_key(Key::Named(Named::F10), Modifiers::SHIFT)]);
    let opened = ui.into_messages().any(|message| {
        matches!(
            message,
            Message::Menu(context_menu::Event::OpenFromKeyboard(3))
        )
    });
    assert!(opened);
}

#[test]
fn the_resume_button_leaves_the_newest_line_in_view() {
    let mut state = State::new();
    state.extend((0..200).map(|n| entry(Level::Info, format!("line {n}"))));
    let _ = state.update(Event::TogglePause);
    let mut ui = sized(view(&state), 720.0);
    let newest = ui.find("line 199").expect("newest line").bounds();
    let resume = ui.find("Resume").expect("resume button").bounds();
    assert!(
        newest.y + newest.height <= resume.y,
        "{newest:?} sits above {resume:?}"
    );
}

#[test]
fn hidden_levels_are_not_rendered() {
    let state = sample().with_levels([Level::Warn, Level::Error]);
    let mut ui = sized(view(&state), 720.0);
    assert!(ui.find("disk almost full").is_ok());
    assert!(ui.find("server started").is_err());
}

#[test]
fn clicking_a_line_focuses_the_log_and_presses_it() {
    let state = sample();
    let mut ui = sized(view(&state), 720.0);
    ui.click("disk almost full").expect("line");
    let emitted = events(ui);
    assert!(has(&emitted, "Focus(true)"), "{emitted:?}");
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Press(2, modifiers) if modifiers.is_empty())),
        "{emitted:?}"
    );
}

#[test]
fn keys_resolve_once_the_log_has_focus() {
    let mut state = sample();
    let _ = state.update(Event::Press(1, Modifiers::empty()));
    let mut ui = sized(view(&state), 720.0);
    let _ = ui.simulate([press_key(Key::Character("f".into()), Modifiers::COMMAND)]);
    assert!(!has(&events(ui), "FocusSearch"), "not focused yet");

    let mut ui = sized(view(&state), 720.0);
    ui.click("loaded config").expect("line");
    let _ = ui.simulate([
        press_key(Key::Character("f".into()), Modifiers::COMMAND),
        press_key(Key::Named(Named::End), Modifiers::empty()),
        press_key(Key::Named(Named::Space), Modifiers::empty()),
        press_key(Key::Character("w".into()), Modifiers::empty()),
        press_key(Key::Character("l".into()), Modifiers::CTRL),
        press_key(Key::Character("c".into()), Modifiers::COMMAND),
    ]);
    let emitted = events(ui);
    for expected in ["FocusSearch", "Last", "ToggleHighlighted", "Clear"] {
        assert!(has(&emitted, expected), "{expected} in {emitted:?}");
    }
    for unbound in ["TogglePause", "ToggleWrap"] {
        assert!(!has(&emitted, unbound), "{unbound} in {emitted:?}");
    }
}

#[test]
fn a_paused_log_offers_to_resume() {
    let mut state = sample();
    let _ = state.update(Event::TogglePause);
    let _ = state.update(Event::Received(vec![entry(Level::Info, "held")]));
    let mut ui = sized(view(&state), 720.0);
    assert!(ui.find("held").is_err(), "held back");
    ui.click("Resume (1 new)").expect("resume button");
    assert!(has(&events(ui), "TogglePause"));
}

#[test]
fn a_log_that_stopped_following_offers_to_jump_back() {
    let mut state = sample();
    {
        let mut ui = sized(view(&state), 720.0);
        assert!(ui.find("Jump to latest").is_err(), "following");
    }
    let _ = state.update(Event::Follow(false));
    let _ = state.update(Event::Received(vec![entry(Level::Info, "newer")]));
    let mut ui = sized(view(&state), 720.0);
    ui.click("Jump to latest (1 new)").expect("jump button");
    assert!(has(&events(ui), "Last"));
}

#[test]
fn scrolling_up_stops_following() {
    let mut state = State::new();
    state.extend((0..200).map(|n| entry(Level::Info, format!("line {n}"))));
    let mut ui = sized(view(&state), 720.0);
    assert!(ui.find("line 199").is_ok(), "follows the newest line");
    ui.point_at(Point::new(300.0, 250.0));
    let _ = ui.simulate([iced::Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: 200.0 },
    })]);
    assert!(has(&events(ui), "Follow(false)"));
}

#[test]
fn a_finger_swipe_scrolls_a_following_log_without_pressing_a_line() {
    let mut state = State::new();
    state.extend((0..200).map(|n| entry(Level::Info, format!("line {n}"))));
    let mut ui = sized(view(&state), 720.0);
    let line = ui.find("line 195").expect("line").bounds().center();
    let finger = touch::Finger(1);
    ui.point_at(line);
    let _ = ui.simulate([iced::Event::Touch(touch::Event::FingerPressed {
        id: finger,
        position: line,
    })]);
    let to = Point::new(line.x, line.y + 150.0);
    ui.point_at(to);
    let _ = ui.simulate([
        iced::Event::Touch(touch::Event::FingerMoved {
            id: finger,
            position: to,
        }),
        iced::Event::Touch(touch::Event::FingerLifted {
            id: finger,
            position: to,
        }),
    ]);
    assert!(
        ui.find("line 184").is_ok(),
        "the lines stay where the finger left them"
    );
    let emitted = events(ui);
    assert!(has(&emitted, "Follow(false)"), "{emitted:?}");
    assert!(
        emitted
            .iter()
            .all(|event| !matches!(event, Event::Press(..))),
        "a swipe is not a tap: {emitted:?}"
    );
    for event in emitted {
        let _ = state.update(event);
    }
    assert!(!state.is_following());
}

#[test]
fn an_empty_log_says_so() {
    let state = State::new();
    let mut ui = sized(view(&state), 720.0);
    assert!(ui.find("No log lines yet").is_ok());
}

#[test]
fn a_log_without_messages_is_disabled() {
    let state = sample();
    let element: Element<'_, Message> = log_viewer(&state).height(360).into();
    let mut ui = sized(element, 720.0);
    ui.click("server started").expect("line");
    ui.click("Info 2").expect("filter");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn the_toolbar_stacks_and_fits_a_phone() {
    let state = sample();
    let element: Element<'_, Message> = container(view(&state)).width(328).into();
    let mut ui = sized(element, 328.0);
    for label in ["Search messages", "Trace 0", "Error 1", "WARN"] {
        let bounds = ui.find(label).expect(label).bounds();
        assert!(
            bounds.x >= 0.0 && bounds.x + bounds.width <= 328.0,
            "{label} runs off the edge: {bounds:?}"
        );
    }
    for label in ["Trace 0", "Error 1"] {
        let bounds = ui.find(label).expect(label).bounds();
        assert!(bounds.height < 24.0, "{label} wraps: {bounds:?}");
    }
}

#[test]
fn a_producer_thread_sends_entries_through_the_channel() {
    let mut events = Box::pin(log_viewer::stream());
    let Some(Event::Ready(sender)) = block_on(events.next()) else {
        panic!("first event must be Ready");
    };
    let mut state = State::new().with_limit(500);
    let producer = thread::spawn(move || {
        let mut sender = sender;
        block_on(async {
            for n in 0..2000 {
                let line = entry(Level::Info, format!("line {n}"));
                sender.send(line).await.expect("channel open");
            }
        });
    });
    let mut received = 0;
    while received < 2000 {
        let Some(event) = block_on(events.next()) else {
            break;
        };
        if let Event::Received(batch) = &event {
            assert!(batch.len() <= log_viewer::BATCH_SIZE);
            received += batch.len();
        }
        if let Some(Output::Ready(_)) = state.update(event) {
            panic!("ready only once");
        }
    }
    producer.join().expect("producer finished");
    assert_eq!(state.len(), 500, "bounded by the limit");
    let newest: Option<&Entry> = state.entries().last();
    assert_eq!(
        newest.map(|entry| entry.message.as_str()),
        Some("line 1999")
    );

    let mut ui = sized(view(&state), 720.0);
    assert!(ui.find("line 1999").is_ok());
    assert!(
        ui.find("line 1500").is_err(),
        "only the lines in view are built"
    );
}
