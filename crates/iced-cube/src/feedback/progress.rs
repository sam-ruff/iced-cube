//! Determinate progress bars.

use iced::widget::{column, progress_bar, row, space, text};
use iced::{Background, Border, Color, Element, Length};

use crate::theme::{Tokens, mix, radius, space as gap, text_size};

/// Colour of the filled part of the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    #[default]
    Default,
    Success,
    Warning,
    Destructive,
}

impl Variant {
    pub const ALL: [Variant; 4] = [
        Variant::Default,
        Variant::Success,
        Variant::Warning,
        Variant::Destructive,
    ];
}

/// Bar thickness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}

impl Size {
    pub const ALL: [Size; 3] = [Size::Sm, Size::Md, Size::Lg];

    /// Thickness of the bar in logical pixels.
    pub const fn girth(self) -> f32 {
        match self {
            Size::Sm => 4.0,
            Size::Md => 8.0,
            Size::Lg => 12.0,
        }
    }
}

/// A progress bar builder. Convert it into an [`Element`] to render.
#[derive(Debug)]
pub struct Progress<'a> {
    value: f32,
    label: Option<text::Fragment<'a>>,
    show_percentage: bool,
    variant: Variant,
    size: Size,
    width: Length,
}

/// Creates a progress bar. `value` runs from 0.0 to 1.0 and is clamped.
pub fn progress<'a>(value: f32) -> Progress<'a> {
    Progress {
        value: clamp(value),
        label: None,
        show_percentage: false,
        variant: Variant::default(),
        size: Size::default(),
        width: Length::Fill,
    }
}

impl<'a> Progress<'a> {
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Shows a caption above the bar.
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Shows the rounded percentage above the bar, on the right.
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Overrides the width. Bars fill the available width by default.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The clamped value, from 0.0 to 1.0.
    pub fn value(&self) -> f32 {
        self.value
    }
}

impl<'a, Message: 'a> From<Progress<'a>> for Element<'a, Message> {
    fn from(progress: Progress<'a>) -> Self {
        let variant = progress.variant;
        let bar = progress_bar(0.0..=1.0, progress.value)
            .length(progress.width)
            .girth(progress.size.girth())
            .style(move |theme| style(&Tokens::of(theme), variant));

        if progress.label.is_none() && !progress.show_percentage {
            return bar.into();
        }

        let muted = |theme: &iced::Theme| text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        };
        let mut header = row![].width(progress.width);
        if let Some(label) = progress.label {
            header = header.push(text(label).size(text_size::SM));
        }
        header = header.push(space::horizontal());
        if progress.show_percentage {
            header = header.push(
                text(format!("{}%", percentage(progress.value)))
                    .size(text_size::SM)
                    .style(muted),
            );
        }

        column![header, bar].spacing(gap::SM).into()
    }
}

/// Clamps a value into 0.0..=1.0. NaN becomes 0.0.
pub fn clamp(value: f32) -> f32 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

/// A value from 0.0 to 1.0 as a whole percentage.
pub fn percentage(value: f32) -> u8 {
    // The clamped value is within 0.0..=100.0, so the cast cannot truncate.
    (clamp(value) * 100.0).round() as u8
}

/// Colours of the track and the filled bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub track: Color,
    pub bar: Color,
}

/// Resolves the colours of a variant.
pub fn colours(tokens: &Tokens, variant: Variant) -> Colours {
    let bar = match variant {
        Variant::Default => tokens.primary,
        Variant::Success => tokens.success,
        Variant::Warning => tokens.warning,
        Variant::Destructive => tokens.destructive,
    };
    let track_mix = if tokens.is_dark { 0.2 } else { 0.15 };
    Colours {
        track: mix(tokens.background, bar, track_mix),
        bar,
    }
}

/// The iced progress bar style for a variant.
pub fn style(tokens: &Tokens, variant: Variant) -> progress_bar::Style {
    let colours = colours(tokens, variant);
    progress_bar::Style {
        background: Background::Color(colours.track),
        bar: Background::Color(colours.bar),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius::FULL.into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn default_builder_is_medium_default_fill_without_caption() {
        let p = progress(0.5);
        assert_eq!(p.variant, Variant::Default);
        assert_eq!(p.size, Size::Md);
        assert_eq!(p.width, Length::Fill);
        assert!(p.label.is_none());
        assert!(!p.show_percentage);
    }

    #[test]
    fn clamp_bounds_values() {
        assert_eq!(clamp(-0.5), 0.0);
        assert_eq!(clamp(0.0), 0.0);
        assert_eq!(clamp(0.42), 0.42);
        assert_eq!(clamp(1.0), 1.0);
        assert_eq!(clamp(3.0), 1.0);
        assert_eq!(clamp(f32::INFINITY), 1.0);
        assert_eq!(clamp(f32::NEG_INFINITY), 0.0);
    }

    #[test]
    fn clamp_maps_nan_to_zero() {
        assert_eq!(clamp(f32::NAN), 0.0);
        assert_eq!(progress(f32::NAN).value(), 0.0);
    }

    #[test]
    fn builder_clamps_its_value() {
        assert_eq!(progress(-1.0).value(), 0.0);
        assert_eq!(progress(2.0).value(), 1.0);
    }

    #[test]
    fn percentage_rounds_and_clamps() {
        assert_eq!(percentage(0.0), 0);
        assert_eq!(percentage(0.424), 42);
        assert_eq!(percentage(0.425), 43);
        assert_eq!(percentage(1.0), 100);
        assert_eq!(percentage(7.0), 100);
        assert_eq!(percentage(-7.0), 0);
        assert_eq!(percentage(f32::NAN), 0);
    }

    #[test]
    fn sizes_grow_monotonically() {
        let girths = Size::ALL.map(Size::girth);
        assert!(girths.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn track_is_a_faint_version_of_the_bar() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                let colours = colours(&tokens, variant);
                assert_ne!(colours.track, colours.bar, "{variant:?}");
                assert_ne!(colours.track, tokens.background, "{variant:?}");
            }
        }
    }

    #[test]
    fn variants_use_their_tokens() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(colours(&tokens, Variant::Default).bar, tokens.primary);
            assert_eq!(colours(&tokens, Variant::Success).bar, tokens.success);
            assert_eq!(colours(&tokens, Variant::Warning).bar, tokens.warning);
            assert_eq!(
                colours(&tokens, Variant::Destructive).bar,
                tokens.destructive
            );
            assert_eq!(
                style(&tokens, Variant::Default).border.radius,
                radius::FULL.into()
            );
        }
    }
}
