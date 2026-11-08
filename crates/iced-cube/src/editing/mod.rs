//! Undo, redo, word deletion and Escape for the library's text fields.
//!
//! iced's fields already handle select all, copy, cut, paste, moving by word
//! and Home and End. This adds the rest as a wrapper widget that resolves a
//! [`Keymap`] of [`Shortcut`]s while the field inside it has focus, and keeps
//! the undo history in the widget tree, so apps get it without storing
//! anything.

mod history;

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, input_method, overlay, renderer};
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::text_input::{self, Value};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::keys::{self, Chord, Keymap};

pub(crate) use history::{Edit, History, Restore};

/// A shortcut that inputs, textareas and the search fields inside other
/// components add to iced's own editing keys. Each can be switched off in a
/// [`Shortcuts`] set.
///
/// The formatting shortcuts, from [`Bold`](Shortcut::Bold) on, only apply
/// to the rich text editor; plain fields leave their keys alone and
/// [`default_keymap`] does not bind them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shortcut {
    Undo,
    Redo,
    DeleteWordBackward,
    DeleteWordForward,
    /// Escape removes focus from the field and goes no further, so a dialog
    /// around it stays open. Switched off, the field ignores Escape and
    /// leaves it to the app.
    EscapeUnfocus,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    BulletedList,
    NumberedList,
    /// Asks the app for a link address for the selection.
    Link,
    ClearFormatting,
}

impl Shortcut {
    const fn bit(self) -> u16 {
        1 << self as u16
    }

    /// Whether the shortcut formats rich text rather than editing plain text.
    pub const fn is_formatting(self) -> bool {
        matches!(
            self,
            Shortcut::Bold
                | Shortcut::Italic
                | Shortcut::Underline
                | Shortcut::Strikethrough
                | Shortcut::BulletedList
                | Shortcut::NumberedList
                | Shortcut::Link
                | Shortcut::ClearFormatting
        )
    }
}

impl keys::Action for Shortcut {
    const ALL: &'static [Self] = &[
        Shortcut::Undo,
        Shortcut::Redo,
        Shortcut::DeleteWordBackward,
        Shortcut::DeleteWordForward,
        Shortcut::EscapeUnfocus,
        Shortcut::Bold,
        Shortcut::Italic,
        Shortcut::Underline,
        Shortcut::Strikethrough,
        Shortcut::BulletedList,
        Shortcut::NumberedList,
        Shortcut::Link,
        Shortcut::ClearFormatting,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Shortcut::Undo => "Undo",
            Shortcut::Redo => "Redo",
            Shortcut::DeleteWordBackward => "Delete word backward",
            Shortcut::DeleteWordForward => "Delete word forward",
            Shortcut::EscapeUnfocus => "Unfocus",
            Shortcut::Bold => "Bold",
            Shortcut::Italic => "Italic",
            Shortcut::Underline => "Underline",
            Shortcut::Strikethrough => "Strikethrough",
            Shortcut::BulletedList => "Bulleted list",
            Shortcut::NumberedList => "Numbered list",
            Shortcut::Link => "Link",
            Shortcut::ClearFormatting => "Clear formatting",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Shortcut::Undo => "Undoes the last word typed, deletion, paste or cut.",
            Shortcut::Redo => "Redoes what undo took back.",
            Shortcut::DeleteWordBackward => {
                "Deletes back to the start of the word, or the selection if there is one."
            }
            Shortcut::DeleteWordForward => {
                "Deletes forward to the end of the word, or the selection if there is one."
            }
            Shortcut::EscapeUnfocus => {
                "Removes focus from the field without closing a dialog around it."
            }
            Shortcut::Bold => "Makes the selection bold, or plain again if it all is.",
            Shortcut::Italic => "Makes the selection italic, or upright again if it all is.",
            Shortcut::Underline => "Underlines the selection, or removes the underline.",
            Shortcut::Strikethrough => "Strikes through the selection, or removes the line.",
            Shortcut::BulletedList => "Turns the selected lines into a bulleted list, or back.",
            Shortcut::NumberedList => "Turns the selected lines into a numbered list, or back.",
            Shortcut::Link => "Asks for an address to link the selection to.",
            Shortcut::ClearFormatting => {
                "Removes bold, italic, underline, strikethrough, sizes and links from the selection."
            }
        }
    }
}

