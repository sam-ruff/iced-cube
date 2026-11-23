//! Dialog stories in a phone-sized preview: sections stack, the body scrolls
//! and the actions stay inside the frame.

use gallery::app::Mode;
use gallery::{FONT, FONT_FILES, Gallery, ThemeChoice};
use iced::{Rectangle, Size};
use iced_test::simulator::Simulator;

/// A 360px phone leaves a 328px preview frame.
const PHONE: Size = Size::new(328.0, 480.0);

fn open(id: &str) -> Gallery {
    iced_cube::theme::set_font(FONT);
    Gallery::new(Mode::Single, Some(id), ThemeChoice::Light)
}

fn render(gallery: &Gallery) -> Simulator<'_, gallery::Message> {
    let settings = iced::Settings {
        default_font: FONT,
        fonts: FONT_FILES.iter().map(|font| (*font).into()).collect(),
        ..iced::Settings::default()
    };
    Simulator::with_size(settings, PHONE, gallery.view())
}

/// Whether the whole of the target is drawn inside the frame.
fn inside(ui: &mut Simulator<'_, gallery::Message>, label: &str) -> bool {
    let frame = Rectangle::with_size(PHONE);
    ui.find(label).is_ok_and(|target| {
        target.visible_bounds().is_some_and(|visible| {
            let bounds = target.bounds();
            (visible.height - bounds.height).abs() < 0.5
                && (visible.width - bounds.width).abs() < 0.5
                && frame.contains(visible.position())
                && visible.x + visible.width <= PHONE.width
                && visible.y + visible.height <= PHONE.height
        })
    })
}

#[test]
fn sections_stack_and_the_actions_stay_in_view_on_a_phone() {
    let gallery = open("dialog/sections");
    let mut ui = render(&gallery);
    let details = ui.find("Details").expect("first section").bounds();
    let time = ui.find("Time").expect("second section").bounds();
    assert_eq!(details.x, time.x, "one column");
    assert!(time.y > details.y + details.height);

    for label in ["New event", "Details", "Cancel", "Save event"] {
        assert!(inside(&mut ui, label), "{label}");
    }
    assert!(!inside(&mut ui, "People"), "the last section scrolls");
}

#[test]
fn long_content_keeps_its_actions_on_a_phone() {
    let gallery = open("dialog/long");
    let mut ui = render(&gallery);
    for label in ["What's new in Calendar", "Later", "Got it"] {
        assert!(inside(&mut ui, label), "{label}");
    }
}
