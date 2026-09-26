//! Lucide icons rendered through iced's `svg` widget.
//!
//! Build a [`Glyph`] with the [`lucide!`](crate::lucide) macro so it is
//! resolved at compile time and only the icons you use end up in the binary.

use iced::widget::{Svg, svg};
use iced::{Color, Length, Theme};

pub use lucide_static_svg::Icon;

/// Embedded SVG source for one icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Glyph {
    name: &'static str,
    svg: &'static str,
}

impl Glyph {
    /// Resolves an icon. Call this in a `const` so unused icons are not linked.
    pub const fn new(icon: Icon) -> Self {
        Self {
            name: icon.name(),
            svg: icon.svg_str(),
        }
    }

    /// Kebab-case Lucide name, such as `chevron-right`.
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Raw SVG markup.
    pub const fn svg(self) -> &'static str {
        self.svg
    }
}

/// Resolves a Lucide icon to a [`Glyph`] at compile time.
///
/// ```
/// let save = iced_cube::lucide!(Save);
/// assert_eq!(save.name(), "save");
/// ```
#[macro_export]
macro_rules! lucide {
    ($name:ident) => {{
        const GLYPH: $crate::icon::Glyph = $crate::icon::Glyph::new($crate::icon::Icon::$name);
        GLYPH
    }};
}

/// Opacity of an icon in a disabled component, matching its faded text.
pub const DISABLED_OPACITY: f32 = 0.5;

/// The opacity for an icon in an enabled or disabled component.
pub const fn opacity(enabled: bool) -> f32 {
    if enabled { 1.0 } else { DISABLED_OPACITY }
}

/// Renders a glyph at `size` pixels, tinted with the theme foreground.
pub fn icon<'a>(glyph: Glyph, size: f32) -> Svg<'a, Theme> {
    themed(glyph, size, 1.0, |theme| {
        theme.extended_palette().background.base.text
    })
}

/// Renders a glyph at `size` pixels in a fixed colour, or the theme
/// foreground when `color` is `None`. The colour's alpha becomes the icon's
/// opacity.
pub fn tinted<'a>(glyph: Glyph, size: f32, color: Option<Color>) -> Svg<'a, Theme> {
    match color {
        Some(color) => themed(glyph, size, color.a, move |_| color),
        None => icon(glyph, size),
    }
}

/// Renders a glyph at `size` pixels, tinted with `color` resolved from the
/// theme on every draw, and drawn at `opacity`.
///
/// iced ignores the alpha of an svg tint, so the tint is drawn opaque and
/// any fading must come through `opacity`. Pass [`opacity`]`(enabled)` for a
/// component that can be disabled rather than fading the colour.
pub fn themed<'a>(
    glyph: Glyph,
    size: f32,
    opacity: f32,
    color: impl Fn(&Theme) -> Color + 'a,
) -> Svg<'a, Theme> {
    svg(svg::Handle::from_memory(glyph.svg().as_bytes()))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .opacity(opacity.clamp(0.0, 1.0))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(opaque(color(theme))),
        })
}

/// `color` with full alpha, which is how iced draws an svg tint anyway.
pub const fn opaque(color: Color) -> Color {
    Color { a: 1.0, ..color }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_icons_are_drawn_at_half_opacity() {
        assert_eq!(opacity(true), 1.0);
        assert_eq!(opacity(false), DISABLED_OPACITY);
    }

    #[test]
    fn opaque_keeps_the_channels_and_drops_the_alpha() {
        let colour = opaque(Color::from_rgba(0.1, 0.2, 0.3, 0.4));
        assert_eq!(colour, Color::from_rgb(0.1, 0.2, 0.3));
    }

    #[test]
    fn macro_resolves_name_and_markup() {
        let glyph = crate::lucide!(ChevronRight);
        assert_eq!(glyph.name(), "chevron-right");
        assert!(glyph.svg().starts_with("<svg"));
    }

    #[test]
    fn lucide_svgs_use_current_colour_so_tinting_works() {
        assert!(crate::lucide!(Save).svg().contains("currentColor"));
    }
}
