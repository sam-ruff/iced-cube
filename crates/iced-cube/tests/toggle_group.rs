#![cfg(feature = "toggle-group")]

use iced::Element;
use iced::widget::column;
use iced_cube::primitives::toggle_group::{Event, State, Variant, item, toggle_group};
use iced_test::simulator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Bold,
    Italic,
    Underline,
}

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Group(Event<Format>),
}

fn items() -> [iced_cube::primitives::toggle_group::Item<Format>; 3] {
    [
        item(Format::Bold, "Bold"),
        item(Format::Italic, "Italic"),
        item(Format::Underline, "Underline").disabled(true),
    ]
}

fn view(state: &State<Format>, variant: Variant) -> Element<'_, Message> {
    toggle_group(state)
        .variant(variant)
        .on_event(Message::Group)
        .into()
}

#[test]
fn clicking_an_item_emits_toggle() {
    for variant in Variant::ALL {
        let state = State::multiple(items());
        let mut ui = simulator(view(&state, variant));
        ui.click("Italic").expect("Italic is rendered");
        ui.click("Bold").expect("Bold is rendered");
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![
                Message::Group(Event::Toggle(Format::Italic)),
                Message::Group(Event::Toggle(Format::Bold)),
            ],
            "{variant:?}"
        );
    }
}

#[test]
fn disabled_items_emit_nothing() {
    let state = State::single(items());
    let mut ui = simulator(view(&state, Variant::Outline));
    ui.click("Underline").expect("Underline is still rendered");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn emitted_messages_drive_single_and_multiple_state() {
    for (mut state, expected) in [
        (State::single(items()), vec![Format::Italic]),
        (State::multiple(items()), vec![Format::Bold, Format::Italic]),
    ] {
        let messages: Vec<_> = {
            let mut ui = simulator(view(&state, Variant::Default));
            ui.click("Bold").expect("Bold is rendered");
            ui.click("Italic").expect("Italic is rendered");
            ui.into_messages().collect()
        };
        for Message::Group(event) in messages {
            let _ = state.update(event);
        }
        assert_eq!(state.selected(), expected);
    }
}

#[test]
fn a_group_without_a_handler_is_disabled() {
    let state = State::single(items());
    let element: Element<'_, Message> = toggle_group(&state).into();
    let mut ui = simulator(element);
    ui.click("Bold").expect("Bold is rendered");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn labels_stay_on_one_line_in_a_narrow_container() {
    let state = State::single([item(0, "Left aligned"), item(1, "Right aligned")]);
    for variant in Variant::ALL {
        let element: Element<'_, u8> = column![toggle_group(&state).variant(variant)]
            .width(60)
            .into();
        let mut ui = simulator(element);
        for label in ["Left aligned", "Right aligned"] {
            let bounds = ui.find(label).expect("item is rendered").bounds();
            assert!(
                bounds.height < 24.0,
                "{variant:?} {label}: {}",
                bounds.height
            );
        }
        let left = ui.find("Left aligned").expect("first item").bounds();
        let right = ui.find("Right aligned").expect("second item").bounds();
        assert!(
            right.x > left.x + left.width,
            "{variant:?} items share a row"
        );
    }
}
