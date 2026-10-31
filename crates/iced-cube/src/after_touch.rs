//! Stops a touch leaving a hover look behind. A finger that lifts leaves the
//! cursor where it was, so iced's buttons stay hovered and tooltips open
//! after every tap until something else is touched.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, touch};

/// Wraps `content` so that, once a finger lifts, it sees the cursor as
/// hidden until a mouse is used again. Touch events always carry the real
/// cursor, so a held finger still presses and hovers.
pub(crate) fn after_touch<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    Element::new(AfterTouch {
        content: content.into(),
    })
}

struct AfterTouch<'a, Message> {
    content: Element<'a, Message>,
}

/// Whether the last pointer to leave was a finger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Touched(bool);

impl Touched {
    fn update(&mut self, event: &Event) {
        match event {
            Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. }) => {
                self.0 = true;
            }
            Event::Touch(touch::Event::FingerPressed { .. }) | Event::Mouse(_) => self.0 = false,
            _ => {}
        }
    }

    /// The cursor the content sees for `event`.
    fn cursor(self, event: Option<&Event>, cursor: mouse::Cursor) -> mouse::Cursor {
        if !self.0 || matches!(event, Some(Event::Touch(_))) {
            return cursor;
        }
        cursor.levitate()
    }
}

impl<Message> Widget<Message, Theme, Renderer> for AfterTouch<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Touched>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Touched::default())
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
        let cursor = tree.state.downcast_ref::<Touched>().cursor(None, cursor);
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
        let touched = tree.state.downcast_mut::<Touched>();
        touched.update(event);
        let cursor = touched.cursor(Some(event), cursor);
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
    use iced::Point;

    const AT: Point = Point::new(4.0, 4.0);

    fn finger(event: fn(touch::Finger, Point) -> touch::Event) -> Event {
        Event::Touch(event(touch::Finger(1), AT))
    }

    fn lifted(id: touch::Finger, position: Point) -> touch::Event {
        touch::Event::FingerLifted { id, position }
    }

    fn pressed(id: touch::Finger, position: Point) -> touch::Event {
        touch::Event::FingerPressed { id, position }
    }

    #[test]
    fn the_cursor_is_hidden_after_a_lift_until_a_mouse_or_finger_comes_back() {
        let cursor = mouse::Cursor::Available(AT);
        let redraw = Event::Window(iced::window::Event::RedrawRequested(
            iced::time::Instant::now(),
        ));
        let mut touched = Touched::default();
        assert_eq!(touched.cursor(Some(&redraw), cursor), cursor);

        touched.update(&finger(pressed));
        assert_eq!(touched.cursor(Some(&redraw), cursor), cursor);
        touched.update(&finger(lifted));
        assert_eq!(touched.cursor(Some(&redraw), cursor), cursor.levitate());
        assert_eq!(touched.cursor(None, cursor), cursor.levitate());
        assert_eq!(touched.cursor(Some(&finger(pressed)), cursor), cursor);

        touched.update(&Event::Mouse(mouse::Event::CursorMoved { position: AT }));
        assert_eq!(touched.cursor(Some(&redraw), cursor), cursor);

        touched.update(&finger(lifted));
        touched.update(&finger(pressed));
        assert_eq!(touched.cursor(Some(&redraw), cursor), cursor);
    }
}
