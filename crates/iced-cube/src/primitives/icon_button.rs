//! Square, icon-only buttons with a label and an optional pressed state.
//!
//! Icon buttons share their [`Variant`], [`Size`] and colours with
//! [`button`](crate::primitives::button). A pressed icon button stays
//! highlighted, which suits toolbar toggles such as bold or italic.

use iced::widget::{self, button::Status, container, text};
use iced::{Background, Border, Element, Length, Padding, Shadow, Theme};

use crate::icon::{Glyph, tinted};
use crate::overlay::tooltip::{Position, tooltip};
use crate::primitives::button::{self, Colours, Size, Variant};
use crate::theme::{Tokens, fade, mix, radius};

/// An icon button builder. Convert it into an [`Element`] to render.
///
/// An icon button without a message is rendered disabled.
#[derive(Debug)]
pub struct IconButton<'a, Message> {
    glyph: Glyph,
    label: Option<text::Fragment<'a>>,
    tooltip: Option<Position>,
    variant: Variant,
    size: Size,
    pressed: bool,
    id: Option<widget::Id>,
    on_press: Option<Message>,
}

/// Creates a square ghost button showing `glyph`.
pub fn icon_button<'a, Message>(glyph: Glyph) -> IconButton<'a, Message> {
    IconButton {
        glyph,
        label: None,
        tooltip: Some(Position::default()),
        variant: Variant::Ghost,
        size: Size::Icon,
        pressed: false,
        id: None,
        on_press: None,
    }
}

impl<'a, Message> IconButton<'a, Message> {
    /// Names the action, such as "Bold". The label shows as a tooltip on
    /// hover unless [`tooltip`](Self::tooltip) turns it off.
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Where the label's tooltip appears, or `None` to never show it.
    /// Defaults to above the button.
    pub fn tooltip(mut self, position: Option<Position>) -> Self {
        self.tooltip = position;
        self
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// The side length follows the button [`Size`]: `Sm` is 32 pixels,
    /// `Md` and `Icon` are 36 and `Lg` is 40.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Draws the button as switched on, for toggles in a toolbar.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    /// Sets a widget id, for clicking the button in tests.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
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

    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    /// The label, if one is set.
    pub fn label_text(&self) -> Option<&str> {
        self.label.as_deref()
    }
}

impl<'a, Message: Clone + 'a> From<IconButton<'a, Message>> for Element<'a, Message> {
    fn from(icon_button: IconButton<'a, Message>) -> Self {
        let IconButton {
            glyph,
            label,
            tooltip: position,
            variant,
            size,
            pressed,
            id,
            on_press,
        } = icon_button;
        let metrics = size.metrics();
        let resting = if on_press.is_some() {
            Status::Active
        } else {
            Status::Disabled
        };

        let glyph =
            tinted(glyph, metrics.icon, None).style(move |theme: &Theme, _| widget::svg::Style {
                color: Some(colours(&Tokens::of(theme), variant, pressed, resting).foreground),
            });

        let button = widget::button(glyph)
            .width(Length::Fixed(metrics.height))
            .height(Length::Fixed(metrics.height))
            .padding(Padding::from((metrics.height - metrics.icon) / 2.0))
            .on_press_maybe(on_press)
            .style(move |theme, status| style(&Tokens::of(theme), variant, pressed, status));

        let mut element: Element<'a, Message> = match id {
            Some(id) => container(button).id(id).into(),
            None => button.into(),
        };
        if let (Some(label), Some(position)) = (label, position) {
            element = tooltip(element, label).position(position).into();
        }
        element
    }
}

/// Colours of an icon button, switched on or off, in a state.
///
/// A pressed button looks held down at rest and darkens a little more on
/// hover, so it reads as on in every state.
pub fn colours(tokens: &Tokens, variant: Variant, pressed: bool, status: Status) -> Colours {
    if !pressed {
        return button::colours(tokens, variant, status);
    }

    let held = button::colours(tokens, variant, Status::Pressed);
    match status {
        Status::Active => held,
        Status::Hovered | Status::Pressed => Colours {
            background: held
                .background
                .map(|background| mix(background, tokens.foreground, 0.08)),
            ..held
        },
        Status::Disabled => Colours {
            background: held.background.map(|c| fade(c, 0.5)),
            foreground: fade(held.foreground, 0.5),
            border: held.border.map(|c| fade(c, 0.5)),
        },
    }
}

