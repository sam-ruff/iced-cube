#![cfg(feature = "spinner")]

use iced::widget::{row, text};
use iced::{Element, Theme};
use iced_cube::feedback::spinner::Size;
use iced_cube::spinner;
use iced_cube::theme::{dark, light};
use iced_test::simulator;

#[test]
fn spinner_renders_next_to_its_label() {
    for size in Size::ALL {
        let element: Element<'_, ()> = row![spinner(0.3).size(size), text("Loading")].into();
        let mut ui = simulator(element);
        assert!(ui.find("Loading").is_ok(), "{size:?}");
    }
}

#[test]
fn spinner_draws_in_both_themes() {
    let themes: [Theme; 2] = [light(), dark()];
    for theme in themes {
        let element: Element<'_, ()> = spinner(0.6).size(Size::Lg).into();
        let mut ui = simulator(element);
        assert!(ui.snapshot(&theme).is_ok(), "{theme}");
    }
}
