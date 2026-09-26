#![cfg(feature = "icon-button")]

use iced::widget::{self, container, row};
use iced::{Element, Event, Length, mouse};
use iced_cube::overlay::tooltip::Position;
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::{icon_button, lucide};
use iced_test::simulator;

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
