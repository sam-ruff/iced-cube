//! Shows the first of several layouts that fits the available width, such
//! as a full breadcrumb trail and then shorter ones.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

/// Lays out each candidate at its natural width and shows the first that
/// fits; the last one is shown, within the usual limits, when none do.
pub(crate) fn first_fit<'a, Message: 'a>(
    candidates: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    Element::new(FirstFit {
        candidates,
        chosen: 0,
    })
}

/// The index of the first width that fits `available`, or the last index.
pub(crate) fn choose(widths: impl IntoIterator<Item = f32>, available: f32) -> usize {
    let mut last = 0;
    for (index, width) in widths.into_iter().enumerate() {
        if width <= available {
            return index;
        }
        last = index;
    }
    last
}

struct FirstFit<'a, Message> {
    candidates: Vec<Element<'a, Message>>,
    chosen: usize,
}

impl<Message> FirstFit<'_, Message> {
    fn parts<'b>(&'b self, tree: &'b Tree) -> Option<(&'b Element<'b, Message>, &'b Tree)> {
        let candidate = self.candidates.get(self.chosen)?;
        let state = tree.children.get(self.chosen)?;
        Some((candidate, state))
    }
}

impl<Message> Widget<Message, Theme, Renderer> for FirstFit<'_, Message> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn children(&self) -> Vec<Tree> {
        self.candidates.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.candidates);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let available = limits.max().width;
        let loose = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, limits.max().height));
        let last = self.candidates.len().saturating_sub(1);
        let mut widths = Vec::with_capacity(self.candidates.len());
        for (index, (candidate, state)) in self
            .candidates
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
        {
            if index == last {
                break;
            }
            let node = candidate.as_widget_mut().layout(state, renderer, &loose);
            widths.push(node.size().width);
        }
        widths.push(0.0);
        self.chosen = choose(widths, available);

        let (Some(candidate), Some(state)) = (
            self.candidates.get_mut(self.chosen),
            tree.children.get_mut(self.chosen),
        ) else {
            return layout::Node::new(Size::ZERO);
        };
        let node = candidate.as_widget_mut().layout(state, renderer, limits);
        layout::Node::with_children(node.size(), vec![node])
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
        let (Some((candidate, state)), Some(layout)) = (self.parts(tree), layout.children().next())
        else {
            return;
        };
        candidate
            .as_widget()
            .draw(state, renderer, theme, style, layout, cursor, viewport);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let (Some(candidate), Some(state), Some(layout)) = (
            self.candidates.get_mut(self.chosen),
            tree.children.get_mut(self.chosen),
            layout.children().next(),
        ) else {
            return;
        };
        candidate
            .as_widget_mut()
            .operate(state, layout, renderer, operation);
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
        let (Some(candidate), Some(state), Some(layout)) = (
            self.candidates.get_mut(self.chosen),
            tree.children.get_mut(self.chosen),
            layout.children().next(),
        ) else {
            return;
        };
        candidate.as_widget_mut().update(
            state, event, layout, cursor, renderer, clipboard, shell, viewport,
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
        let (Some((candidate, state)), Some(layout)) = (self.parts(tree), layout.children().next())
        else {
            return mouse::Interaction::None;
        };
        candidate
            .as_widget()
            .mouse_interaction(state, layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let candidate = self.candidates.get_mut(self.chosen)?;
        let state = tree.children.get_mut(self.chosen)?;
        let layout = layout.children().next()?;
        candidate
            .as_widget_mut()
            .overlay(state, layout, renderer, viewport, translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_first_width_that_fits() {
        assert_eq!(choose([300.0, 200.0, 100.0], 250.0), 1);
        assert_eq!(choose([300.0, 200.0, 100.0], 400.0), 0);
        assert_eq!(choose([300.0, 200.0, 100.0], 200.0), 1);
    }

    #[test]
    fn falls_back_to_the_last_when_nothing_fits() {
        assert_eq!(choose([300.0, 200.0, 100.0], 50.0), 2);
        assert_eq!(choose([], 50.0), 0);
    }
}
