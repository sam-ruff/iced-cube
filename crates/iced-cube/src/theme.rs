//! Design tokens and the built-in light and dark themes.
//!
//! Every component resolves its colours through [`Tokens::of`], so any
//! [`iced::Theme`] works. [`light`] and [`dark`] are tuned for the library,
//! and [`Config`] builds a theme with your own colours on top of either.

use std::borrow::Cow;
use std::fmt;
use std::sync::OnceLock;

use iced::font::Weight;
use iced::theme::Palette;
use iced::theme::palette::{Extended, Pair, Secondary, deviate};
use iced::widget::container;
use iced::{Background, Border, Color, Element, Font, Theme, color};

static BASE_FONT: OnceLock<Font> = OnceLock::new();

/// Registers the font components use for emphasised text, such as card and
/// toast titles. Pass the same font you give iced's `default_font`, before
/// the first view. Returns `false` if a font was already set.
///
/// Without it, emphasised text uses iced's generic sans-serif family, which
/// the system resolves and may differ from your default font.
pub fn set_font(font: Font) -> bool {
    BASE_FONT.set(font).is_ok()
}

/// The registered base font, or iced's default when none is set.
pub fn font() -> Font {
    BASE_FONT.get().copied().unwrap_or(Font::DEFAULT)
}

/// The base font at semibold weight.
pub fn semibold() -> Font {
    Font {
        weight: Weight::Semibold,
        ..font()
    }
}

/// A heading in the semibold base font, as cards, dialogs and dialog
/// sections title their content.
#[cfg(any(feature = "card", feature = "dialog"))]
pub(crate) fn heading<'a>(
    content: impl iced::widget::text::IntoFragment<'a>,
    size: f32,
) -> iced::widget::Text<'a> {
    iced::widget::text(content).size(size).font(semibold())
}

/// The small muted line under a heading.
#[cfg(any(feature = "card", feature = "dialog"))]
pub(crate) fn supporting<'a>(
    content: impl iced::widget::text::IntoFragment<'a>,
) -> iced::widget::Text<'a> {
    iced::widget::text(content)
        .size(text_size::SM)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        })
}

/// Corner radii in logical pixels.
pub mod radius {
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
    pub const FULL: f32 = 9999.0;
}

/// Spacing scale in logical pixels.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// Text sizes in logical pixels.
pub mod text_size {
    pub const XS: f32 = 12.0;
    pub const SM: f32 = 14.0;
    pub const MD: f32 = 16.0;
    pub const LG: f32 = 18.0;
}

/// How far the popover surface moves towards the foreground in dark
/// themes. Floating surfaces sit just above cards.
const POPOVER_LIFT: f32 = 0.06;

/// Semantic colours resolved from a theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    pub background: Color,
    pub foreground: Color,
    /// Floating surfaces: menus, popovers, lists and dialogs. The page
    /// background in light themes, lifted towards the foreground in dark
    /// ones so a menu over a card still reads as raised.
    pub popover: Color,
    pub muted: Color,
    pub muted_foreground: Color,
    pub accent: Color,
    pub accent_foreground: Color,
    pub border: Color,
    pub ring: Color,
    pub primary: Color,
    pub primary_foreground: Color,
    pub secondary: Color,
    pub secondary_foreground: Color,
    pub destructive: Color,
    pub destructive_foreground: Color,
    /// The destructive colour for text and icons on the page, such as an
    /// error message: lightened on dark backgrounds until it reads, while
    /// fills keep [`destructive`](Tokens::destructive).
    pub destructive_text: Color,
    pub success: Color,
    pub warning: Color,
    pub is_dark: bool,
}

impl Tokens {
    /// Resolves the tokens for any iced theme.
    pub fn of(theme: &Theme) -> Self {
        let ext = theme.extended_palette();
        let background = ext.background.base.color;
        let foreground = ext.background.base.text;

        let popover = if ext.is_dark {
            mix(background, foreground, POPOVER_LIFT)
        } else {
            background
        };

        Self {
            background,
            foreground,
            popover,
            muted: ext.background.weak.color,
            muted_foreground: mix(foreground, background, 0.4),
            accent: ext.secondary.weak.color,
            accent_foreground: on(ext.secondary.weak.color),
            border: ext.background.strong.color,
            ring: Color {
                a: 0.5,
                ..ext.primary.base.color
            },
            primary: ext.primary.base.color,
            primary_foreground: on(ext.primary.base.color),
            secondary: ext.secondary.base.color,
            secondary_foreground: on(ext.secondary.base.color),
            destructive: ext.danger.base.color,
            destructive_foreground: on(ext.danger.base.color),
            destructive_text: readable(ext.danger.base.color, background),
            success: ext.success.base.color,
            warning: ext.warning.base.color,
            is_dark: ext.is_dark,
        }
    }

