#![cfg(feature = "accordion")]

use iced::Element;
use iced::widget::text;
use iced_cube::layout::accordion::{Event, Mode, State, accordion};
use iced_test::simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Accordion(Event<u8>),
}

fn view(state: &State<u8>) -> Element<'_, Message> {
    accordion(state)
        .item(1, "Shipping", text("Ships in two days."))
        .item(2, "Returns", text("Return within 30 days."))
        .on_event(Message::Accordion)
        .into()
}

#[test]
fn clicking_headers_emits_toggle() {
    let state = State::new(Mode::Single);
    let mut ui = simulator(view(&state));
    ui.click("Shipping").expect("Shipping header is rendered");
    ui.click("Returns").expect("Returns header is rendered");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        vec![
            Message::Accordion(Event::Toggle(1)),
            Message::Accordion(Event::Toggle(2)),
        ]
    );
}

#[test]
fn content_is_rendered_only_while_open() {
    let closed = State::new(Mode::Single);
    let mut ui = simulator(view(&closed));
    assert!(ui.find("Ships in two days.").is_err());

    let open = State::new(Mode::Single).with_open(1);
    let mut ui = simulator(view(&open));
    assert!(ui.find("Ships in two days.").is_ok());
    assert!(ui.find("Return within 30 days.").is_err());
}

#[test]
fn clicking_the_open_header_closes_it() {
    let mut state = State::new(Mode::Multiple).with_open(1).with_open(2);
    let messages: Vec<_> = {
        let mut ui = simulator(view(&state));
        ui.click("Shipping").expect("Shipping header is rendered");
        ui.into_messages().collect()
    };
    for Message::Accordion(event) in messages {
        let _ = state.update(event);
    }
    assert_eq!(state.open(), [2]);
}

#[test]
fn headers_without_a_handler_are_disabled() {
    let state = State::new(Mode::Single);
    let element: Element<'_, Message> = accordion(&state).item(1, "Shipping", text("Body")).into();
    let mut ui = simulator(element);
    ui.click("Shipping").expect("Shipping header is rendered");
    assert_eq!(ui.into_messages().count(), 0);
}
