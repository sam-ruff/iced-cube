use iced::widget::column;
use iced::{Element, Point};
use iced_cube::primitives::checkbox;
use iced_cube::primitives::checkbox::{BOX_SIZE, CheckState};
use iced_test::simulator;
use iced_test::simulator::click;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Terms(bool),
    All(bool),
    Newsletter(bool),
}

fn view(enabled: bool) -> Element<'static, Message> {
    column![
        checkbox(false)
            .label("Accept terms")
            .on_toggle(Message::Terms),
        checkbox(CheckState::Indeterminate)
            .label("Select all")
            .on_toggle(Message::All),
        checkbox(true)
            .label("Newsletter")
            .on_toggle_maybe(enabled.then_some(Message::Newsletter)),
    ]
    .spacing(12)
    .into()
}

#[test]
fn clicking_a_label_emits_the_next_value() {
    let mut ui = simulator(view(true));
    ui.click("Accept terms").expect("label is rendered");
    ui.click("Select all").expect("label is rendered");
    ui.click("Newsletter").expect("label is rendered");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        vec![
            Message::Terms(true),
            Message::All(true),
            Message::Newsletter(false)
        ]
    );
}

#[test]
fn clicking_the_box_toggles() {
    let mut ui = simulator(view(true));
    ui.point_at(Point::new(BOX_SIZE / 2.0, BOX_SIZE / 2.0));
    ui.simulate(click());

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Terms(true)]);
}

#[test]
fn disabled_checkbox_emits_nothing() {
    let mut ui = simulator(view(false));
    ui.click("Newsletter").expect("label is still rendered");

    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn every_state_renders_its_label() {
    for state in CheckState::ALL {
        let element: Element<'_, Message> = checkbox(state)
            .label("Label")
            .on_toggle(Message::Terms)
            .into();
        let mut ui = simulator(element);
        assert!(ui.find("Label").is_ok(), "{state:?}");
    }
}
