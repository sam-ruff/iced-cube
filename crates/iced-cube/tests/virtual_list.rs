#![cfg(feature = "virtual-list")]

use std::thread;
use std::time::{Duration, Instant};

use futures::executor::block_on;
use futures::{SinkExt, StreamExt};
use iced::keyboard::{Key, Modifiers, key::Named};
use iced::widget::{container, text};
use iced::{Element, Point, Size, keyboard, mouse};
use iced_cube::data::virtual_list::{self, Event, Mode, Output, State, virtual_list};
use iced_test::simulator::Simulator;

#[derive(Debug, Clone)]
enum Message {
    List(Event<u32>),
}

fn numbers(count: u32) -> State<u32> {
    State::new(0..count)
}

fn view(state: &State<u32>, height: f32) -> Element<'_, Message> {
    virtual_list(state, |_, n: &u32| text(format!("Row {n}")).into())
        .height(height)
        .on_event(Message::List)
        .into()
}

fn sized(element: Element<'_, Message>) -> Simulator<'_, Message> {
    Simulator::with_size(iced::Settings::default(), Size::new(400.0, 400.0), element)
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event<u32>> {
    ui.into_messages()
        .map(|message| match message {
            Message::List(event) => event,
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

fn wheel(ui: &mut Simulator<'_, Message>, at: Point, y: f32) {
    ui.point_at(at);
    let _ = ui.simulate([iced::Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 0.0, y },
    })]);
}

#[test]
fn a_hundred_thousand_rows_only_build_the_rows_in_view() {
    let started = Instant::now();
    let state = numbers(100_000);
    let mut ui = sized(view(&state, 300.0));
    assert!(ui.find("Row 0").is_ok());
    assert!(ui.find("Row 50").is_err(), "beyond the view");
    wheel(&mut ui, Point::new(100.0, 100.0), -32.0 * 50_000.0);
    assert!(ui.find("Row 50000").is_ok(), "scrolled half way");
    assert!(ui.find("Row 0").is_err());
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "building and scrolling took {elapsed:?}"
    );
}

#[test]
fn rows_are_equally_tall() {
    let state = numbers(5);
    let mut ui = sized(view(&state, 300.0));
    let first = ui.find("Row 0").expect("first row").bounds();
    let second = ui.find("Row 1").expect("second row").bounds();
    assert!((second.y - first.y - virtual_list::ROW_HEIGHT).abs() < 0.01);
}

#[test]
fn clicking_a_row_focuses_the_list_and_presses_it() {
    let state = numbers(10);
    let mut ui = sized(view(&state, 300.0));
    ui.click("Row 3").expect("row is rendered");
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Focus(true))),
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
fn modifier_clicks_select_a_range() {
    let mut state = numbers(10).with_mode(Mode::Multiple);
    let emitted = {
        let mut ui = sized(view(&state, 300.0));
        ui.click("Row 2").expect("row");
        let _ = ui.simulate([iced::Event::Keyboard(keyboard::Event::ModifiersChanged(
            Modifiers::SHIFT,
        ))]);
        ui.click("Row 5").expect("row");
        events(ui)
    };
    let outputs: Vec<Output<u32>> = emitted
        .into_iter()
        .filter_map(|event| state.update(event))
        .collect();
    assert!(
        matches!(outputs.last(), Some(Output::Selected(indices)) if indices == &[2, 3, 4, 5]),
        "{outputs:?}"
    );
}

#[test]
fn a_double_click_activates() {
    let state = numbers(10);
    let mut ui = sized(view(&state, 300.0));
    ui.click("Row 4").expect("row");
    ui.click("Row 4").expect("row");
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Activate(4))),
        "{emitted:?}"
    );
}

#[test]
fn keys_are_ignored_until_the_list_has_focus() {
    let state = numbers(10);
    let mut ui = sized(view(&state, 300.0));
    let _ = ui.simulate([press_key(Key::Named(Named::ArrowDown), Modifiers::empty())]);
    assert!(!events(ui).iter().any(|event| matches!(event, Event::Next)));

    let mut ui = sized(view(&state, 300.0));
    ui.click("Row 0").expect("row");
    let _ = ui.simulate([
        press_key(Key::Named(Named::ArrowDown), Modifiers::empty()),
        press_key(Key::Named(Named::PageDown), Modifiers::empty()),
        press_key(Key::Named(Named::End), Modifiers::empty()),
    ]);
    let emitted = events(ui);
    for expected in ["Next", "NextPage", "Last"] {
        assert!(
            emitted.iter().any(|event| format!("{event:?}") == expected),
            "{expected} in {emitted:?}"
        );
    }
}

