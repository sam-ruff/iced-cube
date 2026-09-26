#![cfg(all(feature = "scroll-area", feature = "button"))]

use iced::widget::{column, row, text};
use iced::{Element, Length};
use iced_cube::primitives::scroll_area::Direction;
use iced_cube::{button, scroll_area};
use iced_test::simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Open(usize),
}

fn items() -> Element<'static, Message> {
    column((0..40).map(|i| text(format!("Item {i}")).into())).into()
}

#[test]
fn visible_content_is_findable_in_every_direction() {
    for direction in Direction::ALL {
        let element: Element<'_, Message> = scroll_area(items())
            .direction(direction)
            .height(200)
            .width(Length::Fill)
            .into();
        let mut ui = simulator(element);
        assert!(ui.find("Item 0").is_ok(), "{direction:?}");
    }
}

#[test]
fn wide_content_is_findable_horizontally() {
    let wide = row((0..30).map(|i| text(format!("Column {i}")).into())).spacing(24);
    let element: Element<'_, Message> = scroll_area(wide)
        .direction(Direction::Horizontal)
        .width(300)
        .into();
    let mut ui = simulator(element);
    assert!(ui.find("Column 0").is_ok());
}

#[test]
fn buttons_inside_a_scroll_area_are_clickable() {
    let content = column((0..3).map(|i| {
        button(format!("Open {i}"))
            .on_press(Message::Open(i))
            .into()
    }));
    let element: Element<'_, Message> = scroll_area(content).height(200).into();
    let mut ui = simulator(element);
    ui.click("Open 1").expect("Open 1 is rendered");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Open(1)]);
}