/// The set of [`Shortcut`]s a text field handles. Every shortcut is on by
/// default. A shortcut that is off is ignored by the field, so its keys
/// reach the app instead.
///
/// ```
/// use iced_cube::primitives::input::{Shortcut, Shortcuts};
///
/// let no_undo = Shortcuts::all().without(Shortcut::Undo).without(Shortcut::Redo);
/// assert!(!no_undo.contains(Shortcut::Undo));
/// assert!(no_undo.contains(Shortcut::EscapeUnfocus));
/// assert!(Shortcuts::none().with(Shortcut::Undo).contains(Shortcut::Undo));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcuts(u16);

impl Shortcuts {
    /// Every shortcut, the default.
    pub fn all() -> Self {
        <Shortcut as keys::Action>::ALL
            .iter()
            .fold(Self::none(), |set, shortcut| set.with(*shortcut))
    }

    /// No shortcuts: the field behaves as iced's plain widget.
    pub const fn none() -> Self {
        Self(0)
    }

    pub const fn with(self, shortcut: Shortcut) -> Self {
        Self(self.0 | shortcut.bit())
    }

    pub const fn without(self, shortcut: Shortcut) -> Self {
        Self(self.0 & !shortcut.bit())
    }

    pub const fn contains(self, shortcut: Shortcut) -> bool {
        self.0 & shortcut.bit() != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The shortcuts in the set, in the order of [`Shortcut`].
    pub fn iter(self) -> impl Iterator<Item = Shortcut> {
        <Shortcut as keys::Action>::ALL
            .iter()
            .copied()
            .filter(move |shortcut| self.contains(*shortcut))
    }

    /// Whether edits need recording for undo and redo.
    fn has_history(self) -> bool {
        self.contains(Shortcut::Undo) || self.contains(Shortcut::Redo)
    }
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self::all()
    }
}

impl fmt::Debug for Shortcuts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl FromIterator<Shortcut> for Shortcuts {
    fn from_iter<I: IntoIterator<Item = Shortcut>>(shortcuts: I) -> Self {
        shortcuts
            .into_iter()
            .fold(Self::none(), |set, shortcut| set.with(shortcut))
    }
}

/// The default shortcut keys. Mod is Cmd on macOS and Ctrl elsewhere;
/// words are deleted with Alt on macOS.
///
/// | Keys | Shortcut |
/// | --- | --- |
/// | `Mod+Z` | [`Shortcut::Undo`] |
/// | `Mod+Shift+Z`, `Mod+Y` | [`Shortcut::Redo`] |
/// | `Ctrl+Backspace` | [`Shortcut::DeleteWordBackward`] |
/// | `Ctrl+Delete` | [`Shortcut::DeleteWordForward`] |
/// | `Escape` | [`Shortcut::EscapeUnfocus`] |
pub fn default_keymap() -> Keymap<Shortcut> {
    Keymap::new()
        .bind(Chord::character('z').command(), Shortcut::Undo)
        .bind(Chord::character('z').command().shift(), Shortcut::Redo)
        .bind(Chord::character('y').command(), Shortcut::Redo)
        .bind(word(Named::Backspace), Shortcut::DeleteWordBackward)
        .bind(word(Named::Delete), Shortcut::DeleteWordForward)
        .bind(Chord::named(Named::Escape), Shortcut::EscapeUnfocus)
}

