#![cfg(feature = "alert")]

use iced::Element;
use iced_cube::alert;
use iced_cube::feedback::alert::Variant;
use iced_test::simulator;

#[test]
fn every_variant_renders_title_and_description() {
    for variant in Variant::ALL {
        let element: Element<'_, ()> = alert("Heads up")
            .description("Your trial ends in three days.")
            .variant(variant)
            .into();
        let mut ui = simulator(element);
        assert!(ui.find("Heads up").is_ok(), "{variant:?}");
        assert!(
            ui.find("Your trial ends in three days.").is_ok(),
            "{variant:?}"
        );
    }
}

#[test]
fn title_only_alert_renders() {
    let element: Element<'_, ()> = alert("Saved").variant(Variant::Success).into();
    let mut ui = simulator(element);
    assert!(ui.find("Saved").is_ok());
}
