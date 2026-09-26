//! Single-line text inputs with an optional leading icon.

use std::fmt;

use iced::widget::text::LineHeight;
use iced::widget::text_input::{Status, Style};
use iced::widget::{self, Id, container, stack, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::icon::{Glyph, tinted};
use crate::theme::{Tokens, fade, mix, radius, space, text_size};

const LINE_HEIGHT: f32 = 1.3;

/// Input dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
}

impl Size {
    pub const ALL: [Size; 3] = [Size::Sm, Size::Md, Size::Lg];

    /// Height, horizontal padding, text size and icon size.
    pub const fn metrics(self) -> Metrics {
        match self {
            Size::Sm => Metrics {
                height: 32.0,
                padding_x: 10.0,
                text: text_size::SM,
                icon: 14.0,
            },
            Size::Md => Metrics {
                height: 36.0,
                padding_x: 12.0,
                text: text_size::SM,
                icon: 16.0,
            },
            Size::Lg => Metrics {
                height: 40.0,
                padding_x: 14.0,
                text: text_size::MD,
                icon: 18.0,
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

impl Metrics {
    /// Inner padding that makes the input exactly `height` tall, leaving
    /// room for a leading icon when there is one.
    pub fn padding(self, with_icon: bool) -> Padding {
        let vertical = ((self.height - self.text * LINE_HEIGHT) / 2.0).max(0.0);
        let left = if with_icon {
            self.padding_x + self.icon + space::SM
        } else {
            self.padding_x
        };
        Padding {
            top: vertical,
            bottom: vertical,
            left,
            right: self.padding_x,
        }
    }
}

/// A text input builder. Convert it into an [`Element`] to render.
///
/// An input without an `on_input` message is rendered disabled.
pub struct Input<'a, Message> {
    placeholder: String,
    value: String,
    id: Option<Id>,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    icon: Option<Glyph>,
    secure: bool,
    invalid: bool,
    size: Size,
    width: Length,
}

/// Creates a text input showing `value`, with `placeholder` while empty.
pub fn input<'a, Message>(placeholder: &str, value: &str) -> Input<'a, Message> {
    Input {
        placeholder: placeholder.to_owned(),
        value: value.to_owned(),
        id: None,
        on_input: None,
        on_submit: None,
        icon: None,
        secure: false,
        invalid: false,
        size: Size::default(),
        width: Length::Fill,
    }
}

impl<'a, Message> Input<'a, Message> {
    /// Sets the widget id, used to focus the input or find it in tests.
    pub fn id(mut self, id: impl Into<Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the message produced for every edit. Without it the input is disabled.
    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    pub fn on_input_maybe(mut self, on_input: Option<impl Fn(String) -> Message + 'a>) -> Self {
        self.on_input = on_input.map(|f| Box::new(f) as Box<dyn Fn(String) -> Message + 'a>);
        self
    }

    /// Sets the message produced when Enter is pressed.
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    pub fn on_submit_maybe(mut self, message: Option<Message>) -> Self {
        self.on_submit = message;
        self
    }

    /// Adds a Lucide icon at the start of the input.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Masks the value, for passwords and other secrets.
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Marks the value as invalid, drawing a destructive border.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Overrides the width. Inputs fill the available width by default.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_input.is_some()
    }
}

impl<Message: fmt::Debug> fmt::Debug for Input<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Input")
            .field("placeholder", &self.placeholder)
            .field("value", &self.value)
            .field("id", &self.id)
            .field("enabled", &self.is_enabled())
            .field("on_submit", &self.on_submit)
            .field("icon", &self.icon)
            .field("secure", &self.secure)
            .field("invalid", &self.invalid)
            .field("size", &self.size)
            .field("width", &self.width)
            .finish()
    }
}

impl<'a, Message: Clone + 'a> From<Input<'a, Message>> for Element<'a, Message> {
    fn from(input: Input<'a, Message>) -> Self {
        let metrics = input.size.metrics();
        let invalid = input.invalid;
        let enabled = input.is_enabled();

        let mut field = text_input(&input.placeholder, &input.value)
            .secure(input.secure)
            .size(metrics.text)
            .line_height(LineHeight::Relative(LINE_HEIGHT))
            .padding(metrics.padding(input.icon.is_some()))
            .width(Length::Fill)
            .on_submit_maybe(input.on_submit.filter(|_| enabled))
            .style(move |theme, status| style(&Tokens::of(theme), status, invalid));
        if let Some(id) = input.id {
            field = field.id(id);
        }
        if let Some(on_input) = input.on_input {
            field = field.on_input(on_input);
        }

        let Some(glyph) = input.icon else {
            return container(field).width(input.width).into();
        };

        let status = if enabled {
            Status::Active
        } else {
            Status::Disabled
        };
        let icon =
            tinted(glyph, metrics.icon, None).style(move |theme: &Theme, _| widget::svg::Style {
                color: Some(style(&Tokens::of(theme), status, invalid).icon),
            });
        let icon = container(icon)
            .padding(Padding::ZERO.left(metrics.padding_x))
            .height(Length::Fill)
            .align_y(Alignment::Center);

        stack![field, icon].width(input.width).into()
    }
}

/// The iced text input style for a status, optionally marked invalid.
pub fn style(tokens: &Tokens, status: Status, invalid: bool) -> Style {
    let border = match (invalid, status) {
        (true, _) => tokens.destructive,
        (false, Status::Focused { .. }) => mix(tokens.border, tokens.foreground, 0.6),
        (false, Status::Hovered) => mix(tokens.border, tokens.foreground, 0.25),
        (false, Status::Active | Status::Disabled) => tokens.border,
    };

    let active = Style {
        background: Background::Color(tokens.background),
        border: Border {
            color: border,
            width: 1.0,
            radius: radius::MD.into(),
        },
        icon: tokens.muted_foreground,
        placeholder: tokens.muted_foreground,
        value: tokens.foreground,
        selection: fade(tokens.primary, 0.25),
    };

    if status != Status::Disabled {
        return active;
    }
    Style {
        background: Background::Color(fade(tokens.muted, 0.5)),
        border: Border {
            color: fade(border, 0.5),
            ..active.border
        },
        icon: fade(active.icon, 0.5),
        placeholder: fade(active.placeholder, 0.5),
        value: fade(active.value, 0.5),
        selection: Color::TRANSPARENT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const STATES: [Status; 5] = [
        Status::Active,
        Status::Hovered,
        Status::Focused { is_hovered: false },
        Status::Focused { is_hovered: true },
        Status::Disabled,
    ];

    fn border(style: Style) -> Color {
        style.border.color
    }

    #[test]
    fn default_builder_is_medium_fill_and_disabled() {
        let i: Input<'_, ()> = input("Email", "");
        assert_eq!(i.size, Size::Md);
        assert_eq!(i.width, Length::Fill);
        assert!(!i.secure && !i.invalid);
        assert!(i.icon.is_none() && i.id.is_none());
        assert!(!i.is_enabled());
    }

    #[test]
    fn on_input_enables_and_on_input_maybe_none_disables() {
        assert!(input::<String>("", "").on_input(|s| s).is_enabled());
        let none: Option<fn(String) -> String> = None;
        assert!(
            !input::<String>("", "")
                .on_input(|s| s)
                .on_input_maybe(none)
                .is_enabled()
        );
    }

    #[test]
    fn builder_keeps_placeholder_and_value() {
        let i: Input<'_, ()> = input("Search", "iced").secure(true).invalid(true);
        assert_eq!(i.placeholder, "Search");
        assert_eq!(i.value, "iced");
        assert!(i.secure && i.invalid);
    }

    #[test]
    fn sizes_grow_monotonically() {
        let heights = Size::ALL.map(|s| s.metrics().height);
        assert!(heights.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn padding_produces_the_target_height() {
        for size in Size::ALL {
            let m = size.metrics();
            let p = m.padding(false);
            let height = m.text * LINE_HEIGHT + p.top + p.bottom;
            assert!((height - m.height).abs() < 1e-3, "{size:?}");
        }
    }

    #[test]
    fn icon_reserves_space_on_the_left_only() {
        let m = Size::Md.metrics();
        let plain = m.padding(false);
        let with_icon = m.padding(true);
        assert!(with_icon.left > plain.left + m.icon);
        assert_eq!(with_icon.right, plain.right);
        assert_eq!(with_icon.top, plain.top);
    }

    #[test]
    fn hover_and_focus_strengthen_the_border() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let active = border(style(&tokens, Status::Active, false));
            let hovered = border(style(&tokens, Status::Hovered, false));
            let focused = border(style(&tokens, Status::Focused { is_hovered: false }, false));
            assert_ne!(active, hovered);
            assert_ne!(hovered, focused);
            assert_eq!(
                focused,
                border(style(&tokens, Status::Focused { is_hovered: true }, false))
            );
        }
    }

    #[test]
    fn invalid_uses_destructive_border_in_every_enabled_status() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in STATES {
                let style = style(&tokens, status, true);
                let expected = if status == Status::Disabled {
                    fade(tokens.destructive, 0.5)
                } else {
                    tokens.destructive
                };
                assert_eq!(style.border.color, expected, "{status:?}");
            }
        }
    }

    #[test]
    fn disabled_halves_text_alpha_and_mutes_background() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let active = style(&tokens, Status::Active, false);
            let disabled = style(&tokens, Status::Disabled, false);
            assert!((disabled.value.a - active.value.a * 0.5).abs() < 1e-6);
            assert!((disabled.placeholder.a - active.placeholder.a * 0.5).abs() < 1e-6);
            assert_ne!(disabled.background, active.background);
        }
    }

    #[test]
    fn every_status_has_a_visible_border_and_readable_value() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in STATES {
                for invalid in [false, true] {
                    let style = style(&tokens, status, invalid);
                    assert_eq!(style.border.width, 1.0);
                    assert!(style.border.color.a > 0.0);
                    assert_ne!(style.value, tokens.background);
                }
            }
        }
    }
}
