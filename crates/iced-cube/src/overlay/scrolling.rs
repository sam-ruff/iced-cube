//! Lists of rows inside floating panels: a scrolling list that keeps one
//! row in view, and a column whose rows all take the width of the widest.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::operation::scrollable::{AbsoluteOffset, Scrollable};
use iced::advanced::widget::{Id, Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::widget::scrollable;
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::primitives::scroll_area;
use crate::theme::Tokens;

/// `rows` in a vertical scroll area that only scrolls once its panel is
/// limited in height, scrolled so that row `reveal` is in view whenever
/// `reveal` or the area's height changes.
pub(crate) fn scrolling<'a, Message: 'a>(
    rows: impl Into<Element<'a, Message>>,
    reveal: Option<usize>,
) -> Element<'a, Message> {
    let list = scrollable::Scrollable::with_direction(
        rows,
        scrollable::Direction::Vertical(scroll_area::scrollbar()),
    )
    .style(|theme, status| scroll_area::style(&Tokens::of(theme), status));
    Element::new(Reveal {
        content: list.into(),
        index: reveal,
    })
}

/// The offset that shows a row spanning `top` to `bottom` in a viewport of
/// `height` scrolled to `offset`, moving it as little as possible.
pub(crate) fn reveal_offset(offset: f32, height: f32, top: f32, bottom: f32) -> f32 {
    if top < offset {
        return top;
    }
    if bottom > offset + height {
        return (bottom - height).min(top);
    }
    offset
}

/// Scrolls the scrollable it wraps to a row after layout.
struct Reveal<'a, Message> {
    content: Element<'a, Message>,
    index: Option<usize>,
}

/// The row index last scrolled into view, and the height it was shown in.
#[derive(Debug, Default)]
struct Revealed(Option<(Option<usize>, f32)>);

/// Scrolls the first scrollable it meets so that `top..bottom`, in content
/// coordinates, is in view.
struct ScrollTo {
    top: f32,
    bottom: f32,
}

impl Operation for ScrollTo {
    fn traverse(&mut self, _operate: &mut dyn FnMut(&mut dyn Operation)) {}

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        let offset = reveal_offset(translation.y, bounds.height, self.top, self.bottom);
        if offset != translation.y {
            state.scroll_to(AbsoluteOffset {
                x: None,
                y: Some(offset),
            });
        }
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Reveal<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Revealed>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Revealed::default())
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
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let revealed = tree.state.downcast_mut::<Revealed>();
        let key = (self.index, node.size().height);
        if revealed.0 == Some(key) {
            return node;
        }
        revealed.0 = Some(key);
        let Some(index) = self.index else {
            return node;
        };
        let list = Layout::new(&node);
        let Some(content) = list.children().next() else {
            return node;
        };
        let Some(row) = content.children().nth(index) else {
            return node;
        };
        let top = row.bounds().y - content.bounds().y;
        let mut operation = ScrollTo {
            top,
            bottom: top + row.bounds().height,
        };
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], list, renderer, &mut operation);
        node
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

/// Rows stacked top to bottom, all as wide as the widest of them would be
/// on its own, up to `max_width` or the width available. Rows should fill
/// their width.
#[cfg(feature = "document-tabs")]
pub(crate) fn fitted<'a, Message: 'a>(
    rows: impl IntoIterator<Item = Element<'a, Message>>,
    max_width: f32,
) -> Element<'a, Message> {
    Element::new(Fitted {
        rows: rows.into_iter().collect(),
        max_width,
    })
}

#[cfg(feature = "document-tabs")]
struct Fitted<'a, Message> {
    rows: Vec<Element<'a, Message>>,
    max_width: f32,
}

#[cfg(feature = "document-tabs")]
impl<Message> Widget<Message, Theme, Renderer> for Fitted<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn children(&self) -> Vec<Tree> {
        self.rows.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.rows);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max().width.min(self.max_width);
        // Compressed limits lay fluid rows out at their natural width.
        let natural = layout::Limits::with_compression(
            Size::ZERO,
            Size::new(max, f32::INFINITY),
            Size::new(true, false),
        );
        let width = self
            .rows
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(row, tree)| {
                row.as_widget_mut()
                    .layout(tree, renderer, &natural)
                    .size()
                    .width
            })
            .fold(limits.min().width, f32::max)
            .min(max);

        let exact = layout::Limits::new(Size::new(width, 0.0), Size::new(width, f32::INFINITY));
        let mut y = 0.0;
        let nodes = self
            .rows
            .iter_mut()
            .zip(&mut tree.children)
            .map(|(row, tree)| {
                let node = row
                    .as_widget_mut()
                    .layout(tree, renderer, &exact)
                    .move_to((0.0, y));
                y += node.size().height;
                node
            })
            .collect();
        layout::Node::with_children(Size::new(width, y), nodes)
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
        for ((row, tree), layout) in self.rows.iter().zip(&tree.children).zip(layout.children()) {
            if !layout.bounds().intersects(viewport) {
                continue;
            }
            row.as_widget()
                .draw(tree, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            for ((row, tree), layout) in self
                .rows
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
            {
                row.as_widget_mut()
                    .operate(tree, layout, renderer, operation);
            }
        });
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
        for ((row, tree), layout) in self
            .rows
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            row.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
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
        self.rows
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((row, tree), layout)| {
                row.as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reveal_moves_the_least_distance() {
        assert_eq!(reveal_offset(0.0, 100.0, 20.0, 52.0), 0.0, "already shown");
        assert_eq!(reveal_offset(0.0, 100.0, 300.0, 332.0), 232.0, "below");
        assert_eq!(reveal_offset(250.0, 100.0, 64.0, 96.0), 64.0, "above");
        assert_eq!(
            reveal_offset(0.0, 20.0, 40.0, 72.0),
            40.0,
            "a row taller than the viewport shows its top"
        );
    }
}