    /// Background of a disabled text field, select or combobox.
    pub fn disabled_field(&self) -> Color {
        mix(self.background, self.muted, 0.5)
    }
}

/// The library's light theme.
pub fn light() -> Theme {
    Config::light().build()
}

/// The library's dark theme.
pub fn dark() -> Theme {
    Config::dark().build()
}

/// Builds a [`Theme`] from your own colours, starting from the light or
/// dark defaults. Text on coloured surfaces, such as a primary button's
/// label, is picked automatically with [`on`].
///
/// ```
/// use iced::color;
/// use iced_cube::theme::{Config, Tokens};
///
/// let theme = Config::light()
///     .name("Brand")
///     .primary(color!(0x4f46e5))
///     .accent(color!(0xeef2ff))
///     .build();
///
/// assert_eq!(Tokens::of(&theme).primary, color!(0x4f46e5));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    name: Cow<'static, str>,
    background: Color,
    foreground: Color,
    primary: Color,
    secondary: Option<Color>,
    accent: Option<Color>,
    destructive: Color,
    success: Color,
    warning: Color,
    border: Option<Color>,
}

impl Config {
    /// The colours of [`light`].
    pub fn light() -> Self {
        Self {
            name: Cow::Borrowed("Cube Light"),
            background: color!(0xffffff),
            foreground: color!(0x09090b),
            primary: color!(0x18181b),
            secondary: None,
            accent: None,
            destructive: color!(0xdc2626),
            success: color!(0x16a34a),
            warning: color!(0xd97706),
            border: None,
        }
    }

    /// The colours of [`dark`].
    pub fn dark() -> Self {
        Self {
            name: Cow::Borrowed("Cube Dark"),
            background: color!(0x09090b),
            foreground: color!(0xfafafa),
            primary: color!(0xfafafa),
            secondary: None,
            accent: None,
            destructive: color!(0xdc2626),
            success: color!(0x22c55e),
            warning: color!(0xf59e0b),
            border: None,
        }
    }

    /// The theme's name, as shown by iced's theme pickers.
    pub fn name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = name.into();
        self
    }

    /// Filled buttons, checked controls, sliders and focus rings.
    pub fn primary(mut self, colour: Color) -> Self {
        self.primary = colour;
        self
    }

    /// Secondary buttons and badges. Defaults to a shade of the background.
    pub fn secondary(mut self, colour: Color) -> Self {
        self.secondary = Some(colour);
        self
    }

    /// Hover and highlight surfaces, such as the selected option in a
    /// menu. Defaults to a shade of the background.
    pub fn accent(mut self, colour: Color) -> Self {
        self.accent = Some(colour);
        self
    }

    /// Destructive buttons, badges and errors.
    pub fn destructive(mut self, colour: Color) -> Self {
        self.destructive = colour;
        self
    }

    pub fn success(mut self, colour: Color) -> Self {
        self.success = colour;
        self
    }

    pub fn warning(mut self, colour: Color) -> Self {
        self.warning = colour;
        self
    }

    /// The window background. Whether the theme counts as dark follows
    /// from this colour.
    pub fn background(mut self, colour: Color) -> Self {
        self.background = colour;
        self
    }

    /// Body text.
    pub fn foreground(mut self, colour: Color) -> Self {
        self.foreground = colour;
        self
    }

    /// Borders of inputs, cards and separators. Defaults to a shade of the
    /// background.
    pub fn border(mut self, colour: Color) -> Self {
        self.border = Some(colour);
        self
    }

    /// The base iced palette.
    pub fn palette(&self) -> Palette {
        Palette {
            background: self.background,
            text: self.foreground,
            primary: self.primary,
            success: self.success,
            warning: self.warning,
            danger: self.destructive,
        }
    }

    /// The extended iced palette that [`Tokens::of`] reads. Secondary goes
    /// in `secondary.base`, accent in `secondary.weak` and border in
    /// `background.strong`.
    pub fn extended(&self) -> Extended {
        let mut extended = Extended::generate(self.palette());
        let shade = extended.background.weak.color;
        let secondary = self.secondary.unwrap_or(shade);
        let accent = self.accent.unwrap_or(shade);
        let strong = deviate(secondary, 0.1);

        extended.secondary = Secondary {
            base: Pair::new(secondary, on(secondary)),
            weak: Pair::new(accent, on(accent)),
            strong: Pair::new(strong, on(strong)),
        };
        if let Some(border) = self.border {
            extended.background.strong = Pair::new(border, self.foreground);
        }
        extended
    }

    /// Builds the iced theme.
    pub fn build(&self) -> Theme {
        let extended = self.extended();
        Theme::custom_with_fn(self.name.clone(), self.palette(), move |_| extended)
    }
}

