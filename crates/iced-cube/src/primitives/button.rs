//! Buttons with consistent variants, sizes and states.

use iced::widget::{self, button::Status, row, text};
use iced::{Alignment, Background, Border, Element, Length, Padding, Shadow, Theme};

use crate::icon::{Glyph, icon, tinted};
use crate::theme::{Tokens, fade, mix, radius, text_size};

/// Visual emphasis of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    #[default]
    Primary,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

impl Variant {
    pub const ALL: [Variant; 6] = [
        Variant::Primary,
        Variant::Secondary,
        Variant::Destructive,
        Variant::Outline,
        Variant::Ghost,
        Variant::Link,
    ];
}

/// Button dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
    /// Square, for icon-only buttons.
    Icon,
}

impl Size {
    pub const ALL: [Size; 4] = [Size::Sm, Size::Md, Size::Lg, Size::Icon];

    /// Height, horizontal padding, text size and icon size.
    pub const fn metrics(self) -> Metrics {
        match self {
            Size::Sm => Metrics {
                height: 32.0,
                padding_x: 12.0,
                text: text_size::SM,
                icon: 14.0,
            },
            Size::Md => Metrics {
                height: 36.0,
                padding_x: 16.0,
                text: text_size::SM,
                icon: 16.0,
            },
            Size::Lg => Metrics {
                height: 40.0,
                padding_x: 24.0,
                text: text_size::MD,
                icon: 18.0,
            },
            Size::Icon => Metrics {
                height: 36.0,
                padding_x: 0.0,
                text: text_size::SM,
                icon: 16.0,
            },
        }
    }
}

/// Resolved dimensions for a [`Size`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub height: f32,
    pub padding_x: f32,
    pub text: f32,
    pub icon: f32,
}

/// A button builder. Convert it into an [`Element`] to render.
///
/// A button without a message is rendered disabled.
#[derive(Debug)]
pub struct Button<'a, Message> {
    label: Option<text::Fragment<'a>>,
    leading: Option<Glyph>,
    trailing: Option<Glyph>,
    variant: Variant,
    size: Size,
    width: Length,
    on_press: Option<Message>,
}

/// Creates a button with a text label.
pub fn button<'a, Message>(label: impl text::IntoFragment<'a>) -> Button<'a, Message> {
    Button {
        label: Some(label.into_fragment()),
        leading: None,
        trailing: None,
        variant: Variant::default(),
        size: Size::default(),
        width: Length::Shrink,
        on_press: None,
    }
}

/// Creates a square icon-only button.
pub fn icon_button<'a, Message>(glyph: Glyph) -> Button<'a, Message> {
    Button {
        label: None,
        leading: Some(glyph),
        trailing: None,
        variant: Variant::Ghost,
        size: Size::Icon,
        width: Length::Shrink,
        on_press: None,
    }
}

impl<'a, Message> Button<'a, Message> {
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Adds an icon before the label.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.leading = Some(glyph);
        self
    }

    /// Adds an icon after the label.
    pub fn trailing_icon(mut self, glyph: Glyph) -> Self {
        self.trailing = Some(glyph);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_press.is_some()
    }
}

impl<'a, Message: Clone + 'a> From<Button<'a, Message>> for Element<'a, Message> {
    fn from(button: Button<'a, Message>) -> Self {
        let metrics = button.size.metrics();
        let variant = button.variant;
        let enabled = button.is_enabled();

        let glyph = move |glyph: Glyph| -> Element<'a, Message> {
            if variant == Variant::Link {
                return icon(glyph, metrics.icon).into();
            }
            let theme_icon = tinted(glyph, metrics.icon, None).style(move |theme: &Theme, _| {
                let colours = colours(&Tokens::of(theme), variant, status_for(enabled));
                widget::svg::Style {
                    color: Some(colours.foreground),
                }
            });
            theme_icon.into()
        };

        let mut content = row![].spacing(8).align_y(Alignment::Center);
        if let Some(leading) = button.leading {
            content = content.push(glyph(leading));
        }
        if let Some(label) = button.label {
            content = content.push(text(label).size(metrics.text));
        }
        if let Some(trailing) = button.trailing {
            content = content.push(glyph(trailing));
        }

        let (width, padding) = match button.size {
            Size::Icon => (
                Length::Fixed(metrics.height),
                Padding::from((metrics.height - metrics.icon) / 2.0),
            ),
            _ => (button.width, Padding::from([0.0, metrics.padding_x])),
        };

        widget::button(
            widget::container(content)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center)
                .width(Length::Fill),
        )
        .width(width)
        .height(Length::Fixed(metrics.height))
        .padding(padding)
        .on_press_maybe(button.on_press)
        .style(move |theme, status| style(&Tokens::of(theme), variant, status))
        .into()
    }
}

fn status_for(enabled: bool) -> Status {
    if enabled {
        Status::Active
    } else {
        Status::Disabled
    }
}

/// Background, foreground and border colours for a variant in a state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub background: Option<iced::Color>,
    pub foreground: iced::Color,
    pub border: Option<iced::Color>,
}

