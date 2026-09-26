use iced::Element;
use iced::widget::{row, text};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, card};
use iced_test::simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Cancel,
    Save,
}

fn view() -> Element<'static, Message> {
    card()
        .title("Profile")
        .description("Update your public details.")
        .body(text("Display name"))
        .footer(row![
            button("Cancel")
                .variant(Variant::Outline)
                .on_press(Message::Cancel),
            button("Save").on_press(Message::Save),
        ])
        .into()
}

#[test]
fn header_and_body_are_rendered() {
    let mut ui = simulator(view());
    assert!(ui.find("Profile").is_ok());
    assert!(ui.find("Update your public details.").is_ok());
    assert!(ui.find("Display name").is_ok());
}

#[test]
fn footer_buttons_emit_their_messages() {
    let mut ui = simulator(view());
    ui.click("Save").expect("Save is rendered");
    ui.click("Cancel").expect("Cancel is rendered");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Save, Message::Cancel]);
}

#[test]
fn card_without_header_renders_body() {
    let element: Element<'_, Message> = card().body(text("Only body")).into();
    let mut ui = simulator(element);
    assert!(ui.find("Only body").is_ok());
}
