//! Shared pieces of the searchable list components: a widget that routes
//! key presses to a component while it has focus and floats a list below
//! it, and the styles of the list surface and its rows.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Id, Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::widget::container;
use iced::{
    Background, Border, Color, Element, Event, Length, Point, Rectangle, Renderer, Shadow, Size,
    Theme, Vector, keyboard, mouse, touch,
};

use crate::keys::{self, Chord, Keymap};
use crate::theme::{Tokens, fade, radius};

/// Space between the field and a floating list.
pub const GAP: f32 = 4.0;
/// Padding inside a row: vertical, then horizontal.
pub const ROW_PADDING: [f32; 2] = [6.0, 8.0];

/// How a list row is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RowStatus {
    #[default]
    Idle,
    /// Under the pointer or the keyboard highlight.
    Highlighted,
    Disabled,
}

impl RowStatus {
    pub const ALL: [RowStatus; 3] = [RowStatus::Idle, RowStatus::Highlighted, RowStatus::Disabled];
}

/// The floating or inline surface that holds the rows.
pub fn surface_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.background)),
        text_color: Some(tokens.foreground),
        border: Border {
            color: tokens.border,
            width: 1.0,
            radius: radius::MD.into(),
        },
        shadow: Shadow {
            color: fade(Color::BLACK, if tokens.is_dark { 0.5 } else { 0.1 }),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        snap: true,
    }
}

/// One row of a list. Destructive rows keep their colour when highlighted.
pub fn row_style(tokens: &Tokens, status: RowStatus, destructive: bool) -> container::Style {
    let (background, text) = match status {
        RowStatus::Idle => (None, tokens.foreground),
        RowStatus::Highlighted => (Some(tokens.accent), tokens.accent_foreground),
        RowStatus::Disabled => (None, tokens.muted_foreground),
    };
    let text = if destructive && status != RowStatus::Disabled {
        tokens.destructive
    } else {
        text
    };
    container::Style {
        background: background.map(Background::Color),
        text_color: Some(text),
        border: Border {
            radius: radius::SM.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The chords of `keymap` that do something right now, each paired with
/// the message it sends.
pub fn bindings<A: keys::Action, Message>(
    keymap: &Keymap<A>,
    message: impl Fn(A) -> Option<Message>,
) -> Vec<(Chord, Message)> {
    keymap
        .bindings()
        .filter_map(|(chord, action)| message(*action).map(|message| (chord.clone(), message)))
        .collect()
}

/// Wraps `content` so its key bindings fire while a text field inside it
/// has focus, or always when [`Scope::always`] is set.
pub fn scope<'a, Message>(content: impl Into<Element<'a, Message>>) -> Scope<'a, Message> {
    Scope {
        content: content.into(),
        popup: None,
        bindings: Vec::new(),
        always: false,
        on_press: None,
        on_dismiss: None,
    }
}

pub struct Scope<'a, Message> {
    content: Element<'a, Message>,
    popup: Option<Element<'a, Message>>,
    bindings: Vec<(Chord, Message)>,
    always: bool,
    on_press: Option<Message>,
    on_dismiss: Option<Message>,
}

impl<Message> std::fmt::Debug for Scope<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope")
            .field("popup", &self.popup.is_some())
            .field("bindings", &self.bindings.len())
            .field("always", &self.always)
            .finish_non_exhaustive()
    }
}

impl<'a, Message> Scope<'a, Message> {
    /// Floats `popup` below the content, as wide as the content.
    pub fn popup(mut self, popup: Option<Element<'a, Message>>) -> Self {
        self.popup = popup;
        self
    }

    pub fn bindings(mut self, bindings: Vec<(Chord, Message)>) -> Self {
        self.bindings = bindings;
        self
    }

    /// Handles bound keys even when nothing inside has focus.
    pub fn always(mut self, always: bool) -> Self {
        self.always = always;
        self
    }

    /// Sent when the pointer is pressed on the content.
    pub fn on_press(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    /// Sent when the content loses focus or a press lands outside both the
    /// content and the popup.
    pub fn on_dismiss(mut self, message: Option<Message>) -> Self {
        self.on_dismiss = message;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Scope<'a, Message>> for Element<'a, Message> {
    fn from(scope: Scope<'a, Message>) -> Self {
        Element::new(scope)
    }
}

#[derive(Debug, Default)]
struct State {
    focused: bool,
}

fn is_press(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Touch(touch::Event::FingerPressed { .. })
    )
}

/// Whether any focusable widget in a subtree has focus.
#[derive(Default)]
struct FindFocus(bool);

impl Operation for FindFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        self.0 |= state.is_focused();
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Scope<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        let popup = self.popup.as_ref().map_or_else(Tree::empty, Tree::new);
        vec![Tree::new(&self.content), popup]
    }