/// Resolves the colours of a variant for a given interaction state.
pub fn colours(tokens: &Tokens, variant: Variant, status: Status) -> Colours {
    let hover = matches!(status, Status::Hovered | Status::Pressed);
    let pressed = status == Status::Pressed;
    let emphasise = |base: iced::Color| {
        let amount = if pressed { 0.2 } else { 0.1 };
        let target = if tokens.is_dark {
            iced::Color::BLACK
        } else {
            iced::Color::WHITE
        };
        if hover {
            mix(base, target, amount)
        } else {
            base
        }
    };

    let colours = match variant {
        Variant::Primary => Colours {
            background: Some(emphasise(tokens.primary)),
            foreground: tokens.primary_foreground,
            border: None,
        },
        Variant::Secondary => Colours {
            background: Some(if hover {
                mix(tokens.secondary, tokens.border, 0.5)
            } else {
                tokens.secondary
            }),
            foreground: tokens.secondary_foreground,
            border: None,
        },
        Variant::Destructive => Colours {
            background: Some(emphasise(tokens.destructive)),
            foreground: tokens.destructive_foreground,
            border: None,
        },
        Variant::Outline => Colours {
            background: Some(if hover {
                tokens.accent
            } else {
                tokens.background
            }),
            foreground: tokens.foreground,
            border: Some(tokens.border),
        },
        Variant::Ghost => Colours {
            background: hover.then_some(tokens.accent),
            foreground: tokens.foreground,
            border: None,
        },
        Variant::Link => Colours {
            background: None,
            foreground: if hover {
                mix(tokens.primary, tokens.foreground, 0.3)
            } else {
                tokens.primary
            },
            border: None,
        },
    };

    if status != Status::Disabled {
        return colours;
    }
    Colours {
        background: colours.background.map(|c| fade(c, 0.5)),
        foreground: fade(colours.foreground, 0.5),
        border: colours.border.map(|c| fade(c, 0.5)),
    }
}

/// The iced button style for a variant in a state.
pub fn style(tokens: &Tokens, variant: Variant, status: Status) -> widget::button::Style {
    let colours = colours(tokens, variant, status);
    widget::button::Style {
        background: colours.background.map(Background::Color),
        text_color: colours.foreground,
        border: Border {
            color: colours.border.unwrap_or(iced::Color::TRANSPARENT),
            width: if colours.border.is_some() { 1.0 } else { 0.0 },
            radius: radius::MD.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const STATES: [Status; 4] = [
        Status::Active,
        Status::Hovered,
        Status::Pressed,
        Status::Disabled,
    ];

    #[test]
    fn default_builder_is_primary_medium_and_disabled() {
        let b: Button<'_, ()> = button("Save");
        assert_eq!(b.variant, Variant::Primary);
        assert_eq!(b.size, Size::Md);
        assert!(!b.is_enabled());
    }

    #[test]
    fn on_press_enables_and_on_press_maybe_none_disables() {
        assert!(button::<u8>("Go").on_press(1).is_enabled());
        assert!(
            !button::<u8>("Go")
                .on_press(1)
                .on_press_maybe(None)
                .is_enabled()
        );
    }

    #[test]
    fn icon_button_is_square_ghost() {
        let b: Button<'_, ()> = icon_button(crate::lucide!(Plus));
        assert_eq!(b.size, Size::Icon);
        assert_eq!(b.variant, Variant::Ghost);
        assert!(b.label.is_none());
    }

    #[test]
    fn sizes_grow_monotonically() {
        let heights = [Size::Sm, Size::Md, Size::Lg].map(|s| s.metrics().height);
        assert!(heights.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn hover_changes_background_for_filled_variants() {
        let tokens = Tokens::of(&light());
        for variant in [
            Variant::Primary,
            Variant::Secondary,
            Variant::Destructive,
            Variant::Outline,
            Variant::Ghost,
        ] {
            let active = colours(&tokens, variant, Status::Active);
            let hovered = colours(&tokens, variant, Status::Hovered);
            assert_ne!(active.background, hovered.background, "{variant:?}");
        }
    }

    #[test]
    fn pressed_is_stronger_than_hovered_for_primary() {
        let tokens = Tokens::of(&light());
        let hovered = colours(&tokens, Variant::Primary, Status::Hovered);
        let pressed = colours(&tokens, Variant::Primary, Status::Pressed);
        assert_ne!(hovered.background, pressed.background);
    }

    #[test]
    fn disabled_halves_alpha_for_every_variant() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                let active = colours(&tokens, variant, Status::Active);
                let disabled = colours(&tokens, variant, Status::Disabled);
                assert!((disabled.foreground.a - active.foreground.a * 0.5).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn only_outline_has_a_border() {
        let tokens = Tokens::of(&dark());
        for variant in Variant::ALL {
            for status in STATES {
                let style = style(&tokens, variant, status);
                assert_eq!(style.border.width > 0.0, variant == Variant::Outline);
            }
        }
    }

    #[test]
    fn ghost_and_link_are_transparent_at_rest() {
        let tokens = Tokens::of(&light());
        assert!(
            colours(&tokens, Variant::Ghost, Status::Active)
                .background
                .is_none()
        );
        assert!(
            colours(&tokens, Variant::Link, Status::Active)
                .background
                .is_none()
        );
    }
}