/// The [`default_keymap`] plus the formatting shortcuts, for rich text.
///
/// | Keys | Shortcut |
/// | --- | --- |
/// | `Mod+B` | [`Shortcut::Bold`] |
/// | `Mod+I` | [`Shortcut::Italic`] |
/// | `Mod+U` | [`Shortcut::Underline`] |
/// | `Mod+Shift+X` | [`Shortcut::Strikethrough`] |
/// | `Mod+Shift+8` | [`Shortcut::BulletedList`] |
/// | `Mod+Shift+7` | [`Shortcut::NumberedList`] |
/// | `Mod+K` | [`Shortcut::Link`] |
/// | `Mod+\` | [`Shortcut::ClearFormatting`] |
#[cfg(feature = "rich-text-editor")]
pub fn formatting_keymap() -> Keymap<Shortcut> {
    default_keymap()
        .bind(Chord::character('b').command(), Shortcut::Bold)
        .bind(Chord::character('i').command(), Shortcut::Italic)
        .bind(Chord::character('u').command(), Shortcut::Underline)
        .bind(
            Chord::character('x').command().shift(),
            Shortcut::Strikethrough,
        )
        .bind(
            Chord::character('8').command().shift(),
            Shortcut::BulletedList,
        )
        .bind(
            Chord::character('7').command().shift(),
            Shortcut::NumberedList,
        )
        .bind(Chord::character('k').command(), Shortcut::Link)
        .bind(Chord::character('\\').command(), Shortcut::ClearFormatting)
}

fn word(key: Named) -> Chord {
    let chord = Chord::named(key);
    if cfg!(target_os = "macos") {
        chord.alt()
    } else {
        chord.ctrl()
    }
}

/// Which way a word is deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Backward,
    Forward,
}

/// The editing widget an [`Editing`] wraps.
pub(crate) trait Field<Message> {
    /// The text as the app last provided it.
    fn text(&self) -> String;

    /// Whether the field is focused. `tree` is the wrapped widget's tree.
    fn is_focused(&self, tree: &Tree) -> bool;

    fn has_selection(&self, tree: &Tree) -> bool;

    /// Asks the app to show `restore`.
    fn restore(&self, tree: &mut Tree, restore: Restore, shell: &mut Shell<'_, Message>);

    /// Deletes a word or the selection. Returns whether anything was asked of the app.
    fn delete_word(
        &self,
        tree: &mut Tree,
        direction: Direction,
        shell: &mut Shell<'_, Message>,
    ) -> bool;

    fn unfocus(&self, tree: &mut Tree);
}

/// Wraps a field to add the enabled [`Shortcut`]s of `keymap` and an undo
/// history. `edited` must be set by the field's message closure whenever it
/// reports an edit.
pub(crate) struct Editing<'a, Message, F> {
    content: Element<'a, Message>,
    field: F,
    keymap: Keymap<Shortcut>,
    shortcuts: Shortcuts,
    edited: Rc<Cell<bool>>,
}

impl<'a, Message, F> Editing<'a, Message, F> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        field: F,
        keymap: Keymap<Shortcut>,
        shortcuts: Shortcuts,
        edited: Rc<Cell<bool>>,
    ) -> Self {
        Self {
            content: content.into(),
            field,
            keymap,
            shortcuts,
            edited,
        }
    }
}

/// The shortcut bound to a key press. Letters also match by their Latin
/// equivalent, so Ctrl+Z works on other keyboard layouts.
pub(crate) fn resolve(
    keymap: &Keymap<Shortcut>,
    key: &Key,
    physical_key: keyboard::key::Physical,
    modifiers: Modifiers,
) -> Option<Shortcut> {
    let by_character = |c: char| keymap.resolve(&Key::Character(c.to_string().into()), modifiers);
    keymap
        .resolve(key, modifiers)
        .or_else(|| by_character(key.to_latin(physical_key)?))
        // Some platforms report Shift+8 as `*`, so digits also match by key position.
        .or_else(|| by_character(digit(physical_key)?))
}

