#![cfg(feature = "dialog")]

use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::{self, column, text_input};
use iced::{Element, Event, Point, event, mouse};
use iced_cube::button;
use iced_cube::overlay::dialog::{self, CLOSE_BUTTON_ID, alert_dialog, dialog};
use iced_test::selector::Candidate;
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Open,
    Behind,
    Dismiss,
    Save,
    Name(String),
    Email(String),
    Search(String),
    Delete,
}

#[derive(Debug, Clone, Copy)]
struct Options {
    open: bool,
    escape: bool,
    scrim: bool,
    close: bool,
}

const OPEN: Options = Options {
    open: true,
    escape: true,
    scrim: true,
    close: true,
};

fn base() -> Element<'static, Message> {
    column![
        button("Open").on_press(Message::Open),
        button("Behind").on_press(Message::Behind),
    ]
    .into()
}

fn view(options: Options) -> Element<'static, Message> {
    dialog(base())
        .open(options.open)
        .title("Edit profile")
        .description("Make changes to your profile.")
        .action(button("Save").on_press(Message::Save))
        .on_dismiss(Message::Dismiss)
        .dismiss_on_escape(options.escape)
        .dismiss_on_scrim(options.scrim)
        .close_button(options.close)
        .into()
}

fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
    ui.into_messages().collect()
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    let _ = ui.simulate(click());
}

/// A point on the scrim, well away from the centred surface.
const SCRIM: Point = Point::new(8.0, 700.0);

#[test]
fn trigger_opens_and_the_open_dialog_renders_its_content() -> Result<(), iced_test::Error> {
    let closed = Options {
        open: false,
        ..OPEN
    };
    let mut ui = simulator(view(closed));
    assert!(ui.find("Edit profile").is_err());
    assert!(ui.find(CLOSE_BUTTON_ID).is_err());
    ui.click("Open")?;
    assert_eq!(messages(ui), vec![Message::Open]);

    let mut ui = simulator(view(OPEN));
    assert!(ui.find("Edit profile").is_ok());
    assert!(ui.find("Make changes to your profile.").is_ok());
    assert!(ui.find(CLOSE_BUTTON_ID).is_ok());
    ui.click("Save")?;
    assert_eq!(messages(ui), vec![Message::Save]);
    Ok(())
}

#[test]
fn escape_dismisses_unless_turned_off() {
    let mut ui = simulator(view(OPEN));
    let _ = ui.tap_key(Named::Escape);
    assert_eq!(messages(ui), vec![Message::Dismiss]);

    let mut ui = simulator(view(Options {
        escape: false,
        ..OPEN
    }));
    let _ = ui.tap_key(Named::Escape);
    assert!(messages(ui).is_empty());
}

#[test]
fn escape_does_nothing_while_closed() {
    let mut ui = simulator(view(Options {
        open: false,
        ..OPEN
    }));
    let _ = ui.tap_key(Named::Escape);
    assert!(messages(ui).is_empty());
}

#[test]
fn scrim_click_dismisses_unless_turned_off() {
    let mut ui = simulator(view(OPEN));
    click_at(&mut ui, SCRIM);
    assert_eq!(messages(ui), vec![Message::Dismiss]);

    let mut ui = simulator(view(Options {
        scrim: false,
        ..OPEN
    }));
    click_at(&mut ui, SCRIM);
    assert!(messages(ui).is_empty());
}

#[test]
fn clicks_inside_the_surface_do_not_dismiss() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(OPEN));
    ui.click("Edit profile")?;
    assert!(messages(ui).is_empty());
    Ok(())
}

#[test]
fn close_button_dismisses_and_can_be_hidden() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(OPEN));
    ui.click(CLOSE_BUTTON_ID)?;
    assert_eq!(messages(ui), vec![Message::Dismiss]);

    let mut ui = simulator(view(Options {
        close: false,
        ..OPEN
    }));
    assert!(ui.find(CLOSE_BUTTON_ID).is_err());
    Ok(())
}

#[test]
fn scrim_blocks_clicks_on_the_content_underneath() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(OPEN));
    ui.click("Behind")?;
    assert_eq!(messages(ui), vec![Message::Dismiss]);

    let mut ui = simulator(view(Options {
        scrim: false,
        ..OPEN
    }));
    ui.click("Behind")?;
    ui.click("Open")?;
    assert!(messages(ui).is_empty());

    let mut ui = simulator(view(Options {
        open: false,
        ..OPEN
    }));
    ui.click("Behind")?;
    assert_eq!(messages(ui), vec![Message::Behind]);
    Ok(())
}

fn name() -> widget::Id {
    widget::Id::new("name")
}

fn email() -> widget::Id {
    widget::Id::new("email")
}

fn search() -> widget::Id {
    widget::Id::new("search")
}

fn form() -> Element<'static, Message> {
    let base = text_input("Search", "")
        .id(search())
        .on_input(Message::Search);
    let body = column![
        text_input("Name", "").id(name()).on_input(Message::Name),
        text_input("Email", "").id(email()).on_input(Message::Email),
    ];
    dialog(base)
        .open(true)
        .title("Edit profile")
        .body(body)
        .on_dismiss(Message::Dismiss)
        .into()
}

