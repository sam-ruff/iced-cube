#![cfg(all(feature = "tooltip", feature = "button"))]

use iced::Element;
use iced_cube::button;
use iced_cube::overlay::tooltip::{Position, tooltip};
use iced_test::simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Save,
}

#[test]
fn wrapped_content_still_receives_clicks() {
    for position in Position::ALL {
        let element: Element<'_, Message> =
            tooltip(button("Save").on_press(Message::Save), "Save changes")
                .position(position)
                .into();
        let mut ui = simulator(element);
        ui.click("Save").expect("content is rendered");
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![Message::Save],
            "{position:?}"
        );
    }
}

#[test]
fn bubble_is_hidden_until_hovered() {
    let element: Element<'_, Message> =
        tooltip(button("Save").on_press(Message::Save), "Save changes").into();
    let mut ui = simulator(element);
    assert!(ui.find("Save").is_ok());
    assert!(ui.find("Save changes").is_err());
}
