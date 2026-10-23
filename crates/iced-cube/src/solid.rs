//! A layer that keeps presses, from a mouse or a finger, from reaching what
//! lies under it.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, touch};

/// Like iced's `opaque`, which only stops mouse presses, but a finger
/// pressed on `content` is stopped too, so tapping a dialog's panel never
/// reaches the scrim that dismisses it.
pub(crate) fn solid<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    Element::new(Solid {
        content: iced::widget::opaque(content),
    })
}

struct Solid<'a, Message> {
    content: Element<'a, Message>,
}

/// Whether `event` is a finger pressed inside `bounds`.
fn finger_inside(event: &Event, cursor: mouse::Cursor, bounds: Rectangle) -> bool {
    matches!(event, Event::Touch(touch::Event::FingerPressed { .. })) && cursor.is_over(bounds)
}

impl<Message> Widget<Message, Theme, Renderer> for Solid<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
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
        if finger_inside(event, cursor, layout.bounds()) {
            shell.capture_event();
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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    #[test]
    fn only_a_finger_pressed_inside_is_stopped() {
        let bounds = Rectangle::new(Point::new(10.0, 10.0), Size::new(100.0, 50.0));
        let press = Event::Touch(touch::Event::FingerPressed {
            id: touch::Finger(1),
            position: Point::new(20.0, 20.0),
        });
        let inside = mouse::Cursor::Available(Point::new(20.0, 20.0));
        let outside = mouse::Cursor::Available(Point::new(200.0, 20.0));
        assert!(finger_inside(&press, inside, bounds));
        assert!(!finger_inside(&press, outside, bounds));
        let lift = Event::Touch(touch::Event::FingerLifted {
            id: touch::Finger(1),
            position: Point::new(20.0, 20.0),
        });
        assert!(!finger_inside(&lift, inside, bounds));
    }
}
