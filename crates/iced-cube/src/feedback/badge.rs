//! Short status or category markers.

use iced::widget::{container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Padding, Shadow};

use crate::icon::{Glyph, tinted};
use crate::theme::{Tokens, on, radius, text_size};

const ICON_SIZE: f32 = 12.0;

/// Visual emphasis of a badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    #[default]
    Default,
    Secondary,
    Outline,
    Destructive,
    Success,
    Warning,
}

impl Variant {
    pub const ALL: [Variant; 6] = [
        Variant::Default,
        Variant::Secondary,
        Variant::Outline,
        Variant::Destructive,
        Variant::Success,
        Variant::Warning,
    ];
}

/// A badge builder. Convert it into an [`Element`] to render.
#[derive(Debug)]
pub struct Badge<'a> {
    label: text::Fragment<'a>,
    icon: Option<Glyph>,
    variant: Variant,
}

/// Creates a badge with a text label.
pub fn badge<'a>(label: impl text::IntoFragment<'a>) -> Badge<'a> {
    Badge {
        label: label.into_fragment(),
        icon: None,
        variant: Variant::default(),
    }
}

impl Badge<'_> {
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Adds an icon before the label.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }
}

impl<'a, Message: 'a> From<Badge<'a>> for Element<'a, Message> {
    fn from(badge: Badge<'a>) -> Self {
        let variant = badge.variant;

        let mut content = row![].spacing(4).align_y(Alignment::Center);
        if let Some(glyph) = badge.icon {
            content = content.push(tinted(glyph, ICON_SIZE, None).style(move |theme, _| {
                iced::widget::svg::Style {
                    color: Some(colours(&Tokens::of(theme), variant).foreground),
                }
            }));
        }
        content = content.push(text(badge.label).size(text_size::XS));

        container(content)
            .padding(Padding::from([2.0, 10.0]))
            .style(move |theme| style(&Tokens::of(theme), variant))
            .into()
    }
}

/// Background, foreground and border colours for a variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub background: Option<Color>,
    pub foreground: Color,
    pub border: Option<Color>,
}

/// Resolves the colours of a variant.
pub fn colours(tokens: &Tokens, variant: Variant) -> Colours {
    let filled = |background: Color, foreground: Color| Colours {
        background: Some(background),
        foreground,
        border: None,
    };

    match variant {
        Variant::Default => filled(tokens.primary, tokens.primary_foreground),
        Variant::Secondary => filled(tokens.secondary, tokens.secondary_foreground),
        Variant::Outline => Colours {
            background: None,
            foreground: tokens.foreground,
            border: Some(tokens.border),
        },
        Variant::Destructive => filled(tokens.destructive, tokens.destructive_foreground),
        Variant::Success => filled(tokens.success, on(tokens.success)),
        Variant::Warning => filled(tokens.warning, on(tokens.warning)),
    }
}

/// The container style for a variant.
pub fn style(tokens: &Tokens, variant: Variant) -> container::Style {
    let colours = colours(tokens, variant);
    container::Style {
        text_color: Some(colours.foreground),
        background: colours.background.map(Background::Color),
        border: Border {
            color: colours.border.unwrap_or(Color::TRANSPARENT),
            width: if colours.border.is_some() { 1.0 } else { 0.0 },
            radius: radius::FULL.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn default_builder_is_default_variant_without_icon() {
        let b = badge("New");
        assert_eq!(b.variant, Variant::Default);
        assert!(b.icon.is_none());
    }

    #[test]
    fn icon_and_variant_are_kept() {
        let b = badge("Live")
            .icon(crate::lucide!(CircleDot))
            .variant(Variant::Success);
        assert_eq!(b.variant, Variant::Success);
        assert_eq!(b.icon.map(Glyph::name), Some("circle-dot"));
    }

    #[test]
    fn only_outline_has_a_border_and_no_fill() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                let style = style(&tokens, variant);
                let outline = variant == Variant::Outline;
                assert_eq!(style.border.width > 0.0, outline, "{variant:?}");
                assert_eq!(style.background.is_none(), outline, "{variant:?}");
            }
        }
    }

    #[test]
    fn foreground_differs_from_background_in_every_variant() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                let colours = colours(&tokens, variant);
                let behind = colours.background.unwrap_or(tokens.background);
                assert_ne!(colours.foreground, behind, "{variant:?}");
            }
        }
    }

    #[test]
    fn status_variants_use_status_tokens() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(
                colours(&tokens, Variant::Success).background,
                Some(tokens.success)
            );
            assert_eq!(
                colours(&tokens, Variant::Warning).background,
                Some(tokens.warning)
            );
            assert_eq!(
                colours(&tokens, Variant::Destructive).background,
                Some(tokens.destructive)
            );
        }
    }

    #[test]
    fn badges_are_pills() {
        let tokens = Tokens::of(&light());
        for variant in Variant::ALL {
            assert_eq!(style(&tokens, variant).border.radius, radius::FULL.into());
        }
    }
}
