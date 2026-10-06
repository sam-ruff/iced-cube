#![cfg(feature = "toggle")]

use iced::Element;
use iced::widget::column;
use iced_cube::lucide;
use iced_cube::primitives::toggle::{Size, Variant, toggle};
use iced_test::simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Grid(bool),
}

#[test]
fn clicking_sends_the_opposite_state() {
    for variant in Variant::ALL {
        for pressed in [false, true] {
            let element: Element<'_, Message> = toggle("Grid")
                .icon(lucide!(Grid3x3))
                .variant(variant)
                .pressed(pressed)
                .on_toggle(Message::Grid)
                .into();
            let mut ui = simulator(element);
            ui.click("Grid").expect("toggle is rendered");
            let messages: Vec<_> = ui.into_messages().collect();
            assert_eq!(messages, vec![Message::Grid(!pressed)], "{variant:?}");
        }
    }
}

#[test]
fn a_toggle_without_on_toggle_is_disabled() {
    let element: Element<'_, Message> = toggle("Grid").pressed(true).into();
    let mut ui = simulator(element);
    ui.click("Grid").expect("toggle is still rendered");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn every_size_renders_its_label_on_one_line_in_a_narrow_container() {
    for size in Size::ALL {
        let element: Element<'_, Message> = column![
            toggle("Show hidden files")
                .size(size)
                .on_toggle(Message::Grid)
        ]
        .width(40)
        .into();
        let mut ui = simulator(element);
        let bounds = ui.find("Show hidden files").expect("label").bounds();
        assert!(bounds.height < 24.0, "{size:?}: {}", bounds.height);
        assert!(bounds.width > 40.0, "{size:?}: {}", bounds.width);
    }
}
