//! A small label shown while the pointer rests on an element.
//!
//! A finger shows it while held. Once the finger lifts, the bubble stays
//! closed until a mouse moves, so a tap on a phone does not leave it open.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::time::Duration;
use iced::widget::{self, container, text};
use iced::{
    Background, Border, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse,
    touch,
};

use crate::theme::{Tokens, radius, space, text_size};

/// Where the bubble appears relative to its element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Position {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
    FollowCursor,
}

impl Position {
    pub const ALL: [Position; 5] = [
        Position::Top,
        Position::Bottom,
        Position::Left,
        Position::Right,
        Position::FollowCursor,
    ];

    fn to_iced(self) -> widget::tooltip::Position {
        match self {
            Position::Top => widget::tooltip::Position::Top,
            Position::Bottom => widget::tooltip::Position::Bottom,
            Position::Left => widget::tooltip::Position::Left,
            Position::Right => widget::tooltip::Position::Right,
            Position::FollowCursor => widget::tooltip::Position::FollowCursor,
        }
    }
}

/// Gap between the element and the bubble, in logical pixels.
pub const GAP: f32 = 6.0;

/// A tooltip builder. Convert it into an [`Element`] to render.
pub struct Tooltip<'a, Message> {
    content: Element<'a, Message>,
    label: text::Fragment<'a>,
    position: Position,
    delay: Duration,
}

impl<Message> std::fmt::Debug for Tooltip<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tooltip")
            .field("label", &self.label)
            .field("position", &self.position)
            .field("delay", &self.delay)
            .finish_non_exhaustive()
    }
}

/// Shows `label` in a bubble while the pointer is over `content`.
pub fn tooltip<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    label: impl text::IntoFragment<'a>,
) -> Tooltip<'a, Message> {
    Tooltip {
        content: content.into(),
        label: label.into_fragment(),
        position: Position::default(),
        delay: Duration::ZERO,
    }
}

impl<'a, Message> Tooltip<'a, Message> {
    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// How long the pointer has to rest before the bubble appears.
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

impl<'a, Message: 'a> From<Tooltip<'a, Message>> for Element<'a, Message> {
    fn from(tooltip: Tooltip<'a, Message>) -> Self {
        let bubble = container(text(tooltip.label).size(text_size::XS))
            .padding([space::XS + 2.0, space::MD])
            .style(|theme| style(&Tokens::of(theme)));

        Element::new(AfterTouch {
            content: widget::tooltip(tooltip.content, bubble, tooltip.position.to_iced())
                .gap(GAP)
                .delay(tooltip.delay)
                .into(),
        })
    }
}

/// Hides the cursor from a tooltip once a finger lifts, until the mouse
/// moves. A touch leaves the cursor where the finger was, which would
/// otherwise open the bubble and keep it open after every tap.
struct AfterTouch<'a, Message> {
    content: Element<'a, Message>,
}

#[derive(Debug, Default)]
struct Touched(bool);

impl Touched {
    fn update(&mut self, event: &Event) {
        match event {
            Event::Touch(touch::Event::FingerLifted { .. } | touch::Event::FingerLost { .. }) => {
                self.0 = true;
            }
            Event::Touch(touch::Event::FingerPressed { .. })
            | Event::Mouse(mouse::Event::CursorMoved { .. }) => self.0 = false,
            _ => {}
        }
    }

    fn cursor(&self, cursor: mouse::Cursor) -> mouse::Cursor {
        if self.0 { cursor.levitate() } else { cursor }
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
        let cursor = tree.state.downcast_ref::<Touched>().cursor(cursor);
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
        // Touch events keep the real cursor so the content still reads the
        // finger; everything else sees it hidden after a touch.
        let cursor = match event {
            Event::Touch(_) => cursor,
            _ => touched.cursor(cursor),
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

/// The bubble style: inverted colours so it stands out from the page.
pub fn style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.primary)),
        text_color: Some(tokens.primary_foreground),
        border: Border {
            radius: radius::MD.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn defaults_to_top_without_delay() {
        let t: Tooltip<'_, ()> = tooltip(text("Hover"), "Hint");
        assert_eq!(t.position, Position::Top);
        assert_eq!(t.delay, Duration::ZERO);
    }

    #[test]
    fn every_position_maps_to_a_distinct_iced_position() {
        let mapped = Position::ALL.map(Position::to_iced);
        for (i, a) in mapped.iter().enumerate() {
            assert!(mapped[i + 1..].iter().all(|b| a != b));
        }
    }

    #[test]
    fn bubble_inverts_the_page_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = style(&tokens);
            assert_eq!(style.background, Some(Background::Color(tokens.primary)));
            assert_eq!(style.text_color, Some(tokens.primary_foreground));
            assert_ne!(tokens.primary, tokens.background);
        }
    }
}
