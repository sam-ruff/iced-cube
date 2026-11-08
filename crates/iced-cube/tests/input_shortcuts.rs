#![cfg(feature = "input")]

mod support;

use iced::Element;
use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;
use iced_cube::input;
use iced_cube::keys::{Chord, Keymap};
use iced_cube::primitives::input::{Shortcut, Shortcuts, default_keymap};
use support::{COMMAND, Harness, jump, letter};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Input(String),
}

#[derive(Debug, Default)]
struct Form {
    value: String,
    shortcuts: Shortcuts,
    keymap: Option<Keymap<Shortcut>>,
}

fn view(form: &Form) -> Element<'_, Message> {
    let field = input("Name", &form.value)
        .id("name")
        .shortcuts(form.shortcuts)
        .on_input(Message::Input);
    match &form.keymap {
        Some(keymap) => field.keymap(keymap.clone()).into(),
        None => field.into(),
    }
}

fn update(form: &mut Form, message: Message) {
    let Message::Input(value) = message;
    form.value = value;
}

fn field(value: &str) -> Harness<Form, Message> {
    with_shortcuts(value, Shortcuts::all())
}

fn with_shortcuts(value: &str, shortcuts: Shortcuts) -> Harness<Form, Message> {
    let mut ui = Harness::new(
        Form {
            value: value.to_owned(),
            shortcuts,
            keymap: None,
        },
        view,
        update,
    );
    ui.focus("name");
    ui
}

fn value(ui: &Harness<Form, Message>) -> &str {
    &ui.state.value
}

#[test]
fn select_all_then_typing_replaces_the_value() {
    let mut ui = field("old");
    ui.press(COMMAND, letter('a'));
    ui.type_text("new");
    assert_eq!(value(&ui), "new");
}

#[test]
fn copy_and_paste_go_through_the_clipboard() {
    let mut ui = field("copy me");
    ui.press(COMMAND, letter('a'));
    ui.press(COMMAND, letter('c'));
    assert_eq!(ui.clipboard.0.as_deref(), Some("copy me"));
    ui.tap(Named::End);
    ui.press(COMMAND, letter('v'));
    assert_eq!(value(&ui), "copy mecopy me");
}

#[test]
fn cut_removes_the_selection() {
    let mut ui = field("cut me");
    ui.press(COMMAND, letter('a'));
    ui.press(COMMAND, letter('x'));
    assert_eq!(value(&ui), "");
    assert_eq!(ui.clipboard.0.as_deref(), Some("cut me"));
}

#[test]
fn word_jumps_move_the_caret_and_shift_extends() {
    let mut ui = field("one two three");
    ui.press(jump(), Named::ArrowLeft);
    ui.type_text("X");
    assert_eq!(value(&ui), "one two Xthree");

    ui.tap(Named::Home);
    ui.press(jump(), Named::ArrowRight);
    ui.press(jump() | Modifiers::SHIFT, Named::ArrowRight);
    ui.type_text("Y");
    assert_eq!(value(&ui), "oneY Xthree");
}

#[test]
fn home_and_end_move_to_the_ends_and_shift_selects() {
    let mut ui = field("middle");
    ui.tap(Named::Home);
    ui.type_text("<");
    ui.tap(Named::End);
    ui.type_text(">");
    assert_eq!(value(&ui), "<middle>");

    ui.press(Modifiers::SHIFT, Named::Home);
    ui.type_text("all");
    assert_eq!(value(&ui), "all");
}

#[test]
fn ctrl_backspace_and_delete_remove_a_word() {
    let mut ui = field("one two three");
    ui.press(jump(), Named::Backspace);
    assert_eq!(value(&ui), "one two ");

    ui.tap(Named::Home);
    ui.press(jump(), Named::Delete);
    assert_eq!(value(&ui), " two ");
}

#[test]
fn undo_reverts_typing_a_word_at_a_time_and_redo_restores_it() {
    let mut ui = field("");
    ui.type_text("hello world");
    assert!(ui.press(COMMAND, letter('z')));
    assert_eq!(value(&ui), "hello ");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "");

    ui.press(COMMAND | Modifiers::SHIFT, letter('z'));
    assert_eq!(value(&ui), "hello ");
    ui.press(COMMAND, letter('y'));
    assert_eq!(value(&ui), "hello world");
}

#[test]
fn a_word_with_a_v_undoes_in_one_step() {
    let mut ui = field("");
    ui.type_text("love vivid");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "love ");
}

#[test]
fn undo_puts_the_caret_where_the_change_was() {
    let mut ui = field("");
    ui.type_text("abc ");
    ui.tap(Named::Home);
    ui.type_text("x");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "abc ");
    ui.type_text("y");
    assert_eq!(value(&ui), "yabc ");
}

#[test]
fn backspaces_undo_together_and_separately_from_typing() {
    let mut ui = field("");
    ui.type_text("abcd");
    ui.tap(Named::Backspace);
    ui.tap(Named::Backspace);
    assert_eq!(value(&ui), "ab");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "abcd");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "");
}

