//! The wrapper that makes a button, switch, checkbox, radio or toggle
//! focusable.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::widget;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

use super::{State, draw_ring, named_press, report};

/// Whether a key press activates a focused button, switch or checkbox.
pub(crate) fn activates(event: &Event) -> bool {
    matches!(named_press(event), Some(Named::Enter | Named::Space))
}

/// What the ring goes around, and the corner radius of that part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ring {
    /// The control's corner radius; the ring adds its outset. Use
    /// [`radius::FULL`](crate::theme::radius::FULL) for pills and circles.
    pub(crate) radius: f32,
    /// Ring only the first child, such as a switch's track beside its label.
    pub(crate) first_child: bool,
}

impl Ring {
    /// A ring around the whole control.
    pub(crate) const fn around(radius: f32) -> Self {
        Self {
            radius,
            first_child: false,
        }
    }

    fn target(self, layout: Layout<'_>) -> Rectangle {
        let first = layout.children().next().filter(|_| self.first_child);
        first.map_or(layout.bounds(), |child| child.bounds())
    }
}

type OnActivate<'a, Message> = Box<dyn Fn() -> Message + 'a>;

/// Makes `content` focusable. While focused, Enter or Space sends
/// `on_activate`. Without a message the control is disabled and cannot be
/// focused.
pub(crate) fn focusable<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    on_activate: Option<OnActivate<'a, Message>>,
    ring: Ring,
) -> Focus<'a, Message> {
    Focus {
        content: content.into(),
        on_activate,
        ring,
        id: None,
    }
}

/// A focusable control. Convert it into an [`Element`] to render.
pub(crate) struct Focus<'a, Message> {
    content: Element<'a, Message>,
    on_activate: Option<OnActivate<'a, Message>>,
    ring: Ring,
    /// The id the control reports focus under, so the app can focus it
    /// with iced's `focus` operation.
    pub(crate) id: Option<widget::Id>,
}

impl<'a, Message: 'a> From<Focus<'a, Message>> for Element<'a, Message> {
    fn from(focus: Focus<'a, Message>) -> Self {
        Element::new(focus)
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Focus<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
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
        let focused = tree.state.downcast_ref::<State>().is_focused();
        if !focused || self.on_activate.is_none() {
            return;
        }
        draw_ring(renderer, theme, self.ring.target(layout), self.ring.radius);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.on_activate.is_some() {
            let state = tree.state.downcast_mut::<State>();
            report(operation, self.id.as_ref(), layout.bounds(), state);
        }
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
        let state = tree.state.downcast_mut::<State>();
        if state.update(event) {
            shell.request_redraw();
        }
        match &self.on_activate {
            Some(on_activate) if state.is_focused() && activates(event) => {
                shell.publish(on_activate());
                shell.capture_event();
                return;
            }
            None if state.is_focused() => state.unfocus(),
            _ => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard;

    fn key(named: Named, modifiers: keyboard::Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named),
            modified_key: keyboard::Key::Named(named),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    #[test]
    fn enter_and_space_activate_but_not_with_modifiers() {
        let none = keyboard::Modifiers::empty();
        assert!(activates(&key(Named::Enter, none)));
        assert!(activates(&key(Named::Space, none)));
        assert!(!activates(&key(Named::Enter, keyboard::Modifiers::CTRL)));
        assert!(!activates(&key(Named::Space, keyboard::Modifiers::SHIFT)));
        assert!(!activates(&key(Named::Tab, none)));
    }

    #[test]
    fn a_ring_goes_around_the_whole_control_unless_asked() {
        let ring = Ring::around(4.0);
        assert!(!ring.first_child);
        assert_eq!(ring.radius, 4.0);
    }
}
