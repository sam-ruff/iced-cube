#![cfg(feature = "rich-text-editor")]

mod support;

use iced::advanced::input_method;
use iced::keyboard::key::Named;
use iced::keyboard::{self, Modifiers};
use iced::widget::{Id, container};
use iced::{Element, Event, Point};
use iced_cube::rich_text_editor::{
    Action, BlockKind, Content, Format, Mark, Position, Selection, Shortcut, Shortcuts,
    rich_text_editor,
};
use iced_test::{Error, Simulator, simulator};
use support::{COMMAND, Harness, jump, letter};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Edit(Action),
    Link(String),
}

struct Note {
    content: Content,
    shortcuts: Shortcuts,
    enabled: bool,
    width: f32,
    links: Vec<String>,
}

fn view(note: &Note) -> Element<'_, Message> {
    let editor = rich_text_editor(&note.content)
        .id("body")
        .shortcuts(note.shortcuts)
        .on_action_maybe(note.enabled.then_some(Message::Edit))
        .on_link(Message::Link);
    container(editor).width(note.width).into()
}

fn update(note: &mut Note, message: Message) {
    match message {
        Message::Edit(action) => note.content.perform(action),
        Message::Link(url) => note.links.push(url),
    }
}

fn note(content: Content) -> Note {
    Note {
        content,
        shortcuts: Shortcuts::all(),
        enabled: true,
        width: 600.0,
        links: Vec::new(),
    }
}

fn editor(text: &str) -> Harness<Note, Message> {
    with(note(Content::with_text(text)))
}

fn with(note: Note) -> Harness<Note, Message> {
    let mut ui = Harness::new(note, view, update);
    ui.focus("body");
    ui
}

fn text(ui: &Harness<Note, Message>) -> String {
    ui.state.content.text()
}

fn bold_runs(ui: &Harness<Note, Message>, block: usize) -> Vec<(String, bool)> {
    ui.state.content.document().blocks()[block]
        .runs()
        .iter()
        .map(|run| (run.text.clone(), run.style.bold))
        .collect()
}

fn shift() -> Modifiers {
    Modifiers::SHIFT
}

#[test]
fn typing_and_enter_build_blocks() {
    let mut ui = editor("");
    ui.type_text("hello");
    ui.tap(keyboard::Key::Named(Named::Enter));
    ui.type_text("world");
    assert_eq!(text(&ui), "hello\nworld");
    assert_eq!(
        ui.state.content.selection(),
        Selection::caret(Position::new(1, 5))
    );
}

#[test]
fn shift_arrows_select_part_of_a_word_and_mod_b_bolds_it() {
    let mut ui = editor("hello");
    ui.tap(keyboard::Key::Named(Named::Home));
    ui.tap(keyboard::Key::Named(Named::ArrowRight));
    ui.press(shift(), keyboard::Key::Named(Named::ArrowRight));
    ui.press(shift(), keyboard::Key::Named(Named::ArrowRight));
    assert_eq!(ui.state.content.selected_text(), "el");
    assert!(ui.press(COMMAND, letter('b')));
    assert_eq!(
        bold_runs(&ui, 0),
        [
            ("h".into(), false),
            ("el".into(), true),
            ("lo".into(), false)
        ]
    );
    assert!(ui.state.content.formatting().bold);
}

#[test]
fn every_formatting_shortcut_works() {
    let mut ui = editor("one\ntwo");
    ui.press(COMMAND, letter('a'));
    ui.press(COMMAND, letter('i'));
    ui.press(COMMAND, letter('u'));
    ui.press(COMMAND | shift(), letter('x'));
    let formatting = ui.state.content.formatting();
    assert!(formatting.italic && formatting.underline && formatting.strikethrough);
    ui.press(COMMAND | shift(), letter('8'));
    assert_eq!(ui.state.content.formatting().block, Some(BlockKind::Bullet));
    ui.press(COMMAND | shift(), letter('7'));
    assert_eq!(
        ui.state.content.formatting().block,
        Some(BlockKind::Numbered)
    );
    ui.press(COMMAND, letter('\\'));
    let formatting = ui.state.content.formatting();
    assert!(!formatting.italic && !formatting.underline && !formatting.strikethrough);
    assert!(ui.press(COMMAND, letter('k')));
    assert_eq!(ui.state.content.link_requests(), 1);
}