#[test]
fn the_list_reports_how_many_rows_fit() {
    let state = numbers(100);
    let mut ui = sized(view(&state, 200.0));
    let _ = ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved {
        position: Point::new(100.0, 100.0),
    })]);
    let pages: Vec<usize> = events(ui)
        .into_iter()
        .filter_map(|event| match event {
            Event::Page(rows) => Some(rows),
            _ => None,
        })
        .collect();
    // 200 pixels, less the frame's padding, fit 6 rows of 32.
    assert_eq!(pages, [6]);
}

#[test]
fn the_highlight_scrolls_into_view() {
    let mut state = numbers(1000);
    let _ = state.update(Event::Page(5));
    let _ = state.update(Event::First);
    for _ in 0..100 {
        let _ = state.update(Event::NextPage);
    }
    assert_eq!(state.highlighted(), Some(500));
    {
        let mut ui = sized(view(&state, 200.0));
        assert!(ui.find("Row 500").is_ok());
        assert!(ui.find("Row 0").is_err());
    }

    state.scroll_to(900);
    let mut ui = sized(view(&state, 200.0));
    assert!(
        ui.find("Row 900").is_ok(),
        "scroll_to reveals without the highlight"
    );
}

#[test]
fn a_following_list_shows_its_end() {
    let state = numbers(500).with_follow(true);
    let mut ui = sized(view(&state, 200.0));
    assert!(ui.find("Row 499").is_ok());
    assert!(ui.find("Row 0").is_err());
}

#[test]
fn scrolling_away_from_the_end_stops_following() {
    let state = numbers(500).with_follow(true);
    let mut ui = sized(view(&state, 200.0));
    wheel(&mut ui, Point::new(100.0, 100.0), 120.0);
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Follow(false))),
        "{emitted:?}"
    );
}

#[test]
fn scrolling_back_to_the_end_follows_again() {
    let mut state = numbers(500).with_follow(true);
    let _ = state.update(Event::Follow(false));
    let mut ui = sized(view(&state, 200.0));
    assert!(ui.find("Row 0").is_ok(), "not following, so at the top");
    wheel(&mut ui, Point::new(100.0, 100.0), -100_000.0);
    let emitted = events(ui);
    assert!(
        emitted
            .iter()
            .any(|event| matches!(event, Event::Follow(true))),
        "{emitted:?}"
    );
}

#[test]
fn a_list_that_does_not_follow_never_reports_it() {
    let state = numbers(500);
    let mut ui = sized(view(&state, 200.0));
    wheel(&mut ui, Point::new(100.0, 100.0), -100_000.0);
    wheel(&mut ui, Point::new(100.0, 100.0), 100.0);
    assert!(
        !events(ui)
            .iter()
            .any(|event| matches!(event, Event::Follow(_)))
    );
}

#[test]
fn a_list_without_messages_is_disabled() {
    let state = numbers(5);
    let element: Element<'_, Message> =
        virtual_list(&state, |_, n: &u32| text(format!("Row {n}")).into()).into();
    let mut ui = sized(element);
    ui.click("Row 1").expect("row is rendered");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn rows_fit_a_narrow_container() {
    let state = numbers(3);
    let element: Element<'_, Message> = container(
        virtual_list(&state, |_, n: &u32| text(format!("Row {n}")).into()).on_event(Message::List),
    )
    .width(160)
    .into();
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(160.0, 200.0), element);
    let row = ui.find("Row 2").expect("row").bounds();
    assert!(row.x + row.width <= 160.0);
}

#[test]
fn a_producer_thread_sends_items_through_the_channel() {
    let mut events = Box::pin(virtual_list::stream::<u32>());
    let Some(Event::Ready(sender)) = block_on(events.next()) else {
        panic!("first event must be Ready");
    };
    let mut state = State::default().with_follow(true);
    let producer = thread::spawn(move || {
        let mut sender = sender;
        block_on(async {
            for n in 0..1000 {
                sender.send(n).await.expect("channel open");
            }
        });
    });
    let mut received = 0;
    while received < 1000 {
        let Some(event) = block_on(events.next()) else {
            break;
        };
        if let Event::Received(batch) = &event {
            assert!(batch.len() <= virtual_list::BATCH_SIZE);
            received += batch.len();
        }
        let _ = state.update(event);
    }
    producer.join().expect("producer finished");
    assert_eq!(state.len(), 1000);
    assert_eq!(state.get(999), Some(&999));
    assert!(state.is_following());

    let mut ui = sized(view(&state, 200.0));
    assert!(ui.find("Row 999").is_ok(), "follows the newest item");
}
