//! Multi-line text editing.
//!
//! A tap places the caret and a vertical drag scrolls the text, as with the
//! mouse, so a textarea works on a touch screen.
//!
//! The app owns the [`Content`] and applies each [`Action`] in `update`:
//!
//! ```no_run
//! use iced::widget::text_editor::{Action, Content};
//! use iced_cube::textarea;
//!
//! #[derive(Debug, Clone)]
//! enum Message { Edit(Action) }
//!
//! let content = Content::new();
//! let view: iced::Element<'_, Message> = textarea(&content)
//!     .placeholder("Write a note")
//!     .on_action(Message::Edit)
//!     .into();
//! ```

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::text::highlighter::PlainText;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, input_method, overlay, renderer};
use iced::widget::text::LineHeight;
use iced::widget::text_editor::{self, Status, Style};
use iced::widget::{Id, container, text_input};
use iced::{
    Color, Element, Event, Length, Padding, Rectangle, Renderer, Size, Theme, Vector, mouse,
};

pub use iced::widget::text_editor::{Action, Content};

use crate::primitives::input;
use crate::primitives::touch_clicks::touch_clicks;
use crate::theme::{Tokens, space, text_size};

/// Minimum height while the height is [`Length::Shrink`], roughly three lines.
pub const MIN_HEIGHT: f32 = 80.0;
/// Line height as a multiple of the text size.
const LINE_HEIGHT: f32 = 1.5;

/// A textarea builder. Convert it into an [`Element`] to render.
///
/// A textarea without an `on_action` message is rendered disabled.
pub struct Textarea<'a, Message> {
    content: &'a Content,
    placeholder: Option<iced::widget::text::Fragment<'a>>,
    id: Option<Id>,
    on_action: Option<Box<dyn Fn(Action) -> Message + 'a>>,
    invalid: bool,
    width: Length,
    height: Length,
    min_height: f32,
}

/// Creates a textarea editing `content`.
pub fn textarea<'a, Message>(content: &'a Content) -> Textarea<'a, Message> {
    Textarea {
        content,
        placeholder: None,
        id: None,
        on_action: None,
        invalid: false,
        width: Length::Fill,
        height: Length::Shrink,
        min_height: MIN_HEIGHT,
    }
}

impl<'a, Message> Textarea<'a, Message> {
    pub fn placeholder(mut self, placeholder: impl iced::widget::text::IntoFragment<'a>) -> Self {
        self.placeholder = Some(placeholder.into_fragment());
        self
    }

    /// Sets the widget id, used to focus the textarea or find it in tests.
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the message produced for every edit. Without it the textarea is disabled.
    pub fn on_action(mut self, on_action: impl Fn(Action) -> Message + 'a) -> Self {
        self.on_action = Some(Box::new(on_action));
        self
    }

    pub fn on_action_maybe(mut self, on_action: Option<impl Fn(Action) -> Message + 'a>) -> Self {
        self.on_action = on_action.map(|f| Box::new(f) as Box<dyn Fn(Action) -> Message + 'a>);
        self
    }

    /// Marks the value as invalid, drawing a destructive border.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Overrides the width. Textareas fill the available width by default.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height. The default shrinks to the content and grows as
    /// lines are added, starting at [`MIN_HEIGHT`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the minimum height used while the height is [`Length::Shrink`].
    pub fn min_height(mut self, min_height: f32) -> Self {
        self.min_height = min_height.max(0.0);
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_action.is_some()
    }
}

impl<Message> fmt::Debug for Textarea<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Textarea")
            .field("placeholder", &self.placeholder)
            .field("id", &self.id)
            .field("enabled", &self.is_enabled())
            .field("invalid", &self.invalid)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("min_height", &self.min_height)
            .finish()
    }
}

impl<'a, Message: Clone + 'a> From<Textarea<'a, Message>> for Element<'a, Message> {
    fn from(textarea: Textarea<'a, Message>) -> Self {
        let invalid = textarea.invalid;
        let enabled = textarea.is_enabled();
        let composing = Rc::new(Cell::new(false));
        let hide_placeholder = Rc::clone(&composing);

        // iced never reports Disabled for an editor without `on_action`.
        let mut editor = iced::widget::text_editor(textarea.content)
            .size(text_size::SM)
            .line_height(LineHeight::Relative(LINE_HEIGHT))
            .padding(Padding::from([space::SM, space::MD]))
            .height(textarea.height)
            .style(move |theme, status| {
                let status = if enabled { status } else { Status::Disabled };
                let mut style = style(&Tokens::of(theme), status, invalid);
                // The keyboard draws the word it is composing over the
                // editor, so the placeholder would show through it.
                if hide_placeholder.get() && matches!(status, Status::Focused { .. }) {
                    style.placeholder = Color::TRANSPARENT;
                }
                style
            });
        if textarea.height == Length::Shrink {
            editor = editor.min_height(textarea.min_height);
        }
        if let Some(placeholder) = textarea.placeholder {
            editor = editor.placeholder(placeholder);
        }
        if let Some(id) = textarea.id {
            editor = editor.id(id);
        }
        if let Some(on_action) = textarea.on_action {
            editor = editor.on_action(on_action);
        }

        let editor = Element::new(Composing {
            content: editor.into(),
            composing,
        });

        // iced's editor reads only the mouse, so taps and drags are
        // translated for it.
        touch_clicks(container(editor).width(textarea.width))
            .scroll(true)
            .line_height(text_size::SM * LINE_HEIGHT)
            .into()
    }
}

