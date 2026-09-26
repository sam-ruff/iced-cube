use iced::keyboard::key::Named;
use iced::widget::{Id, column};
use iced::{Element, Point};
use iced_cube::primitives::input::Size;
use iced_cube::{input, lucide};
use iced_test::{Error, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Name(String),
    Search(String),
    Submit,
}

fn view<'a>(name: &'a str, enabled: bool) -> Element<'a, Message> {
    column![
        input("Your name", name)
            .id("name")
            .on_input_maybe(enabled.then_some(Message::Name))
            .on_submit(Message::Submit),
        input("Search", "")
            .id("search")
            .icon(lucide!(Search))
            .on_input(Message::Search),
    ]
    .into()
}

#[test]
fn typing_emits_the_new_value() -> Result<(), Error> {
    let mut ui = simulator(view("Ad", true));
    ui.click(Id::new("name"))?;
    ui.typewrite("a");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Name("Ada".into())]);
    Ok(())
}

#[test]
fn each_keystroke_emits_a_message() -> Result<(), Error> {
    let mut ui = simulator(view("", true));
    ui.click(Id::new("name"))?;
    ui.typewrite("hi");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        vec![Message::Name("h".into()), Message::Name("hi".into())]
    );
    Ok(())
}

#[test]
fn enter_submits() -> Result<(), Error> {
    let mut ui = simulator(view("Ada", true));
    ui.click(Id::new("name"))?;
    ui.tap_key(Named::Enter);

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Submit]);
    Ok(())
}

#[test]
fn disabled_input_emits_nothing() -> Result<(), Error> {
    let mut ui = simulator(view("Ada", false));
    ui.click(Id::new("name"))?;
    ui.typewrite("x");
    ui.tap_key(Named::Enter);

    assert_eq!(ui.into_messages().count(), 0);
    Ok(())
}

#[test]
fn clicking_the_icon_focuses_the_input() -> Result<(), Error> {
    let mut ui = simulator(view("", true));
    let bounds = ui.find(Id::new("search"))?.bounds();
    ui.point_at(Point::new(bounds.x + 18.0, bounds.center_y()));
    let _ = ui.simulate(iced_test::simulator::click());
    ui.typewrite("q");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Search("q".into())]);
    Ok(())
}

#[test]
fn value_is_rendered_and_found() {
    let mut ui = simulator(view("Grace", true));
    assert!(ui.find("Grace").is_ok());
}

#[test]
fn every_size_renders_with_and_without_icon() {
    for size in Size::ALL {
        for with_icon in [false, true] {
            let mut field = input("Placeholder", "Value")
                .size(size)
                .on_input(Message::Name);
            if with_icon {
                field = field.icon(lucide!(Mail));
            }
            let mut ui = simulator(Element::from(field));
            assert!(ui.find("Value").is_ok(), "{size:?} {with_icon}");
        }
    }
}

#[test]
fn secure_input_still_reports_its_value() -> Result<(), Error> {
    let element: Element<'_, Message> = input("Password", "hunter2")
        .id("password")
        .secure(true)
        .on_input(Message::Name)
        .into();
    let mut ui = simulator(element);
    ui.click(Id::new("password"))?;
    ui.typewrite("!");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Name("hunter2!".into())]);
    Ok(())
}
