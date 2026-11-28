#![cfg(feature = "button")]

use iced::widget::column;
use iced::{Alignment, Element, Length};
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::{button, lucide};
use iced_test::simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Save,
    Delete,
}

fn view(enabled: bool) -> Element<'static, Message> {
    column![
        button("Save").on_press(Message::Save),
        button("Delete")
            .variant(Variant::Destructive)
            .on_press_maybe(enabled.then_some(Message::Delete)),
    ]
    .into()
}

#[test]
fn clicking_a_label_emits_its_message() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(true));
    ui.click("Save")?;
    ui.click("Delete")?;

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Save, Message::Delete]);
    Ok(())
}

#[test]
fn disabled_button_emits_nothing() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(false));
    ui.click("Delete")?;

    assert_eq!(ui.into_messages().count(), 0);
    Ok(())
}

#[test]
fn label_stays_on_one_line_in_a_narrow_container() -> Result<(), iced_test::Error> {
    let element: Element<'_, Message> = column![
        button("Save changes")
            .icon(lucide!(Save))
            .on_press(Message::Save)
    ]
    .width(40)
    .into();
    let mut ui = simulator(element);

    let bounds = ui.find("Save changes")?.bounds();
    assert!(bounds.height < 24.0, "one line, got {}", bounds.height);
    assert!(
        bounds.width > 40.0,
        "sized to its label, got {}",
        bounds.width
    );

    ui.click("Save changes")?;
    assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![Message::Save]);
    Ok(())
}

#[test]
fn fill_width_button_keeps_the_container_width() -> Result<(), iced_test::Error> {
    let element: Element<'_, Message> =
        column![button("Save").width(Length::Fill).on_press(Message::Save)]
            .width(300)
            .into();
    let mut ui = simulator(element);

    let label = ui.find("Save")?.bounds();
    let centre = label.x + label.width / 2.0;
    assert!(
        (centre - 150.0).abs() < 1.0,
        "label centred in 300px, got {centre}"
    );
    Ok(())
}

#[test]
fn start_aligned_buttons_line_their_labels_up_on_the_left() -> Result<(), iced_test::Error> {
    let padding = Size::Md.metrics().padding_x;
    let element: Element<'_, Message> = column![
        button("General")
            .variant(Variant::Ghost)
            .width(Length::Fill)
            .align_x(Alignment::Start)
            .on_press(Message::Save),
        button("Notifications")
            .variant(Variant::Ghost)
            .icon(lucide!(Bell))
            .width(Length::Fill)
            .align_x(Alignment::Start)
            .on_press(Message::Delete),
        button("End")
            .width(Length::Fill)
            .align_x(Alignment::End)
            .on_press(Message::Save),
    ]
    .width(300)
    .into();
    let mut ui = simulator(element);

    let general = ui.find("General")?.bounds();
    assert!(
        (general.x - padding).abs() < 1.0,
        "label starts after the padding, got {}",
        general.x
    );
    let notifications = ui.find("Notifications")?.bounds();
    let icon_and_gap = Size::Md.metrics().icon + 8.0;
    assert!(
        (notifications.x - padding - icon_and_gap).abs() < 1.0,
        "label follows the icon, got {}",
        notifications.x
    );
    let end = ui.find("End")?.bounds();
    assert!(
        (end.x + end.width - (300.0 - padding)).abs() < 1.0,
        "label ends before the padding, got {}",
        end.x + end.width
    );

    ui.click("Notifications")?;
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Delete]
    );
    Ok(())
}

#[test]
fn every_variant_and_size_renders() {
    for variant in Variant::ALL {
        for size in Size::ALL {
            let element: Element<'_, Message> = button("Label")
                .variant(variant)
                .size(size)
                .on_press(Message::Save)
                .into();
            let mut ui = simulator(element);
            assert!(ui.find("Label").is_ok(), "{variant:?} {size:?}");
        }
    }
}

#[cfg(feature = "icedwright")]
#[path = "support/semantic.rs"]
mod semantic;

#[cfg(feature = "icedwright")]
#[test]
fn buttons_report_their_role_name_and_disabled_state() {
    assert_eq!(
        semantic::yaml(view(false)),
        "- button \"Save\" [focused=false] [focusable]:\n  - text: Save\n\
         - button \"Delete\" [disabled]:\n  - text: Delete\n"
    );
}
