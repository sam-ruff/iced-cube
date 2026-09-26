#![cfg(feature = "command")]

use std::thread;

use futures::executor::block_on;
use futures::{SinkExt, StreamExt};
use iced::keyboard::key::Named;
use iced::{Element, widget};
use iced_cube::lucide;
use iced_cube::navigation::command::{self, Event, Output, State, command, group, item, results};
use iced_test::simulator::{Simulator, simulator};

#[derive(Debug, Clone)]
enum Message {
    Command(Event<&'static str>),
}

fn search_id() -> widget::Id {
    widget::Id::new("command-search")
}

fn sample() -> State<&'static str> {
    State::new([
        group(
            "Suggestions",
            [
                item("calendar", "Calendar").icon(lucide!(Calendar)),
                item("emoji", "Search emoji").icon(lucide!(Image)),
                item("calculator", "Calculator").disabled(true),
            ],
        ),
        group(
            "Settings",
            [
                item("profile", "Profile").shortcut("Ctrl+P"),
                item("billing", "Billing").shortcut("Ctrl+B"),
            ],
        ),
    ])
}

fn view<'a>(state: &'a State<&'static str>) -> Element<'a, Message> {
    command(state)
        .id(search_id())
        .on_event(Message::Command)
        .into()
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event<&'static str>> {
    ui.into_messages()
        .map(|Message::Command(event)| event)
        .collect()
}

fn run(state: &mut State<&'static str>, events: Vec<Event<&'static str>>) -> Vec<&'static str> {
    events
        .into_iter()
        .filter_map(|event| match state.update(event) {
            Some(Output::Activated(id)) => Some(id),
            _ => None,
        })
        .collect()
}

#[test]
fn renders_groups_icons_and_shortcuts() {
    let state = sample();
    let mut ui = simulator(view(&state));
    for text in ["Suggestions", "Settings", "Calendar", "Billing", "Ctrl+B"] {
        assert!(ui.find(text).is_ok(), "{text}");
    }
}

#[test]
fn typing_filters_the_list() {
    let mut state = sample();
    let mut ui = simulator(view(&state));
    ui.click(search_id()).expect("search field is rendered");
    let _ = ui.typewrite("b");

    let events = events(ui);
    assert!(
        matches!(events.as_slice(), [Event::Focus(true), Event::Input(query)] if query == "b"),
        "{events:?}"
    );
    let outputs: Vec<_> = events.into_iter().map(|e| state.update(e)).collect();
    assert!(matches!(outputs.as_slice(), [None, Some(Output::Search(query))] if query == "b"));

    let mut ui = simulator(view(&state));
    assert!(ui.find("Billing").is_ok());
    assert!(ui.find("Calendar").is_err());
    assert!(ui.find("Suggestions").is_err());
}

#[test]
fn arrows_and_enter_run_the_highlighted_item() {
    let mut state = sample();
    let mut ui = simulator(view(&state));
    ui.click(search_id()).expect("search field is rendered");
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);

    let events = events(ui);
    assert!(
        matches!(
            events.as_slice(),
            [
                Event::Focus(true),
                Event::Next,
                Event::Next,
                Event::ActivateHighlighted
            ]
        ),
        "{events:?}"
    );
    // The disabled calculator is skipped.
    assert_eq!(run(&mut state, events), ["profile"]);
}

#[test]
fn escape_clears_the_query() {
    let mut state = sample();
    let _ = state.update(Event::Input("bill".into()));

    let mut ui = simulator(view(&state));
    ui.click(search_id()).expect("search field is rendered");
    let _ = ui.tap_key(Named::Escape);

    let events = events(ui);
    assert!(
        matches!(events.as_slice(), [Event::Focus(true), Event::Close]),
        "{events:?}"
    );
    for event in events {
        let _ = state.update(event);
    }
    assert_eq!(state.query(), "");
    assert_eq!(state.len(), 5);
}

#[test]
fn clicking_an_item_runs_it() {
    let mut state = sample();
    let mut ui = simulator(view(&state));
    ui.click("Billing").expect("item is rendered");
    let events = events(ui);
    assert_eq!(run(&mut state, events), ["billing"]);
}

