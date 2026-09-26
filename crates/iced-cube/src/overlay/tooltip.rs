//! A small label shown while the pointer rests on an element.

use iced::time::Duration;
use iced::widget::{self, container, text};
use iced::{Background, Border, Element};

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

        widget::tooltip(tooltip.content, bubble, tooltip.position.to_iced())
            .gap(GAP)
            .delay(tooltip.delay)
            .into()
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