#[test]
fn a_shortcut_that_is_off_reaches_the_app() {
    let mut ui = with(Note {
        shortcuts: Shortcuts::all().without(Shortcut::Bold),
        ..note(Content::with_text("text"))
    });
    ui.press(COMMAND, letter('a'));
    assert!(!ui.press(COMMAND, letter('b')));
    assert!(!ui.state.content.has_formatting());
    assert!(ui.press(COMMAND, letter('i')));
    assert!(ui.state.content.formatting().italic);
}

#[test]
fn no_shortcuts_leaves_their_keys_to_the_app_and_editing_keys_work() {
    let mut ui = with(Note {
        shortcuts: Shortcuts::none(),
        ..note(Content::with_text("one two"))
    });
    for (modifiers, key) in [
        (COMMAND, letter('z')),
        (COMMAND, letter('b')),
        (COMMAND, letter('k')),
        (jump(), keyboard::Key::Named(Named::Backspace)),
        (Modifiers::empty(), keyboard::Key::Named(Named::Escape)),
    ] {
        assert!(!ui.press(modifiers, key.clone()), "{modifiers:?} {key:?}");
    }
    assert_eq!(text(&ui), "one two");
    assert!(!ui.state.content.has_formatting());
    assert!(ui.tap(keyboard::Key::Named(Named::Backspace)));
    ui.press(COMMAND, letter('a'));
    ui.type_text("new");
    assert_eq!(text(&ui), "new");
}

#[test]
fn undo_and_redo_cover_typing_and_formatting() {
    let mut ui = editor("");
    ui.type_text("one two");
    ui.press(COMMAND, letter('a'));
    ui.press(COMMAND, letter('b'));
    assert!(ui.state.content.has_formatting());
    assert!(ui.press(COMMAND, letter('z')));
    assert!(!ui.state.content.has_formatting());
    ui.press(COMMAND, letter('z'));
    assert_eq!(text(&ui), "one ");
    ui.press(COMMAND | shift(), letter('z'));
    assert_eq!(text(&ui), "one two");
    ui.press(COMMAND, letter('y'));
    assert!(ui.state.content.has_formatting());
}

#[test]
fn word_deletion_and_escape() {
    let mut ui = editor("one two");
    assert!(ui.press(jump(), keyboard::Key::Named(Named::Backspace)));
    assert_eq!(text(&ui), "one ");
    assert!(ui.tap(keyboard::Key::Named(Named::Escape)));
    ui.type_text("x");
    assert_eq!(text(&ui), "one ");
}

#[test]
fn the_input_method_composes_then_commits() {
    let mut ui = editor("a");
    ui.batch([
        Event::InputMethod(input_method::Event::Opened),
        Event::InputMethod(input_method::Event::Preedit("ka".into(), Some(2..2))),
    ]);
    assert_eq!(text(&ui), "a");
    ui.batch([Event::InputMethod(input_method::Event::Commit("か".into()))]);
    assert_eq!(text(&ui), "aか");
    ui.tap(keyboard::Key::Named(Named::Escape));
    ui.batch([Event::InputMethod(input_method::Event::Commit("x".into()))]);
    assert_eq!(text(&ui), "aか");
}

#[test]
fn copying_and_pasting_inside_the_editor_keeps_formatting() {
    let mut ui = editor("bold plain");
    ui.press(COMMAND, letter('a'));
    ui.press(COMMAND, letter('c'));
    ui.tap(keyboard::Key::Named(Named::Home));
    for _ in 0..4 {
        ui.press(shift(), keyboard::Key::Named(Named::ArrowRight));
    }
    ui.press(COMMAND, letter('b'));
    ui.press(COMMAND, letter('c'));
    assert_eq!(ui.clipboard.0.as_deref(), Some("bold"));
    ui.press(jump(), keyboard::Key::Named(Named::End));
    ui.press(COMMAND, letter('v'));
    assert_eq!(text(&ui), "bold plainbold");
    assert_eq!(
        bold_runs(&ui, 0),
        [
            ("bold".into(), true),
            (" plain".into(), false),
            ("bold".into(), true)
        ]
    );
    ui.press(COMMAND, letter('a'));
    ui.press(COMMAND, letter('x'));
    assert!(ui.state.content.is_empty());
    assert_eq!(ui.clipboard.0.as_deref(), Some("bold plainbold"));
}

