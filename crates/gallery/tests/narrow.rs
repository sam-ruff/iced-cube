//! Renders stories in a phone-sized preview, where rows of cards wrap and the
//! story grows taller than its frame instead of being squeezed.

use gallery::app::Mode;
use gallery::{FONT, FONT_FILES, Gallery, ThemeChoice};
use iced::Size;
use iced_test::simulator::Simulator;

/// A 360px phone leaves a 328px preview frame.
const PHONE: Size = Size::new(328.0, 280.0);

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

#[test]
fn stat_cards_stack_at_full_size_on_a_phone() {
    let gallery = open("card/stats");
    let mut ui = render(&gallery);
    let revenue = ui.find("Revenue").expect("first card").bounds();
    let churn = ui.find("Churn").expect("last card").bounds();

    assert_eq!(revenue.x, churn.x, "cards share one column");
    assert!(churn.y > PHONE.height, "the last card sits below the fold");

    let badge = ui.find("+20.1% this month").expect("badge").bounds();
    assert!(
        badge.height < 20.0,
        "badge on one line, got {}",
        badge.height
    );
}

#[test]
fn alignment_panels_keep_their_badges_on_one_line() {
    let gallery = open("stack/alignment");
    let mut ui = render(&gallery);
    for label in ["A longer label", "Medium one"] {
        let bounds = ui.find(label).expect("badge").bounds();
        assert!(bounds.height < 20.0, "{label}: {}", bounds.height);
    }
}

#[test]
fn settings_footer_fits_the_card() {
    let gallery = open("showcase/settings");
    let mut ui = render(&gallery);
    let title = ui.find("Preferences").expect("title").bounds();
    let save = ui.find("Save").expect("save button").bounds();
    // The card is centred, so its right edge mirrors the left edge of its title.
    let card_right = PHONE.width - title.x;
    assert!(
        save.x + save.width <= card_right,
        "Save ends at {}",
        save.x + save.width
    );
}

#[test]
fn payments_become_cards_that_fit_the_phone() {
    let gallery = open("data-table/default");
    let mut ui = render(&gallery);
    for text in [
        "Columns",
        "Paid",
        "Processing",
        "ken99@example.com",
        "$316.00",
        "monserrat44@example.com",
    ] {
        let bounds = ui.find(text).expect(text).bounds();
        assert!(
            bounds.x >= 0.0 && bounds.x + bounds.width <= PHONE.width,
            "{text} runs off the edge: {bounds:?}"
        );
        assert!(bounds.height < 24.0, "{text} wraps: {bounds:?}");
    }
    let email = ui.find("ken99@example.com").expect("card line").bounds();
    let label = ui.find("Email").expect("line label").bounds();
    assert_eq!(
        email.center_y().round(),
        label.center_y().round(),
        "a labelled line"
    );
}

#[test]
fn a_long_file_name_ends_in_an_ellipsis_on_a_phone() {
    let gallery = open("tree/file-explorer");
    let mut ui = render(&gallery);
    let name = ui
        .find("a_file_whose_name_is_far_too_long_for_the_row.rs")
        .expect("the full name is reported")
        .bounds();
    assert!(
        name.x + name.width <= PHONE.width,
        "ends at {}",
        name.x + name.width
    );
    assert!(name.height <= 20.0, "one line, got {}", name.height);
}
