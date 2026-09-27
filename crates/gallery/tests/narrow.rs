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

/// Sends the preview one pointer move and applies what the story sends
/// back, such as a sidebar noticing the narrow width.
fn settle(gallery: &mut Gallery) {
    let messages: Vec<gallery::Message> = {
        let mut ui = render(gallery);
        let _ = ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: iced::Point::new(200.0, 200.0),
        })]);
        ui.into_messages().collect()
    };
    for message in messages {
        let _ = gallery.update(message);
    }
}

#[test]
fn the_sidebar_is_a_closed_drawer_on_a_phone() {
    let mut gallery = open("sidebar/default");
    settle(&mut gallery);
    let mut ui = render(&gallery);
    assert!(ui.find("Dashboard").is_err(), "navigation hidden");
    let title = ui.find("Everything in this project, in one place.");
    let title = title.expect("page content").bounds();
    assert!(
        title.x < 64.0,
        "content uses the width, starts at {}",
        title.x
    );
}

#[test]
fn nested_panels_stack_on_a_phone() {
    let gallery = open("resizable-panel/nested");
    let mut ui = render(&gallery);
    let files = ui.find("Files").expect("files pane").bounds();
    let editor = ui.find("Editor").expect("editor pane").bounds();
    let terminal = ui.find("Terminal").expect("terminal pane").bounds();
    assert!(files.y < editor.y && editor.y < terminal.y, "one column");
    let centre = |bounds: iced::Rectangle| bounds.x + bounds.width / 2.0;
    assert!((centre(files) - centre(editor)).abs() < 1.0);
}

#[test]
fn the_editor_and_preview_stack_on_a_phone() {
    let gallery = open("split-pane/editor-preview");
    let mut ui = render(&gallery);
    let heading = ui.find("Fixes").expect("preview heading").bounds();
    assert!(
        heading.y > 150.0,
        "preview below the editor at {}",
        heading.y
    );
    assert!(heading.width < PHONE.width, "preview fits the frame");
}

#[test]
fn collapsible_panels_keep_their_minimums_on_a_phone() {
    let gallery = open("resizable-panel/collapsible");
    let mut ui = render(&gallery);
    let archive = ui.find("Archive").expect("folders stay open").bounds();
    let hide = ui.find("Hide").expect("toggle").bounds();
    assert!(archive.x < hide.x, "folders beside the messages");
    assert!(hide.height < 24.0, "button label on one line");
    // The label, the button's own padding, then the pane's padding, the
    // card's border and the gallery's padding.
    let right = hide.x + hide.width + 12.0 + 16.0 + 1.0 + 24.0;
    assert!(
        right <= PHONE.width,
        "button fits its pane, ends at {right}"
    );
}