/// Tracks whether the input method is composing a word, which iced's editor
/// does not consider when it draws its placeholder.
struct Composing<'a, Message> {
    content: Element<'a, Message>,
    composing: Rc<Cell<bool>>,
}

impl<Message> Widget<Message, Theme, Renderer> for Composing<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<bool>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(false)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
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
        self.composing.set(*tree.state.downcast_ref::<bool>());
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
        // Mirrors the editor, which keeps a composing word only while focused,
        // so a word composed in another field is never counted.
        if let Event::InputMethod(event) = event {
            match event {
                input_method::Event::Opened | input_method::Event::Closed => {
                    *tree.state.downcast_mut::<bool>() = false;
                }
                input_method::Event::Preedit(text, _) if editor_focused(&tree.children[0]) => {
                    *tree.state.downcast_mut::<bool>() = !text.is_empty();
                }
                _ => {}
            }
        }
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

fn editor_focused(tree: &Tree) -> bool {
    let tree::State::Some(state) = &tree.state else {
        return false;
    };
    state
        .downcast_ref::<text_editor::State<PlainText>>()
        .is_some_and(text_editor::State::is_focused)
}

/// The iced text editor style for a status, matching the input's look.
pub fn style(tokens: &Tokens, status: Status, invalid: bool) -> Style {
    let status = match status {
        Status::Active => text_input::Status::Active,
        Status::Hovered => text_input::Status::Hovered,
        Status::Focused { is_hovered } => text_input::Status::Focused { is_hovered },
        Status::Disabled => text_input::Status::Disabled,
    };
    let style = input::style(tokens, status, invalid);

    text_editor::Style {
        background: style.background,
        border: style.border,
        placeholder: style.placeholder,
        value: style.value,
        selection: style.selection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const STATES: [Status; 5] = [
        Status::Active,
        Status::Hovered,
        Status::Focused { is_hovered: false },
        Status::Focused { is_hovered: true },
        Status::Disabled,
    ];

    #[test]
    fn default_builder_shrinks_with_minimum_and_is_disabled() {
        let content = Content::new();
        let t: Textarea<'_, ()> = textarea(&content);
        assert_eq!(t.height, Length::Shrink);
        assert_eq!(t.width, Length::Fill);
        assert_eq!(t.min_height, MIN_HEIGHT);
        assert!(t.placeholder.is_none() && !t.invalid);
        assert!(!t.is_enabled());
    }

    #[test]
    fn on_action_enables_and_none_disables() {
        let content = Content::new();
        assert!(textarea::<Action>(&content).on_action(|a| a).is_enabled());
        let none: Option<fn(Action) -> Action> = None;
        assert!(
            !textarea::<Action>(&content)
                .on_action(|a| a)
                .on_action_maybe(none)
                .is_enabled()
        );
    }

    #[test]
    fn negative_min_height_clamps_to_zero() {
        let content = Content::new();
        let t: Textarea<'_, ()> = textarea(&content).min_height(-5.0);
        assert_eq!(t.min_height, 0.0);
    }

    #[test]
    fn style_matches_input_for_every_status() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for (editor, field) in STATES.into_iter().zip([
                text_input::Status::Active,
                text_input::Status::Hovered,
                text_input::Status::Focused { is_hovered: false },
                text_input::Status::Focused { is_hovered: true },
                text_input::Status::Disabled,
            ]) {
                for invalid in [false, true] {
                    let a = style(&tokens, editor, invalid);
                    let b = input::style(&tokens, field, invalid);
                    assert_eq!(a.border, b.border, "{editor:?}");
                    assert_eq!(a.background, b.background);
                    assert_eq!(a.value, b.value);
                    assert_eq!(a.placeholder, b.placeholder);
                }
            }
        }
    }

    #[test]
    fn invalid_and_disabled_are_distinct_from_active() {
        let tokens = Tokens::of(&light());
        let active = style(&tokens, Status::Active, false);
        assert_ne!(style(&tokens, Status::Active, true).border, active.border);
        assert_ne!(style(&tokens, Status::Disabled, false).value, active.value);
    }
}
