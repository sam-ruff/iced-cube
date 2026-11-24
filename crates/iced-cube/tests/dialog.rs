#![cfg(feature = "dialog")]

#[cfg(feature = "combobox")]
mod support;

use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::{self, column, text, text_input};
use iced::{Element, Event, Point, Rectangle, Settings, Size, Vector, event, mouse, touch};
use iced_cube::button;
use iced_cube::overlay::dialog::{self, CLOSE_BUTTON_ID, alert_dialog, dialog};
use iced_test::selector::{Bounded, Candidate};
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

fn tap_at(ui: &mut Simulator<'_, Message>, position: Point) {
    let id = touch::Finger(1);
    ui.point_at(position);
    let _ = ui.simulate([
        Event::Touch(touch::Event::FingerPressed { id, position }),
        Event::Touch(touch::Event::FingerLifted { id, position }),
    ]);
}

#[test]
fn taps_inside_the_surface_do_not_dismiss_but_taps_on_the_scrim_do() -> Result<(), iced_test::Error>
{
    let mut ui = simulator(view(OPEN));
    let title = ui.find("Edit profile")?.bounds().center();
    tap_at(&mut ui, title);
    let description = ui.find("Make changes to your profile.")?.bounds().center();
    tap_at(&mut ui, description);
    assert!(messages(ui).is_empty(), "the panel stops the finger");

    let mut ui = simulator(view(OPEN));
    tap_at(&mut ui, SCRIM);
    assert_eq!(messages(ui), vec![Message::Dismiss]);
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

/// Whether any field or control has focus, with or without an id.
fn any_focused(ui: &mut Simulator<'_, Message>) -> bool {
    ui.find(|candidate: Candidate<'_>| match candidate {
        Candidate::Focusable { state, .. } if state.is_focused() => Some(()),
        _ => None,
    })
    .is_ok()
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
    assert_eq!(order, vec![email(), CLOSE_BUTTON_ID, name()]);

    press(&mut ui, Named::Tab, Modifiers::SHIFT);
    assert_eq!(focused(&mut ui), Some(CLOSE_BUTTON_ID));
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

#[test]
fn enter_does_not_run_the_destructive_action_by_default() {
    let mut ui = simulator(confirmation(true));
    assert_eq!(ui.tap_key(Named::Enter), event::Status::Captured);
    assert!(messages(ui).is_empty());
}

#[test]
fn tab_reaches_the_alert_buttons_and_enter_or_space_presses_them() {
    let mut ui = simulator(confirmation(true));
    nudge(&mut ui);
    assert!(!any_focused(&mut ui), "nothing focused on open");

    press(&mut ui, Named::Tab, Modifiers::empty());
    assert!(any_focused(&mut ui), "Tab focuses Cancel");
    let _ = ui.tap_key(Named::Enter);

    press(&mut ui, Named::Tab, Modifiers::empty());
    let _ = ui.tap_key(Named::Space);

    press(&mut ui, Named::Tab, Modifiers::SHIFT);
    let _ = ui.tap_key(Named::Space);
    assert_eq!(
        messages(ui),
        vec![Message::Dismiss, Message::Delete, Message::Dismiss]
    );
}

#[test]
fn a_pointer_press_clears_the_focused_alert_button() {
    let mut ui = simulator(confirmation(true));
    nudge(&mut ui);
    press(&mut ui, Named::Tab, Modifiers::empty());
    press(&mut ui, Named::Tab, Modifiers::empty());
    assert!(any_focused(&mut ui));
    click_at(&mut ui, SCRIM);
    assert!(!any_focused(&mut ui));
    let _ = ui.tap_key(Named::Enter);
    assert!(messages(ui).is_empty());
}

#[test]
fn enter_confirms_an_alert_dialog_that_opts_in() {
    let element: Element<'_, Message> =
        alert_dialog(base(), "Delete project?", "This cannot be undone.")
            .open(true)
            .on_cancel(Message::Dismiss)
            .on_confirm(Message::Delete)
            .confirm_on_enter(true)
            .into();
    let mut ui = simulator(element);
    assert_eq!(ui.tap_key(Named::Enter), event::Status::Captured);
    assert_eq!(messages(ui), vec![Message::Delete]);
}

#[test]
fn enter_follows_the_keymap() {
    let element: Element<'_, Message> =
        alert_dialog(base(), "Delete project?", "This cannot be undone.")
            .open(true)
            .on_cancel(Message::Dismiss)
            .on_confirm(Message::Delete)
            .confirm_on_enter(true)
            .keymap(
                dialog::default_keymap()
                    .unbind(&iced_cube::Chord::named(Named::Enter))
                    .bind(iced_cube::Chord::named(Named::F2), dialog::Action::Confirm),
            )
            .into();
    let mut ui = simulator(element);
    let _ = ui.tap_key(Named::Enter);
    let _ = ui.tap_key(Named::F2);
    assert_eq!(messages(ui), vec![Message::Delete]);
}

#[test]
fn a_field_that_submits_on_enter_keeps_the_key() {
    let submit = || -> Element<'static, Message> {
        dialog(base())
            .open(true)
            .title("Rename")
            .body(
                text_input("Name", "")
                    .on_input(Message::Name)
                    .on_submit(Message::Save),
            )
            .on_confirm(Message::Delete)
            .into()
    };
    let mut ui = simulator(submit());
    nudge(&mut ui);
    let _ = ui.tap_key(Named::Enter);
    assert_eq!(messages(ui), vec![Message::Save]);

    let plain: Element<'_, Message> = dialog(base()).open(true).title("Plain").into();
    let mut ui = simulator(plain);
    let _ = ui.tap_key(Named::Enter);
    assert!(messages(ui).is_empty(), "no on_confirm, no message");
}

