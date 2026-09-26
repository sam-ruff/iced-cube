//! Renders every story in both themes and compares against the committed PNGs.
//!
//! A missing PNG is written on first run. To accept an intentional visual
//! change, delete the affected files under `snapshots/` and run the tests again.

use gallery::app::Mode;
use gallery::registry::{Meta, WIDTH};
use gallery::stories::ALL;
use gallery::{FONT, FONT_FILES, Gallery, ThemeChoice};
use iced::Size;
use iced_test::simulator::Simulator;

/// Matches the story's preview frame on the docs site.
fn snapshot_size(meta: &Meta) -> Size {
    Size::new(WIDTH as f32, meta.height as f32)
}

fn snapshot_path(id: &str, theme: ThemeChoice) -> String {
    let theme = match theme {
        ThemeChoice::Light => "light",
        ThemeChoice::Dark => "dark",
    };
    format!(
        "{}/snapshots/{}-{theme}.png",
        env!("CARGO_MANIFEST_DIR"),
        id.replace('/', "--")
    )
}

#[test]
fn every_story_matches_its_snapshot() {
    let mut mismatches = Vec::new();

    for meta in ALL {
        for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
            let gallery = Gallery::new(Mode::Single, Some(meta.id), choice);
            let settings = iced::Settings {
                default_font: FONT,
                fonts: FONT_FILES.iter().map(|font| (*font).into()).collect(),
                ..iced::Settings::default()
            };
            let mut ui = Simulator::with_size(settings, snapshot_size(meta), gallery.view());
            let snapshot = ui
                .snapshot(&gallery.theme())
                .unwrap_or_else(|error| panic!("{}: {error}", meta.id));

            let matches = snapshot
                .matches_image(snapshot_path(meta.id, choice))
                .unwrap_or_else(|error| panic!("{}: {error}", meta.id));
            if !matches {
                mismatches.push(format!("{} ({choice:?})", meta.id));
            }
        }
    }

    assert!(mismatches.is_empty(), "snapshots changed: {mismatches:?}");
}
