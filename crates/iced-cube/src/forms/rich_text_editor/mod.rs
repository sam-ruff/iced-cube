//! A rich text editor: bold, italic, underline, strikethrough, links,
//! text sizes, bulleted and numbered lists, drawn as they will look.
//!
//! The app owns a [`Content`] and applies every [`Action`] the editor sends
//! in `update`, as with iced's own text editor. The content keeps the
//! [`Document`], the selection and the undo history, so formatting from a
//! toolbar, a shortcut or the app is undone like typing. Read it back as
//! HTML with [`Content::html`] and as the plain-text alternative with
//! [`Content::plain_text`], and load a draft with [`Content::with_html`].
//!
//! The editor handles the usual keys: arrows, Home, End, Page Up and Page
//! Down (with Shift to select and Ctrl, or Alt on macOS, to move by word),
//! select all, copy, cut and paste. Undo, redo, word deletion, Escape and
//! the formatting chords are [`Shortcut`]s from the same set inputs and
//! textareas take, bound by [`default_keymap`]; pass a [`Shortcuts`] set to
//! [`RichTextEditor::shortcuts`] to switch some or all of them off, and
//! their keys reach the app instead.
//!
//! Copying puts plain text on the clipboard. Pasting that same text back
//! into the editor keeps its formatting.
//!
//! Bold uses [`theme::semibold`](crate::theme::semibold) and italic the
//! italic face of [`theme::font`](crate::theme::font), so the app must
//! load an italic face of its font for italic text to slant.
//!
//! ```no_run
//! use iced::Element;
//! use iced_cube::forms::rich_text_editor::{self, Action, Content, rich_text_editor};
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Edit(Action),
//! }
//!
//! struct Compose {
//!     body: Content,
//! }
//!
//! impl Compose {
//!     fn update(&mut self, message: Message) {
//!         let Message::Edit(action) = message;
//!         self.body.perform(action);
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         rich_text_editor(&self.body)
//!             .placeholder("Write your message")
//!             .on_action(Message::Edit)
//!             .into()
//!     }
//!
//!     fn send(&self) -> (String, String) {
//!         (self.body.html(), self.body.plain_text())
//!     }
//! }
//! ```

mod content;
pub mod document;
pub mod html;
mod layout;
pub mod plain;
mod snapshot;
mod widget;

use std::fmt;

use iced::widget::Id;
use iced::widget::text_input;
use iced::{Element, Length, Padding};

pub use crate::editing::{Shortcut, Shortcuts};
pub use content::{Action, Content, Edit, Format, Formatting, Motion};
pub use document::{
    Block, BlockKind, Document, Mark, Position, Run, Selection, Size, Style, normalise_link,
};

use crate::keys::Keymap;
use crate::primitives::input;
use crate::theme::{Tokens, space, text_size};

/// Minimum height while the height is [`Length::Shrink`], roughly four lines.
pub const MIN_HEIGHT: f32 = 96.0;

/// The default shortcuts: undo, redo, word deletion and Escape as in
/// [`input::default_keymap`], plus the formatting chords. Mod is Cmd on
/// macOS and Ctrl elsewhere.
///
/// | Keys | Shortcut |
/// | --- | --- |
/// | `Mod+Z` | [`Shortcut::Undo`] |
/// | `Mod+Shift+Z`, `Mod+Y` | [`Shortcut::Redo`] |
/// | `Ctrl+Backspace` | [`Shortcut::DeleteWordBackward`] |
/// | `Ctrl+Delete` | [`Shortcut::DeleteWordForward`] |
/// | `Escape` | [`Shortcut::EscapeUnfocus`] |
/// | `Mod+B` | [`Shortcut::Bold`] |
/// | `Mod+I` | [`Shortcut::Italic`] |
/// | `Mod+U` | [`Shortcut::Underline`] |
/// | `Mod+Shift+X` | [`Shortcut::Strikethrough`] |
/// | `Mod+Shift+8` | [`Shortcut::BulletedList`] |
/// | `Mod+Shift+7` | [`Shortcut::NumberedList`] |
/// | `Mod+K` | [`Shortcut::Link`] |
/// | `Mod+\` | [`Shortcut::ClearFormatting`] |
pub fn default_keymap() -> Keymap<Shortcut> {
    crate::editing::formatting_keymap()
}

