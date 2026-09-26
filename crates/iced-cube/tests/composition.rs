//! Overlays inside other overlays: one click or one Escape closes only the
//! innermost layer.
#![cfg(all(
    feature = "combobox",
    feature = "command",
    feature = "dialog",
    feature = "dropdown-menu",
    feature = "icon-button",
    feature = "popover"
))]

use iced::keyboard::key::Named;
use iced::widget::{self, button, container, text};
use iced::{Element, Event, Point, mouse};
use iced_cube::forms::combobox::{self, combobox};
use iced_cube::navigation::command::{self, command, group, item};
use iced_cube::overlay::dialog::dialog;
use iced_cube::overlay::dropdown_menu::{self, dropdown_menu};
use iced_cube::overlay::popover::popover;
use iced_cube::overlay::tooltip::Position;
use iced_cube::{icon_button, lucide};
use iced_test::selector::Candidate;
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Menu(dropdown_menu::Event<u8>),
    Combo(combobox::Event),
    /// The command event's debug text, since its events hold a sender.
    Command(String),
    Toggle,
    Dismiss,
}

fn field_id() -> widget::Id {
    widget::Id::new("fruit")
}

/// The id of the focused text field, if any.
fn focused(ui: &mut Simulator<'_, Message>) -> Option<widget::Id> {
    ui.find(|candidate: Candidate<'_>| match candidate {
        Candidate::Focusable { id, state, .. } if state.is_focused() => id.cloned(),
        _ => None,
    })
    .ok()
}

fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
    ui.into_messages().collect()
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    let _ = ui.simulate(click());
}

/// A point on the dialog scrim, well away from the centred surface.
const SCRIM: Point = Point::new(8.0, 700.0);

fn open_menu() -> dropdown_menu::State<u8> {
    let mut state = dropdown_menu::State::new([
        dropdown_menu::item(1, "Edit"),
        dropdown_menu::item(2, "Copy"),
    ]);
    let _ = state.update(dropdown_menu::Event::Open);
    state
}

fn open_combobox() -> combobox::State<&'static str> {
    let mut state = combobox::State::new(["Apple", "Banana", "Cherry"]);
    let _ = state.update(combobox::Event::Open);
    state
}

fn in_dialog<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    dialog(text("Page"))
        .open(true)
        .title("Dialog")
        .body(body)
        .on_dismiss(Message::Dismiss)
        .into()
}

fn in_popover<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        popover(button(text("Open")).on_press(Message::Toggle), content)
            .open(true)
            .on_dismiss(Message::Dismiss),
    )
    .padding(40)
    .into()
}

#[test]
fn menu_in_dialog_one_outside_click_closes_only_the_menu() {
    let state = open_menu();
    let menu = dropdown_menu(&state, button(text("Actions")).on_press(Message::Toggle))
        .on_event(Message::Menu);
    let mut ui = simulator(in_dialog(menu));
    assert!(ui.find("Copy").is_ok(), "the menu is open over the dialog");
    click_at(&mut ui, SCRIM);
    assert_eq!(
        messages(ui),
        vec![Message::Menu(dropdown_menu::Event::Close)]
    );
}

#[test]
fn a_closed_menu_in_a_dialog_leaves_the_scrim_click_to_the_dialog() {
    let state = dropdown_menu::State::new([dropdown_menu::item(1u8, "Edit")]);
    let menu = dropdown_menu(&state, button(text("Actions")).on_press(Message::Toggle))
        .on_event(Message::Menu);
    let mut ui = simulator(in_dialog(menu));
    click_at(&mut ui, SCRIM);
    assert_eq!(messages(ui), vec![Message::Dismiss]);
}

#[test]
fn combobox_in_dialog_one_outside_click_closes_only_the_list_once() {
    let state = open_combobox();
    let field = combobox(&state).id("fruit").on_event(Message::Combo);
    let mut ui = simulator(in_dialog(field));
    ui.click(field_id()).expect("field is rendered");
    click_at(&mut ui, SCRIM);
    // Further events let the field give up focus, which must not close
    // the list a second time.
    for _ in 0..2 {
        let position = Point::new(10.0, 10.0);
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    }
    assert_eq!(focused(&mut ui), None, "the field lost focus");
    let emitted = messages(ui);
    assert_eq!(
        emitted
            .iter()
            .filter(|message| **message == Message::Combo(combobox::Event::Close))
            .count(),
        1,
        "{emitted:?}"
    );
    assert!(!emitted.contains(&Message::Dismiss), "{emitted:?}");
}

