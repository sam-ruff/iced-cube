#![cfg(feature = "select")]

use iced::keyboard::key::Named;
use iced::widget::{button, column, container, text};
use iced::{Element, Event, Length, Point, mouse};
use iced_cube::forms::select;
use iced_cube::forms::select::HEIGHT;
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Fruit(&'static str),
    Other,
}

const FRUITS: &[&str] = &["Apple", "Banana", "Cherry"];

fn view(selected: Option<&'static str>, enabled: bool) -> Element<'static, Message> {
    let select = select(FRUITS, selected).placeholder("Pick a fruit");
    let select: Element<'static, Message> = if enabled {
        select.on_select(Message::Fruit).into()
    } else {
        select.into()
    };
    column![
        select,
        container(button(text("Other")).on_press(Message::Other))
            .padding([240, 0])
            .width(Length::Fill),
    ]
    .into()
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    ui.simulate(click());
}

fn open(ui: &mut Simulator<'_, Message>) {
    click_at(ui, Point::new(100.0, HEIGHT / 2.0));
}

#[test]
fn clicking_the_field_opens_the_list() {
    let mut ui = simulator(view(None, true));
    assert!(ui.find("Pick a fruit").is_ok());
    assert!(ui.find("Banana").is_err());
    open(&mut ui);
    for fruit in FRUITS {
        assert!(ui.find(*fruit).is_ok(), "{fruit}");
    }
    assert!(ui.into_messages().next().is_none());
}

#[test]
fn choosing_an_option_emits_it_and_closes_the_list() {
    let mut ui = simulator(view(None, true));
    open(&mut ui);
    let banana = ui.find("Banana").expect("list is open");
    assert!(banana.bounds().y > HEIGHT, "the list opens below the field");
    ui.click("Banana").expect("option is rendered");
    assert!(ui.find("Cherry").is_err(), "the list closes after a choice");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Fruit("Banana")]);
}

#[test]
fn escape_and_outside_clicks_close_the_list_without_choosing() {
    let mut ui = simulator(view(Some("Apple"), true));
    open(&mut ui);
    let _ = ui.tap_key(Named::Escape);
    assert!(ui.find("Banana").is_err());

    open(&mut ui);
    click_at(&mut ui, Point::new(600.0, 150.0));
    assert!(ui.find("Banana").is_err());

    open(&mut ui);
    ui.click("Other").expect("button below is rendered");
    assert!(ui.find("Banana").is_err());
    assert_eq!(
        ui.into_messages().count(),
        0,
        "the click that closes the list goes no further"
    );
}

#[test]
fn clicking_the_field_again_closes_the_list() {
    let mut ui = simulator(view(None, true));
    open(&mut ui);
    assert!(ui.find("Cherry").is_ok());
    open(&mut ui);
    assert!(ui.find("Cherry").is_err());
}

#[test]
fn disabled_select_does_not_open() {
    let mut ui = simulator(view(Some("Apple"), false));
    open(&mut ui);
    assert!(ui.find("Banana").is_err());
    assert_eq!(ui.into_messages().count(), 0);
}
