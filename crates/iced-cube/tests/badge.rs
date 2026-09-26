use iced::Element;
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

#[test]
fn badge_with_icon_renders_its_label() {
    let element: Element<'_, ()> = badge("Live")
        .icon(lucide!(CircleDot))
        .variant(Variant::Success)
        .into();
    let mut ui = simulator(element);
    assert!(ui.find("Live").is_ok());
}
