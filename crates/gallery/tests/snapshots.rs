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

type Ui<'a> = Simulator<'a, gallery::Message>;

fn simulate<'a>(gallery: &'a Gallery, meta: &Meta) -> Ui<'a> {
    let settings = iced::Settings {
        default_font: FONT,
        fonts: FONT_FILES.iter().map(|font| (*font).into()).collect(),
        ..iced::Settings::default()
    };
    Simulator::with_size(settings, snapshot_size(meta), gallery.view())
}

/// Snapshots what `ui` shows and returns the path of the written PNG.
fn capture(ui: &mut Ui<'_>, gallery: &Gallery, stem: &str, out: &Path) -> PathBuf {
    let snapshot = ui
        .snapshot(&gallery.theme())
        .unwrap_or_else(|error| panic!("{stem}: {error}"));
    // iced appends the renderer name, so the file lands at `<stem>-tiny-skia.png`.
    let written = out.join(format!("{stem}-tiny-skia.png"));
    let _ = fs::remove_file(&written);
    snapshot
        .matches_image(out.join(format!("{stem}.png")))
        .unwrap_or_else(|error| panic!("{stem}: {error}"));
    written
}

/// Renders a story and returns the path of the written PNG.
fn render(meta: &Meta, choice: ThemeChoice, out: &Path) -> PathBuf {
    let gallery = Gallery::new(Mode::Single, Some(meta.id), choice);
    let mut ui = simulate(&gallery, meta);
    capture(&mut ui, &gallery, &file_stem(meta.id, choice), out)
}

struct Folders {
    committed: PathBuf,
    rendered: PathBuf,
    failed: PathBuf,
}

impl Folders {
    fn new() -> Self {
        iced_cube::theme::set_font(FONT);
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let folders = Self {
            committed: root.join("snapshots"),
            rendered: root.join("../../target/snapshots-rendered"),
            failed: root.join("../../target/snapshots-failed"),
        };
        fs::create_dir_all(&folders.rendered).unwrap_or_else(|error| panic!("{error}"));
        folders
    }

    /// Compares a render with its committed PNG, writing the PNG when
    /// there is none yet. Returns whether they differ.
    fn differs(&self, actual_path: &Path) -> bool {
        let Some(name) = actual_path.file_name() else {
            return false;
        };
        let expected_path = self.committed.join(name);
        if !expected_path.exists() {
            fs::copy(actual_path, &expected_path).unwrap_or_else(|error| panic!("{error}"));
            return false;
        }
        if !differs(&read_png(&expected_path), &read_png(actual_path)) {
            return false;
        }
        fs::create_dir_all(&self.failed).unwrap_or_else(|error| panic!("{error}"));
        let _ = fs::copy(actual_path, self.failed.join(name));
        true
    }
}

#[test]
fn every_story_matches_its_snapshot() {
    let folders = Folders::new();
    let mut mismatches = Vec::new();

    for meta in ALL {
        for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
            let actual_path = render(meta, choice, &folders.rendered);
            if folders.differs(&actual_path) {
                mismatches.push(format!("{} ({choice:?})", meta.id));
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "snapshots changed: {mismatches:?}; see target/snapshots-failed/"
    );
}

/// A story caught after some input, such as an open select.
struct Scene {
    story: &'static str,
    name: &'static str,
    actions: fn(&mut Ui<'_>),
    /// Hands what the actions sent to the story before the snapshot.
    apply: bool,
}

fn open_select(ui: &mut Ui<'_>, showing: &str) {
    if let Err(error) = ui.click(showing) {
        panic!("{showing} cannot be clicked: {error:?}");
    }
}

const SCENES: &[Scene] = &[
    Scene {
        story: "select/long-list",
        name: "open",
        actions: |ui| open_select(ui, "14:00"),
        apply: false,
    },
    Scene {
        story: "select/near-bottom",
        name: "open",
        actions: |ui| open_select(ui, "Norway"),
        apply: false,
    },
    Scene {
        story: "select/searchable",
        name: "filtered",
        actions: |ui| {
            open_select(ui, "Select a timezone");
            let _ = ui.typewrite("cet");
        },
        apply: false,
    },
    Scene {
        story: "select/custom-value",
        name: "typed",
        actions: |ui| {
            open_select(ui, "09:00");
            let _ = ui.typewrite("9:40");
        },
        apply: false,
    },
    Scene {
        story: "select/custom-value",
        name: "accepted",
        actions: |ui| {
            open_select(ui, "09:00");
            let _ = ui.typewrite("9:40");
            let _ = ui.tap_key(iced::keyboard::key::Named::Enter);
        },
        apply: true,
    },
    Scene {
        story: "select/custom-value",
        name: "rejected",
        actions: |ui| {
            open_select(ui, "09:00");
            let _ = ui.typewrite("25:00");
            let _ = ui.tap_key(iced::keyboard::key::Named::Enter);
        },
        apply: false,
    },
];

#[test]
fn stories_after_input_match_their_snapshots() {
    let folders = Folders::new();
    let mut mismatches = Vec::new();

    for scene in SCENES {
        let Some(meta) = ALL.iter().find(|meta| meta.id == scene.story) else {
            panic!("{} is not a story", scene.story);
        };
        for choice in [ThemeChoice::Light, ThemeChoice::Dark] {
            let mut gallery = Gallery::new(Mode::Single, Some(meta.id), choice);
            let stem = file_stem(&format!("{}-{}", meta.id, scene.name), choice);
            let messages: Vec<gallery::Message> = {
                let mut ui = simulate(&gallery, meta);
                (scene.actions)(&mut ui);
                if !scene.apply {
                    let actual_path = capture(&mut ui, &gallery, &stem, &folders.rendered);
                    if folders.differs(&actual_path) {
                        mismatches.push(format!("{stem} ({choice:?})"));
                    }
                    continue;
                }
                ui.into_messages().collect()
            };
            assert!(!messages.is_empty(), "{stem}: the input sent nothing");
            for message in messages {
                let _ = gallery.update(message);
            }
            let mut ui = simulate(&gallery, meta);
            let actual_path = capture(&mut ui, &gallery, &stem, &folders.rendered);
            if folders.differs(&actual_path) {
                mismatches.push(format!("{stem} ({choice:?})"));
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