#[test]
fn text_pasted_from_elsewhere_is_plain_and_splits_lines() {
    let mut ui = editor("");
    ui.clipboard.0 = Some("first\r\nsecond".into());
    ui.press(COMMAND, letter('v'));
    assert_eq!(text(&ui), "first\nsecond");
    assert!(!ui.state.content.has_formatting());
}

#[test]
fn a_click_focuses_and_places_the_caret() {
    let mut ui = Harness::new(note(Content::with_text("hello\nworld")), view, update);
    ui.type_text("x");
    assert_eq!(text(&ui), "hello\nworld");
    ui.click(Point::new(13.0, 16.0));
    assert_eq!(
        ui.state.content.selection(),
        Selection::caret(Position::new(0, 0))
    );
    ui.type_text("x");
    assert_eq!(text(&ui), "xhello\nworld");
    ui.click(Point::new(500.0, 38.0));
    assert_eq!(ui.state.content.selection().head, Position::new(1, 5));
}

#[test]
fn a_click_outside_unfocuses() {
    let mut ui = with(Note {
        width: 300.0,
        ..note(Content::with_text("a"))
    });
    ui.click(Point::new(500.0, 16.0));
    ui.type_text("b");
    assert_eq!(text(&ui), "a");
}

#[test]
fn mod_click_on_a_link_sends_its_address() {
    let mut ui = with(note(Content::with_html(
        "<a href=\"https://example.com\">example</a> text",
    )));
    let before = ui.state.content.selection();
    ui.batch([Event::Keyboard(keyboard::Event::ModifiersChanged(COMMAND))]);
    ui.click(Point::new(20.0, 16.0));
    assert_eq!(ui.state.links, ["https://example.com"]);
    assert_eq!(ui.state.content.selection(), before);
}

#[test]
fn up_and_down_follow_wrapped_lines_and_blocks() {
    let mut ui = with(Note {
        width: 140.0,
        ..note(Content::with_text("one two three four five six\nlast"))
    });
    ui.press(jump(), keyboard::Key::Named(Named::Home));
    ui.tap(keyboard::Key::Named(Named::ArrowDown));
    let head = ui.state.content.selection().head;
    assert_eq!(head.block, 0, "a wrapped line is still the first block");
    assert!(head.offset > 0);
    for _ in 0..10 {
        ui.tap(keyboard::Key::Named(Named::ArrowDown));
    }
    assert_eq!(ui.state.content.selection().head.block, 1);
    ui.tap(keyboard::Key::Named(Named::End));
    assert_eq!(ui.state.content.selection().head, Position::new(1, 4));
}

#[test]
fn a_disabled_editor_ignores_input() {
    let mut ui = with(Note {
        enabled: false,
        ..note(Content::with_text("read only"))
    });
    ui.type_text("x");
    ui.press(COMMAND, letter('b'));
    assert_eq!(text(&ui), "read only");
    assert!(!ui.state.content.has_formatting());
}

#[test]
fn bold_typed_after_mod_b_stays_bold() {
    let mut ui = editor("plain ");
    ui.press(COMMAND, letter('b'));
    ui.type_text("bold");
    ui.press(COMMAND, letter('b'));
    ui.type_text(" again");
    assert_eq!(
        bold_runs(&ui, 0),
        [
            ("plain ".into(), false),
            ("bold".into(), true),
            (" again".into(), false)
        ]
    );
}

#[test]
fn enter_continues_a_list_and_twice_ends_it() {
    let mut ui = editor("");
    ui.press(COMMAND | shift(), letter('8'));
    ui.type_text("one");
    ui.tap(keyboard::Key::Named(Named::Enter));
    ui.type_text("two");
    ui.tap(keyboard::Key::Named(Named::Enter));
    ui.tap(keyboard::Key::Named(Named::Enter));
    ui.type_text("after");
    let kinds: Vec<BlockKind> = ui
        .state
        .content
        .document()
        .blocks()
        .iter()
        .map(|block| block.kind())
        .collect();
    assert_eq!(
        kinds,
        [BlockKind::Bullet, BlockKind::Bullet, BlockKind::Paragraph]
    );
    assert_eq!(ui.state.content.plain_text(), "- one\n- two\nafter");
}