/// Default diameter of a [`Swatch`] in logical pixels.
pub const SWATCH_SIZE: f32 = 12.0;

/// A small filled circle of colour, for colour pickers, legends and the
/// options of a select. Convert it into an [`Element`] to render.
pub struct Swatch<'a> {
    colour: Box<dyn Fn(&Theme) -> Color + 'a>,
    size: f32,
}

impl fmt::Debug for Swatch<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Swatch").field("size", &self.size).finish()
    }
}

/// Creates a swatch whose colour is resolved from the theme each time it
/// is drawn, so it follows theme changes.
///
/// ```
/// use iced_cube::theme::{self, Tokens};
///
/// let danger: iced::Element<'_, ()> =
///     theme::swatch(|theme| Tokens::of(theme).destructive).into();
/// ```
pub fn swatch<'a>(colour: impl Fn(&Theme) -> Color + 'a) -> Swatch<'a> {
    Swatch {
        colour: Box::new(colour),
        size: SWATCH_SIZE,
    }
}

/// Creates a swatch of one fixed colour, whatever the theme.
pub fn swatch_color<'a>(colour: Color) -> Swatch<'a> {
    swatch(move |_| colour)
}

impl Swatch<'_> {
    /// Sets the diameter in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// The diameter in logical pixels.
    pub fn diameter(&self) -> f32 {
        self.size
    }

    /// The colour the swatch draws in `theme`.
    pub fn colour(&self, theme: &Theme) -> Color {
        (self.colour)(theme)
    }
}

