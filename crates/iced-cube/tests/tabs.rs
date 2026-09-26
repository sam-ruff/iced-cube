#![cfg(feature = "tabs")]

use iced::Element;
use iced::widget::column;
use iced_cube::navigation::tabs::{self, Event, State, Variant, tab};
use iced_test::simulator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Account,
    Password,
    Billing,
}

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Tabs(Event<Page>),
}

fn state() -> State<Page> {
    State::new([
        tab(Page::Account, "Account"),
        tab(Page::Password, "Password"),
        tab(Page::Billing, "Billing").disabled(true),
    ])
}

fn view(state: &State<Page>, variant: Variant) -> Element<'_, Message> {
    tabs::tabs(state)
        .variant(variant)
        .on_event(Message::Tabs)
        .into()
}

#[test]
fn clicking_a_tab_emits_select() {
    for variant in Variant::ALL {
        let state = state();
        let mut ui = simulator(view(&state, variant));
        ui.click("Password").expect("Password tab is rendered");
        ui.click("Account").expect("Account tab is rendered");

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![
                Message::Tabs(Event::Select(Page::Password)),
                Message::Tabs(Event::Select(Page::Account)),
            ],
            "{variant:?}"
        );
    }
}

#[test]
fn disabled_tab_emits_nothing() {
    let state = state();
    let mut ui = simulator(view(&state, Variant::Underline));
    ui.click("Billing").expect("Billing tab is still rendered");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn emitted_messages_drive_the_state() {
    let mut state = state();
    let messages: Vec<_> = {
        let mut ui = simulator(view(&state, Variant::Pills));
        ui.click("Password").expect("Password tab is rendered");
        ui.into_messages().collect()
    };
    for Message::Tabs(event) in messages {
        let _ = state.update(event);
    }
    assert_eq!(state.selected(), Some(Page::Password));
}

#[test]
fn triggers_stay_on_one_line_in_a_narrow_container() {
    let state = State::new([
        tab(Page::Account, "Account settings"),
        tab(Page::Password, "Change password"),
    ]);
    for variant in [Variant::Underline, Variant::Pills] {
        let element: Element<'_, Message> = column![view(&state, variant)].width(60).into();
        let mut ui = simulator(element);
        for label in ["Account settings", "Change password"] {
            let bounds = ui.find(label).expect("trigger is rendered").bounds();
            assert!(
                bounds.height < 24.0,
                "{variant:?} {label}: {}",
                bounds.height
            );
            assert!(bounds.width > 60.0, "{variant:?} {label}: {}", bounds.width);
        }
    }
}

#[test]
fn tabs_without_a_handler_are_disabled() {
    let state = state();
    let element: Element<'_, Message> = tabs::tabs(&state).into();
    let mut ui = simulator(element);
    ui.click("Password").expect("Password tab is rendered");
    assert_eq!(ui.into_messages().count(), 0);
}