#[test]
fn alert_dialog_cancels_on_the_scrim_when_it_opts_in() {
    let element: Element<'_, Message> =
        alert_dialog(base(), "Discard draft?", "It has not been saved.")
            .open(true)
            .on_cancel(Message::Dismiss)
            .on_confirm(Message::Delete)
            .dismiss_on_scrim(true)
            .into();
    let mut ui = simulator(element);
    click_at(&mut ui, SCRIM);
    assert_eq!(messages(ui), vec![Message::Dismiss]);
}

/// A window much shorter than a tall dialog.
const SMALL: Size = Size::new(480.0, 360.0);

fn small<'a>(element: impl Into<Element<'a, Message>>) -> Simulator<'a, Message> {
    Simulator::with_size(Settings::default(), SMALL, element)
}

fn lines(count: usize) -> iced::widget::Column<'static, Message> {
    column((0..count).map(|line| text(format!("Line {line}")).into())).spacing(8)
}

fn tall(count: usize) -> dialog::Dialog<'static, Message> {
    dialog(base())
        .open(true)
        .title("Edit event")
        .description("Everything about this event.")
        .body(lines(count))
        .action(button("Cancel").on_press(Message::Dismiss))
        .action(button("Save").on_press(Message::Save))
        .on_dismiss(Message::Dismiss)
}

/// Whether the whole of a target is on screen. Scrolled targets report
/// their visible bounds where they are drawn, so only the sizes compare.
fn shows_whole(target: &impl Bounded) -> bool {
    let bounds = target.bounds();
    target.visible_bounds().is_some_and(|visible| {
        (visible.width - bounds.width).abs() < 0.5 && (visible.height - bounds.height).abs() < 0.5
    })
}

fn fully_visible(ui: &mut Simulator<'_, Message>, label: &str) -> Result<bool, iced_test::Error> {
    let target = ui.find(label)?;
    Ok(shows_whole(&target))
}

#[test]
fn a_tall_dialog_keeps_its_header_and_actions_in_a_small_window() -> Result<(), iced_test::Error> {
    let mut ui = small(tall(60));
    let window = Rectangle::with_size(SMALL);
    for label in [
        "Edit event",
        "Everything about this event.",
        "Cancel",
        "Save",
    ] {
        let bounds = ui.find(label)?.bounds();
        assert!(fully_visible(&mut ui, label)?, "{label} is cut off");
        assert!(
            window.contains(bounds.position())
                && window.contains(bounds.position() + Vector::new(bounds.width, bounds.height)),
            "{label} is outside the window: {bounds:?}"
        );
    }
    assert!(
        !fully_visible(&mut ui, "Line 59")?,
        "the end of the body is out of view"
    );

    ui.click("Save")?;
    ui.click("Cancel")?;
    assert_eq!(messages(ui), vec![Message::Save, Message::Dismiss]);
    Ok(())
}

