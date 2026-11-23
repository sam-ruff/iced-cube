//! Compact pills for chosen values, such as guests or tags, with an
//! optional x button that removes them.
//!
//! Chips share the [badge](mod@crate::feedback::badge) colours and are
//! [`Variant::Secondary`] by default. Give a chip [`Chip::on_remove`] to
//! draw the x, and [`Chip::id`] to find that x in tests through
//! [`remove_id`].
//!
//! ```no_run
//! use iced::Element;
//! use iced_cube::feedback::chip::chip;
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Remove(usize),
//! }
//!
//! fn guests(names: &[&str]) -> Element<'static, Message> {
//!     iced::widget::row(names.iter().enumerate().map(|(index, name)| {
//!         chip(name.to_string())
//!             .on_remove(Message::Remove(index))
//!             .into()
//!     }))
//!     .spacing(4)
//!     .wrap()
//!     .into()
//! }
//! ```

use std::fmt;

use iced::widget::{self, button, container, row, text};
use iced::{Alignment, Background, Border, Element, Length, Padding, Shadow};

pub use crate::feedback::badge::Variant;
use crate::feedback::badge::{self, colours};
use crate::icon::{Glyph, themed};
use crate::natural::natural;
use crate::theme::{Tokens, fade, radius, text_size};

/// Space between the leading icon or element, the label and the x.
const GAP: f32 = 4.0;
/// Padding around the x glyph inside its round button.
const REMOVE_PADDING: f32 = 1.0;

/// Chip dimensions. [`Size::Sm`] matches a badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
}

impl Size {
    pub const ALL: [Size; 2] = [Size::Sm, Size::Md];

    /// Height, horizontal padding, text size, icon size and x size.
    pub const fn metrics(self) -> Metrics {
        match self {
            Size::Sm => Metrics {
                height: 20.0,
                padding_x: 10.0,
                text: text_size::XS,
                icon: 12.0,
                remove: 12.0,
            },
            Size::Md => Metrics {
                height: 26.0,
                padding_x: 10.0,
                text: text_size::SM,
                icon: 14.0,
                remove: 14.0,
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
    pub remove: f32,
}

impl Metrics {
    /// The chip's padding: less on the left beside a leading element, so an
    /// avatar sits in the curve, and less on the right beside the x.
    pub fn padding(self, leading_element: bool, removable: bool) -> Padding {
        Padding {
            top: 0.0,
            bottom: 0.0,
            left: if leading_element { 1.0 } else { self.padding_x },
            right: if removable { 3.0 } else { self.padding_x },
        }
    }
}

/// The widget id of the x button of the chip with [`Chip::id`] `key`.
pub fn remove_id(key: &str) -> widget::Id {
    widget::Id::from(format!("iced-cube-chip-remove-{key}"))
}

enum Leading<'a, Message> {
    None,
    Icon(Glyph),
    Element(Element<'a, Message>),
}

/// A chip builder. Convert it into an [`Element`] to render.
///
/// A chip without [`on_remove`](Chip::on_remove) has no x button.
pub struct Chip<'a, Message> {
    label: text::Fragment<'a>,
    leading: Leading<'a, Message>,
    variant: Variant,
    size: Size,
    key: Option<String>,
    on_remove: Option<Message>,
}

/// Creates a chip with a text label.
pub fn chip<'a, Message>(label: impl text::IntoFragment<'a>) -> Chip<'a, Message> {
    Chip {
        label: label.into_fragment(),
        leading: Leading::None,
        variant: Variant::Secondary,
        size: Size::default(),
        key: None,
        on_remove: None,
    }
}

impl<Message: fmt::Debug> fmt::Debug for Chip<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let leading = match &self.leading {
            Leading::None => "none",
            Leading::Icon(_) => "icon",
            Leading::Element(_) => "element",
        };
        f.debug_struct("Chip")
            .field("label", &self.label)
            .field("leading", &leading)
            .field("variant", &self.variant)
            .field("size", &self.size)
            .field("key", &self.key)
            .field("on_remove", &self.on_remove)
            .finish()
    }
}

impl<'a, Message> Chip<'a, Message> {
    /// Defaults to [`Variant::Secondary`].
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Defaults to [`Size::Md`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Adds an icon before the label. Replaces any leading element.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.leading = Leading::Icon(glyph);
        self
    }

    /// Adds an element before the label, such as a small avatar, laid out at
    /// its own size against the chip's left edge. Replaces any icon.
    pub fn leading(mut self, element: impl Into<Element<'a, Message>>) -> Self {
        self.leading = Leading::Element(element.into());
        self
    }

