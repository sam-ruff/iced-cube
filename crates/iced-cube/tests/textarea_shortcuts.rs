#![cfg(feature = "textarea")]

mod support;

use iced::Element;
use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;
use iced::widget::text_editor::{Action, Content};
use iced_cube::textarea;
use iced_cube::textarea::{Shortcut, Shortcuts};
use support::{COMMAND, Harness, jump, letter, press};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Edit(Action),
}

#[derive(Debug, Default)]
struct Note {
    content: Content,
    shortcuts: Shortcuts,
}

fn view(note: &Note) -> Element<'_, Message> {
    textarea(&note.content)
        .id("notes")
        .shortcuts(note.shortcuts)
        .on_action(Message::Edit)
        .into()
}

fn update(note: &mut Note, message: Message) {
    let Message::Edit(action) = message;
    note.content.perform(action);
}

fn editor(text: &str) -> Harness<Note, Message> {
    with_shortcuts(text, Shortcuts::all())
}

fn with_shortcuts(text: &str, shortcuts: Shortcuts) -> Harness<Note, Message> {
    let mut ui = Harness::new(
        Note {
            content: Content::with_text(text),
            shortcuts,
        },
        view,
        update,
    );
    ui.focus("notes");
    ui
}

fn text(ui: &Harness<Note, Message>) -> String {
    ui.state.content.text()
}

#[test]
fn select_all_then_typing_replaces_the_text() {
    let mut ui = editor("old\ntext");
    ui.press(COMMAND, letter('a'));
    ui.type_text("new");
    assert_eq!(text(&ui), "new");
}

#[test]
fn copy_cut_and_paste_go_through_the_clipboard() {
    let mut ui = editor("one\ntwo");
    ui.press(COMMAND, letter('a'));
    ui.press(COMMAND, letter('c'));
    assert_eq!(ui.clipboard.0.as_deref(), Some("one\ntwo"));
    ui.press(COMMAND, letter('x'));
    assert_eq!(text(&ui), "");
    ui.press(COMMAND, letter('v'));
    ui.press(COMMAND, letter('v'));
    assert_eq!(text(&ui), "one\ntwoone\ntwo");
}

#[test]
fn word_jumps_move_the_caret_and_shift_extends() {
    let mut ui = editor("one two three");
    ui.press(jump(), Named::End);
    ui.press(jump(), Named::ArrowLeft);
    ui.type_text("X");
    assert_eq!(text(&ui), "one two Xthree");

    ui.tap(Named::Home);
    ui.press(jump(), Named::ArrowRight);
    ui.press(jump() | Modifiers::SHIFT, Named::ArrowRight);
    ui.type_text("Y");
    assert_eq!(text(&ui), "oneY Xthree");
}

#[test]
fn home_end_and_document_jumps() {
    let mut ui = editor("first\nsecond\nthird");
    ui.press(jump(), Named::End);
    ui.tap(Named::Home);
    ui.type_text("<");
    ui.tap(Named::End);
    ui.type_text(">");
    ui.press(jump(), Named::Home);
    ui.type_text("^");
    ui.press(jump(), Named::End);
    ui.type_text("$");
    assert_eq!(text(&ui), "^first\nsecond\n<third>$");

    ui.press(jump() | Modifiers::SHIFT, Named::Home);
    ui.type_text("all");
    assert_eq!(text(&ui), "all");
}

#[test]
fn ctrl_backspace_and_delete_remove_a_word() {
    let mut ui = editor("one two three");
    ui.press(jump(), Named::End);
    ui.press(jump(), Named::Backspace);
    assert_eq!(text(&ui), "one two ");

    ui.press(jump(), Named::Home);
    ui.press(jump(), Named::Delete);
    assert_eq!(text(&ui), " two ");
}

#[test]
fn ctrl_backspace_with_a_selection_removes_only_the_selection() {
    let mut ui = editor("one two");
    ui.press(jump(), Named::End);
    ui.press(Modifiers::SHIFT, Named::ArrowLeft);
    ui.press(jump(), Named::Backspace);
    assert_eq!(text(&ui), "one tw");
}

#[test]
fn undo_reverts_typing_a_word_at_a_time_and_redo_restores_it() {
    let mut ui = editor("");
    ui.type_text("hello world");
    assert!(ui.press(COMMAND, letter('z')));
    assert_eq!(text(&ui), "hello ");
    ui.press(COMMAND, letter('z'));
    assert_eq!(text(&ui), "");

    ui.press(COMMAND | Modifiers::SHIFT, letter('z'));
    assert_eq!(text(&ui), "hello ");
    ui.press(COMMAND, letter('y'));
    assert_eq!(text(&ui), "hello world");
}

