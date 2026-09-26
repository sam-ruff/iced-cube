#![cfg(feature = "progress")]

use iced::Element;
use iced_cube::feedback::progress::{Size, Variant};
use iced_cube::progress;
use iced_test::simulator;

#[test]
fn label_and_percentage_are_rendered() {
    let element: Element<'_, ()> = progress(0.424)
        .label("Uploading")
        .show_percentage(true)
        .into();
    let mut ui = simulator(element);
    assert!(ui.find("Uploading").is_ok());
    assert!(ui.find("42%").is_ok());
}

#[test]
fn out_of_range_values_show_clamped_percentage() {
    for (value, shown) in [(-1.0, "0%"), (5.0, "100%"), (f32::NAN, "0%")] {
        let element: Element<'_, ()> = progress(value).show_percentage(true).into();
        let mut ui = simulator(element);
        assert!(ui.find(shown).is_ok(), "{value}");
    }
}

#[test]
fn every_variant_and_size_renders() {
    for variant in Variant::ALL {
        for size in Size::ALL {
            let element: Element<'_, ()> = progress(0.5)
                .variant(variant)
                .size(size)
                .label("Label")
                .into();
            let mut ui = simulator(element);
            assert!(ui.find("Label").is_ok(), "{variant:?} {size:?}");
        }
    }
}