/// The iced button style of an icon button, switched on or off, in a state.
pub fn style(
    tokens: &Tokens,
    variant: Variant,
    pressed: bool,
    status: Status,
) -> widget::button::Style {
    let colours = colours(tokens, variant, pressed, status);
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
    fn defaults_to_a_ghost_icon_size_button_that_is_off_and_disabled() {
        let b: IconButton<'_, ()> = icon_button(crate::lucide!(Plus));
        assert_eq!(b.variant, Variant::Ghost);
        assert_eq!(b.size, Size::Icon);
        assert!(!b.is_pressed());
        assert!(!b.is_enabled());
        assert!(b.label_text().is_none());
        assert_eq!(b.tooltip, Some(Position::Top));
    }

    #[test]
    fn builder_sets_every_option() {
        let b = icon_button::<u8>(crate::lucide!(Bold))
            .label("Bold")
            .tooltip(None)
            .variant(Variant::Outline)
            .size(Size::Sm)
            .pressed(true)
            .on_press(1);
        assert_eq!(b.label_text(), Some("Bold"));
        assert!(b.tooltip.is_none());
        assert_eq!(b.variant, Variant::Outline);
        assert_eq!(b.size, Size::Sm);
        assert!(b.is_pressed());
        assert!(b.is_enabled());
        assert!(!b.on_press_maybe(None).is_enabled());
    }

    #[test]
    fn unpressed_matches_the_button_colours() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                for status in STATES {
                    assert_eq!(
                        colours(&tokens, variant, false, status),
                        button::colours(&tokens, variant, status),
                        "{variant:?} {status:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn pressed_ghost_and_outline_use_the_accent_at_rest() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in [Variant::Ghost, Variant::Outline] {
                let on = colours(&tokens, variant, true, Status::Active);
                assert_eq!(on.background, Some(tokens.accent), "{variant:?}");
                assert_ne!(
                    on.background,
                    colours(&tokens, variant, false, Status::Active).background
                );
            }
        }
    }

    #[test]
    fn pressed_differs_from_unpressed_at_rest_for_filled_variants() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in [
                Variant::Primary,
                Variant::Secondary,
                Variant::Destructive,
                Variant::Outline,
                Variant::Ghost,
            ] {
                assert_ne!(
                    colours(&tokens, variant, true, Status::Active).background,
                    colours(&tokens, variant, false, Status::Active).background,
                    "{variant:?}"
                );
            }
        }
    }

    #[test]
    fn hovering_a_pressed_button_darkens_it() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in [Variant::Ghost, Variant::Outline, Variant::Primary] {
                let rest = colours(&tokens, variant, true, Status::Active);
                for status in [Status::Hovered, Status::Pressed] {
                    let active = colours(&tokens, variant, true, status);
                    assert_ne!(active.background, rest.background, "{variant:?} {status:?}");
                    assert_eq!(active.foreground, rest.foreground);
                }
            }
        }
    }

    #[test]
    fn disabled_pressed_keeps_the_on_look_at_half_alpha() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                let on = colours(&tokens, variant, true, Status::Active);
                let disabled = colours(&tokens, variant, true, Status::Disabled);
                assert!((disabled.foreground.a - on.foreground.a * 0.5).abs() < 1e-6);
                assert_eq!(
                    disabled.background.map(|c| c.a),
                    on.background.map(|c| c.a * 0.5)
                );
            }
        }
    }

    #[test]
    fn only_outline_has_a_border_in_every_state() {
        let tokens = Tokens::of(&light());
        for variant in Variant::ALL {
            for pressed in [false, true] {
                for status in STATES {
                    let style = style(&tokens, variant, pressed, status);
                    assert_eq!(style.border.width > 0.0, variant == Variant::Outline);
                    assert_eq!(style.border.radius, radius::MD.into());
                }
            }
        }
    }
}
