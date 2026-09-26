//! Scrollable regions with a thin, rounded scrollbar.

use iced::widget::scrollable::{self, AutoScroll, Rail, Scrollbar, Scroller, Status};
use iced::widget::{Id, container};
use iced::{Background, Border, Color, Element, Length, Shadow, Vector};

use crate::theme::{Tokens, fade, mix, radius, space};

/// Width of the scrollbar rail in logical pixels.
pub const THICKNESS: f32 = 6.0;

/// Which way the content scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    #[default]
    Vertical,
    Horizontal,
    Both,
}

impl Direction {
    pub const ALL: [Direction; 3] = [Direction::Vertical, Direction::Horizontal, Direction::Both];

    fn scrollable(self) -> scrollable::Direction {
        match self {
            Direction::Vertical => scrollable::Direction::Vertical(scrollbar()),
            Direction::Horizontal => scrollable::Direction::Horizontal(scrollbar()),
            Direction::Both => scrollable::Direction::Both {
                vertical: floating_scrollbar(),
                horizontal: floating_scrollbar(),
            },
        }
    }
}

/// The thin scrollbar used by single-direction scroll areas. It sits beside
/// the content rather than over it.
pub fn scrollbar() -> Scrollbar {
    floating_scrollbar().spacing(space::XS)
}

/// The thin scrollbar used when scrolling both ways. iced cannot reserve
/// space for two scrollbars, so these float over the content's edges.
pub fn floating_scrollbar() -> Scrollbar {
    Scrollbar::new()
        .width(THICKNESS)
        .scroller_width(THICKNESS)
        .margin(2)
}

/// A scroll area builder. Convert it into an [`Element`] to render.
pub struct ScrollArea<'a, Message> {
    content: Element<'a, Message>,
    direction: Direction,
    width: Length,
    height: Length,
    id: Option<Id>,
}

impl<Message> std::fmt::Debug for ScrollArea<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScrollArea")
            .field("direction", &self.direction)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// Creates a vertically scrolling area around `content`.
pub fn scroll_area<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> ScrollArea<'a, Message> {
    ScrollArea {
        content: content.into(),
        direction: Direction::default(),
        width: Length::Shrink,
        height: Length::Shrink,
        id: None,
    }
}

impl<'a, Message> ScrollArea<'a, Message> {
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the widget id, for scrolling programmatically.
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl<'a, Message: 'a> From<ScrollArea<'a, Message>> for Element<'a, Message> {
    fn from(area: ScrollArea<'a, Message>) -> Self {
        let mut scrollable =
            scrollable::Scrollable::with_direction(area.content, area.direction.scrollable())
                .width(area.width)
                .height(area.height)
                .style(|theme, status| style(&Tokens::of(theme), status));
        if let Some(id) = area.id {
            scrollable = scrollable.id(id);
        }
        scrollable.into()
    }
}

/// How the pointer is interacting with one scrollbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interaction {
    Idle,
    Hovered,
    Dragged,
}

/// Scroller colour for an interaction.
pub fn scroller_colour(tokens: &Tokens, interaction: Interaction) -> Color {
    match interaction {
        Interaction::Idle => tokens.border,
        Interaction::Hovered => mix(tokens.border, tokens.foreground, 0.25),
        Interaction::Dragged => mix(tokens.border, tokens.foreground, 0.45),
    }
}

/// Splits a scrollable status into vertical and horizontal interactions.
pub fn interactions(status: Status) -> (Interaction, Interaction) {
    let pick = |dragged: bool, hovered: bool| {
        if dragged {
            Interaction::Dragged
        } else if hovered {
            Interaction::Hovered
        } else {
            Interaction::Idle
        }
    };
    match status {
        Status::Active { .. } => (Interaction::Idle, Interaction::Idle),
        Status::Hovered {
            is_vertical_scrollbar_hovered,
            is_horizontal_scrollbar_hovered,
            ..
        } => (
            pick(false, is_vertical_scrollbar_hovered),
            pick(false, is_horizontal_scrollbar_hovered),
        ),
        Status::Dragged {
            is_vertical_scrollbar_dragged,
            is_horizontal_scrollbar_dragged,
            ..
        } => (
            pick(is_vertical_scrollbar_dragged, false),
            pick(is_horizontal_scrollbar_dragged, false),
        ),
    }
}

