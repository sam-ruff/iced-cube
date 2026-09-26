//! Vertical and horizontal stacks with token spacing.
//!
//! These are thin wrappers over iced's `column` and `row` that only take
//! spacing from the theme's scale.

use iced::widget::{Column, Row};
use iced::{Alignment, Element, Length, Padding};

use crate::theme::space;

/// Space between children, taken from the spacing scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Gap {
    None,
    Xs,
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
}

impl Gap {
    pub const ALL: [Gap; 6] = [Gap::None, Gap::Xs, Gap::Sm, Gap::Md, Gap::Lg, Gap::Xl];

    /// The gap in logical pixels.
    pub const fn pixels(self) -> f32 {
        match self {
            Gap::None => 0.0,
            Gap::Xs => space::XS,
            Gap::Sm => space::SM,
            Gap::Md => space::MD,
            Gap::Lg => space::LG,
            Gap::Xl => space::XL,
        }
    }
}

/// Which way a stack lays out its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    Vertical,
    Horizontal,
}

/// A stack builder. Convert it into an [`Element`] to render.
pub struct Stack<'a, Message> {
    axis: Axis,
    children: Vec<Element<'a, Message>>,
    gap: Gap,
    align: Alignment,
    padding: Padding,
    /// `None` follows the children, as an iced column does: a stack with a
    /// `Fill` child fills.
    width: Option<Length>,
    height: Option<Length>,
    wrap: bool,
}

impl<Message> std::fmt::Debug for Stack<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stack")
            .field("axis", &self.axis)
            .field("children", &self.children.len())
            .field("gap", &self.gap)
            .field("align", &self.align)
            .field("wrap", &self.wrap)
            .finish_non_exhaustive()
    }
}

/// Stacks children top to bottom.
pub fn vstack<'a, Message>(
    children: impl IntoIterator<Item = Element<'a, Message>>,
) -> Stack<'a, Message> {
    Stack::new(Axis::Vertical, children)
}

/// Stacks children left to right.
pub fn hstack<'a, Message>(
    children: impl IntoIterator<Item = Element<'a, Message>>,
) -> Stack<'a, Message> {
    Stack::new(Axis::Horizontal, children)
}

impl<'a, Message> Stack<'a, Message> {
    fn new(axis: Axis, children: impl IntoIterator<Item = Element<'a, Message>>) -> Self {
        Self {
            axis,
            children: children.into_iter().collect(),
            gap: Gap::default(),
            align: Alignment::Start,
            padding: Padding::ZERO,
            width: None,
            height: None,
            wrap: false,
        }
    }

    /// Adds a child at the end.
    pub fn push(mut self, child: impl Into<Element<'a, Message>>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn gap(mut self, gap: Gap) -> Self {
        self.gap = gap;
        self
    }

    /// Aligns children across the stack: horizontally in a `vstack`,
    /// vertically in an `hstack`.
    pub fn align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Overrides the width. Without it the stack shrinks to its children,
    /// or fills when a child fills.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Overrides the height, which otherwise follows the children as the
    /// width does.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Moves children onto a new line when they run out of room.
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }

    pub fn axis(&self) -> Axis {
        self.axis
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl<'a, Message: 'a> From<Stack<'a, Message>> for Element<'a, Message> {
    fn from(stack: Stack<'a, Message>) -> Self {
        let gap = stack.gap.pixels();
        match stack.axis {
            Axis::Vertical => {
                let mut column = Column::with_children(stack.children)
                    .spacing(gap)
                    .align_x(stack.align)
                    .padding(stack.padding);
                if let Some(width) = stack.width {
                    column = column.width(width);
                }
                if let Some(height) = stack.height {
                    column = column.height(height);
                }
                if stack.wrap {
                    column.wrap().into()
                } else {
                    column.into()
                }
            }
            Axis::Horizontal => {
                let mut row = Row::with_children(stack.children)
                    .spacing(gap)
                    .align_y(stack.align)
                    .padding(stack.padding);
                if let Some(width) = stack.width {
                    row = row.width(width);
                }
                if let Some(height) = stack.height {
                    row = row.height(height);
                }
                if stack.wrap {
                    row.wrap().into()
                } else {
                    row.into()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_builder_uses_medium_gap_and_start_alignment() {
        let s: Stack<'_, ()> = vstack([]);
        assert_eq!(s.axis(), Axis::Vertical);
        assert_eq!(s.gap, Gap::Md);
        assert_eq!(s.align, Alignment::Start);
        assert_eq!(s.width, None);
        assert_eq!(s.height, None);
        assert!(!s.wrap);
        assert!(s.is_empty());
    }

    #[test]
    fn hstack_is_horizontal() {
        let s: Stack<'_, ()> = hstack([]);
        assert_eq!(s.axis(), Axis::Horizontal);
    }

    #[test]
    fn push_appends_children() {
        let s: Stack<'_, ()> = vstack(["a".into()]).push("b").push("c");
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn gaps_follow_the_spacing_scale() {
        assert_eq!(Gap::None.pixels(), 0.0);
        assert_eq!(Gap::Xs.pixels(), space::XS);
        assert_eq!(Gap::Xl.pixels(), space::XL);
        let pixels = Gap::ALL.map(Gap::pixels);
        assert!(pixels.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn options_are_kept() {
        let s: Stack<'_, ()> = hstack([])
            .gap(Gap::Xl)
            .align(Alignment::Center)
            .width(Length::Fill)
            .wrap();
        assert_eq!(s.gap, Gap::Xl);
        assert_eq!(s.align, Alignment::Center);
        assert_eq!(s.width, Some(Length::Fill));
        assert!(s.wrap);
    }

    #[test]
    fn an_unset_size_follows_the_children() {
        let filling: Element<'_, ()> =
            vstack([iced::widget::space().width(Length::Fill).into()]).into();
        assert_eq!(filling.as_widget().size().width, Length::Fill);

        let shrinking: Element<'_, ()> = vstack(["a".into()]).into();
        assert_eq!(shrinking.as_widget().size().width, Length::Shrink);

        let fixed: Element<'_, ()> = hstack([iced::widget::space().width(Length::Fill).into()])
            .width(120)
            .into();
        assert_eq!(fixed.as_widget().size().width, Length::Fixed(120.0));
    }
}
