//! Inline callouts with a title, description and icon.

use iced::widget::{column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Shadow};

use crate::icon::{Glyph, themed};
use crate::theme::{Tokens, mix, radius, space, text_size};

const ICON_SIZE: f32 = 16.0;

/// The kind of message an alert carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    #[default]
    Info,
    Success,
    Warning,
    Destructive,
}

impl Variant {
    pub const ALL: [Variant; 4] = [
        Variant::Info,
        Variant::Success,
        Variant::Warning,
        Variant::Destructive,
    ];

    /// The icon shown when none is set.
    pub const fn glyph(self) -> Glyph {
        match self {
            Variant::Info => crate::lucide!(Info),
            Variant::Success => crate::lucide!(CircleCheck),
            Variant::Warning => crate::lucide!(TriangleAlert),
            Variant::Destructive => crate::lucide!(CircleAlert),
        }
    }
}

/// An alert builder. Convert it into an [`Element`] to render.
#[derive(Debug)]
pub struct Alert<'a> {
    title: text::Fragment<'a>,
    description: Option<text::Fragment<'a>>,
    icon: Option<Glyph>,
    variant: Variant,
    width: Length,
}

/// Creates an alert with a title.
pub fn alert<'a>(title: impl text::IntoFragment<'a>) -> Alert<'a> {
    Alert {
        title: title.into_fragment(),
        description: None,
        icon: None,
        variant: Variant::default(),
        width: Length::Fill,
    }
}

impl<'a> Alert<'a> {
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Adds supporting text under the title.
    pub fn description(mut self, description: impl text::IntoFragment<'a>) -> Self {
        self.description = Some(description.into_fragment());
        self
    }

    /// Replaces the variant's default icon.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Overrides the width. Alerts fill the available width by default.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The icon that will be rendered.
    pub fn glyph(&self) -> Glyph {
        self.icon.unwrap_or(self.variant.glyph())
    }
}

impl<'a, Message: 'a> From<Alert<'a>> for Element<'a, Message> {
    fn from(alert: Alert<'a>) -> Self {
        let variant = alert.variant;
        let glyph = alert.glyph();

        let icon = themed(glyph, ICON_SIZE, 1.0, move |theme| {
            colours(&Tokens::of(theme), variant).accent
        });

        let title = text(alert.title)
            .size(text_size::SM)
            .style(move |theme| text::Style {
                color: Some(colours(&Tokens::of(theme), variant).title),
            });

        let mut body = column![title].spacing(space::XS).width(Length::Fill);
        if let Some(description) = alert.description {
            body =
                body.push(
                    text(description)
                        .size(text_size::SM)
                        .style(move |theme| text::Style {
                            color: Some(colours(&Tokens::of(theme), variant).description),
                        }),
                );
        }

        container(row![container(icon).padding([2, 0]), body].spacing(space::MD))
            .padding([space::MD, space::LG])
            .width(alert.width)
            .style(move |theme| style(&Tokens::of(theme), variant))
            .into()
    }
}

/// Colours of an alert's surface and text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub background: Color,
    pub border: Color,
    /// Icon colour.
    pub accent: Color,
    pub title: Color,
    pub description: Color,
}

/// Resolves the colours of a variant.
pub fn colours(tokens: &Tokens, variant: Variant) -> Colours {
    let accent = match variant {
        Variant::Info => tokens.foreground,
        Variant::Success => tokens.success,
        Variant::Warning => tokens.warning,
        Variant::Destructive => tokens.destructive,
    };
    let tint = if tokens.is_dark { 0.12 } else { 0.06 };
    let edge = if tokens.is_dark { 0.35 } else { 0.3 };
    let (background, border) = match variant {
        Variant::Info => (tokens.background, tokens.border),
        _ => (
            mix(tokens.background, accent, tint),
            mix(tokens.background, accent, edge),
        ),
    };

    Colours {
        background,
        border,
        accent,
        title: mix(accent, tokens.foreground, 0.25),
        description: match variant {
            Variant::Info => tokens.muted_foreground,
            _ => mix(accent, tokens.foreground, 0.55),
        },
    }
}

/// The container style for a variant.
pub fn style(tokens: &Tokens, variant: Variant) -> container::Style {
    let colours = colours(tokens, variant);
    container::Style {
        text_color: Some(colours.title),
        background: Some(Background::Color(colours.background)),
        border: Border {
            color: colours.border,
            width: 1.0,
            radius: radius::LG.into(),
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
    fn default_builder_is_info_fill_width_with_variant_icon() {
        let a = alert("Heads up");
        assert_eq!(a.variant, Variant::Info);
        assert_eq!(a.width, Length::Fill);
        assert!(a.description.is_none());
        assert_eq!(a.glyph().name(), "info");
    }

    #[test]
    fn each_variant_has_its_own_icon() {
        let mut names: Vec<_> = Variant::ALL.map(|v| v.glyph().name()).to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Variant::ALL.len());
    }

    #[test]
    fn custom_icon_overrides_variant_icon() {
        let a = alert("Saved")
            .variant(Variant::Success)
            .icon(crate::lucide!(Save));
        assert_eq!(a.glyph().name(), "save");
    }

    #[test]
    fn status_variants_tint_the_surface() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in [Variant::Success, Variant::Warning, Variant::Destructive] {
                let colours = colours(&tokens, variant);
                assert_ne!(colours.background, tokens.background, "{variant:?}");
                assert_ne!(colours.border, tokens.border, "{variant:?}");
            }
        }
    }

    #[test]
    fn info_uses_neutral_surface() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let colours = colours(&tokens, Variant::Info);
            assert_eq!(colours.background, tokens.background);
            assert_eq!(colours.accent, tokens.foreground);
        }
    }

    #[test]
    fn text_is_distinct_from_surface_in_every_variant() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                let colours = colours(&tokens, variant);
                assert_ne!(colours.title, colours.background, "{variant:?}");
                assert_ne!(colours.description, colours.background, "{variant:?}");
                assert_eq!(style(&tokens, variant).border.width, 1.0);
            }
        }
    }
}
