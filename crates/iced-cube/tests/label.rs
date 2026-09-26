use iced::Element;
use iced::widget::Id;
use iced_cube::forms::label::State;
use iced_cube::{field, input, label};
use iced_test::{Error, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Email(String),
}

fn view(error: Option<&str>) -> Element<'_, Message> {
    field(
        "Email",
        input("you@example.com", "ada")
            .id("email")
            .on_input(Message::Email),
    )
    .description("We only use it for receipts.")
    .error_maybe(error)
    .required(true)
    .into()
}

#[test]
fn field_renders_label_description_and_required_marker() {
    let mut ui = simulator(view(None));
    assert!(ui.find("Email").is_ok());
    assert!(ui.find("*").is_ok());
    assert!(ui.find("We only use it for receipts.").is_ok());
    assert!(ui.find("Enter a valid email").is_err());
}

#[test]
fn field_renders_error_text() {
    let mut ui = simulator(view(Some("Enter a valid email")));
    assert!(ui.find("Enter a valid email").is_ok());
}

#[test]
fn control_inside_a_field_still_emits_messages() -> Result<(), Error> {
    let mut ui = simulator(view(None));
    ui.click(Id::new("email"))?;
    ui.typewrite("!");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Email("ada!".into())]);
    Ok(())
}

#[test]
fn every_label_state_renders() {
    for state in State::ALL {
        let element: Element<'_, Message> = label("Name").state(state).required(true).into();
        let mut ui = simulator(element);
        assert!(ui.find("Name").is_ok(), "{state:?}");
    }
}
