use iced::Element;
use iced::widget::column;
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::{button, icon_button, lucide};
use iced_test::simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Save,
    Delete,
    Add,
}

fn view(enabled: bool) -> Element<'static, Message> {
    column![
        button("Save").on_press(Message::Save),
        button("Delete")
            .variant(Variant::Destructive)
            .on_press_maybe(enabled.then_some(Message::Delete)),
        icon_button(lucide!(Plus))
            .size(Size::Icon)
            .on_press(Message::Add),
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