#[test]
fn undo_restores_lines_and_puts_the_caret_back() {
    let mut ui = editor("top\nbottom");
    ui.press(jump(), Named::End);
    ui.tap(Named::Enter);
    ui.type_text("added");
    ui.press(jump(), Named::Home);
    ui.type_text("start ");
    assert_eq!(text(&ui), "start top\nbottom\nadded");

    ui.press(COMMAND, letter('z'));
    assert_eq!(text(&ui), "top\nbottom\nadded");
    ui.type_text("#");
    assert_eq!(text(&ui), "#top\nbottom\nadded");
}

#[test]
fn undo_of_a_cut_brings_the_text_back() {
    let mut ui = editor("keep\ncut this");
    ui.press(jump(), Named::End);
    ui.press(Modifiers::SHIFT, Named::Home);
    ui.press(COMMAND, letter('x'));
    assert_eq!(text(&ui), "keep\n");
    ui.press(COMMAND, letter('z'));
    assert_eq!(text(&ui), "keep\ncut this");
}

#[test]
fn undo_word_delete_and_backspaces() {
    let mut ui = editor("");
    ui.type_text("keep drop");
    ui.press(jump(), Named::Backspace);
    ui.tap(Named::Backspace);
    assert_eq!(text(&ui), "keep");
    ui.press(COMMAND, letter('z'));
    assert_eq!(text(&ui), "keep ");
    ui.press(COMMAND, letter('z'));
    assert_eq!(text(&ui), "keep drop");
}

#[test]
fn keys_arriving_in_one_frame_still_undo_cleanly() {
    let mut ui = editor("");
    let events: Vec<_> = iced_test::simulator::typewrite("ab cd").collect();
    ui.batch(events);
    assert_eq!(text(&ui), "ab cd");
    ui.batch([
        iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(COMMAND)),
        press(COMMAND, letter('z')),
    ]);
    assert_eq!(text(&ui), "");
}

#[test]
fn content_replaced_by_the_app_forgets_the_history() {
    let mut ui = editor("");
    ui.type_text("first document");
    ui.state.content = Content::with_text("second");
    ui.batch([]);
    ui.press(COMMAND, letter('z'));
    assert_eq!(text(&ui), "second");
}

#[test]
fn escape_unfocuses_without_reaching_the_app() {
    let mut ui = editor("text");
    assert!(ui.tap(Named::Escape), "a dialog around it must not close");
    ui.type_text("x");
    assert_eq!(text(&ui), "text");
    assert!(
        !ui.tap(Named::Escape),
        "an unfocused textarea passes Escape on"
    );
}

#[test]
fn ctrl_with_a_key_that_carries_text_types_nothing() {
    let mut ui = editor("ab");
    assert!(!ui.press_with_text(Modifiers::CTRL, '1'));
    assert!(!ui.press_with_text(Modifiers::CTRL, ','));
    ui.press_with_text(Modifiers::CTRL | Modifiers::ALT, '@');
    assert_eq!(text(&ui), "@ab");
}

#[test]
fn no_shortcuts_leaves_their_keys_to_the_app() {
    let mut ui = with_shortcuts("", Shortcuts::none());
    ui.type_text("hello");
    assert!(!ui.press(COMMAND, letter('z')));
    assert!(
        !ui.press(jump(), Named::Backspace),
        "not even one character goes"
    );
    assert_eq!(text(&ui), "hello");
    assert!(!ui.tap(Named::Escape));
    assert!(!ui.press_with_text(Modifiers::CTRL, '1'));
    assert_eq!(text(&ui), "hello");
}

#[test]
fn escape_off_on_its_own_reaches_the_app() {
    let mut ui = with_shortcuts("", Shortcuts::all().without(Shortcut::EscapeUnfocus));
    ui.type_text("one two");
    assert!(!ui.tap(Named::Escape), "a dialog around it may close");
    ui.press(COMMAND, letter('z'));
    assert_eq!(
        text(&ui),
        "one ",
        "undo still works and the field kept focus"
    );
}

#[test]
fn word_delete_off_on_its_own() {
    let mut ui = with_shortcuts("", Shortcuts::all().without(Shortcut::DeleteWordBackward));
    ui.type_text("one two");
    assert!(!ui.press(jump(), Named::Backspace));
    assert_eq!(text(&ui), "one two");
    ui.press(jump(), Named::ArrowLeft);
    ui.press(jump(), Named::Delete);
    assert_eq!(text(&ui), "one ");
}
