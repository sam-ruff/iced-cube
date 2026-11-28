#![cfg(feature = "input")]

mod support;

use iced::keyboard::key::Named;
use iced::widget::{Id, column};
use iced::{Element, Point};
use iced_cube::primitives::input::{Size, clear_id};
use iced_cube::{input, lucide};
use iced_test::{Error, simulator};
use support::{COMMAND, Harness, letter};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Clear,
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

fn clearable(value: &str, enabled: bool) -> Element<'_, Message> {
    input("Search", value)
        .id("search")
        .icon(lucide!(Search))
        .clearable(Message::Clear)
        .on_input_maybe(enabled.then_some(Message::Name))
        .width(300)
        .into()
}

#[test]
fn the_clear_button_emits_its_message() -> Result<(), Error> {
    let mut ui = simulator(clearable("query", true));
    let button = ui.find(clear_id())?.bounds();
    assert!(button.x > 250.0, "drawn at the end, got {}", button.x);
    ui.click(clear_id())?;

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Clear]);
    Ok(())
}

#[test]
fn the_clear_button_hides_while_empty_or_disabled() {
    let mut ui = simulator(clearable("", true));
    assert!(ui.find(clear_id()).is_err());
    let mut ui = simulator(clearable("query", false));
    assert!(ui.find(clear_id()).is_err());
}

#[test]
fn the_clear_button_sits_in_the_room_the_field_leaves_for_it() -> Result<(), Error> {
    let mut ui = simulator(clearable("query", true));
    let field = ui.find(Id::new("search"))?.bounds();
    let button = ui.find(clear_id())?.bounds();
    let metrics = Size::Md.metrics();
    let right = field.x + field.width;
    let room = metrics.padding_x + metrics.icon + 8.0;
    assert!(button.x >= right - room, "starts at {}", button.x);
    assert!(button.x + button.width <= right, "ends inside the field");
    assert!(
        (button.center_y() - field.center_y()).abs() < 1.0,
        "centred"
    );
    Ok(())
}

#[derive(Debug, Default)]
struct Form {
    value: String,
    max_length: usize,
}

fn form_view(form: &Form) -> Element<'_, Message> {
    input("Code", &form.value)
        .id("code")
        .max_length(form.max_length)
        .clearable(Message::Clear)
        .on_input(Message::Name)
        .into()
}

fn form_update(form: &mut Form, message: Message) {
    match message {
        Message::Name(value) => form.value = value,
        Message::Clear => form.value.clear(),
        Message::Search(_) | Message::Submit => {}
    }
}

fn form(value: &str, max_length: usize) -> Harness<Form, Message> {
    let mut ui = Harness::new(
        Form {
            value: value.to_owned(),
            max_length,
        },
        form_view,
        form_update,
    );
    ui.focus("code");
    ui
}

#[test]
fn typing_stops_at_the_limit() {
    let mut ui = form("", 4);
    ui.type_text("abcdef");
    assert_eq!(ui.state.value, "abcd");
}

#[test]
fn the_limit_counts_characters_not_bytes() {
    let mut ui = form("", 3);
    ui.type_text("éàüö");
    assert_eq!(ui.state.value, "éàü");
}

#[test]
fn typing_in_the_middle_at_the_limit_keeps_the_caret_in_place() {
    let mut ui = form("abcde", 5);
    ui.tap(Named::Home);
    ui.type_text("XY");
    assert_eq!(ui.state.value, "abcde");
    ui.tap(Named::Delete);
    ui.type_text("Z");
    assert_eq!(ui.state.value, "Zbcde");
}

#[test]
fn a_paste_is_cut_to_what_fits() {
    let mut ui = form("ab", 5);
    ui.clipboard.0 = Some("123456".into());
    ui.tap(Named::Home);
    ui.press(COMMAND, letter('v'));
    assert_eq!(ui.state.value, "123ab");
    ui.type_text("x");
    assert_eq!(
        ui.state.value, "123ab",
        "the caret sits after the pasted text"
    );
}

#[test]
fn a_paste_over_a_selection_uses_the_room_it_frees() {
    let mut ui = form("abcde", 5);
    ui.clipboard.0 = Some("123456".into());
    ui.press(COMMAND, letter('a'));
    ui.press(COMMAND, letter('v'));
    assert_eq!(ui.state.value, "12345");
}

#[test]
fn undo_and_redo_stay_within_the_limit() {
    let mut ui = form("", 5);
    ui.type_text("abc");
    ui.clipboard.0 = Some("123456".into());
    ui.press(COMMAND, letter('v'));
    assert_eq!(ui.state.value, "abc12");
    ui.press(COMMAND, letter('z'));
    assert_eq!(ui.state.value, "abc");
    ui.press(COMMAND, letter('y'));
    assert_eq!(ui.state.value, "abc12");
}

#[test]
fn a_longer_value_from_the_app_can_shrink_but_not_grow() {
    let mut ui = form("abcdef", 3);
    ui.tap(Named::End);
    ui.type_text("g");
    assert_eq!(ui.state.value, "abcdef");
    ui.tap(Named::Backspace);
    assert_eq!(ui.state.value, "abcde");
}

#[test]
fn a_clearable_field_keeps_focus_as_the_button_appears() {
    let mut ui = form("", 10);
    ui.type_text("ab");
    assert_eq!(ui.state.value, "ab");
    form_update(&mut ui.state, Message::Clear);
    ui.type_text("c");
    assert_eq!(ui.state.value, "c");
}

#[cfg(feature = "icedwright")]
#[path = "support/semantic.rs"]
mod semantic;

#[cfg(feature = "icedwright")]
#[test]
fn inputs_report_value_placeholder_and_states() {
    let element: Element<'_, Message> = column![
        input("Your name", "Ada")
            .on_input(Message::Name)
            .clearable(Message::Clear),
        input("Search", "").invalid(true),
        input("Password", "secret")
            .secure(true)
            .on_input(Message::Name),
    ]
    .into();
    assert_eq!(
        semantic::yaml(element),
        r#"- textbox "Your name" [focused=false] [focusable] [placeholder="Your name"] [prop:caret-clock="iced"]: Ada
- button "Clear"
- textbox "Search" [disabled] [focused=false] [invalid] [focusable] [placeholder="Search"] [prop:caret-clock="iced"]: ""
- password "Password" [focused=false] [focusable] [placeholder="Password"] [prop:caret-clock="iced"]: ••••••
"#
    );
}
