//! Renders every story in both themes and compares against the committed PNGs.
//!
//! A missing PNG is written on first run. To accept an intentional visual
//! change, delete the affected files under `snapshots/` and run the tests again.
//! Failing renders are saved under `target/snapshots-failed/` for inspection.

use std::fs;
use std::path::{Path, PathBuf};

use gallery::app::Mode;
use gallery::registry::{Meta, WIDTH};
use gallery::stories::ALL;
use gallery::{FONT, FONT_FILES, Gallery, ThemeChoice};
use iced::Size;
use iced_test::simulator::Simulator;

/// A channel difference above this counts the pixel as changed.
const CHANNEL_TOLERANCE: u8 = 24;
/// Share of pixels allowed to change, which absorbs anti-aliasing differences
/// between CPUs while still catching any real layout or colour change.
const CHANGED_SHARE: f64 = 0.002;

/// Matches the story's preview frame on the docs site.
fn snapshot_size(meta: &Meta) -> Size {
    Size::new(WIDTH as f32, meta.height as f32)
}

fn file_stem(id: &str, theme: ThemeChoice) -> String {
    let theme = match theme {
        ThemeChoice::Light => "light",
        ThemeChoice::Dark => "dark",
    };
    format!("{}-{theme}", id.replace('/', "--"))
}

struct Image {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn read_png(path: &Path) -> Image {
    let file = fs::File::open(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder
        .read_info()
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let size = reader.output_buffer_size().unwrap_or_default();
    let mut rgba = vec![0; size];
    let info = reader
        .next_frame(&mut rgba)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    rgba.truncate(info.buffer_size());
    Image {
        width: info.width,
        height: info.height,
        rgba,
    }
}

/// Whether two renders differ by more than anti-aliasing noise.
fn differs(expected: &Image, actual: &Image) -> bool {
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return true;
    }
    let changed = expected
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(actual.rgba.as_chunks::<4>().0)
        .filter(|(a, b)| {
            a.iter()
                .zip(b.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count();
    let pixels = (expected.width as usize) * (expected.height as usize);
    changed as f64 > pixels as f64 * CHANGED_SHARE
}

/// Renders a story and returns the path of the written PNG.
fn render(meta: &Meta, choice: ThemeChoice, out: &Path) -> PathBuf {
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

    // iced appends the renderer name, so the file lands at `<stem>-tiny-skia.png`.
    let stem = file_stem(meta.id, choice);
    let written = out.join(format!("{stem}-tiny-skia.png"));
    let _ = fs::remove_file(&written);
    snapshot
        .matches_image(out.join(format!("{stem}.png")))
        .unwrap_or_else(|error| panic!("{}: {error}", meta.id));
    written
}

#[test]
fn every_story_matches_its_snapshot() {
    iced_cube::theme::set_font(FONT);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let committed = root.join("snapshots");
    let rendered = root.join("../../target/snapshots-rendered");
    let failed = root.join("../../target/snapshots-failed");
    fs::create_dir_all(&rendered).unwrap_or_else(|error| panic!("{error}"));
    let mut mismatches = Vec::new();

    for meta in ALL {
        for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
            let actual_path = render(meta, choice, &rendered);
            let Some(name) = actual_path.file_name() else {
                continue;
            };
            let expected_path = committed.join(name);

            if !expected_path.exists() {
                fs::copy(&actual_path, &expected_path).unwrap_or_else(|error| panic!("{error}"));
                continue;
            }
            if differs(&read_png(&expected_path), &read_png(&actual_path)) {
                fs::create_dir_all(&failed).unwrap_or_else(|error| panic!("{error}"));
                let _ = fs::copy(&actual_path, failed.join(name));
                mismatches.push(format!("{} ({choice:?})", meta.id));
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "snapshots changed: {mismatches:?}; see target/snapshots-failed/"
    );
}

#[cfg(test)]
mod tolerance {
    use super::*;

    fn image(pixels: &[[u8; 4]], width: u32) -> Image {
        Image {
            width,
            height: pixels.len() as u32 / width,
            rgba: pixels.concat(),
        }
    }

    #[test]
    fn identical_images_do_not_differ() {
        let a = image(&[[10, 20, 30, 255]; 1000], 100);
        assert!(!differs(&a, &image(&[[10, 20, 30, 255]; 1000], 100)));
    }

    #[test]
    fn small_channel_noise_is_ignored() {
        let a = image(&[[100, 100, 100, 255]; 1000], 100);
        let b = image(&[[110, 90, 100, 255]; 1000], 100);
        assert!(!differs(&a, &b));
    }

    #[test]
    fn a_few_changed_pixels_are_tolerated() {
        let a = image(&[[0, 0, 0, 255]; 1000], 100);
        let mut pixels = [[0, 0, 0, 255]; 1000];
        pixels[0] = [255, 255, 255, 255];
        assert!(!differs(&a, &image(&pixels, 100)));
    }

    #[test]
    fn a_real_change_is_caught() {
        let a = image(&[[0, 0, 0, 255]; 1000], 100);
        let mut pixels = [[0, 0, 0, 255]; 1000];
        for pixel in pixels.iter_mut().take(50) {
            *pixel = [255, 255, 255, 255];
        }
        assert!(differs(&a, &image(&pixels, 100)));
    }

    #[test]
    fn a_size_change_is_caught() {
        let a = image(&[[0, 0, 0, 255]; 1000], 100);
        assert!(differs(&a, &image(&[[0, 0, 0, 255]; 1000], 50)));
    }
}