#[test]
fn the_body_of_a_tall_dialog_scrolls_with_the_wheel() -> Result<(), iced_test::Error> {
    let mut ui = small(tall(60));
    let first = ui.find("Line 0")?.bounds().center();
    ui.point_at(first);
    let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 0.0, y: -100.0 },
    })]);

    assert!(fully_visible(&mut ui, "Line 59")?, "scrolled to the end");
    assert!(!fully_visible(&mut ui, "Line 0")?);
    assert!(fully_visible(&mut ui, "Edit event")?, "the title stays");
    assert!(fully_visible(&mut ui, "Save")?, "the actions stay");
    assert!(messages(ui).is_empty(), "scrolling dismisses nothing");
    Ok(())
}

#[test]
fn a_short_dialog_shows_its_whole_body_and_does_not_scroll() -> Result<(), iced_test::Error> {
    let mut ui = small(tall(3));
    let before = ui.find("Line 0")?.bounds();
    ui.point_at(before.center());
    let _ = ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 0.0, y: -100.0 },
    })]);
    for label in ["Line 0", "Line 2", "Save"] {
        assert!(fully_visible(&mut ui, label)?, "{label}");
    }
    let after = ui.find("Line 0")?.visible_bounds().map(|bounds| bounds.y);
    assert!(
        after.is_some_and(|y| (y - before.y).abs() < 0.5),
        "{after:?} moved from {before:?}"
    );
    Ok(())
}

#[test]
fn max_height_caps_the_dialog_below_the_window() -> Result<(), iced_test::Error> {
    let mut ui = simulator(tall(60).max_height(300.0));
    let title = ui.find("Edit event")?.bounds();
    let save = ui.find("Save")?.bounds();
    assert!(fully_visible(&mut ui, "Save")?);
    assert!(
        save.y + save.height - title.y < 300.0,
        "title at {title:?}, save at {save:?}"
    );
    ui.click("Save")?;
    assert_eq!(messages(ui), vec![Message::Save]);
    Ok(())
}

fn late() -> widget::Id {
    widget::Id::new("late")
}

#[test]
fn tab_scrolls_a_field_below_the_fold_into_view() {
    let body = column![
        text_input("Name", "").id(name()).on_input(Message::Name),
        lines(40),
        text_input("Notes", "").id(late()).on_input(Message::Email),
    ]
    .spacing(8);
    let element: Element<'_, Message> = dialog(base())
        .open(true)
        .title("Edit event")
        .body(body)
        .action(button("Save").on_press(Message::Save))
        .on_dismiss(Message::Dismiss)
        .into();
    let mut ui = small(element);
    nudge(&mut ui);
    assert_eq!(focused(&mut ui), Some(name()));
    let hidden = ui
        .find(late())
        .ok()
        .and_then(|field| field.visible_bounds());
    assert_eq!(hidden, None, "the second field starts out of view");

    press(&mut ui, Named::Tab, Modifiers::empty());
    assert_eq!(focused(&mut ui), Some(late()));
    let field = ui.find(late()).expect("the field is rendered");
    assert!(shows_whole(&field), "Tab scrolled it fully into view");

    press(&mut ui, Named::Tab, Modifiers::SHIFT);
    assert_eq!(focused(&mut ui), Some(name()));
    let field = ui.find(name()).expect("the field is rendered");
    assert!(shows_whole(&field), "and back up");
}

fn sectioned() -> Element<'static, Message> {
    dialog(base())
        .open(true)
        .title("New event")
        .size(dialog::Size::Lg)
        .body(dialog::columns([
            dialog::section("When", text("Tomorrow")).into(),
            dialog::section("Who", text("Everyone")).into(),
        ]))
        .on_dismiss(Message::Dismiss)
        .into()
}

#[test]
fn sections_sit_side_by_side_and_stack_in_a_narrow_window() -> Result<(), iced_test::Error> {
    let mut ui = simulator(sectioned());
    let when = ui.find("When")?.bounds();
    let who = ui.find("Who")?.bounds();
    assert_eq!(when.y, who.y, "one row");
    assert!(who.x > when.x + when.width, "two columns");

    let mut ui = Simulator::with_size(Settings::default(), Size::new(360.0, 640.0), sectioned());
    let when = ui.find("When")?.bounds();
    let who = ui.find("Who")?.bounds();
    assert_eq!(when.x, who.x, "one column");
    assert!(who.y > when.y + when.height, "stacked");
    Ok(())
}

