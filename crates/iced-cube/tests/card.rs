#![cfg(all(feature = "card", feature = "button"))]

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

// A text node's bounds are clamped to the room it is given, so a word that
// runs past the border still reports one line. Breaking it adds lines.
#[test]
fn a_word_wider_than_the_card_breaks_instead_of_running_past_the_border() {
    let element: Element<'_, Message> = row![
        card()
            .title("Subscribers")
            .description("Subscriptions")
            .width(90)
    ]
    .into();
    let mut ui = simulator(element);

    let title = ui.find("Subscribers").expect("title is rendered").bounds();
    assert!(title.width <= 90.0 - 48.0, "title width {}", title.width);
    assert!(title.height > 40.0, "title height {}", title.height);

    let description = ui.find("Subscriptions").expect("description is rendered");
    assert!(description.bounds().height > 30.0);
}

#[test]
fn short_headers_keep_one_line() {
    let element: Element<'_, Message> = row![card().title("Profile").width(200)].into();
    let mut ui = simulator(element);
    let title = ui.find("Profile").expect("title is rendered").bounds();
    assert!(title.height < 30.0, "title height {}", title.height);
}

#[test]
fn card_without_header_renders_body() {
    let element: Element<'_, Message> = card().body(text("Only body")).into();
    let mut ui = simulator(element);
    assert!(ui.find("Only body").is_ok());
}

#[test]
fn a_link_in_the_footer_lines_up_with_the_body() {
    let element: Element<'_, Message> = card()
        .title("Running now")
        .body(text("Body text"))
        .footer(
            button("View all")
                .variant(Variant::Link)
                .on_press(Message::Save),
        )
        .into();
    let mut ui = simulator(element);
    let body = ui.find("Body text").expect("body is rendered").bounds().x;
    let link = ui.find("View all").expect("link is rendered").bounds().x;
    assert!((body - link).abs() < 0.5, "body at {body}, link at {link}");
}

#[cfg(feature = "stack")]
#[test]
fn a_stack_body_with_a_filling_row_fills_the_card() {
    use iced::Length;
    use iced::widget::space;
    use iced_cube::{hstack, vstack};

    let element: Element<'_, Message> = card()
        .body(vstack([
            hstack([
                text("Left").into(),
                space().width(Length::Fill).into(),
                text("Right").into(),
            ])
            .into(),
            text("Below").into(),
        ]))
        .width(400)
        .into();
    let mut ui = simulator(element);
    let right = ui.find("Right").expect("row is rendered").bounds();
    // 24 pixels of card padding and a 1 pixel border on each side.
    assert!(
        right.x + right.width > 400.0 - 24.0 - 2.0,
        "the row reaches the card's right padding: {right:?}"
    );
}