#[test]
fn disabled_items_do_not_run() {
    let mut state = sample();
    let mut ui = simulator(view(&state));
    ui.click("Calculator").expect("item is rendered");
    let events = events(ui);
    assert!(run(&mut state, events).is_empty());
}

#[test]
fn focusing_and_leaving_the_field_toggle_the_highlight() {
    let mut state = sample();
    let mut ui = simulator(view(&state));
    ui.click(search_id()).expect("search field is rendered");
    ui.point_at(iced::Point::new(900.0, 700.0));
    let _ = ui.simulate(iced_test::simulator::click());
    let events = events(ui);
    assert!(
        matches!(events.as_slice(), [Event::Focus(true), Event::Focus(false)]),
        "{events:?}"
    );

    assert!(!state.shows_highlight());
    let _ = state.update(Event::Focus(true));
    assert!(state.shows_highlight());
    let _ = state.update(Event::Focus(false));
    assert!(!state.shows_highlight());
}

#[test]
fn a_capped_list_shrinks_to_a_few_results() {
    use iced::widget::{column, text};

    let mut state = sample();
    let _ = state.update(Event::Input("bill".into()));
    let element: Element<'_, Message> = column![
        command(&state).max_height(400.0).on_event(Message::Command),
        text("Below"),
    ]
    .into();
    let mut ui = simulator(element);
    let billing = ui.find("Billing").expect("result is rendered").bounds();
    let below = ui.find("Below").expect("text under the list").bounds();
    assert!(
        below.y < billing.y + billing.height + 40.0,
        "the list ends just after its one result: {billing:?} {below:?}"
    );
}

#[test]
fn a_capped_list_stops_at_its_maximum() {
    use iced::widget::{column, text};

    let state = sample();
    let element: Element<'_, Message> = column![
        command(&state).max_height(120.0).on_event(Message::Command),
        text("Below"),
    ]
    .into();
    let mut ui = simulator(element);
    let below = ui.find("Below").expect("text under the list").bounds();
    assert!(below.y <= 120.5, "{below:?}");
}

#[test]
fn keys_pass_through_without_focus() {
    let state = sample();
    let mut ui = simulator(view(&state));
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);
    assert!(events(ui).is_empty());
}

#[test]
fn empty_and_loading_messages() {
    let mut state = sample();
    let _ = state.update(Event::Input("zzz".into()));

    let element: Element<'_, Message> = command(&state)
        .empty("Nothing here.")
        .on_event(Message::Command)
        .into();
    let mut ui = simulator(element);
    assert!(ui.find("Nothing here.").is_ok());

    let element: Element<'_, Message> = command(&state)
        .loading(true)
        .on_event(Message::Command)
        .into();
    let mut ui = simulator(element);
    assert!(ui.find(command::SEARCHING).is_ok());
}

#[test]
fn producer_thread_streams_results_and_stale_ones_are_dropped() {
    let mut events = Box::pin(command::stream::<&'static str>());
    let Some(Event::Ready(sender)) = block_on(events.next()) else {
        panic!("the first event must be Ready");
    };

    let mut state = sample();
    let _ = state.update(Event::Input("rep".into()));

    let producer = thread::spawn(move || {
        let mut sender = sender;
        block_on(async {
            sender
                .send(results("re", "Files", [item("stale", "readme.md")]))
                .await
                .expect("receiver is alive");
            for name in ["report.pdf", "reports.csv", "repo.zip"] {
                sender
                    .send(results("rep", "Files", [item(name, name)]))
                    .await
                    .expect("receiver is alive");
            }
        });
    });
    producer.join().expect("producer finished");

    let mut received = 0;
    while received < 4 {
        let Some(event) = block_on(events.next()) else {
            break;
        };
        let Event::Received(batch) = &event else {
            panic!("expected a batch, got {event:?}");
        };
        assert!(!batch.is_empty() && batch.len() <= command::BATCH_SIZE);
        received += batch.len();
        let _ = state.update(event);
    }

    let files: Vec<_> = state
        .results()
        .filter(|(group, _)| *group == "Files")
        .map(|(_, item)| item.id)
        .collect();
    assert_eq!(files, ["report.pdf", "reports.csv", "repo.zip"]);
}