#[test]
fn one_escape_dismisses_even_while_a_field_has_focus() {
    let mut ui = simulator(form());
    nudge(&mut ui);
    assert_eq!(focused(&mut ui), Some(name()));
    let _ = ui.tap_key(Named::Escape);
    assert_eq!(messages(ui), vec![Message::Dismiss]);
}

#[test]
fn escape_still_reaches_a_field_when_the_dialog_keeps_it() {
    let element: Element<'_, Message> = dialog(base())
        .open(true)
        .title("Edit profile")
        .body(text_input("Name", "").id(name()).on_input(Message::Name))
        .on_dismiss(Message::Dismiss)
        .dismiss_on_escape(false)
        .into();
    let mut ui = simulator(element);
    nudge(&mut ui);
    let _ = ui.tap_key(Named::Escape);
    assert_eq!(focused(&mut ui), None, "the field lets go of focus");
    assert!(messages(ui).is_empty());
}

#[cfg(all(feature = "switch", feature = "checkbox", feature = "select"))]
mod form_keys {
    use iced::keyboard::{self, Key, Modifiers, key::Named};
    use iced::widget::{self, column, text, text_input};
    use iced::{Element, Event, Point, Settings, Size, mouse};
    use iced_cube::button;
    use iced_cube::forms::select::select;
    use iced_cube::overlay::dialog::{CLOSE_BUTTON_ID, dialog};
    use iced_cube::primitives::button::Variant;
    use iced_cube::primitives::{checkbox, switch};
    use iced_test::selector::Candidate;
    use iced_test::simulator::{Simulator, simulator};

    #[derive(Debug, Clone, PartialEq)]
    enum Form {
        Name(String),
        Notify(bool),
        Agree(bool),
        Fruit(&'static str),
        Pick,
        Cancel,
        Save,
        Dismiss,
    }

    const FRUITS: &[&str] = &["Apple", "Banana", "Cherry"];

    fn name() -> widget::Id {
        widget::Id::new("form-name")
    }

    fn view() -> Element<'static, Form> {
        let body = column![
            text_input("Name", "").id(name()).on_input(Form::Name),
            switch(false).label("Notify me").on_toggle(Form::Notify),
            checkbox(false).label("I agree").on_toggle(Form::Agree),
            select(FRUITS, Some("Apple")).on_select(Form::Fruit),
            button("Pick date")
                .variant(Variant::Outline)
                .on_press(Form::Pick),
        ]
        .spacing(12);
        dialog(text("Base"))
            .open(true)
            .title("New event")
            .body(body)
            .action(
                button("Cancel")
                    .variant(Variant::Outline)
                    .on_press(Form::Cancel),
            )
            .action(button("Save").on_press(Form::Save))
            .on_dismiss(Form::Dismiss)
            .into()
    }

    fn nudge(ui: &mut Simulator<'_, Form>) {
        let position = Point::new(1.0, 1.0);
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    }

    fn tab(ui: &mut Simulator<'_, Form>, modifiers: Modifiers) {
        let key = Key::Named(Named::Tab);
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

    /// The focused widget: its id, or `None` inside when it has none.
    fn focused(ui: &mut Simulator<'_, Form>) -> Option<Option<widget::Id>> {
        ui.find(|candidate: Candidate<'_>| match candidate {
            Candidate::Focusable { id, state, .. } if state.is_focused() => Some(id.cloned()),
            _ => None,
        })
        .ok()
    }

