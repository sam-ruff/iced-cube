#![cfg(feature = "status-bar")]

use std::thread;

use futures::executor::block_on;
use futures::{SinkExt, StreamExt};
use iced::{Element, Settings, Size};
use iced_cube::feedback::badge::Variant;
use iced_cube::lucide;
use iced_cube::status_bar::{
    self, Event, Output, Section, State, Update, action, badge, progress, spinner, status_bar, text,
};
use iced_test::simulator::{Simulator, simulator};

#[derive(Debug, Clone)]
enum Message {
    Status(Event<&'static str>),
}

fn view<'a>(state: &'a State<&'static str>) -> Element<'a, Message> {
    status_bar(state).on_event(Message::Status).into()
}

fn sample() -> State<&'static str> {
    State::new().with_items([
        text("branch", "main").icon(lucide!(GitBranch)),
        spinner("sync", "Syncing"),
        progress("build", 0.5)
            .label("Building")
            .section(Section::Centre),
        action("problems", "2 problems")
            .icon(lucide!(TriangleAlert))
            .section(Section::End),
        badge("mode", "Insert", Variant::Secondary).section(Section::End),
    ])
}

#[test]
fn a_producer_thread_feeds_items_through_the_ready_channel() {
    let mut state = State::new();
    let mut events = Box::pin(status_bar::stream::<&'static str>());
    let Some(Event::Ready(sender)) = block_on(events.next()) else {
        panic!("the first event must be Ready");
    };
    let Some(Output::Ready(sender)) = state.update(Event::Ready(sender)) else {
        panic!("Ready hands the sender back");
    };

    let producer = thread::spawn(move || {
        let mut sender = sender;
        block_on(async {
            for line in 1..=20 {
                let cursor = text("cursor", format!("Ln {line}, Col 1")).section(Section::End);
                sender
                    .send(Update::Set(cursor))
                    .await
                    .expect("receiver is alive");
            }
            sender
                .send(Update::Set(spinner("sync", "Syncing")))
                .await
                .expect("receiver is alive");
            sender
                .send(Update::Set(text("sync", "Up to date").icon(lucide!(Check))))
                .await
                .expect("receiver is alive");
        });
    });

    let mut batches = 0;
    while state
        .item(&"sync")
        .is_none_or(|item| item.label != "Up to date")
    {
        let Some(event) = block_on(events.next()) else {
            panic!("the channel closed early");
        };
        assert!(matches!(event, Event::Received(_)));
        batches += 1;
        let _ = state.update(event);
    }
    producer.join().expect("producer finished");
    assert!(batches >= 1);
    assert_eq!(state.items().len(), 2, "updates replace items by id");
    assert!(!state.needs_frames(), "the spinner was replaced");

    let mut ui = simulator(view(&state));
    assert!(ui.find("Ln 20, Col 1").is_ok());
    assert!(ui.find("Up to date").is_ok());
    assert!(ui.find("Ln 19, Col 1").is_err());
}

#[test]
fn renders_every_kind_of_item() {
    let state = sample();
    let mut ui = simulator(view(&state));
    for label in ["main", "Syncing", "Building", "2 problems", "Insert"] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
}

#[test]
fn sections_sit_at_the_start_centre_and_end() {
    let state = sample();
    let mut ui = Simulator::with_size(Settings::default(), Size::new(900.0, 200.0), view(&state));
    let start = ui.find("main").expect("start item").bounds();
    let centre = ui.find("Building").expect("centre item").bounds();
    let end = ui.find("Insert").expect("end item").bounds();
    assert!(start.x < 60.0, "{start:?}");
    assert!(centre.x > 300.0 && centre.x < 600.0, "{centre:?}");
    assert!(end.x > 800.0, "{end:?}");
    assert!((start.y - end.y).abs() < 2.0, "one line: {start:?} {end:?}");
}

#[test]
fn a_narrow_bar_wraps_its_items_onto_more_lines() {
    let state = sample();
    let mut ui = Simulator::with_size(Settings::default(), Size::new(260.0, 200.0), view(&state));
    let first = ui.find("main").expect("first item").bounds();
    let last = ui.find("Insert").expect("last item").bounds();
    assert!(last.y > first.y, "the end section wraps below");
    assert!(last.x + last.width <= 260.0, "nothing runs off the edge");
}

#[test]
fn clicking_an_action_reports_it() {
    let mut state = sample();
    let mut ui = simulator(view(&state));
    ui.click("2 problems").expect("action is rendered");
    let messages: Vec<Message> = ui.into_messages().collect();
    let [Message::Status(event)] = messages.as_slice() else {
        panic!("expected one message, got {messages:?}");
    };
    assert!(matches!(event, Event::Press("problems")));
    assert!(matches!(
        state.update(event.clone()),
        Some(Output::Pressed("problems"))
    ));
}

#[test]
fn a_bar_without_messages_is_inert() {
    let state = sample();
    let element: Element<'_, Message> = status_bar(&state).into();
    let mut ui = simulator(element);
    ui.click("2 problems").expect("action is rendered");
    assert!(ui.into_messages().next().is_none());
}
