#![cfg(feature = "spinner")]

use std::path::Path;

use iced::widget::{container, row, text};
use iced::{Element, Length, Theme};
use iced_cube::feedback::spinner::Size;
use iced_cube::spinner;
use iced_cube::theme::{Tokens, dark, light};
use iced_test::simulator;
use iced_test::simulator::Simulator;

const CELL: f32 = 40.0;

#[test]
fn spinner_renders_next_to_its_label() {
    for size in Size::ALL {
        let element: Element<'_, ()> = row![spinner(0.3).size(size), text("Loading")].into();
        let mut ui = simulator(element);
        assert!(ui.find("Loading").is_ok(), "{size:?}");
    }
}

#[test]
fn spinner_draws_in_both_themes() {
    let themes: [Theme; 2] = [light(), dark()];
    for theme in themes {
        let element: Element<'_, ()> = spinner(0.6).size(Size::Lg).into();
        let mut ui = simulator(element);
        assert!(ui.snapshot(&theme).is_ok(), "{theme}");
    }
}

/// Renders one Lg spinner per phase side by side in `CELL` wide cells and
/// returns the pixels with the image width.
fn render(
    theme: &Theme,
    phases: &[f32],
    name: &str,
) -> Result<(Vec<[u8; 4]>, usize), iced_test::Error> {
    let background = Tokens::of(theme).background;
    let cells = phases.iter().map(|&phase| {
        container(spinner(phase).size(Size::Lg))
            .center(Length::Fixed(CELL))
            .into()
    });
    let element: Element<'_, ()> = container(row(cells))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style::default().background(background))
        .into();
    let width = CELL * phases.len() as f32;
    let mut ui = Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(width, CELL),
        element,
    );

    let directory = std::env::temp_dir().join(format!("iced-cube-spinner-{}", std::process::id()));
    let _ = ui.snapshot(theme)?.matches_image(directory.join(name))?;
    let written = directory.join(format!("{name}-tiny-skia.png"));
    let image = read_rgba(&written)?;
    std::fs::remove_file(written)?;
    Ok(image)
}

fn read_rgba(path: &Path) -> Result<(Vec<[u8; 4]>, usize), iced_test::Error> {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path)?));
    let mut reader = decoder.read_info()?;
    let mut bytes = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let info = reader.next_frame(&mut bytes)?;
    bytes.truncate(info.buffer_size());
    Ok((bytes.as_chunks::<4>().0.to_vec(), info.width as usize))
}

/// Counts strongly contrasting pixels in the top and bottom halves of the
/// cell at `index`, which is where the arc is.
fn arc_pixels(pixels: &[[u8; 4]], width: usize, cells: usize, index: usize) -> (usize, usize) {
    let Some(corner) = pixels.first().copied() else {
        return (0, 0);
    };
    let cell = width / cells.max(1);
    let height = pixels.len() / width.max(1);
    let strong = |pixel: &[u8; 4]| (0..3).any(|c| pixel[c].abs_diff(corner[c]) > 160);
    let mut halves = (0, 0);
    for y in 0..height {
        for x in index * cell..(index + 1) * cell {
            let Some(pixel) = pixels.get(y * width + x) else {
                continue;
            };
            if !strong(pixel) {
                continue;
            }
            if y < height / 2 {
                halves.0 += 1;
            } else {
                halves.1 += 1;
            }
        }
    }
    halves
}

#[test]
fn every_spinner_in_a_row_draws_its_arc_where_its_phase_says() -> Result<(), iced_test::Error> {
    for (theme, name) in [(light(), "light"), (dark(), "dark")] {
        let phases = [0.85, 0.35, 0.85, 0.35];
        let (pixels, width) = render(&theme, &phases, name)?;
        for (index, phase) in phases.iter().enumerate() {
            let (top, bottom) = arc_pixels(&pixels, width, phases.len(), index);
            assert!(top + bottom > 20, "{name} spinner {index} drew no arc");
            if *phase > 0.5 {
                assert!(
                    top > bottom * 3,
                    "{name} spinner {index}: {top} top, {bottom} bottom"
                );
            } else {
                assert!(
                    bottom > top * 3,
                    "{name} spinner {index}: {top} top, {bottom} bottom"
                );
            }
        }
    }
    Ok(())
}
