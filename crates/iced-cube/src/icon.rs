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

/// Renders a glyph at `size` pixels, tinted with the theme foreground.
pub fn icon<'a>(glyph: Glyph, size: f32) -> Svg<'a, Theme> {
    tinted(glyph, size, None)
}

/// Renders a glyph at `size` pixels in a fixed colour, or the theme
/// foreground when `color` is `None`.
pub fn tinted<'a>(glyph: Glyph, size: f32, color: Option<Color>) -> Svg<'a, Theme> {
    svg(svg::Handle::from_memory(glyph.svg().as_bytes()))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(color.unwrap_or(theme.extended_palette().background.base.text)),
        })
}

#[cfg(test)]
mod tests {
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