    /// A key, unique on screen, that gives the x button the widget id
    /// [`remove_id`]`(key)`, to find or click it in tests.
    pub fn id(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Draws an x button after the label that sends `message` when pressed.
    pub fn on_remove(mut self, message: Message) -> Self {
        self.on_remove = Some(message);
        self
    }

    /// Like [`on_remove`](Self::on_remove); `None` leaves the x out.
    pub fn on_remove_maybe(mut self, message: Option<Message>) -> Self {
        self.on_remove = message;
        self
    }

    pub fn is_removable(&self) -> bool {
        self.on_remove.is_some()
    }
}

impl<'a, Message: Clone + 'a> From<Chip<'a, Message>> for Element<'a, Message> {
    fn from(chip: Chip<'a, Message>) -> Self {
        let Chip {
            label,
            leading,
            variant,
            size,
            key,
            on_remove,
        } = chip;
        let metrics = size.metrics();
        let foreground = move |theme: &iced::Theme| colours(&Tokens::of(theme), variant).foreground;

        let leading_element = matches!(leading, Leading::Element(_));
        let mut content = row![].spacing(GAP).align_y(Alignment::Center);
        match leading {
            Leading::None => {}
            Leading::Icon(glyph) => {
                content = content.push(themed(glyph, metrics.icon, 1.0, foreground));
            }
            Leading::Element(element) => content = content.push(element),
        }
        content = content.push(
            text(label)
                .size(metrics.text)
                .wrapping(text::Wrapping::None),
        );

        let removable = on_remove.is_some();
        if let Some(message) = on_remove {
            let remove = button(themed(crate::lucide!(X), metrics.remove, 1.0, foreground))
                .padding(REMOVE_PADDING)
                .on_press(message)
                .style(move |theme, status| remove_style(&Tokens::of(theme), variant, status));
            let mut remove = container(remove);
            if let Some(key) = &key {
                remove = remove.id(remove_id(key));
            }
            content = content.push(remove);
        }

        natural(
            container(content)
                .height(Length::Fixed(metrics.height))
                .align_y(Alignment::Center)
                .padding(metrics.padding(leading_element, removable))
                .style(move |theme| style(&Tokens::of(theme), variant)),
        )
    }
}

/// The chip's container style: the badge style of the same variant.
pub fn style(tokens: &Tokens, variant: Variant) -> container::Style {
    badge::style(tokens, variant)
}

/// The x button style: clear until hovered or pressed, then a faint circle
/// of the chip's foreground colour.
pub fn remove_style(tokens: &Tokens, variant: Variant, status: button::Status) -> button::Style {
    let foreground = colours(tokens, variant).foreground;
    let background = match status {
        button::Status::Hovered => Some(fade(foreground, 0.15)),
        button::Status::Pressed => Some(fade(foreground, 0.25)),
        button::Status::Active | button::Status::Disabled => None,
    };
    button::Style {
        background: background.map(Background::Color),
        text_color: foreground,
        border: Border {
            radius: radius::FULL.into(),
            ..Border::default()
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const STATUSES: [button::Status; 4] = [
        button::Status::Active,
        button::Status::Hovered,
        button::Status::Pressed,
        button::Status::Disabled,
    ];

    #[test]
    fn default_builder_is_a_secondary_medium_chip_without_x() {
        let c: Chip<'_, ()> = chip("Ada");
        assert_eq!(c.variant, Variant::Secondary);
        assert_eq!(c.size, Size::Md);
        assert!(matches!(c.leading, Leading::None));
        assert!(c.key.is_none());
        assert!(!c.is_removable());
    }

    #[test]
    fn builder_keeps_its_options() {
        let c = chip("Ada")
            .variant(Variant::Outline)
            .size(Size::Sm)
            .icon(crate::lucide!(User))
            .id("ada")
            .on_remove(1);
        assert_eq!(c.variant, Variant::Outline);
        assert_eq!(c.size, Size::Sm);
        assert!(matches!(c.leading, Leading::Icon(glyph) if glyph.name() == "user"));
        assert_eq!(c.key.as_deref(), Some("ada"));
        assert_eq!(c.on_remove, Some(1));
        assert!(!c.on_remove_maybe(None).is_removable());
    }

    #[test]
    fn leading_element_replaces_the_icon_and_back() {
        let c: Chip<'_, ()> = chip("Ada").icon(crate::lucide!(User)).leading(text("A"));
        assert!(matches!(c.leading, Leading::Element(_)));
        let c = c.icon(crate::lucide!(User));
        assert!(matches!(c.leading, Leading::Icon(_)));
    }

    #[test]
    fn small_chips_match_badges_and_sizes_grow() {
        let sm = Size::Sm.metrics();
        let md = Size::Md.metrics();
        assert_eq!(sm.text, text_size::XS);
        assert_eq!(sm.icon, 12.0);
        assert!(md.height > sm.height);
        assert!(md.text > sm.text);
        for size in Size::ALL {
            let metrics = size.metrics();
            assert!(
                metrics.remove + 2.0 * REMOVE_PADDING < metrics.height,
                "{size:?}"
            );
        }
    }

    #[test]
    fn padding_tightens_beside_an_element_and_the_x() {
        let metrics = Size::Md.metrics();
        let plain = metrics.padding(false, false);
        assert_eq!((plain.left, plain.right), (10.0, 10.0));
        let both = metrics.padding(true, true);
        assert!(both.left < plain.left && both.right < plain.right);
    }

    #[test]
    fn chips_use_the_badge_style_of_their_variant() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                assert_eq!(style(&tokens, variant), badge::style(&tokens, variant));
                assert_eq!(style(&tokens, variant).border.radius, radius::FULL.into());
            }
        }
    }

    #[test]
    fn the_x_is_clear_until_hovered_or_pressed() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for variant in Variant::ALL {
                let foreground = colours(&tokens, variant).foreground;
                for status in STATUSES {
                    let style = remove_style(&tokens, variant, status);
                    assert_eq!(style.text_color, foreground, "{variant:?} {status:?}");
                    let filled =
                        matches!(status, button::Status::Hovered | button::Status::Pressed);
                    assert_eq!(style.background.is_some(), filled, "{variant:?} {status:?}");
                }
                let hovered = remove_style(&tokens, variant, button::Status::Hovered);
                let pressed = remove_style(&tokens, variant, button::Status::Pressed);
                assert_ne!(hovered.background, pressed.background, "{variant:?}");
            }
        }
    }

    #[test]
    fn remove_ids_differ_by_key() {
        assert_eq!(remove_id("ada"), remove_id("ada"));
        assert_ne!(remove_id("ada"), remove_id("bob"));
    }
}
