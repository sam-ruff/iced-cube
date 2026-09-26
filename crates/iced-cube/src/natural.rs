//! Keeps a shrink-width widget at its natural width when its parent has less
//! room, so short labels such as badges and buttons overflow as a whole
//! rather than being squeezed until their text breaks onto several lines.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

/// Wraps `content` so a [`Length::Shrink`] width is never limited by the
/// parent. Other widths are laid out as usual.
pub(crate) fn natural<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    Element::new(Natural {
        content: content.into(),
    })
}

/// The limits a shrink-width child is laid out with: the parent's, with no
/// upper bound on the width.
pub(crate) fn unbounded_width(limits: &layout::Limits) -> layout::Limits {
    layout::Limits::with_compression(
        limits.min(),
        Size::new(f32::INFINITY, limits.max().height),
        limits.compression(),
    )
}

struct Natural<'a, Message> {
    content: Element<'a, Message>,
}

impl<Message> Widget<Message, Theme, Renderer> for Natural<'_, Message> {
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
        let content = self.content.as_widget_mut();
        if content.size().width != Length::Shrink {
            return content.layout(tree, renderer, limits);
        }
        content.layout(tree, renderer, &unbounded_width(limits))
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
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
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
            .operate(tree, layout, renderer, operation);
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
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
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
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbounded_width_lifts_only_the_maximum_width() {
        let limits = layout::Limits::new(Size::new(4.0, 6.0), Size::new(80.0, 40.0));
        let loosened = unbounded_width(&limits);
        assert_eq!(loosened.min(), Size::new(4.0, 6.0));
        assert_eq!(loosened.max(), Size::new(f32::INFINITY, 40.0));
    }

    #[test]
    fn unbounded_width_keeps_compression() {
        let limits = layout::Limits::new(Size::ZERO, Size::new(80.0, 40.0)).width(Length::Shrink);
        assert_eq!(unbounded_width(&limits).compression(), limits.compression());
    }
}
