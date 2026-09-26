#![cfg(feature = "popover")]

use iced::keyboard::{Key, key::Named};
use iced::widget::{Button, column, text};
use iced::{Element, Length, Point};
use iced_cube::Chord;
use iced_cube::overlay::popover::{Action, Side, default_keymap, popover};
use iced_test::simulator::{self, Simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Toggle,
    Dismiss,
    Apply,
    Behind,
}

fn button(label: &'static str) -> Button<'static, Message> {
    iced::widget::button(text(label))
}

fn view(open: bool, dismissable: bool) -> Element<'static, Message> {
    let content = column![text("Dimensions"), button("Apply").on_press(Message::Apply)].spacing(8);
    let popover = popover(button("Open").on_press(Message::Toggle), content)
        .open(open)
        .side(Side::Bottom);
    let popover = if dismissable {
        popover.on_dismiss(Message::Dismiss)
    } else {
        popover
    };

    column![
        popover,
        button("Behind")
            .on_press(Message::Behind)
            .width(Length::Fixed(400.0))
    ]
    .into()
}

fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
    ui.into_messages().collect()
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate(simulator::click());
}

#[test]
fn closed_popover_shows_only_the_trigger() {
    let mut ui = simulator::simulator(view(false, true));
    assert!(ui.find("Open").is_ok());
    assert!(ui.find("Dimensions").is_err());
    ui.click("Open").expect("trigger is rendered");
    assert_eq!(messages(ui), vec![Message::Toggle]);
}

#[test]
fn open_popover_renders_its_content_and_it_is_interactive() {
    let mut ui = simulator::simulator(view(true, true));
    assert!(ui.find("Dimensions").is_ok());
    ui.click("Apply").expect("content is rendered");
    assert_eq!(messages(ui), vec![Message::Apply]);
}

#[test]
fn escape_dismisses_an_open_popover() {
    let mut ui = simulator::simulator(view(true, true));
    let _ = ui.tap_key(Key::Named(Named::Escape));
    assert_eq!(messages(ui), vec![Message::Dismiss]);

    let mut closed = simulator::simulator(view(false, true));
    let _ = closed.tap_key(Key::Named(Named::Escape));
    assert!(messages(closed).is_empty());
}

#[test]
fn clicking_outside_dismisses() {
    let mut ui = simulator::simulator(view(true, true));
    click_at(&mut ui, Point::new(900.0, 700.0));
    assert_eq!(messages(ui), vec![Message::Dismiss]);
}

#[test]
fn the_click_that_dismisses_still_reaches_what_it_landed_on() {
    let mut ui = simulator::simulator(view(true, true));
    let behind = ui
        .find("Behind")
        .expect("button behind is rendered")
        .bounds();
    // The right end of the wide button, clear of the panel.
    click_at(&mut ui, Point::new(390.0, behind.center_y()));
    assert_eq!(messages(ui), vec![Message::Dismiss, Message::Behind]);
}

#[test]
fn the_keymap_decides_which_keys_dismiss() {
    let keymap = default_keymap()
        .unbind(&Chord::named(Named::Escape))
        .bind(Chord::character('q'), Action::Close);
    let element: Element<'_, Message> = popover(button("Open"), text("Content"))
        .open(true)
        .keymap(keymap)
        .on_dismiss(Message::Dismiss)
        .into();
    let mut ui = simulator::simulator(element);
    let _ = ui.tap_key(Key::Named(Named::Escape));
    let _ = ui.tap_key(Key::Character("q".into()));
    assert_eq!(messages(ui), vec![Message::Dismiss]);
}

#[test]
fn clicking_inside_does_not_reach_the_widgets_underneath() {
    let mut ui = simulator::simulator(view(true, true));
    ui.click("Dimensions").expect("content is rendered");
    assert!(messages(ui).is_empty());
}

#[test]
fn the_trigger_toggles_without_being_dismissed() {
    let mut ui = simulator::simulator(view(true, true));
    ui.click("Open").expect("trigger is rendered");
    assert_eq!(messages(ui), vec![Message::Toggle]);
}

#[test]
fn without_on_dismiss_only_the_app_closes_it() {
    let mut ui = simulator::simulator(view(true, false));
    let _ = ui.tap_key(Key::Named(Named::Escape));
    click_at(&mut ui, Point::new(900.0, 700.0));
    assert!(messages(ui).is_empty());
}
