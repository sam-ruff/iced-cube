//! Checks that a selection is drawn in both themes and that the article
//! story fits a phone, with the bundled fonts so the result is the same on
//! every machine.

use gallery::app::Mode;
use gallery::{FONT, FONT_FILES, Gallery, ThemeChoice};
use iced::{Event, Point, Rectangle, Size, mouse};
use iced_test::Error;
use iced_test::simulator::Simulator;

const STORY: &str = "selectable-text/article";
const HEADING: &str = "Getting started";
const LAST: &str = "A click on a link still opens it. See the changelog for what is new.";

fn open(gallery: &Gallery, size: Size) -> Simulator<'_, gallery::Message> {
    let settings = iced::Settings {
        default_font: FONT,
        fonts: FONT_FILES.iter().map(|font| (*font).into()).collect(),
        ..iced::Settings::default()
    };
    Simulator::with_size(settings, size, gallery.view())
}

fn visible(ui: &mut Simulator<'_, gallery::Message>, text: &str) -> Result<Rectangle, Error> {
    let found = ui.find(text)?;
    Ok(found.visible_bounds().unwrap_or_else(|| found.bounds()))
}

/// Drags from the start of the heading into the last paragraph, across the
/// link in the paragraph between them.
fn select(ui: &mut Simulator<'_, gallery::Message>) -> Result<(), Error> {
    let heading = visible(ui, HEADING)?;
    let last = visible(ui, LAST)?;
    let from = Point::new(heading.x + 0.5, heading.center_y());
    let to = Point::new(last.x + last.width / 2.0, last.y + 8.0);
    ui.point_at(from);
    let _ = ui.simulate([Event::Mouse(mouse::Event::ButtonPressed(
        mouse::Button::Left,
    ))]);
    ui.point_at(to);
    let _ = ui.simulate([
        Event::Mouse(mouse::Event::CursorMoved { position: to }),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]);
    Ok(())
}

/// A hash of the story as drawn, with or without a selection.
fn draw(choice: ThemeChoice, selected: bool, name: &str) -> Result<String, Error> {
    iced_cube::theme::set_font(FONT);
    let gallery = Gallery::new(Mode::Single, Some(STORY), choice);
    let mut ui = open(&gallery, Size::new(720.0, 260.0));
    if selected {
        select(&mut ui)?;
    }
    let directory = std::env::temp_dir().join(format!(
        "gallery-selectable-text-{name}-{}",
        std::process::id()
    ));
    let _ = ui
        .snapshot(&gallery.theme())?
        .matches_hash(directory.join(name))?;
    let mut hash = String::new();
    for file in std::fs::read_dir(&directory)?.filter_map(Result::ok) {
        hash = std::fs::read_to_string(file.path())?;
    }
    std::fs::remove_dir_all(&directory)?;
    Ok(hash)
}

#[test]
fn a_selection_is_drawn_in_both_themes() -> Result<(), Error> {
    let light = draw(ThemeChoice::Light, false, "light")?;
    let light_selected = draw(ThemeChoice::Light, true, "light-selected")?;
    let dark = draw(ThemeChoice::Dark, false, "dark")?;
    let dark_selected = draw(ThemeChoice::Dark, true, "dark-selected")?;

    assert_ne!(light, light_selected);
    assert_ne!(dark, dark_selected);
    assert_ne!(light_selected, dark_selected);
    Ok(())
}

#[test]
fn drawing_the_same_selection_twice_gives_the_same_picture() -> Result<(), Error> {
    let first = draw(ThemeChoice::Light, true, "again-first")?;
    let second = draw(ThemeChoice::Light, true, "again-second")?;
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn the_article_wraps_inside_a_phone_preview() -> Result<(), Error> {
    iced_cube::theme::set_font(FONT);
    for width in [360.0, 390.0] {
        let gallery = Gallery::new(Mode::Single, Some(STORY), ThemeChoice::Light);
        let mut ui = open(&gallery, Size::new(width, 320.0));
        for text in [HEADING, LAST] {
            let bounds = visible(&mut ui, text)?;
            assert!(
                bounds.x >= 0.0 && bounds.x + bounds.width <= width,
                "{text} runs off a {width} pixel preview: {bounds:?}"
            );
        }
        // Still selectable when the lines have wrapped.
        select(&mut ui)?;
    }
    Ok(())
}