    #[test]
    fn tab_reaches_every_control_and_each_works_from_the_keyboard() {
        let mut ui = simulator(view());
        nudge(&mut ui);
        assert_eq!(
            focused(&mut ui),
            Some(Some(name())),
            "opening focuses the field"
        );

        tab(&mut ui, Modifiers::empty());
        assert_eq!(focused(&mut ui), Some(None), "the switch");
        let _ = ui.tap_key(Named::Space);

        tab(&mut ui, Modifiers::empty());
        let _ = ui.tap_key(Named::Space);

        tab(&mut ui, Modifiers::empty());
        assert!(ui.find("Banana").is_err());
        let _ = ui.tap_key(Named::Space);
        assert!(ui.find("Banana").is_ok(), "Space opens the select");
        let _ = ui.tap_key(Named::Escape);
        assert!(ui.find("Banana").is_err(), "Escape closes only the list");

        tab(&mut ui, Modifiers::empty());
        let _ = ui.tap_key(Named::Enter);

        tab(&mut ui, Modifiers::empty());
        let _ = ui.tap_key(Named::Enter);

        tab(&mut ui, Modifiers::empty());
        let _ = ui.tap_key(Named::Space);

        tab(&mut ui, Modifiers::empty());
        assert_eq!(focused(&mut ui), Some(Some(CLOSE_BUTTON_ID)));
        tab(&mut ui, Modifiers::empty());
        assert_eq!(focused(&mut ui), Some(Some(name())), "and round again");
        tab(&mut ui, Modifiers::SHIFT);
        assert_eq!(focused(&mut ui), Some(Some(CLOSE_BUTTON_ID)));

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![
                Form::Notify(true),
                Form::Agree(true),
                Form::Pick,
                Form::Cancel,
                Form::Save,
            ]
        );
    }

    #[test]
    fn escape_after_tabbing_to_a_control_dismisses_at_once() {
        let mut ui = simulator(view());
        nudge(&mut ui);
        tab(&mut ui, Modifiers::empty());
        let _ = ui.tap_key(Named::Escape);
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![Form::Dismiss]);
    }

    #[test]
    fn a_pointer_press_leaves_no_focus_ring_behind() {
        let mut ui = simulator(view());
        nudge(&mut ui);
        tab(&mut ui, Modifiers::empty());
        assert_eq!(focused(&mut ui), Some(None));
        ui.click("Pick date").expect("the button is rendered");
        assert_eq!(focused(&mut ui), None, "a click focuses nothing");
        let _ = ui.tap_key(Named::Space);
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![Form::Pick]);
    }

    #[test]
    fn tab_scrolls_a_control_below_the_fold_into_view() {
        let lines = column((0..40).map(|line| text(format!("Line {line}")).into())).spacing(8);
        let body = column![
            text_input("Name", "").id(name()).on_input(Form::Name),
            lines,
            button("Pick date").on_press(Form::Pick),
        ]
        .spacing(8);
        let element: Element<'_, Form> = dialog(text("Base"))
            .open(true)
            .title("Edit event")
            .body(body)
            .on_dismiss(Form::Dismiss)
            .into();
        let mut ui = Simulator::with_size(Settings::default(), Size::new(480.0, 360.0), element);
        nudge(&mut ui);
        let hidden = ui
            .find("Pick date")
            .ok()
            .and_then(|label| label.visible_bounds());
        assert_eq!(hidden, None, "the button starts out of view");

        tab(&mut ui, Modifiers::empty());
        let label = ui.find("Pick date").expect("the button is rendered");
        assert!(
            label.visible_bounds().is_some(),
            "Tab scrolled it into view"
        );
    }
}

#[cfg(feature = "combobox")]
mod combobox_escape {
    use iced::keyboard::key::Named;
    use iced::widget::text;
    use iced::{Element, Event, Point, mouse};
    use iced_cube::forms::combobox::{self, combobox};
    use iced_cube::overlay::dialog::dialog;

    use super::support::Harness;

    struct App {
        fruit: combobox::State<&'static str>,
        open: bool,
    }

    #[derive(Debug, Clone)]
    enum Msg {
        Fruit(combobox::Event),
        Dismiss,
    }

    fn view(app: &App) -> Element<'_, Msg> {
        dialog(text("Base"))
            .open(app.open)
            .title("Pick a fruit")
            .body(combobox(&app.fruit).on_event(Msg::Fruit))
            .on_dismiss(Msg::Dismiss)
            .into()
    }

    fn update(app: &mut App, message: Msg) {
        match message {
            Msg::Fruit(event) => {
                let _ = app.fruit.update(event);
            }
            Msg::Dismiss => app.open = false,
        }
    }

    #[test]
    fn escape_closes_the_open_list_and_the_next_escape_dismisses() {
        let mut fruit = combobox::State::new(["Apple", "Banana"]);
        let _ = fruit.update(combobox::Event::Open);
        let mut ui = Harness::new(App { fruit, open: true }, view, update);
        ui.batch([Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(1.0, 1.0),
        })]);
        assert!(ui.state.fruit.is_open());

        ui.tap(Named::Escape);
        assert!(
            !ui.state.fruit.is_open(),
            "the first Escape closes the list"
        );
        assert!(ui.state.open, "and leaves the dialog open");

        ui.tap(Named::Escape);
        assert!(!ui.state.open, "the second dismisses the dialog");
    }
}