fn digit(physical_key: keyboard::key::Physical) -> Option<char> {
    use keyboard::key::{Code, Physical};

    let Physical::Code(code) = physical_key else {
        return None;
    };
    let digit = match code {
        Code::Digit0 => '0',
        Code::Digit1 => '1',
        Code::Digit2 => '2',
        Code::Digit3 => '3',
        Code::Digit4 => '4',
        Code::Digit5 => '5',
        Code::Digit6 => '6',
        Code::Digit7 => '7',
        Code::Digit8 => '8',
        Code::Digit9 => '9',
        _ => return None,
    };
    Some(digit)
}

/// A key pressed with Ctrl or Cmd is a shortcut, but some platforms still
/// attach its text, which iced would type. Returns the press without it.
/// Ctrl+Alt is AltGr on Windows, so it keeps its text.
pub(crate) fn without_shortcut_text(event: &Event) -> Option<Event> {
    let Event::Keyboard(keyboard::Event::KeyPressed {
        key,
        modified_key,
        physical_key,
        location,
        modifiers,
        repeat,
        text: Some(text),
    }) = event
    else {
        return None;
    };
    if !modifiers.command() || modifiers.alt() || text.chars().any(char::is_control) {
        return None;
    }
    Some(Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: modified_key.clone(),
        physical_key: *physical_key,
        location: *location,
        modifiers: *modifiers,
        repeat: *repeat,
        text: None,
    }))
}

/// Whether a captured event that edited nothing may have moved the caret.
/// iced's text input captures the release of V even after typing a plain v,
/// so key releases never count.
fn moves_caret(event: &Event) -> bool {
    !matches!(event, Event::Keyboard(keyboard::Event::KeyReleased { .. }))
}

/// How an event that produced an edit groups in the history.
pub(crate) fn classify(event: &Event) -> Edit {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key,
            modifiers,
            text,
            ..
        }) => match key.as_ref() {
            Key::Named(Named::Enter) => Edit::Insert("\n".into()),
            Key::Named(Named::Backspace | Named::Delete)
                if !modifiers.control() && !modifiers.alt() && !modifiers.logo() =>
            {
                Edit::Delete
            }
            _ => match text {
                Some(text) if !text.chars().any(char::is_control) => Edit::Insert(text.to_string()),
                _ => Edit::Other,
            },
        },
        Event::InputMethod(input_method::Event::Commit(text)) => Edit::Insert(text.clone()),
        _ => Edit::Other,
    }
}

impl<Message, F: Field<Message>> Widget<Message, Theme, Renderer> for Editing<'_, Message, F> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<History>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(History::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
        let history = tree.state.downcast_mut::<History>();
        if self.shortcuts.has_history() {
            history.settle(|| self.field.text());
        } else {
            *history = History::default();
        }
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let focused = self.field.is_focused(&tree.children[0]);
        if focused
            && !shell.is_event_captured()
            && let Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                ..
            }) = event
            && let Some(shortcut) = resolve(&self.keymap, key, *physical_key, *modifiers)
                .filter(|shortcut| !shortcut.is_formatting())
        {
            // A shortcut that is off is left to the app: the field does not
            // act on its keys either.
            if self.shortcuts.contains(shortcut) {
                self.perform(tree, shortcut, shell);
                shell.capture_event();
            }
            return;
        }

        let stripped = focused.then(|| without_shortcut_text(event)).flatten();
        let event = stripped.as_ref().unwrap_or(event);
        let replacing = focused && self.field.has_selection(&tree.children[0]);
        let captured = shell.is_event_captured();
        self.edited.set(false);
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if !self.shortcuts.has_history() {
            return;
        }
        let history = tree.state.downcast_mut::<History>();
        if self.edited.replace(false) {
            history.record(classify(event), replacing, || self.field.text());
        } else if !captured && shell.is_event_captured() && moves_caret(event) {
            history.interrupt();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<Message, F: Field<Message>> Editing<'_, Message, F> {
    fn perform(&self, tree: &mut Tree, shortcut: Shortcut, shell: &mut Shell<'_, Message>) {
        let (state, children) = (&mut tree.state, &mut tree.children);
        let history = state.downcast_mut::<History>();
        let Some(child) = children.first_mut() else {
            return;
        };
        let direction = match shortcut {
            Shortcut::Undo | Shortcut::Redo => {
                let text = self.field.text();
                let restore = if shortcut == Shortcut::Undo {
                    history.undo(text)
                } else {
                    history.redo(text)
                };
                if let Some(restore) = restore {
                    self.field.restore(child, restore, shell);
                }
                return;
            }
            Shortcut::EscapeUnfocus => {
                self.field.unfocus(child);
                history.interrupt();
                shell.request_redraw();
                return;
            }
            Shortcut::DeleteWordBackward => Direction::Backward,
            Shortcut::DeleteWordForward => Direction::Forward,
            // Plain fields have nothing to format.
            _ => return,
        };
        if self.field.delete_word(child, direction, shell) && self.shortcuts.has_history() {
            history.record(Edit::Other, true, || self.field.text());
        }
    }
}