/// The iced scrollable style for a status.
pub fn style(tokens: &Tokens, status: Status) -> scrollable::Style {
    let (vertical, horizontal) = interactions(status);
    let rail = |interaction| Rail {
        background: None,
        border: Border::default(),
        scroller: Scroller {
            background: Background::Color(scroller_colour(tokens, interaction)),
            border: Border {
                radius: radius::FULL.into(),
                ..Border::default()
            },
        },
    };

    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail(vertical),
        horizontal_rail: rail(horizontal),
        gap: None,
        auto_scroll: AutoScroll {
            background: Background::Color(fade(tokens.background, 0.9)),
            border: Border {
                color: tokens.border,
                width: 1.0,
                radius: radius::FULL.into(),
            },
            shadow: Shadow {
                color: fade(tokens.foreground, 0.1),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 3.0,
            },
            icon: tokens.muted_foreground,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const ACTIVE: Status = Status::Active {
        is_horizontal_scrollbar_disabled: false,
        is_vertical_scrollbar_disabled: false,
    };

    fn hovered(vertical: bool, horizontal: bool) -> Status {
        Status::Hovered {
            is_horizontal_scrollbar_hovered: horizontal,
            is_vertical_scrollbar_hovered: vertical,
            is_horizontal_scrollbar_disabled: false,
            is_vertical_scrollbar_disabled: false,
        }
    }

    fn dragged(vertical: bool, horizontal: bool) -> Status {
        Status::Dragged {
            is_horizontal_scrollbar_dragged: horizontal,
            is_vertical_scrollbar_dragged: vertical,
            is_horizontal_scrollbar_disabled: false,
            is_vertical_scrollbar_disabled: false,
        }
    }

    fn scroller(rail: Rail) -> Background {
        rail.scroller.background
    }

    #[test]
    fn default_builder_is_vertical_and_shrinks() {
        let area: ScrollArea<'_, ()> = scroll_area("content");
        assert_eq!(area.direction, Direction::Vertical);
        assert_eq!(area.width, Length::Shrink);
        assert!(area.id.is_none());
    }

    #[test]
    fn directions_map_to_the_right_scrollbars() {
        let vertical = Direction::Vertical.scrollable();
        assert!(vertical.vertical().is_some() && vertical.horizontal().is_none());
        let horizontal = Direction::Horizontal.scrollable();
        assert!(horizontal.vertical().is_none() && horizontal.horizontal().is_some());
        let both = Direction::Both.scrollable();
        assert!(both.vertical().is_some() && both.horizontal().is_some());
    }

    #[test]
    fn scrollbars_are_thin() {
        let floating = Scrollbar::new().width(6).scroller_width(6).margin(2);
        assert_eq!(floating_scrollbar(), floating);
        assert_eq!(scrollbar(), floating.spacing(4));
    }

    #[test]
    fn only_both_directions_float() {
        assert_eq!(
            Direction::Vertical.scrollable().vertical(),
            Some(&scrollbar())
        );
        assert_eq!(
            Direction::Both.scrollable().horizontal(),
            Some(&floating_scrollbar())
        );
    }

    #[test]
    fn interactions_track_each_axis() {
        assert_eq!(interactions(ACTIVE), (Interaction::Idle, Interaction::Idle));
        assert_eq!(
            interactions(hovered(true, false)),
            (Interaction::Hovered, Interaction::Idle)
        );
        assert_eq!(
            interactions(dragged(false, true)),
            (Interaction::Idle, Interaction::Dragged)
        );
    }

    #[test]
    fn scroller_strengthens_with_interaction() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = style(&tokens, ACTIVE);
            let hover = style(&tokens, hovered(true, true));
            let drag = style(&tokens, dragged(true, true));
            assert_ne!(scroller(idle.vertical_rail), scroller(hover.vertical_rail));
            assert_ne!(scroller(hover.vertical_rail), scroller(drag.vertical_rail));
            assert_ne!(
                scroller(idle.horizontal_rail),
                scroller(drag.horizontal_rail)
            );
        }
    }

    #[test]
    fn rails_are_transparent_and_scrollers_rounded() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in [ACTIVE, hovered(true, false), dragged(false, true)] {
                let style = style(&tokens, status);
                for rail in [style.vertical_rail, style.horizontal_rail] {
                    assert!(rail.background.is_none());
                    assert_eq!(rail.scroller.border.radius, radius::FULL.into());
                }
            }
        }
    }

    #[test]
    fn idle_scroller_uses_border_token() {
        let tokens = Tokens::of(&dark());
        assert_eq!(scroller_colour(&tokens, Interaction::Idle), tokens.border);
    }
}
