#![cfg(feature = "chip")]

use iced::widget::{column, container, row, text};
use iced::{Element, widget};
use iced_cube::feedback::chip::{Size, Variant, chip, remove_id};
use iced_cube::lucide;
use iced_test::{Simulator, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Remove(&'static str),
}

#[test]
fn every_variant_and_size_renders_its_label() {
    for variant in Variant::ALL {
        for size in Size::ALL {
            let element: Element<'_, Message> = chip("Ada").variant(variant).size(size).into();
            let mut ui = simulator(element);
            assert!(ui.find("Ada").is_ok(), "{variant:?} {size:?}");
        }
    }
}

#[test]
fn clicking_the_x_sends_the_remove_message() {
    let element: Element<'_, Message> = row![
        chip("Ada").id("ada").on_remove(Message::Remove("Ada")),
        chip("Grace")
            .id("grace")
            .on_remove(Message::Remove("Grace")),
    ]
    .spacing(4)
    .into();
    let mut ui = simulator(element);
    ui.click(remove_id("grace")).expect("x is rendered");
    let messages: Vec<Message> = ui.into_messages().collect();
    assert_eq!(messages, [Message::Remove("Grace")]);
}

#[test]
fn clicking_the_label_does_not_remove() {
    let element: Element<'_, Message> = chip("Ada")
        .id("ada")
        .on_remove(Message::Remove("Ada"))
        .into();
    let mut ui = simulator(element);
    ui.click("Ada").expect("label is rendered");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn a_chip_without_on_remove_has_no_x() {
    let element: Element<'_, Message> = chip("Ada").id("ada").into();
    let mut ui = simulator(element);
    assert!(ui.find("Ada").is_ok());
    assert!(ui.find(remove_id("ada")).is_err());
}

#[test]
fn the_x_sits_after_the_label() {
    let element: Element<'_, Message> = chip("Ada")
        .id("ada")
        .on_remove(Message::Remove("Ada"))
        .into();
    let mut ui = simulator(element);
    let label = ui.find("Ada").expect("label").bounds();
    let x = ui.find(remove_id("ada")).expect("x").bounds();
    assert!(x.x >= label.x + label.width);
    let middle = |b: iced::Rectangle| b.y + b.height / 2.0;
    assert!((middle(x) - middle(label)).abs() < 1.5, "centred together");
}

#[test]
fn icon_and_leading_element_render_before_the_label() {
    let element: Element<'_, Message> = column![
        chip("Ada").icon(lucide!(User)),
        chip("Grace").leading(text("G")),
    ]
    .into();
    let mut ui = simulator(element);
    let leading = ui.find("G").expect("leading element").bounds();
    let label = ui.find("Grace").expect("label").bounds();
    assert!(leading.x + leading.width <= label.x);
    assert!(ui.find("Ada").is_ok());
}

fn chip_bounds(size: Size, removable: bool) -> Option<iced::Rectangle> {
    let chip = chip("Ada")
        .size(size)
        .on_remove_maybe(removable.then_some(Message::Remove("Ada")));
    let element: Element<'_, Message> = container(chip).id("chip").into();
    let mut ui = simulator(element);
    ui.find(widget::Id::new("chip"))
        .ok()
        .map(|target| target.bounds())
}

#[test]
fn heights_follow_the_size_with_or_without_the_x() {
    for size in Size::ALL {
        let plain = chip_bounds(size, false).expect("chip is rendered");
        let removable = chip_bounds(size, true).expect("chip is rendered");
        assert_eq!(plain.height, size.metrics().height, "{size:?}");
        assert_eq!(removable.height, size.metrics().height, "{size:?}");
        assert!(removable.width > plain.width, "{size:?}");
    }
}

fn narrow(content: Element<'_, Message>) -> Simulator<'_, Message> {
    simulator(column![content].width(40))
}

#[test]
fn label_stays_on_one_line_in_a_narrow_container() {
    let label = "Margaret Hamilton";
    let mut ui = narrow(chip(label).on_remove(Message::Remove("M")).into());
    let bounds = ui.find(label).expect("label").bounds();
    assert!(bounds.height < 20.0, "one line, got {}", bounds.height);
    assert!(bounds.width > 40.0, "sized to its label");
}

#[test]
fn chips_in_a_wrapping_row_move_onto_new_lines() {
    let names = ["Ada Lovelace", "Grace Hopper", "Alan Turing", "Linus"];
    let chips = row(names.map(|name| chip(name).on_remove(Message::Remove(name)).into()))
        .spacing(4)
        .wrap()
        .vertical_spacing(4);
    let mut ui = simulator(column![chips].width(200));
    let mut lines: Vec<f32> = Vec::new();
    for name in names {
        let bounds = ui.find(name).expect("chip").bounds();
        assert!(bounds.x + bounds.width <= 200.0, "{name} fits across");
        if !lines.contains(&bounds.y) {
            lines.push(bounds.y);
        }
    }
    assert!(lines.len() > 1, "wrapped onto {} lines", lines.len());
}

#[cfg(feature = "icedwright")]
#[path = "support/semantic.rs"]
mod semantic;

#[cfg(feature = "icedwright")]
#[test]
fn a_chip_reports_a_group_with_its_remove_button() {
    let element: Element<'_, Message> = chip("Rust").on_remove(Message::Remove("rust")).into();
    assert_eq!(
        semantic::yaml(element),
        "- group \"Rust\":\n  - text: Rust\n  - button \"Remove Rust\"\n"
    );
}