impl<'a, Message: 'a> From<Swatch<'a>> for Element<'a, Message> {
    fn from(swatch: Swatch<'a>) -> Self {
        let Swatch { colour, size } = swatch;
        container(iced::widget::space())
            .width(size)
            .height(size)
            .style(move |theme| container::Style {
                background: Some(Background::Color(colour(theme))),
                border: Border {
                    radius: radius::FULL.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into()
    }
}

/// Linearly blends `a` towards `b` by `t` (0.0 is `a`, 1.0 is `b`).
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// Near-black or near-white, whichever has the higher contrast ratio on
/// `background`.
pub fn on(background: Color) -> Color {
    let (dark, light) = (color!(0x09090b), color!(0xfafafa));
    if contrast(dark, background) >= contrast(light, background) {
        dark
    } else {
        light
    }
}

/// The relative luminance of `colour`, from 0 for black to 1 for white,
/// as the WCAG contrast formula uses it.
pub fn luminance(colour: Color) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.040_45 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(colour.r) + 0.7152 * linear(colour.g) + 0.0722 * linear(colour.b)
}

/// The WCAG contrast ratio between two colours, from 1 to 21.
pub fn contrast(a: Color, b: Color) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The contrast ratio body text needs against its background.
pub const TEXT_CONTRAST: f32 = 4.5;

/// `colour`, moved just far enough towards near-white or near-black,
/// whichever stands out from `background`, for text in it to reach
/// [`TEXT_CONTRAST`]. A colour that already reads is left as it is.
pub fn readable(colour: Color, background: Color) -> Color {
    let towards = on(background);
    (0..=20)
        .map(|step| mix(colour, towards, step as f32 / 20.0))
        .find(|candidate| contrast(*candidate, background) >= TEXT_CONTRAST)
        .unwrap_or(towards)
}

/// Returns `color` with its alpha multiplied by `alpha`.
pub fn fade(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha.clamp(0.0, 1.0),
        ..color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semibold_keeps_the_base_family() {
        let font = semibold();
        assert_eq!(font.weight, Weight::Semibold);
        assert_eq!(font.family, super::font().family);
    }

    #[test]
    fn contrast_follows_the_wcag_formula() {
        assert!((contrast(Color::BLACK, Color::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast(Color::WHITE, Color::WHITE) - 1.0).abs() < 1e-6);
        assert_eq!(
            contrast(Color::BLACK, Color::WHITE),
            contrast(Color::WHITE, Color::BLACK)
        );
    }

    #[test]
    fn text_on_a_colour_takes_whichever_reads_better() {
        // Mid green reads better with dark text than with white.
        let green = color!(0x16a34a);
        assert_eq!(on(green), color!(0x09090b));
        assert!(contrast(on(green), green) >= TEXT_CONTRAST);
        assert_eq!(on(Color::BLACK), color!(0xfafafa));
        assert_eq!(on(Color::WHITE), color!(0x09090b));
    }

    #[test]
    fn destructive_text_reads_on_the_page_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert!(contrast(tokens.destructive_text, tokens.background) >= TEXT_CONTRAST);
            // Fills keep the red they were given.
            assert_eq!(
                tokens.destructive,
                theme.extended_palette().danger.base.color
            );
        }
        let tokens = Tokens::of(&dark());
        assert_ne!(
            tokens.destructive_text, tokens.destructive,
            "lightened on dark"
        );
    }

    #[test]
    fn a_colour_that_already_reads_is_left_alone() {
        let ink = color!(0x1d4ed8);
        assert_eq!(readable(ink, Color::WHITE), ink);
    }

    #[test]
    fn mix_endpoints_return_inputs() {
        let a = Color::from_rgb(0.0, 0.2, 0.4);
        let b = Color::from_rgb(1.0, 0.8, 0.6);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
    }

    #[test]
    fn mix_clamps_out_of_range_factor() {
        let a = Color::BLACK;
        let b = Color::WHITE;
        assert_eq!(mix(a, b, -1.0), a);
        assert_eq!(mix(a, b, 2.0), b);
    }

    #[test]
    fn fade_scales_alpha_only() {
        let faded = fade(Color::from_rgba(0.1, 0.2, 0.3, 0.8), 0.5);
        assert_eq!((faded.r, faded.g, faded.b), (0.1, 0.2, 0.3));
        assert!((faded.a - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn on_picks_dark_text_for_light_backgrounds() {
        assert_eq!(on(Color::WHITE), color!(0x09090b));
        assert_eq!(on(Color::BLACK), color!(0xfafafa));
        assert_eq!(on(color!(0xdc2626)), color!(0xfafafa));
    }

    #[test]
    fn popover_is_the_background_in_light_and_lifted_in_dark() {
        let light = Tokens::of(&light());
        assert_eq!(light.popover, light.background);

        let dark = Tokens::of(&dark());
        let card = mix(dark.background, dark.foreground, 0.04);
        let brightness = |colour: Color| colour.r + colour.g + colour.b;
        assert!(brightness(dark.popover) > brightness(card));
        assert!(brightness(dark.popover) < brightness(dark.foreground));
    }

    #[test]
    fn light_and_dark_report_their_mode() {
        assert!(!Tokens::of(&light()).is_dark);
        assert!(Tokens::of(&dark()).is_dark);
    }

    #[test]
    fn primary_foreground_contrasts_with_primary() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_ne!(tokens.primary, tokens.primary_foreground);
            assert_ne!(tokens.background, tokens.foreground);
        }
    }

    #[test]
    fn defaults_match_the_original_palettes() {
        let light = Tokens::of(&light());
        assert_eq!(light.background, color!(0xffffff));
        assert_eq!(light.foreground, color!(0x09090b));
        assert_eq!(light.primary, color!(0x18181b));
        assert_eq!(light.destructive, color!(0xdc2626));
        assert_eq!(light.secondary, light.muted);
        assert_eq!(light.accent, light.muted);
        assert_eq!(light.secondary_foreground, light.foreground);
        assert_eq!(light.accent_foreground, light.foreground);

        let dark = Tokens::of(&dark());
        assert_eq!(dark.background, color!(0x09090b));
        assert_eq!(dark.primary, color!(0xfafafa));
        assert_eq!(dark.success, color!(0x22c55e));
        assert_eq!(dark.secondary, dark.muted);
        assert_eq!(dark.secondary_foreground, dark.foreground);
        assert_eq!(dark.accent_foreground, dark.foreground);
    }

    #[test]
    fn defaults_keep_the_generated_border() {
        for config in [Config::light(), Config::dark()] {
            let generated = Extended::generate(config.palette());
            let tokens = Tokens::of(&config.build());
            assert_eq!(tokens.border, generated.background.strong.color);
            assert_eq!(tokens.muted, generated.background.weak.color);
        }
    }

    #[test]
    fn config_colours_reach_tokens() {
        let theme = Config::light()
            .name("Brand")
            .primary(color!(0x4f46e5))
            .secondary(color!(0xe0e7ff))
            .accent(color!(0xfef3c7))
            .destructive(color!(0xbe123c))
            .success(color!(0x059669))
            .warning(color!(0xca8a04))
            .background(color!(0xfafaf9))
            .foreground(color!(0x1c1917))
            .border(color!(0xd6d3d1))
            .build();
        let tokens = Tokens::of(&theme);

        assert_eq!(theme.to_string(), "Brand");
        assert_eq!(tokens.primary, color!(0x4f46e5));
        assert_eq!(tokens.primary_foreground, on(color!(0x4f46e5)));
        assert_eq!(tokens.secondary, color!(0xe0e7ff));
        assert_eq!(tokens.secondary_foreground, on(color!(0xe0e7ff)));
        assert_eq!(tokens.accent, color!(0xfef3c7));
        assert_eq!(tokens.destructive, color!(0xbe123c));
        assert_eq!(tokens.success, color!(0x059669));
        assert_eq!(tokens.warning, color!(0xca8a04));
        assert_eq!(tokens.background, color!(0xfafaf9));
        assert_eq!(tokens.foreground, color!(0x1c1917));
        assert_eq!(tokens.border, color!(0xd6d3d1));
    }

    #[test]
    fn unset_surfaces_follow_a_new_background() {
        let background = color!(0xf5f5f4);
        let tokens = Tokens::of(&Config::light().background(background).build());
        let shade = Extended::generate(Palette {
            background,
            ..Config::light().palette()
        })
        .background
        .weak
        .color;
        assert_eq!(tokens.secondary, shade);
        assert_eq!(tokens.accent, shade);
    }

    #[test]
    fn dark_mode_follows_the_background() {
        assert!(!Tokens::of(&Config::light().build()).is_dark);
        assert!(Tokens::of(&Config::dark().build()).is_dark);
        let night = Config::light()
            .background(color!(0x0f172a))
            .foreground(color!(0xf8fafc))
            .build();
        assert!(Tokens::of(&night).is_dark);
        let paper = Config::dark()
            .background(color!(0xfffbeb))
            .foreground(color!(0x292524))
            .build();
        assert!(!Tokens::of(&paper).is_dark);
    }

    #[test]
    fn tokens_resolve_for_builtin_iced_themes() {
        for theme in Theme::ALL {
            let tokens = Tokens::of(theme);
            assert_ne!(tokens.background, tokens.foreground, "{theme}");
        }
    }

    #[test]
    fn a_swatch_is_twelve_pixels_unless_sized() {
        let swatch = swatch(|theme| Tokens::of(theme).primary);
        assert_eq!(swatch.diameter(), SWATCH_SIZE);
        assert_eq!(swatch.size(20.0).diameter(), 20.0);
    }

    #[test]
    fn a_swatch_resolves_its_colour_from_each_theme() {
        let swatch = swatch(|theme| Tokens::of(theme).primary);
        assert_eq!(swatch.colour(&light()), Tokens::of(&light()).primary);
        assert_eq!(swatch.colour(&dark()), Tokens::of(&dark()).primary);
        assert_ne!(swatch.colour(&light()), swatch.colour(&dark()));
    }

    #[test]
    fn a_fixed_swatch_ignores_the_theme() {
        let teal = color!(0x14b8a6);
        let swatch = swatch_color(teal);
        assert_eq!(swatch.colour(&light()), teal);
        assert_eq!(swatch.colour(&dark()), teal);
    }
}
