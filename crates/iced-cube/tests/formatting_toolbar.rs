#![cfg(feature = "formatting-toolbar")]

mod support;

use iced::keyboard::key::Named;
use iced::keyboard::{self, Modifiers};
use iced::time::Instant;
use iced::widget::{column, container};
use iced::{Element, Event, Point, window};
use iced_cube::formatting_toolbar::{
    self, LINK_FIELD, Output, SIZE_WIDTH, Tool, formatting_toolbar,
};
use iced_cube::rich_text_editor::{Action, Content, Size, rich_text_editor};
use iced_cube::toolbar::{GAP, PADDING, SEPARATOR_WIDTH};
use support::{COMMAND, Harness, letter};

const BODY: &str = "body";
/// Side of a small toolbar button.
const SIDE: f32 = 32.0;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Edit(Action),
    Format(formatting_toolbar::Event),
}

struct Compose {
    body: Content,
    formatting: formatting_toolbar::State,
    width: f32,
    /// The widget the app would focus next, standing in for a focus task.
    focus: Option<&'static str>,
}

fn view(compose: &Compose) -> Element<'_, Message> {
    container(column![
        formatting_toolbar(&compose.formatting).on_event(Message::Format),
        rich_text_editor(&compose.body)
            .id(BODY)
            .on_action(Message::Edit),
    ])
    .width(compose.width)
    .into()
}

fn update(compose: &mut Compose, message: Message) {
    let output = match message {
        Message::Edit(action) => {
            compose.body.perform(action);
            compose.formatting.sync(&compose.body)
        }
        Message::Format(event) => compose.formatting.update(event, &compose.body),
    };
    compose.focus = match output {
        Some(Output::Format(action)) => {
            compose.body.perform(action);
            compose.formatting.sync(&compose.body);
            Some(BODY)
        }
        Some(Output::EditLink) => Some(LINK_FIELD),
        Some(Output::Closed) => Some(BODY),
        Some(Output::Unfocus) | None => compose.focus,
    };
}

fn compose(text: &str, width: f32) -> Harness<Compose, Message> {
    let mut compose = Compose {
        body: Content::with_text(text),
        formatting: formatting_toolbar::State::new(),
        width,
        focus: None,
    };
    compose.formatting.sync(&compose.body);
    let mut ui = Harness::new(compose, view, update);
    ui.focus(BODY);
    // The toolbar measures itself as it is drawn.
    ui.batch([Event::Window(
        window::Event::RedrawRequested(Instant::now()),
    )]);
    ui
}

/// Runs the focus the app asked for, as its task would.
fn settle(ui: &mut Harness<Compose, Message>) {
    if let Some(id) = ui.state.focus.take() {
        ui.focus(id);
    }
}

/// The centre of the strip button in `slot`, after the size menu.
fn slot(slot: usize) -> Point {
    let strip = PADDING + SIZE_WIDTH + PADDING + 1.0;
    let x = strip + PADDING + slot as f32 * (SIDE + GAP) + SIDE / 2.0;
    Point::new(x, PADDING + SIDE / 2.0)
}

#[test]
fn the_buttons_follow_the_caret() {
    let mut ui = compose("plain", 720.0);
    assert!(!ui.state.formatting.is_on(Tool::Bold));
    ui.press(COMMAND, letter('a'));
    ui.press(COMMAND, letter('b'));
    assert!(ui.state.formatting.is_on(Tool::Bold));
    ui.press(Modifiers::SHIFT | COMMAND, letter('8'));
    assert!(ui.state.formatting.is_on(Tool::Bullets));
    ui.tap(keyboard::Key::Named(Named::End));
    ui.type_text(" more");
    assert!(ui.state.formatting.is_on(Tool::Bold));
}

#[test]
fn a_button_formats_the_selection_and_gives_focus_back() {
    let mut ui = compose("plain text", 720.0);
    ui.press(COMMAND, letter('a'));
    ui.click(slot(1));
    assert_eq!(ui.state.focus, Some(BODY));
    assert!(ui.state.body.formatting().italic);
    assert!(ui.state.formatting.is_on(Tool::Italic));
    settle(&mut ui);
    ui.press(COMMAND, letter('i'));
    assert!(!ui.state.body.formatting().italic);
    assert!(!ui.state.formatting.is_on(Tool::Italic));
}