impl<'a, Message: 'a, F: Field<Message> + 'a> From<Editing<'a, Message, F>>
    for Element<'a, Message>
{
    fn from(editing: Editing<'a, Message, F>) -> Self {
        Element::new(editing)
    }
}

type InputState = text_input::State<<Renderer as iced::advanced::text::Renderer>::Paragraph>;

/// A single-line iced text input.
pub(crate) struct Input<'a, Message> {
    value: String,
    secure: bool,
    on_input: Rc<dyn Fn(String) -> Message + 'a>,
}

impl<'a, Message> Input<'a, Message> {
    pub fn new(value: &str, secure: bool, on_input: Rc<dyn Fn(String) -> Message + 'a>) -> Self {
        Self {
            value: value.to_owned(),
            secure,
            on_input,
        }
    }
}

fn input_state(tree: &Tree) -> Option<&InputState> {
    match &tree.state {
        tree::State::Some(state) => state.downcast_ref::<InputState>(),
        tree::State::None => None,
    }
}

fn input_state_mut(tree: &mut Tree) -> Option<&mut InputState> {
    match &mut tree.state {
        tree::State::Some(state) => state.downcast_mut::<InputState>(),
        tree::State::None => None,
    }
}

impl<Message> Field<Message> for Input<'_, Message> {
    fn text(&self) -> String {
        self.value.clone()
    }

    fn is_focused(&self, tree: &Tree) -> bool {
        input_state(tree).is_some_and(InputState::is_focused)
    }

    fn has_selection(&self, tree: &Tree) -> bool {
        input_state(tree)
            .is_some_and(|state| state.cursor().selection(&Value::new(&self.value)).is_some())
    }

    fn restore(&self, tree: &mut Tree, restore: Restore, shell: &mut Shell<'_, Message>) {
        let caret = restore
            .text
            .get(..restore.caret)
            .map_or(0, |head| Value::new(head).len());
        shell.publish((self.on_input)(restore.text));
        if let Some(state) = input_state_mut(tree) {
            state.move_cursor_to(caret);
        }
    }

    fn delete_word(
        &self,
        tree: &mut Tree,
        direction: Direction,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let Some(state) = input_state_mut(tree) else {
            return false;
        };
        let value = Value::new(&self.value);
        let (start, end) = state
            .cursor()
            .selection(&value)
            .unwrap_or_else(|| word_range(&value, caret(state, &value), direction, self.secure));
        if start == end {
            return false;
        }
        let kept = format!(
            "{}{}",
            value.select(0, start),
            value.select(end, value.len())
        );
        shell.publish((self.on_input)(kept));
        state.move_cursor_to(start);
        true
    }

    fn unfocus(&self, tree: &mut Tree) {
        if let Some(state) = input_state_mut(tree) {
            state.unfocus();
        }
    }
}

fn caret(state: &InputState, value: &Value) -> usize {
    match state.cursor().state(value) {
        text_input::cursor::State::Index(index) => index,
        text_input::cursor::State::Selection { end, .. } => end,
    }
}

