#![cfg(feature = "badge")]

use iced::Element;
use iced::widget::{column, text};
use iced_cube::feedback::badge::Variant;
use iced_cube::{badge, lucide};
use iced_test::simulator;

#[test]
fn every_variant_renders_its_label() {
    for variant in Variant::ALL {
        let element: Element<'_, ()> = badge("Beta").variant(variant).into();
        let mut ui = simulator(element);
        assert!(ui.find("Beta").is_ok(), "{variant:?}");
    }
}

/// Lays `content` out in a column far narrower than its label.
fn narrow(content: Element<'_, ()>) -> iced_test::Simulator<'_, ()> {
    simulator(column![content].width(40))
}

#[test]
fn label_stays_on_one_line_in_a_narrow_container() {
    let label = "+20.1% this month";

    let mut plain = narrow(text(label).size(12).into());
    let wrapped = plain.find(label).expect("plain text is rendered").bounds();

    let mut ui = narrow(badge(label).variant(Variant::Success).into());
    let bounds = ui.find(label).expect("badge label is rendered").bounds();

    assert!(
        wrapped.height > 30.0,
        "the container is narrow enough to wrap text"
    );
    assert!(bounds.height < 20.0, "one line, got {}", bounds.height);
    assert!(
        bounds.width > 40.0,
        "sized to its label, got {}",
        bounds.width
    );
}

#[test]
fn badge_with_icon_renders_its_label() {
    let element: Element<'_, ()> = badge("Live")
        .icon(lucide!(CircleDot))
        .variant(Variant::Success)
        .into();
    let mut ui = simulator(element);
    assert!(ui.find("Live").is_ok());
}