#[test]
fn a_paste_is_its_own_step() {
    let mut ui = field("");
    ui.clipboard.0 = Some("pasted".into());
    ui.type_text("ab");
    ui.press(COMMAND, letter('v'));
    ui.type_text("cd");
    assert_eq!(value(&ui), "abpastedcd");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "abpasted");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "ab");
}

#[test]
fn moving_the_caret_starts_a_new_step() {
    let mut ui = field("");
    ui.type_text("ab");
    ui.tap(Named::ArrowLeft);
    ui.type_text("x");
    assert_eq!(value(&ui), "axb");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "ab");
}

#[test]
fn a_new_edit_clears_redo() {
    let mut ui = field("");
    ui.type_text("one two");
    ui.press(COMMAND, letter('z'));
    ui.type_text("2");
    ui.press(COMMAND, letter('y'));
    assert_eq!(value(&ui), "one 2");
}

#[test]
fn undo_word_delete() {
    let mut ui = field("");
    ui.type_text("keep drop");
    ui.press(jump(), Named::Backspace);
    assert_eq!(value(&ui), "keep ");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "keep drop");
}

#[test]
fn a_value_changed_by_the_app_forgets_the_history() {
    let mut ui = field("");
    ui.type_text("draft");
    ui.state.value = "loaded".into();
    ui.batch([]);
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "loaded");
}

#[test]
fn no_shortcuts_leaves_their_keys_to_the_app() {
    let mut ui = with_shortcuts("", Shortcuts::none());
    ui.type_text("hello world");
    assert!(!ui.press(COMMAND, letter('z')), "the app hears Ctrl+Z");
    assert!(!ui.press(jump(), Named::Backspace));
    assert_eq!(value(&ui), "hello world");
    assert!(!ui.tap(Named::Escape), "the app hears Escape");
    ui.type_text("!");
    assert_eq!(value(&ui), "hello world!", "the field kept its focus");
}

#[test]
fn one_shortcut_off_leaves_the_others_working() {
    let mut ui = with_shortcuts("", Shortcuts::all().without(Shortcut::Undo));
    ui.type_text("keep drop");
    assert!(!ui.press(COMMAND, letter('z')));
    assert_eq!(value(&ui), "keep drop");
    assert!(ui.press(jump(), Named::Backspace));
    assert_eq!(value(&ui), "keep ");

    ui.state.shortcuts = Shortcuts::all();
    ui.press(COMMAND, letter('z'));
    assert_eq!(
        value(&ui),
        "keep drop",
        "the history was kept while undo was off"
    );
}

#[test]
fn escape_off_on_its_own() {
    let mut ui = with_shortcuts("a", Shortcuts::all().without(Shortcut::EscapeUnfocus));
    assert!(!ui.tap(Named::Escape));
    ui.type_text("b");
    ui.press(COMMAND, letter('z'));
    assert_eq!(value(&ui), "a");
}

#[test]
fn ctrl_key_text_is_dropped_even_with_no_shortcuts() {
    let mut ui = with_shortcuts("ab", Shortcuts::none());
    assert!(!ui.press_with_text(Modifiers::CTRL, '1'));
    assert_eq!(value(&ui), "ab");
}

#[test]
fn the_keymap_rebinds_shortcuts() {
    let mut ui = field("");
    ui.state.keymap = Some(default_keymap().unbind_action(&Shortcut::Undo));
    ui.type_text("hello");
    assert!(!ui.press(COMMAND, letter('z')));
    assert_eq!(value(&ui), "hello");

    ui.state.keymap = Some(Keymap::new().bind(Chord::named(Named::F2), Shortcut::Undo));
    ui.tap(Named::F2);
    assert_eq!(value(&ui), "");
}

#[test]
fn undo_with_nothing_to_undo_still_keeps_the_key() {
    let mut ui = field("same");
    assert!(ui.press(COMMAND, letter('z')));
    assert_eq!(value(&ui), "same");
}

#[test]
fn ctrl_with_a_key_that_carries_text_types_nothing() {
    let mut ui = field("ab");
    assert!(
        !ui.press_with_text(Modifiers::CTRL, '1'),
        "the app hears Ctrl+1"
    );
    assert!(!ui.press_with_text(Modifiers::CTRL, ','));
    assert_eq!(value(&ui), "ab");
}

#[test]
fn altgr_still_types() {
    let mut ui = field("a");
    ui.press_with_text(Modifiers::CTRL | Modifiers::ALT, '@');
    assert_eq!(value(&ui), "a@");
}

#[test]
fn an_unfocused_field_ignores_undo() {
    let mut ui = field("");
    ui.type_text("abc");
    ui.tap(Named::Escape);
    assert!(!ui.press(COMMAND, letter('z')));
    assert_eq!(value(&ui), "abc");
}
