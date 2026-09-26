#![cfg(feature = "radio")]

use std::fmt;

use iced::Element;
use iced::widget::column;
use iced_cube::primitives::radio::Direction;
use iced_cube::primitives::{radio, radio_group};
use iced_test::simulator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Plan {
    Free,
    Pro,
    Team,
}

impl fmt::Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Plan::Free => "Free",
            Plan::Pro => "Pro",
            Plan::Team => "Team",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Plan(Plan),
    Size(u8),
}

fn group(enabled: bool, direction: Direction) -> Element<'static, Message> {
    let group =
        radio_group([Plan::Free, Plan::Pro, Plan::Team], Some(Plan::Free)).direction(direction);
    if enabled {
        group.on_select(Message::Plan).into()
    } else {
        group.into()
    }
}

#[test]
fn clicking_an_option_emits_its_value() {
    for direction in Direction::ALL {
        let mut ui = simulator(group(true, direction));
        ui.click("Team").expect("option is rendered");
        ui.click("Free").expect("option is rendered");

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![Message::Plan(Plan::Team), Message::Plan(Plan::Free)],
            "{direction:?}"
        );
    }
}

#[test]
fn disabled_group_emits_nothing() {
    let mut ui = simulator(group(false, Direction::Vertical));
    ui.click("Pro").expect("option is still rendered");

    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn single_radios_emit_their_own_value() {
    let view: Element<'_, Message> = column![
        radio("Small", 1, Some(2)).on_select(Message::Size),
        radio("Medium", 2, Some(2)).on_select(Message::Size),
        radio("Large", 3, Some(2)),
    ]
    .into();
    let mut ui = simulator(view);
    ui.click("Small").expect("radio is rendered");
    ui.click("Large").expect("disabled radio is rendered");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Size(1)]);
}