/// The [`Action`] a shortcut sends, or `None` for Escape, which the editor
/// handles itself.
pub fn shortcut_action(shortcut: Shortcut) -> Option<Action> {
    Some(match shortcut {
        Shortcut::Undo => Action::Undo,
        Shortcut::Redo => Action::Redo,
        Shortcut::DeleteWordBackward => Action::Edit(Edit::DeleteWordBackward),
        Shortcut::DeleteWordForward => Action::Edit(Edit::DeleteWordForward),
        Shortcut::EscapeUnfocus => return None,
        Shortcut::Bold => Action::Format(Format::Toggle(Mark::Bold)),
        Shortcut::Italic => Action::Format(Format::Toggle(Mark::Italic)),
        Shortcut::Underline => Action::Format(Format::Toggle(Mark::Underline)),
        Shortcut::Strikethrough => Action::Format(Format::Toggle(Mark::Strikethrough)),
        Shortcut::BulletedList => Action::Format(Format::Block(BlockKind::Bullet)),
        Shortcut::NumberedList => Action::Format(Format::Block(BlockKind::Numbered)),
        Shortcut::Link => Action::RequestLink,
        Shortcut::ClearFormatting => Action::Format(Format::Clear),
    })
}

/// A rich text editor builder. Convert it into an [`Element`] to render.
///
/// An editor without an `on_action` message is rendered disabled.
pub struct RichTextEditor<'a, Message> {
    content: &'a Content,
    placeholder: Option<String>,
    id: Option<Id>,
    on_action: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    on_link: Option<Box<dyn Fn(String) -> Message + 'a>>,
    invalid: bool,
    width: Length,
    height: Length,
    min_height: f32,
    max_height: f32,
    text_size: f32,
    padding: Padding,
    shortcuts: Shortcuts,
    keymap: Keymap<Shortcut>,
}

/// Creates a rich text editor for `content`.
pub fn rich_text_editor<'a, Message>(content: &'a Content) -> RichTextEditor<'a, Message> {
    RichTextEditor {
        content,
        placeholder: None,
        id: None,
        on_action: None,
        on_link: None,
        invalid: false,
        width: Length::Fill,
        height: Length::Shrink,
        min_height: MIN_HEIGHT,
        max_height: f32::INFINITY,
        text_size: text_size::SM,
        padding: Padding::from([space::SM, space::MD]),
        shortcuts: Shortcuts::all(),
        keymap: default_keymap(),
    }
}

impl<'a, Message> RichTextEditor<'a, Message> {
    /// Text shown while the document is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Sets the widget id, used to focus the editor or find it in tests.
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the message for every action. Without it the editor is disabled.
    pub fn on_action(mut self, on_action: impl Fn(Action) -> Message + 'a) -> Self {
        self.on_action = Some(Box::new(on_action));
        self
    }

    pub fn on_action_maybe(mut self, on_action: Option<impl Fn(Action) -> Message + 'a>) -> Self {
        self.on_action = on_action.map(|f| Box::new(f) as Box<dyn Fn(Action) -> Message + 'a>);
        self
    }

    /// Sets the message for a link clicked with Ctrl, or Cmd on macOS, held.
    /// It carries the address; opening it is up to the app.
    pub fn on_link(mut self, on_link: impl Fn(String) -> Message + 'a) -> Self {
        self.on_link = Some(Box::new(on_link));
        self
    }

    /// Marks the value as invalid, drawing a destructive border.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Overrides the width. Editors fill the available width by default.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height. The default shrinks to the content and grows as
    /// lines are added, from [`MIN_HEIGHT`] up to the maximum height. A
    /// fixed or filling height scrolls once the text overflows.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the starting height used while the height is [`Length::Shrink`].
    pub fn min_height(mut self, min_height: f32) -> Self {
        self.min_height = min_height.max(0.0);
        self
    }

    /// Caps the height while it is [`Length::Shrink`]; longer text scrolls.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height.max(0.0);
        self
    }

    /// The size of normal text. Small, large and huge text scale from it.
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = size.max(1.0);
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets which [`Shortcut`]s the editor handles. All of them by
    /// default; pass [`Shortcuts::none`] to turn them all off. The keys of
    /// a shortcut that is off reach the app, and the editor's own keys,
    /// such as arrows and copy, always work.
    pub fn shortcuts(mut self, shortcuts: Shortcuts) -> Self {
        self.shortcuts = shortcuts;
        self
    }

    /// Replaces the [`default_keymap`], to bind the shortcuts to other keys.
    pub fn keymap(mut self, keymap: Keymap<Shortcut>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_action.is_some()
    }
}

impl<Message> fmt::Debug for RichTextEditor<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RichTextEditor")
            .field("placeholder", &self.placeholder)
            .field("id", &self.id)
            .field("enabled", &self.is_enabled())
            .field("invalid", &self.invalid)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("min_height", &self.min_height)
            .field("max_height", &self.max_height)
            .field("text_size", &self.text_size)
            .field("shortcuts", &self.shortcuts)
            .finish_non_exhaustive()
    }
}

