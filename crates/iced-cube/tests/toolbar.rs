#![cfg(feature = "toolbar")]

use iced::keyboard::{Key, Modifiers, key::Named};
use iced::time::Instant;
use iced::{Element, Point, Settings, Size, event, mouse, window};
use iced_cube::dropdown_menu;
use iced_cube::keys;
use iced_cube::lucide;
use iced_cube::toolbar::{
    self, Event, GAP, Highlight, Output, PADDING, State, button, choice, separator, spacer, toggle,
    toolbar,
};
use iced_test::simulator::{self, Simulator};

const BOLD: u8 = 1;
const ITALIC: u8 = 2;
const UNDERLINE: u8 = 3;
const LEFT: u8 = 4;
const CENTRE: u8 = 5;
const UNDO: u8 = 6;
const REDO: u8 = 7;

/// Side of a small toolbar button.
const SIDE: f32 = 32.0;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Toolbar(Event<u8>),
}

fn state() -> State<u8> {
    State::new([
        toggle(BOLD, lucide!(Bold), "Bold", false).shortcut("Ctrl+B"),
        toggle(ITALIC, lucide!(Italic), "Italic", false),
        toggle(UNDERLINE, lucide!(Underline), "Underline", false),
        separator(),
        choice(LEFT, lucide!(TextAlignStart), "Align left", true),
        choice(CENTRE, lucide!(TextAlignCenter), "Align centre", false),
        spacer(),
        button(UNDO, lucide!(Undo2), "Undo"),
        button(REDO, lucide!(Redo2), "Redo").disabled(true),
    ])
}

fn view(state: &State<u8>) -> Element<'_, Message> {
    toolbar(state).on_event(Message::Toolbar).into()
}

fn sized(state: &State<u8>, width: f32) -> Simulator<'_, Message> {
    Simulator::with_size(Settings::default(), Size::new(width, 300.0), view(state))
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event<u8>> {
    ui.into_messages()
        .map(|Message::Toolbar(event)| event)
        .collect()
}

fn apply(state: &mut State<u8>, events: Vec<Event<u8>>) -> Option<Output<u8>> {
    events
        .into_iter()
        .fold(None, |output, event| state.update(event).or(output))
}

/// The centre of the button in slot `slot`, counting from the left when
/// only buttons come before it.
fn slot(slot: usize) -> Point {
    let x = PADDING + slot as f32 * (SIDE + GAP) + SIDE / 2.0;
    Point::new(x, PADDING + SIDE / 2.0)
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate(simulator::click());
}

#[test]
fn clicking_a_toggle_presses_it() {
    let mut state = state();
    let mut ui = sized(&state, 720.0);
    click_at(&mut ui, slot(1));
    let emitted = events(ui);
    assert_eq!(emitted, vec![Event::Press(ITALIC)]);
    assert_eq!(
        apply(&mut state, emitted),
        Some(Output::Toggled(ITALIC, true))
    );
    assert!(state.is_on(ITALIC));
}

#[test]
fn the_spacer_pushes_the_last_buttons_to_the_far_end() {
    let state = state();
    let mut ui = sized(&state, 720.0);
    let far = Point::new(
        720.0 - PADDING - SIDE - GAP - SIDE / 2.0,
        PADDING + SIDE / 2.0,
    );
    click_at(&mut ui, far);
    assert_eq!(events(ui), vec![Event::Press(UNDO)]);
}

#[test]
fn disabled_buttons_send_nothing() {
    let state = state();
    let mut ui = sized(&state, 720.0);
    let redo = Point::new(720.0 - PADDING - SIDE / 2.0, PADDING + SIDE / 2.0);
    click_at(&mut ui, redo);
    assert!(events(ui).is_empty());
}

#[test]
fn a_wide_toolbar_shows_no_more_button() {
    let mut state = state();
    let mut ui = sized(&state, 720.0);
    let _ = ui.simulate([iced::Event::Window(window::Event::RedrawRequested(
        Instant::now(),
    ))]);
    let emitted = events(ui);
    assert_eq!(
        emitted,
        vec![Event::Resized(720.0)],
        "the strip measures itself"
    );
    let _ = apply(&mut state, emitted);
    assert_eq!(state.visible(), state.entries().len());
}

