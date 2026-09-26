//! Thin lines that divide groups of content, optionally with inline text.

use iced::widget::{column, row, rule, text};
use iced::{Alignment, Element, Theme};

use crate::theme::{Tokens, space, text_size};

/// Direction of the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    pub const ALL: [Orientation; 2] = [Orientation::Horizontal, Orientation::Vertical];
}

/// A separator builder. Convert it into an [`Element`] to render.
///
/// Horizontal separators fill the available width, vertical ones the
/// available height.
#[derive(Debug)]
pub struct Separator<'a> {
    orientation: Orientation,
    label: Option<text::Fragment<'a>>,
}

/// Creates a horizontal separator.
pub fn separator<'a>() -> Separator<'a> {
    Separator {
        orientation: Orientation::Horizontal,
        label: None,
    }
}

/// Creates a vertical separator.
pub fn vertical_separator<'a>() -> Separator<'a> {
    separator().orientation(Orientation::Vertical)
}

impl<'a> Separator<'a> {
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Shows short text, such as "or", in the middle of the line.
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }
}

impl<'a, Message: 'a> From<Separator<'a>> for Element<'a, Message> {
    fn from(separator: Separator<'a>) -> Self {
        let line = move || -> rule::Rule<'a, Theme> {
            let line = match separator.orientation {
                Orientation::Horizontal => rule::horizontal(1),
                Orientation::Vertical => rule::vertical(1),
            };
            line.style(|theme| style(&Tokens::of(theme)))
        };

        let Some(label) = separator.label else {
            return line().into();
        };

        let label = text(label).size(text_size::XS).style(|theme| text::Style {
            color: Some(label_colour(&Tokens::of(theme))),
        });

        match separator.orientation {
            Orientation::Horizontal => row![line(), label, line()]
                .spacing(space::SM)
                .align_y(Alignment::Center)
                .into(),
            Orientation::Vertical => column![line(), label, line()]
                .spacing(space::SM)
                .align_x(Alignment::Center)
                .into(),
        }
    }
}

/// The iced rule style for a separator line.
pub fn style(tokens: &Tokens) -> rule::Style {
    rule::Style {
        color: tokens.border,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

/// Colour of the inline text.
pub fn label_colour(tokens: &Tokens) -> iced::Color {
    tokens.muted_foreground
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn default_builder_is_horizontal_without_label() {
        let s = separator();
        assert_eq!(s.orientation, Orientation::Horizontal);
        assert!(s.label.is_none());
    }

    #[test]
    fn vertical_constructor_sets_orientation() {
        assert_eq!(vertical_separator().orientation, Orientation::Vertical);
        for orientation in Orientation::ALL {
            assert_eq!(
                separator().orientation(orientation).orientation,
                orientation
            );
        }
    }

    #[test]
    fn label_is_kept() {
        let s = separator().label("or");
        assert_eq!(s.label.as_deref(), Some("or"));
    }

    #[test]
    fn line_uses_border_token_and_fills() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let style = style(&tokens);
            assert_eq!(style.color, tokens.border);
            assert_eq!(style.fill_mode, rule::FillMode::Full);
            assert_ne!(style.color, tokens.background);
        }
    }

    #[test]
    fn label_is_muted_and_readable() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(label_colour(&tokens), tokens.muted_foreground);
            assert_ne!(label_colour(&tokens), tokens.background);
        }
    }
}