impl<'a, Message: Clone + 'a> From<RichTextEditor<'a, Message>> for Element<'a, Message> {
    fn from(editor: RichTextEditor<'a, Message>) -> Self {
        Element::new(widget::Editor::new(editor))
    }
}

/// How the editor looks in a status.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Appearance {
    pub field: text_input::Style,
    /// Underlines under links.
    pub link: iced::Color,
    /// Quote bars and the text of quoted lines.
    pub quote: iced::Color,
    pub quote_text: iced::Color,
    pub caret: iced::Color,
    pub scrollbar: iced::Color,
}

/// The editor's look, matching the input's for the field itself.
pub fn style(tokens: &Tokens, status: text_input::Status, invalid: bool) -> Appearance {
    let field = input::style(tokens, status, invalid);
    let disabled = status == text_input::Status::Disabled;
    let fade = |colour| {
        if disabled {
            crate::theme::fade(colour, 0.5)
        } else {
            colour
        }
    };
    Appearance {
        field,
        link: fade(tokens.primary),
        quote: tokens.border,
        quote_text: fade(tokens.muted_foreground),
        caret: tokens.foreground,
        scrollbar: crate::theme::fade(tokens.foreground, 0.25),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys;
    use crate::theme::{dark, light};

    const STATES: [text_input::Status; 4] = [
        text_input::Status::Active,
        text_input::Status::Hovered,
        text_input::Status::Focused { is_hovered: false },
        text_input::Status::Disabled,
    ];

    #[test]
    fn default_builder_fills_shrinks_and_is_disabled() {
        let content = Content::new();
        let editor: RichTextEditor<'_, ()> = rich_text_editor(&content);
        assert_eq!(editor.width, Length::Fill);
        assert_eq!(editor.height, Length::Shrink);
        assert_eq!(editor.min_height, MIN_HEIGHT);
        assert_eq!(editor.text_size, text_size::SM);
        assert_eq!(editor.shortcuts, Shortcuts::all());
        assert_eq!(editor.keymap, default_keymap());
        assert!(!editor.is_enabled());
        let enabled = rich_text_editor::<Action>(&content).on_action(|action| action);
        assert!(enabled.is_enabled());
        let none: Option<fn(Action) -> Action> = None;
        assert!(!enabled.on_action_maybe(none).is_enabled());
    }

    #[test]
    fn builder_options_clamp() {
        let content = Content::new();
        let editor: RichTextEditor<'_, ()> = rich_text_editor(&content)
            .min_height(-1.0)
            .max_height(-1.0)
            .text_size(0.0)
            .shortcuts(Shortcuts::none());
        assert_eq!(editor.min_height, 0.0);
        assert_eq!(editor.max_height, 0.0);
        assert_eq!(editor.text_size, 1.0);
        assert_eq!(editor.shortcuts, Shortcuts::none());
    }

    #[test]
    fn every_shortcut_maps_to_an_action_except_escape() {
        for shortcut in <Shortcut as keys::Action>::ALL {
            assert_eq!(
                shortcut_action(*shortcut).is_none(),
                *shortcut == Shortcut::EscapeUnfocus,
                "{shortcut:?}"
            );
        }
        assert_eq!(
            shortcut_action(Shortcut::Bold),
            Some(Action::Format(Format::Toggle(Mark::Bold)))
        );
        assert_eq!(shortcut_action(Shortcut::Link), Some(Action::RequestLink));
    }

    #[test]
    fn the_keymap_can_be_overridden_and_unbound() {
        use crate::keys::Chord;
        let keymap = default_keymap()
            .unbind_action(&Shortcut::Link)
            .bind(Chord::character('l').command(), Shortcut::Link);
        let chord = Chord::character('k').command();
        assert_eq!(keymap.resolve(chord.key(), chord.modifiers()), None);
        let chord = Chord::character('l').command();
        assert_eq!(
            keymap.resolve(chord.key(), chord.modifiers()),
            Some(Shortcut::Link)
        );
    }

    #[test]
    fn style_matches_the_input_and_fades_when_disabled() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in STATES {
                for invalid in [false, true] {
                    let style = style(&tokens, status, invalid);
                    assert_eq!(style.field, input::style(&tokens, status, invalid));
                }
            }
            let active = style(&tokens, text_input::Status::Active, false);
            let disabled = style(&tokens, text_input::Status::Disabled, false);
            assert_ne!(active.link, disabled.link);
            assert_eq!(active.link, tokens.primary);
        }
    }
}