/// Renders an empty editor and returns a hash of the window.
fn render(
    placeholder: bool,
    focused: bool,
    composing: Option<&str>,
    name: &str,
) -> Result<String, Error> {
    let content = Content::new();
    let editor = rich_text_editor(&content)
        .id("body")
        .on_action(Message::Edit);
    let editor = if placeholder {
        editor.placeholder("Write a message")
    } else {
        editor
    };
    let mut ui = simulator(Element::from(editor));
    if focused {
        ui.click(Id::new("body"))?;
    }
    if let Some(word) = composing {
        let _ = ui.simulate([Event::InputMethod(input_method::Event::Preedit(
            word.to_owned(),
            Some(word.len()..word.len()),
        ))]);
    }
    hash(&mut ui, name)
}

fn hash(ui: &mut Simulator<'_, Message>, name: &str) -> Result<String, Error> {
    hash_in(ui, &iced_cube::theme::light(), name)
}

fn hash_in(
    ui: &mut Simulator<'_, Message>,
    theme: &iced::Theme,
    name: &str,
) -> Result<String, Error> {
    let directory =
        std::env::temp_dir().join(format!("iced-cube-rich-text-{name}-{}", std::process::id()));
    let _ = ui.snapshot(theme)?.matches_hash(directory.join(name))?;
    let mut hash = String::new();
    for file in std::fs::read_dir(&directory)?.filter_map(Result::ok) {
        hash = std::fs::read_to_string(file.path())?;
    }
    std::fs::remove_dir_all(&directory)?;
    Ok(hash)
}

#[test]
fn the_placeholder_hides_while_a_word_is_composed() -> Result<(), Error> {
    let composing = render(true, true, Some("ka"), "composing")?;
    let bare = render(false, true, Some("ka"), "bare")?;
    let waiting = render(true, true, None, "waiting")?;
    assert!(!composing.is_empty());
    assert_eq!(composing, bare);
    assert_ne!(composing, waiting);
    Ok(())
}

/// Font faces depend on the fonts a machine has, so the gallery's tests
/// check them with bundled fonts; lines and lists are drawn the same
/// everywhere.
#[test]
fn lines_and_lists_change_what_is_drawn() -> Result<(), Error> {
    let draw = |format: Option<Format>, name: &str| -> Result<String, Error> {
        let mut content = Content::with_text("Some text");
        if let Some(format) = format {
            content.perform(Action::SelectAll);
            content.perform(Action::Format(format));
        }
        let mut ui = simulator(Element::from(
            rich_text_editor::<Message>(&content).on_action(Message::Edit),
        ));
        hash(&mut ui, name)
    };
    let plain = draw(None, "plain")?;
    let underline = draw(Some(Format::Toggle(Mark::Underline)), "underline")?;
    let struck = draw(Some(Format::Toggle(Mark::Strikethrough)), "struck")?;
    let listed = draw(Some(Format::Block(BlockKind::Bullet)), "listed")?;
    let linked = draw(Some(Format::Link("example.com".into())), "linked")?;
    for (name, other) in [
        ("underline", &underline),
        ("struck", &struck),
        ("listed", &listed),
        ("linked", &linked),
    ] {
        assert_ne!(&plain, other, "{name}");
    }
    assert_ne!(underline, struck);
    Ok(())
}

#[test]
fn links_take_the_primary_colour() -> Result<(), Error> {
    let red = iced_cube::theme::Config::light()
        .primary(iced::Color::from_rgb(0.8, 0.1, 0.1))
        .build();
    let draw = |html: &str, theme: &iced::Theme, name: &str| -> Result<String, Error> {
        let content = Content::with_html(html);
        let mut ui = simulator(Element::from(
            rich_text_editor::<Message>(&content).on_action(Message::Edit),
        ));
        hash_in(&mut ui, theme, name)
    };
    let plain = "<div>plain words</div>";
    let linked = "<div><a href=\"https://e.com\">linked words</a></div>";
    let light = iced_cube::theme::light();
    assert_eq!(
        draw(plain, &light, "plain-light")?,
        draw(plain, &red, "plain-red")?
    );
    assert_ne!(
        draw(linked, &light, "linked-light")?,
        draw(linked, &red, "linked-red")?
    );
    Ok(())
}