/// The id of the focused text field, if any.
fn focused(ui: &mut Simulator<'_, Message>) -> Option<widget::Id> {
    ui.find(|candidate: Candidate<'_>| match candidate {
        Candidate::Focusable { id, state, .. } if state.is_focused() => id.cloned(),
        _ => None,
    })
    .ok()
}

fn press(ui: &mut Simulator<'_, Message>, key: Named, modifiers: Modifiers) {
    let key = Key::Named(key);
    let _ = ui.simulate([Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
        repeat: false,
        text: None,
    })]);
}

fn nudge(ui: &mut Simulator<'_, Message>) {
    let position = Point::new(1.0, 1.0);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
}

#[test]
fn opening_focuses_the_first_field_in_the_dialog() {
    let mut ui = simulator(form());
    assert_eq!(focused(&mut ui), None);
    nudge(&mut ui);
    assert_eq!(focused(&mut ui), Some(name()));
}

#[test]
fn tab_and_shift_tab_cycle_inside_the_dialog() {
    let mut ui = simulator(form());
    nudge(&mut ui);

    let mut order = Vec::new();
    for _ in 0..3 {
        press(&mut ui, Named::Tab, Modifiers::empty());
        order.extend(focused(&mut ui));
    }
    assert_eq!(order, vec![email(), name(), email()]);

    press(&mut ui, Named::Tab, Modifiers::SHIFT);
    assert_eq!(focused(&mut ui), Some(name()));
    press(&mut ui, Named::Tab, Modifiers::SHIFT);
    assert_eq!(focused(&mut ui), Some(email()));
    assert!(ui.into_messages().next().is_none());
}

#[test]
fn typing_reaches_the_dialog_and_never_the_content_underneath() {
    let mut ui = simulator(form());
    nudge(&mut ui);
    let _ = ui.typewrite("Al");

    assert_eq!(
        messages(ui),
        vec![Message::Name("A".into()), Message::Name("Al".into())]
    );
}

#[test]
fn a_custom_keymap_replaces_the_defaults() {
    let element: Element<'_, Message> = dialog(base())
        .open(true)
        .title("Custom keys")
        .on_dismiss(Message::Dismiss)
        .keymap(dialog::default_keymap().unbind(&iced_cube::Chord::named(Named::Escape)))
        .into();
    let mut ui = simulator(element);
    let _ = ui.tap_key(Named::Escape);
    assert!(messages(ui).is_empty());
}

#[test]
fn an_open_dialog_captures_every_key_so_app_shortcuts_stop_there() {
    let mut ui = simulator(view(OPEN));
    for key in [Named::ArrowDown, Named::Enter, Named::Space, Named::F2] {
        assert_eq!(ui.tap_key(key), event::Status::Captured, "{key:?}");
    }
    let _ = ui.tap_key(Key::Character("k".into()));
    assert!(messages(ui).is_empty());

    let closed = Options {
        open: false,
        ..OPEN
    };
    let mut ui = simulator(view(closed));
    assert_eq!(ui.tap_key(Named::ArrowDown), event::Status::Ignored);
}

#[test]
fn pass_through_chords_reach_the_app() {
    let toggle = iced_cube::Chord::named(Named::F2);
    let element: Element<'_, Message> = dialog(base())
        .open(true)
        .title("Palette")
        .on_dismiss(Message::Dismiss)
        .pass_through([toggle])
        .into();
    let mut ui = simulator(element);
    assert_eq!(ui.tap_key(Named::F2), event::Status::Ignored);
    assert_eq!(ui.tap_key(Named::F3), event::Status::Captured);
}

fn confirmation(escape: bool) -> Element<'static, Message> {
    alert_dialog(base(), "Delete project?", "This cannot be undone.")
        .open(true)
        .confirm("Delete")
        .on_cancel(Message::Dismiss)
        .on_confirm(Message::Delete)
        .dismiss_on_escape(escape)
        .into()
}

#[test]
fn alert_dialog_answers_with_cancel_or_the_destructive_action() -> Result<(), iced_test::Error> {
    let mut ui = simulator(confirmation(true));
    assert!(ui.find(CLOSE_BUTTON_ID).is_err());
    ui.click("Cancel")?;
    ui.click("Delete")?;
    assert_eq!(messages(ui), vec![Message::Dismiss, Message::Delete]);
    Ok(())
}

#[test]
fn alert_dialog_ignores_the_scrim_and_cancels_on_escape() {
    let mut ui = simulator(confirmation(true));
    click_at(&mut ui, SCRIM);
    let _ = ui.tap_key(Named::Escape);
    assert_eq!(messages(ui), vec![Message::Dismiss]);

    let mut ui = simulator(confirmation(false));
    let _ = ui.tap_key(Named::Escape);
    assert!(messages(ui).is_empty());
}
