//! Lets a small control take presses from a margin around it without changing
//! the layout. A cursor on the margin reaches the content as if it were on the
//! content's nearest edge, so a button there hovers, presses and fires on
//! release exactly as it does under the cursor.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{
    Element, Event, Length, Padding, Point, Rectangle, Renderer, Size, Theme, Vector, mouse,
};

/// Wraps `content` so presses up to `slop` outside its bounds reach it. Keep
/// the margin within half the gap to a neighbour: where margins overlap, the
/// first widget to capture a press keeps it, but both draw as hovered.
pub(crate) fn hit_slop<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    slop: impl Into<Padding>,
) -> Element<'a, Message> {
    Element::new(HitSlop {
        content: content.into(),
        slop: slop.into(),
    })
}

struct HitSlop<'a, Message> {
    content: Element<'a, Message>,
    slop: Padding,
}

/// The cursor `content` with `bounds` should see: inside the bounds it is
/// unchanged, on the margin it moves to the nearest point inside, and further
/// out it is unchanged again.
fn reach(cursor: mouse::Cursor, bounds: Rectangle, slop: Padding) -> mouse::Cursor {
    let (position, levitating) = match cursor {
        mouse::Cursor::Available(position) => (position, false),
        mouse::Cursor::Levitating(position) => (position, true),
        mouse::Cursor::Unavailable => return cursor,
    };
    if bounds.contains(position) || !bounds.expand(slop).contains(position) {
        return cursor;
    }
    // `contains` excludes the right and bottom edges, so stay half a pixel inside.
    let inside = Point::new(
        position
            .x
            .clamp(bounds.x, (bounds.x + bounds.width - 0.5).max(bounds.x)),
        position
            .y
            .clamp(bounds.y, (bounds.y + bounds.height - 0.5).max(bounds.y)),
    );
    if levitating {
        mouse::Cursor::Levitating(inside)
    } else {
        mouse::Cursor::Available(inside)
    }
}

impl<Message> Widget<Message, Theme, Renderer> for HitSlop<'_, Message> {
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            reach(cursor, layout.bounds(), self.slop),
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
        let cursor = if shell.is_event_captured() {
            cursor
        } else {
            reach(cursor, layout.bounds(), self.slop)
        };
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
            reach(cursor, layout.bounds(), self.slop),
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

    const BOUNDS: Rectangle = Rectangle {
        x: 10.0,
        y: 10.0,
        width: 20.0,
        height: 20.0,
    };

    const SLOP: Padding = Padding::new(8.0);

    fn at(x: f32, y: f32) -> mouse::Cursor {
        mouse::Cursor::Available(Point::new(x, y))
    }

    #[test]
    fn a_cursor_inside_is_unchanged() {
        assert_eq!(reach(at(15.0, 15.0), BOUNDS, SLOP), at(15.0, 15.0));
    }

    #[test]
    fn a_cursor_on_the_margin_moves_to_the_nearest_point_inside() {
        assert_eq!(reach(at(4.0, 15.0), BOUNDS, SLOP), at(10.0, 15.0));
        assert_eq!(reach(at(35.0, 36.0), BOUNDS, SLOP), at(29.5, 29.5));
        assert!(reach(at(37.9, 2.1), BOUNDS, SLOP).is_over(BOUNDS));
    }

    #[test]
    fn a_cursor_beyond_the_margin_is_unchanged() {
        assert_eq!(reach(at(1.0, 15.0), BOUNDS, SLOP), at(1.0, 15.0));
        assert_eq!(reach(at(15.0, 39.0), BOUNDS, SLOP), at(15.0, 39.0));
    }

    #[test]
    fn each_side_has_its_own_margin() {
        let slop = Padding::from([8.0, 4.0]);
        assert_eq!(reach(at(15.0, 3.0), BOUNDS, slop), at(15.0, 10.0));
        assert_eq!(reach(at(5.0, 15.0), BOUNDS, slop), at(5.0, 15.0));
    }

    #[test]
    fn a_levitating_cursor_stays_levitating_and_no_cursor_stays_none() {
        let levitating = mouse::Cursor::Levitating(Point::new(4.0, 15.0));
        assert_eq!(
            reach(levitating, BOUNDS, SLOP),
            mouse::Cursor::Levitating(Point::new(10.0, 15.0))
        );
        assert_eq!(
            reach(mouse::Cursor::Unavailable, BOUNDS, SLOP),
            mouse::Cursor::Unavailable
        );
    }
}
