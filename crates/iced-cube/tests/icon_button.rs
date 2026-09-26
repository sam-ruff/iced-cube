#![cfg(feature = "icon-button")]

use std::path::Path;

use iced::widget::{self, container, row};
use iced::{Element, Event, Length, Theme, mouse};
use iced_cube::overlay::tooltip::Position;
use iced_cube::primitives::button::Size;
use iced_cube::primitives::icon_button::Variant;
use iced_cube::theme::{Tokens, dark, light};
use iced_cube::{icon_button, lucide};
use iced_test::simulator;
use iced_test::simulator::Simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Bold,
    Italic,
}

fn bold() -> widget::Id {
    widget::Id::new("bold")
}

fn italic() -> widget::Id {
    widget::Id::new("italic")
}

fn toolbar(enabled: bool) -> Element<'static, Message> {
    row![
        icon_button(lucide!(Bold))
            .label("Bold")
            .pressed(true)
            .id(bold())
            .on_press(Message::Bold),
        icon_button(lucide!(Italic))
            .label("Italic")
            .id(italic())
            .on_press_maybe(enabled.then_some(Message::Italic)),
    ]
    .into()
}

#[test]
fn clicking_emits_the_message_whether_pressed_or_not() -> Result<(), iced_test::Error> {
    let mut ui = simulator(toolbar(true));
    ui.click(bold())?;
    ui.click(italic())?;

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Bold, Message::Italic]);
    Ok(())
}

#[test]
fn disabled_icon_button_emits_nothing() -> Result<(), iced_test::Error> {
    let mut ui = simulator(toolbar(false));
    ui.click(italic())?;

    assert_eq!(ui.into_messages().count(), 0);
    Ok(())
}

/// Hovers the button and returns a hash of the rendered window.
///
/// iced's tooltip overlay cannot be found by text, so the tests compare
/// renders instead.
fn hovered_render(element: Element<'_, Message>, name: &str) -> Result<String, iced_test::Error> {
    let mut ui = simulator(element);
    let centre = ui.find(bold())?.bounds().center();
    ui.point_at(centre);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: centre })]);

    let directory =
        std::env::temp_dir().join(format!("iced-cube-icon-button-{}", std::process::id()));
    let path = directory.join(name);
    let _ = ui
        .snapshot(&iced_cube::theme::light())?
        .matches_hash(&path)?;
    let Some(file) = std::fs::read_dir(&directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|file| file.to_string_lossy().contains(name))
    else {
        return Ok(String::new());
    };
    let hash = std::fs::read_to_string(&file)?;
    std::fs::remove_file(file)?;
    Ok(hash)
}

fn labelled(tooltip: Option<Position>, label: bool) -> Element<'static, Message> {
    let button = icon_button(lucide!(Bold))
        .tooltip(tooltip)
        .id(bold())
        .on_press(Message::Bold);
    let button = if label { button.label("Bold") } else { button };
    container(button).center(Length::Fill).into()
}

#[test]
fn label_shows_as_a_tooltip_on_hover_unless_turned_off() -> Result<(), iced_test::Error> {
    let with_tooltip = hovered_render(labelled(Some(Position::Top), true), "with-tooltip")?;
    let turned_off = hovered_render(labelled(None, true), "turned-off")?;
    let unlabelled = hovered_render(labelled(Some(Position::Top), false), "unlabelled")?;

    assert!(!with_tooltip.is_empty());
    assert_ne!(with_tooltip, turned_off);
    assert_eq!(turned_off, unlabelled);
    Ok(())
}

/// Renders a lone ghost icon button and returns how far its most distinct
/// pixel is from the background, as the largest difference in any channel.
fn icon_contrast(theme: &Theme, enabled: bool, name: &str) -> Result<u8, iced_test::Error> {
    let background = Tokens::of(theme).background;
    let element: Element<'_, Message> = container(
        icon_button(lucide!(Plus))
            .tooltip(None)
            .on_press_maybe(enabled.then_some(Message::Bold)),
    )
    .center(Length::Fill)
    .style(move |_| container::Style::default().background(background))
    .into();
    let mut ui = Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(64.0, 64.0),
        element,
    );

    let directory =
        std::env::temp_dir().join(format!("iced-cube-icon-contrast-{}", std::process::id()));
    let stem = directory.join(name);
    let _ = ui.snapshot(theme)?.matches_image(&stem)?;
    let written = directory.join(format!("{name}-tiny-skia.png"));
    let pixels = read_rgba(&written)?;
    std::fs::remove_file(written)?;

    let Some(corner) = pixels.first().copied() else {
        return Ok(0);
    };
    Ok(pixels
        .iter()
        .map(|pixel| {
            (0..3)
                .map(|channel| pixel[channel].abs_diff(corner[channel]))
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0))
}

fn read_rgba(path: &Path) -> Result<Vec<[u8; 4]>, iced_test::Error> {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path)?));
    let mut reader = decoder.read_info()?;
    let mut bytes = vec![0; reader.output_buffer_size().unwrap_or_default()];
    let info = reader.next_frame(&mut bytes)?;
    bytes.truncate(info.buffer_size());
    Ok(bytes.as_chunks::<4>().0.to_vec())
}

#[test]
fn a_disabled_icon_is_drawn_lighter_than_an_enabled_one() -> Result<(), iced_test::Error> {
    for (theme, name) in [(light(), "light"), (dark(), "dark")] {
        let enabled = icon_contrast(&theme, true, &format!("enabled-{name}"))?;
        let disabled = icon_contrast(&theme, false, &format!("disabled-{name}"))?;

        assert!(enabled > 150, "{name}: enabled icon contrast {enabled}");
        assert!(
            disabled > 40,
            "{name}: disabled icon still visible, {disabled}"
        );
        assert!(
            f32::from(disabled) < f32::from(enabled) * 0.7,
            "{name}: disabled {disabled} should be well below enabled {enabled}"
        );
    }
    Ok(())
}

#[test]
fn every_variant_and_size_renders_square() -> Result<(), iced_test::Error> {
    for variant in Variant::ALL {
        for size in Size::ALL {
            for pressed in [false, true] {
                let element: Element<'_, Message> = icon_button(lucide!(Plus))
                    .variant(variant)
                    .size(size)
                    .pressed(pressed)
                    .id(bold())
                    .on_press(Message::Bold)
                    .into();
                let mut ui = simulator(element);
                let bounds = ui.find(bold())?.bounds();
                assert_eq!(bounds.width, size.metrics().height, "{variant:?} {size:?}");
                assert_eq!(bounds.height, bounds.width, "{variant:?} {size:?}");
            }
        }
    }
    Ok(())
}