#[test]
fn the_size_menu_opens_and_sets_the_size() {
    let mut ui = compose("text", 720.0);
    ui.press(COMMAND, letter('a'));
    ui.click(Point::new(PADDING + SIZE_WIDTH / 2.0, PADDING + SIDE / 2.0));
    ui.tap(keyboard::Key::Named(Named::ArrowDown));
    ui.tap(keyboard::Key::Named(Named::ArrowDown));
    ui.tap(keyboard::Key::Named(Named::ArrowDown));
    ui.tap(keyboard::Key::Named(Named::Enter));
    assert_eq!(ui.state.body.formatting().size, Some(Size::Large));
    assert_eq!(ui.state.formatting.size(), Some(Size::Large));
    assert_eq!(ui.state.focus, Some(BODY));
}

#[test]
fn the_link_shortcut_opens_the_field_and_enter_applies_it() {
    let mut ui = compose("see here", 720.0);
    assert!(ui.press(COMMAND, letter('k')));
    assert_eq!(ui.state.focus, Some(LINK_FIELD));
    assert_eq!(ui.state.formatting.link(), Some(""));
    settle(&mut ui);
    ui.type_text("example.com");
    ui.tap(keyboard::Key::Named(Named::Enter));
    assert_eq!(ui.state.formatting.link(), None);
    assert_eq!(ui.state.focus, Some(BODY));
    assert_eq!(
        ui.state.body.formatting().link.as_deref(),
        Some("https://example.com")
    );
    assert!(ui.state.formatting.is_on(Tool::Link));
    assert_eq!(ui.state.body.plain_text(), "see here <https://example.com>");
}

#[test]
fn escape_closes_the_link_field_and_returns_focus() {
    let mut ui = compose("see here", 720.0);
    ui.press(COMMAND, letter('k'));
    settle(&mut ui);
    ui.type_text("e.com");
    assert!(ui.tap(keyboard::Key::Named(Named::Escape)));
    assert_eq!(ui.state.formatting.link(), None);
    assert_eq!(ui.state.focus, Some(BODY));
    settle(&mut ui);
    ui.type_text("!");
    assert_eq!(ui.state.body.text(), "see here!");
    assert!(!ui.state.body.has_formatting());
}

#[test]
fn closing_the_size_menu_hands_focus_back() {
    let mut ui = compose("text", 720.0);
    ui.click(Point::new(PADDING + SIZE_WIDTH / 2.0, PADDING + SIDE / 2.0));
    assert!(ui.tap(keyboard::Key::Named(Named::Escape)));
    assert_eq!(ui.state.focus, Some(BODY));
    settle(&mut ui);
    ui.type_text("!");
    assert_eq!(ui.state.body.text(), "text!");
}

/// The centre of the Link button: four buttons, a separator, two
/// buttons and another separator come before it.
fn link_button() -> Point {
    let strip = PADDING + SIZE_WIDTH + PADDING + 1.0;
    let x = strip + PADDING + 6.0 * (SIDE + GAP) + 2.0 * (SEPARATOR_WIDTH + GAP) + SIDE / 2.0;
    Point::new(x, PADDING + SIDE / 2.0)
}

#[test]
fn pressing_link_twice_closes_the_field() {
    let mut ui = compose("see here", 720.0);
    ui.click(link_button());
    assert_eq!(ui.state.formatting.link(), Some(""));
    assert_eq!(ui.state.focus, Some(LINK_FIELD));
    settle(&mut ui);
    ui.click(link_button());
    assert_eq!(ui.state.formatting.link(), None);
    assert_eq!(ui.state.focus, Some(BODY));
}

#[test]
fn a_button_pressed_from_the_keyboard_ends_keyboard_focus() {
    let mut state = formatting_toolbar::State::new();
    let content = Content::with_text("x");
    let keymap = iced_cube::toolbar::default_keymap();
    let key = |key: keyboard::Key, modifiers: Modifiers| iced_cube::keys::Event { key, modifiers };
    let f10 = key(keyboard::Key::Named(Named::F10), Modifiers::CTRL);
    let enter = key(keyboard::Key::Named(Named::Enter), Modifiers::empty());
    let run = |state: &mut formatting_toolbar::State, event: &iced_cube::keys::Event| {
        state
            .key_event(&keymap, event)
            .and_then(|event| state.update(event, &content))
    };
    assert_eq!(run(&mut state, &f10), Some(Output::Unfocus));
    assert!(matches!(run(&mut state, &enter), Some(Output::Format(_))));
    assert!(!state.toolbar().is_focused());
    assert_eq!(run(&mut state, &f10), Some(Output::Unfocus));
}

#[test]
fn a_narrow_toolbar_moves_buttons_into_more() {
    let wide = compose("x", 720.0);
    let narrow = compose("x", 300.0);
    let all = wide.state.formatting.toolbar().entries().len();
    assert_eq!(wide.state.formatting.toolbar().visible(), all);
    let shown = narrow.state.formatting.toolbar().visible();
    assert!(shown < all && shown >= 4, "{shown} of {all}");
}