#[test]
fn escape_in_a_combobox_inside_a_popover_closes_the_list_first() {
    let state = open_combobox();
    let field = combobox(&state).id("fruit").on_event(Message::Combo);
    let mut ui = simulator(in_popover(field));
    ui.click(field_id()).expect("field is rendered");
    let _ = ui.tap_key(Named::Escape);
    assert_eq!(messages(ui), vec![Message::Combo(combobox::Event::Close)]);
}

#[test]
fn escape_in_a_command_inside_a_popover_clears_the_query_first() {
    let mut state = command::State::new([group(
        "Suggestions",
        [item(1u8, "Calendar"), item(2, "Settings")],
    )]);
    let _ = state.update(command::Event::Input("cal".into()));
    let list = command(&state)
        .id("search")
        .on_event(|event| Message::Command(format!("{event:?}")));
    let mut ui = simulator(in_popover(list));
    ui.click(widget::Id::new("search"))
        .expect("search field is rendered");
    let _ = ui.tap_key(Named::Escape);
    assert_eq!(
        messages(ui),
        vec![
            Message::Command("Focus(true)".into()),
            Message::Command("Close".into())
        ]
    );
}

#[test]
fn escape_reaches_the_popover_once_the_inner_layers_are_closed() {
    let state = combobox::State::new(["Apple", "Banana"]);
    let field = combobox(&state).id("fruit").on_event(Message::Combo);
    let mut ui = simulator(in_popover(field));
    let _ = ui.tap_key(Named::Escape);
    assert_eq!(messages(ui), vec![Message::Dismiss]);
}

/// Hovers the trigger and returns a hash of the rendered window. iced's
/// tooltip overlay cannot be found by text, so the test compares renders.
fn hovered_render(open: bool, tooltip: bool, name: &str) -> Result<String, iced_test::Error> {
    let state = if open {
        open_menu()
    } else {
        dropdown_menu::State::new([dropdown_menu::item(1u8, "Edit")])
    };
    let trigger = icon_button(lucide!(Ellipsis))
        .label("More actions")
        .tooltip(tooltip.then_some(Position::Top))
        .id("more")
        .on_press(Message::Toggle);
    let menu = dropdown_menu(&state, trigger).on_event(Message::Menu);
    let element: Element<'_, Message> = container(menu).padding(80).into();
    let mut ui = simulator(element);
    let centre = ui.find(widget::Id::new("more"))?.bounds().center();
    ui.point_at(centre);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: centre })]);

    let directory =
        std::env::temp_dir().join(format!("iced-cube-composition-{}", std::process::id()));
    let _ = ui
        .snapshot(&iced_cube::theme::light())?
        .matches_hash(directory.join(name))?;
    let Some(file) = std::fs::read_dir(&directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|file| file.to_string_lossy().contains(name))
    else {
        return Ok(String::new());
    };
    let hash = std::fs::read_to_string(&file)?;
    std::fs::remove_file(file)?;
    Ok(hash)
}

#[test]
fn a_trigger_tooltip_stays_hidden_while_its_menu_is_open() -> Result<(), iced_test::Error> {
    let closed_with = hovered_render(false, true, "closed-with-tooltip")?;
    let closed_without = hovered_render(false, false, "closed-without-tooltip")?;
    assert_ne!(closed_with, closed_without, "the tooltip shows on hover");

    let open_with = hovered_render(true, true, "open-with-tooltip")?;
    let open_without = hovered_render(true, false, "open-without-tooltip")?;
    assert!(!open_with.is_empty());
    assert_eq!(open_with, open_without, "no tooltip over the open menu");
    Ok(())
}

#[test]
fn a_menu_in_a_popover_closes_before_the_popover() {
    let state = open_menu();
    let menu = dropdown_menu(&state, button(text("Actions")).on_press(Message::Toggle))
        .on_event(Message::Menu);
    let mut ui = simulator(in_popover(menu));
    let _ = ui.tap_key(Named::Escape);
    assert_eq!(
        messages(ui),
        vec![Message::Menu(dropdown_menu::Event::Close)]
    );
}