/// The graphemes a word deletion removes from `caret`. A secure field
/// hides its words, so it deletes to the start or end instead.
fn word_range(value: &Value, caret: usize, direction: Direction, secure: bool) -> (usize, usize) {
    match (direction, secure) {
        (Direction::Backward, true) => (0, caret),
        (Direction::Backward, false) => (value.previous_start_of_word(caret), caret),
        (Direction::Forward, true) => (caret, value.len()),
        (Direction::Forward, false) => (caret, value.next_end_of_word(caret)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::key::{NativeCode, Physical};

    fn press(chord: &str) -> Option<Shortcut> {
        let chord: Chord = chord.parse().ok()?;
        default_keymap().resolve(chord.key(), chord.modifiers())
    }

    fn key_press(key: Key, modifiers: Modifiers, text: Option<&str>) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers,
            repeat: false,
            text: text.map(Into::into),
        })
    }

    #[test]
    fn default_keymap_binds_undo_redo_and_word_deletion() {
        assert_eq!(press("Mod+Z"), Some(Shortcut::Undo));
        assert_eq!(press("Mod+Shift+Z"), Some(Shortcut::Redo));
        assert_eq!(press("Mod+Y"), Some(Shortcut::Redo));
        let word = if cfg!(target_os = "macos") {
            "Alt"
        } else {
            "Ctrl"
        };
        assert_eq!(
            press(&format!("{word}+Backspace")),
            Some(Shortcut::DeleteWordBackward)
        );
        assert_eq!(
            press(&format!("{word}+Delete")),
            Some(Shortcut::DeleteWordForward)
        );
        assert_eq!(press("Escape"), Some(Shortcut::EscapeUnfocus));
        assert_eq!(press("Z"), None);
        assert_eq!(press("Backspace"), None);
    }

    #[test]
    fn every_editing_shortcut_has_a_default_chord_and_formatting_has_none() {
        let keymap = default_keymap();
        for shortcut in <Shortcut as keys::Action>::ALL {
            assert_eq!(
                keymap.chords(shortcut).is_empty(),
                shortcut.is_formatting(),
                "{shortcut:?}"
            );
        }
    }

    #[cfg(feature = "rich-text-editor")]
    #[test]
    fn the_formatting_keymap_binds_every_shortcut() {
        let keymap = formatting_keymap();
        for shortcut in <Shortcut as keys::Action>::ALL {
            assert!(!keymap.chords(shortcut).is_empty(), "{shortcut:?}");
        }
        let physical = Physical::Unidentified(NativeCode::Unidentified);
        let b = Key::Character("b".into());
        assert_eq!(
            resolve(&keymap, &b, physical, Modifiers::COMMAND),
            Some(Shortcut::Bold)
        );
        assert_eq!(
            resolve(&default_keymap(), &b, physical, Modifiers::COMMAND),
            None
        );
    }

    #[cfg(feature = "rich-text-editor")]
    #[test]
    fn shifted_digits_match_by_key_position() {
        let star = Key::Character("*".into());
        let eight = Physical::Code(keyboard::key::Code::Digit8);
        assert_eq!(
            resolve(
                &formatting_keymap(),
                &star,
                eight,
                Modifiers::COMMAND | Modifiers::SHIFT
            ),
            Some(Shortcut::BulletedList)
        );
    }

    #[test]
    fn shortcut_sets() {
        let all = Shortcuts::all();
        assert_eq!(Shortcuts::default(), all);
        assert_eq!(all.iter().count(), <Shortcut as keys::Action>::ALL.len());
        assert!(Shortcuts::none().is_empty());
        assert_eq!(Shortcuts::none().iter().count(), 0);

        let no_undo = all.without(Shortcut::Undo);
        assert!(!no_undo.contains(Shortcut::Undo));
        assert!(no_undo.contains(Shortcut::Redo));
        assert_eq!(no_undo.with(Shortcut::Undo), all);
        assert_eq!(all.without(Shortcut::Undo).without(Shortcut::Undo), no_undo);

        let picked: Shortcuts = [Shortcut::EscapeUnfocus].into_iter().collect();
        assert_eq!(picked, Shortcuts::none().with(Shortcut::EscapeUnfocus));
        assert!(!picked.has_history());
        assert!(Shortcuts::none().with(Shortcut::Redo).has_history());
        assert_eq!(format!("{picked:?}"), "{EscapeUnfocus}");
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let keymap = default_keymap()
            .unbind_action(&Shortcut::Redo)
            .bind(Chord::named(Named::F4), Shortcut::Redo);
        let physical = Physical::Unidentified(NativeCode::Unidentified);
        let y = Key::Character("y".into());
        assert_eq!(resolve(&keymap, &y, physical, Modifiers::COMMAND), None);
        assert_eq!(
            resolve(
                &keymap,
                &Key::Named(Named::F4),
                physical,
                Modifiers::empty()
            ),
            Some(Shortcut::Redo)
        );
        assert_eq!(
            resolve(&Keymap::new(), &y, physical, Modifiers::COMMAND),
            None
        );
    }

    #[test]
    fn shift_with_a_capital_letter_still_matches() {
        let physical = Physical::Unidentified(NativeCode::Unidentified);
        let z = Key::Character("Z".into());
        assert_eq!(
            resolve(
                &default_keymap(),
                &z,
                physical,
                Modifiers::COMMAND | Modifiers::SHIFT
            ),
            Some(Shortcut::Redo)
        );
    }

    #[test]
    fn letters_match_by_their_latin_key_on_other_layouts() {
        let physical = Physical::Code(keyboard::key::Code::KeyZ);
        let cyrillic = Key::Character("я".into());
        assert_eq!(
            resolve(&default_keymap(), &cyrillic, physical, Modifiers::COMMAND),
            Some(Shortcut::Undo)
        );
    }

    #[test]
    fn classify_groups_typing_and_deletion() {
        let a = Key::Character("a".into());
        assert_eq!(
            classify(&key_press(a.clone(), Modifiers::empty(), Some("a"))),
            Edit::Insert("a".into())
        );
        assert_eq!(
            classify(&key_press(a, Modifiers::CTRL, Some("\u{1}"))),
            Edit::Other
        );
        assert_eq!(
            classify(&key_press(
                Key::Named(Named::Enter),
                Modifiers::empty(),
                Some("\r")
            )),
            Edit::Insert("\n".into())
        );
        assert_eq!(
            classify(&key_press(
                Key::Named(Named::Backspace),
                Modifiers::empty(),
                None
            )),
            Edit::Delete
        );
        assert_eq!(
            classify(&key_press(
                Key::Named(Named::Backspace),
                Modifiers::CTRL,
                None
            )),
            Edit::Other
        );
        assert_eq!(
            classify(&Event::InputMethod(input_method::Event::Commit(
                "word".into()
            ))),
            Edit::Insert("word".into())
        );
    }

    #[test]
    fn shortcut_text_is_dropped_but_altgr_and_plain_typing_keep_it() {
        let one = Key::Character("1".into());
        let Some(Event::Keyboard(keyboard::Event::KeyPressed { text, .. })) =
            without_shortcut_text(&key_press(one.clone(), Modifiers::COMMAND, Some("1")))
        else {
            panic!("Ctrl+1 loses its text");
        };
        assert_eq!(text, None);
        assert_eq!(
            without_shortcut_text(&key_press(
                Key::Character("@".into()),
                Modifiers::CTRL | Modifiers::ALT,
                Some("@")
            )),
            None
        );
        assert_eq!(
            without_shortcut_text(&key_press(one, Modifiers::empty(), Some("1"))),
            None
        );
    }

    #[test]
    fn word_ranges_follow_the_caret() {
        let value = Value::new("one two");
        assert_eq!(word_range(&value, 7, Direction::Backward, false), (4, 7));
        assert_eq!(word_range(&value, 0, Direction::Forward, false), (0, 3));
        assert_eq!(word_range(&value, 7, Direction::Backward, true), (0, 7));
        assert_eq!(word_range(&value, 2, Direction::Forward, true), (2, 7));
    }
}