#[test]
fn a_narrow_toolbar_moves_the_rest_into_more() {
    let mut state = state();
    let mut ui = sized(&state, 180.0);
    let _ = ui.simulate([iced::Event::Window(window::Event::RedrawRequested(
        Instant::now(),
    ))]);
    let emitted = events(ui);
    let _ = apply(&mut state, emitted);
    assert_eq!(state.visible(), 3, "three toggles fit before More");

    // More sits where the separator would be.
    let mut ui = sized(&state, 180.0);
    click_at(&mut ui, slot(3));
    let emitted = events(ui);
    assert_eq!(
        emitted,
        vec![Event::Overflow(3, dropdown_menu::Event::Toggle)]
    );
    let _ = apply(&mut state, emitted);
    assert!(state.overflow().is_open());

    let mut ui = sized(&state, 180.0);
    for label in ["Align left", "Align centre", "Undo", "Redo"] {
        assert!(ui.find(label).is_ok(), "{label} is in the overflow menu");
    }
    assert!(ui.find("Bold").is_err(), "buttons in view stay out of it");
    ui.click("Align centre").expect("row is rendered");
    let emitted = events(ui);
    assert_eq!(apply(&mut state, emitted), Some(Output::Selected(CENTRE)));
    assert!(state.is_on(CENTRE) && !state.is_on(LEFT));
    assert!(!state.overflow().is_open());
}

#[test]
fn the_open_overflow_menu_takes_its_own_keys() {
    let mut state = state();
    let _ = state.update(Event::Resized(180.0));
    let _ = state.update(Event::Overflow(3, dropdown_menu::Event::Toggle));
    let mut ui = sized(&state, 180.0);
    assert_eq!(
        ui.tap_key(Key::Named(Named::ArrowDown)),
        event::Status::Captured
    );
    let _ = ui.tap_key(Key::Named(Named::Enter));
    let emitted = events(ui);
    assert_eq!(
        emitted,
        vec![
            Event::Overflow(3, dropdown_menu::Event::Next),
            Event::Overflow(3, dropdown_menu::Event::ActivateHighlighted),
        ]
    );
}

#[test]
fn escape_and_a_click_outside_close_the_overflow_menu() {
    let mut state = state();
    let _ = state.update(Event::Resized(180.0));
    let _ = state.update(Event::Overflow(3, dropdown_menu::Event::Toggle));

    let mut ui = sized(&state, 180.0);
    let _ = ui.tap_key(Key::Named(Named::Escape));
    assert_eq!(
        events(ui),
        vec![Event::Overflow(3, dropdown_menu::Event::Close)]
    );

    let mut ui = sized(&state, 180.0);
    click_at(&mut ui, Point::new(20.0, 280.0));
    assert_eq!(
        events(ui),
        vec![Event::Overflow(3, dropdown_menu::Event::Close)]
    );
}

#[test]
fn the_keyboard_moves_the_highlight_and_opens_more() {
    let mut state = state();
    let _ = state.update(Event::Resized(180.0));
    let keymap = toolbar::default_keymap();
    let key = |key: Key, modifiers: Modifiers| keys::Event { key, modifiers };
    let presses = [
        key(Key::Named(Named::F10), Modifiers::CTRL),
        key(Key::Named(Named::ArrowRight), Modifiers::empty()),
        key(Key::Named(Named::Space), Modifiers::empty()),
        key(Key::Named(Named::End), Modifiers::empty()),
    ];
    let mut outputs = Vec::new();
    for press in presses {
        let event = state
            .key_event(&keymap, &press)
            .expect("the toolbar claims it");
        outputs.extend(state.update(event));
    }
    assert_eq!(outputs, vec![Output::Toggled(ITALIC, true)]);
    assert_eq!(state.highlighted(), Some(Highlight::More));

    let enter = key(Key::Named(Named::Enter), Modifiers::empty());
    let event = state.key_event(&keymap, &enter).expect("Enter opens More");
    let _ = state.update(event);
    assert_eq!(state.overflow().highlighted(), Some(LEFT));

    let mut ui = sized(&state, 180.0);
    assert!(ui.find("Align left").is_ok());
}

#[test]
fn an_unfocused_toolbar_claims_no_keys() {
    let state = state();
    for named in [Named::ArrowRight, Named::Enter, Named::Escape] {
        let mut ui = sized(&state, 720.0);
        assert_eq!(ui.tap_key(Key::Named(named)), event::Status::Ignored);
        assert!(events(ui).is_empty());
    }
}

#[test]
fn a_toolbar_without_messages_is_inert() {
    let state = state();
    let element: Element<'_, Message> = toolbar(&state).into();
    let mut ui = Simulator::with_size(Settings::default(), Size::new(180.0, 300.0), element);
    ui.point_at(slot(0));
    let _ = ui.simulate(simulator::click());
    let _ = ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved {
        position: slot(3),
    })]);
    assert!(ui.into_messages().next().is_none());
}