    fn diff(&self, tree: &mut Tree) {
        let [content, popup] = tree.children.as_mut_slice() else {
            tree.children = self.children();
            return;
        };
        content.diff(&self.content);
        match &self.popup {
            Some(element) => popup.diff(element),
            None => *popup = Tree::empty(),
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
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        let Some(content_tree) = children.first_mut() else {
            return;
        };

        if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event
            && (self.always || state.focused)
            && let Some((_, message)) = self
                .bindings
                .iter()
                .find(|(chord, _)| chord.matches(key, *modifiers))
        {
            shell.publish(message.clone());
            shell.capture_event();
            return;
        }

        if is_press(event)
            && cursor.is_over(layout.bounds())
            && let Some(message) = &self.on_press
        {
            shell.publish(message.clone());
        }

        self.content.as_widget_mut().update(
            content_tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let mut focus = FindFocus::default();
        self.content
            .as_widget_mut()
            .operate(content_tree, layout, renderer, &mut focus);
        if state.focused
            && !focus.0
            && let Some(message) = &self.on_dismiss
        {
            shell.publish(message.clone());
        }
        state.focused = focus.0;
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

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Scope {
            content,
            popup,
            on_dismiss,
            ..
        } = self;
        let [content_tree, popup_tree] = tree.children.as_mut_slice() else {
            return None;
        };
        let Some(popup) = popup else {
            return content.as_widget_mut().overlay(
                content_tree,
                layout,
                renderer,
                viewport,
                translation,
            );
        };

        Some(overlay::Element::new(Box::new(Popup {
            content: popup,
            tree: popup_tree,
            anchor: layout.bounds() + translation,
            on_dismiss: on_dismiss.as_ref(),
        })))
    }
}

struct Popup<'a, 'b, Message> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    anchor: Rectangle,
    on_dismiss: Option<&'b Message>,
}

impl<Message: Clone> overlay::Overlay<Message, Theme, Renderer> for Popup<'_, '_, Message> {
    /// Below the anchor, or above it when that leaves more room.
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let below = self.anchor.y + self.anchor.height + GAP;
        let space_below = (bounds.height - below).max(0.0);
        let space_above = (self.anchor.y - GAP).max(0.0);
        let width = self.anchor.width;
        let limits = layout::Limits::new(
            Size::new(width, 0.0),
            Size::new(width, space_below.max(space_above)),
        );
        let node = self
            .content
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);

        let height = node.size().height;
        let top = if height > space_below && space_above > space_below {
            self.anchor.y - GAP - height
        } else {
            below
        };
        node.move_to(Point::new(self.anchor.x, top))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();
        if is_press(event)
            && !cursor.is_over(bounds)
            && !cursor.is_over(self.anchor)
            && let Some(message) = self.on_dismiss
        {
            shell.publish(message.clone());
        }

        self.content.as_widget_mut().update(
            self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds,
        );

        let over = cursor.is_over(bounds);
        if over
            && (is_press(event)
                || matches!(event, Event::Mouse(mouse::Event::WheelScrolled { .. })))
        {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn surface_matches_the_menu_family() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = surface_style(&tokens);
            assert_eq!(style.background, Some(Background::Color(tokens.background)));
            assert_eq!(style.border.color, tokens.border);
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius, radius::MD.into());
        }
        let light = surface_style(&Tokens::of(&light())).shadow.color.a;
        let dark = surface_style(&Tokens::of(&dark())).shadow.color.a;
        assert!(dark > light);
    }

    #[test]
    fn highlighted_rows_use_the_accent() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = row_style(&tokens, RowStatus::Highlighted, false);
            assert_eq!(style.background, Some(Background::Color(tokens.accent)));
            assert_eq!(style.text_color, Some(tokens.accent_foreground));
            assert_eq!(style.border.radius, radius::SM.into());
        }
    }

    #[test]
    fn idle_and_disabled_rows_have_no_background() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = row_style(&tokens, RowStatus::Idle, false);
            let disabled = row_style(&tokens, RowStatus::Disabled, false);
            assert!(idle.background.is_none() && disabled.background.is_none());
            assert_eq!(idle.text_color, Some(tokens.foreground));
            assert_eq!(disabled.text_color, Some(tokens.muted_foreground));
        }
    }

    #[test]
    fn destructive_rows_stay_destructive_unless_disabled() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in RowStatus::ALL {
                let style = row_style(&tokens, status, true);
                let expected = if status == RowStatus::Disabled {
                    tokens.muted_foreground
                } else {
                    tokens.destructive
                };
                assert_eq!(style.text_color, Some(expected), "{status:?}");
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Test {
        Go,
        Stop,
    }

    impl keys::Action for Test {
        const ALL: &'static [Self] = &[Test::Go, Test::Stop];

        fn defaults() -> Keymap<Self> {
            Keymap::new()
        }

        fn name(self) -> &'static str {
            "Test"
        }

        fn description(self) -> &'static str {
            "Test."
        }
    }

    #[test]
    fn bindings_keep_only_actions_that_do_something() {
        let keymap = Keymap::new()
            .bind(Chord::character('g'), Test::Go)
            .bind(Chord::character('s'), Test::Stop);
        let bound = bindings(&keymap, |action| (action == Test::Go).then_some(1));
        assert_eq!(bound, vec![(Chord::character('g'), 1)]);
    }
}
