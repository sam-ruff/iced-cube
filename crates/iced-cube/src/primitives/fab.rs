//! A floating action button: the one main action of a screen, drawn raised
//! above the content.
//!
//! A fab does not position itself. Place it over the content with an iced
//! `stack` and a container aligned to a corner:
//!
//! ```no_run
//! use iced::widget::{container, stack, text};
//! use iced::Length;
//! use iced_cube::{fab, lucide};
//!
//! #[derive(Debug, Clone)]
//! enum Message { New }
//!
//! let screen: iced::Element<'_, Message> = stack![
//!     text("The list"),
//!     container(fab(lucide!(Plus)).label("New note").on_press(Message::New))
//!         .width(Length::Fill)
//!         .height(Length::Fill)
//!         .align_right(Length::Fill)
//!         .align_bottom(Length::Fill)
//!         .padding(16),
//! ]
//! .into();
//! ```

use iced::widget::{self, button::Status, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Vector};

use crate::icon::{Glyph, opacity, themed};
use crate::natural::natural;
use crate::overlay::tooltip::{Position, tooltip};
use crate::primitives::button::{self, Variant};
use crate::theme::{Tokens, fade, text_size};

/// Height of a fab, and the side of an icon-only one.
pub const SIZE: f32 = 56.0;
/// Corner radius.
pub const RADIUS: f32 = 16.0;
/// Icon size.
pub const ICON: f32 = 24.0;
/// Horizontal padding of an extended fab.
pub const PADDING_X: f32 = 16.0;

/// A fab builder. Convert it into an [`Element`] to render.
///
/// A fab without a message is rendered disabled.
#[derive(Debug)]
pub struct Fab<'a, Message> {
    glyph: Glyph,
    label: Option<text::Fragment<'a>>,
    tooltip: Option<text::Fragment<'a>>,
    id: Option<widget::Id>,
    on_press: Option<Message>,
}

/// Creates a square fab showing `glyph`.
pub fn fab<'a, Message>(glyph: Glyph) -> Fab<'a, Message> {
    Fab {
        glyph,
        label: None,
        tooltip: None,
        id: None,
        on_press: None,
    }
}

impl<'a, Message> Fab<'a, Message> {
    /// Shows a label beside the icon, making the fab an extended pill.
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Names the action of an icon-only fab, such as "New note", shown as a
    /// tooltip on hover. An extended fab already shows its label and
    /// ignores this.
    pub fn tooltip(mut self, tooltip: impl text::IntoFragment<'a>) -> Self {
        self.tooltip = Some(tooltip.into_fragment());
        self
    }

    /// Sets a widget id, for finding the fab in tests.
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

    pub fn is_extended(&self) -> bool {
        self.label.is_some()
    }
}

impl<'a, Message: Clone + 'a> From<Fab<'a, Message>> for Element<'a, Message> {
    fn from(fab: Fab<'a, Message>) -> Self {
        let Fab {
            glyph,
            label,
            tooltip: hint,
            id,
            on_press,
        } = fab;
        let enabled = on_press.is_some();
        let resting = if enabled {
            Status::Active
        } else {
            Status::Disabled
        };
        let icon = themed(glyph, ICON, opacity(enabled), move |theme| {
            style(&Tokens::of(theme), resting).text_color
        });

        let extended = label.is_some();
        let content: Element<'a, Message> = match label {
            Some(label) => row![
                icon,
                text(label)
                    .size(text_size::SM)
                    .font(crate::theme::semibold())
                    .wrapping(text::Wrapping::None)
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
            None => icon.into(),
        };

        let (width, padding) = if extended {
            (Length::Shrink, Padding::from([0.0, PADDING_X]))
        } else {
            (Length::Fixed(SIZE), Padding::from((SIZE - ICON) / 2.0))
        };

        let button = widget::button(
            container(content)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
        )
        .width(width)
        .height(Length::Fixed(SIZE))
        .padding(padding)
        .on_press_maybe(on_press)
        .style(|theme, status| style(&Tokens::of(theme), status));

        let mut element: Element<'a, Message> = match id {
            Some(id) => container(button).id(id).into(),
            None => button.into(),
        };
        if let (Some(hint), false) = (hint, extended) {
            element = tooltip(element, hint).position(Position::Left).into();
        }
        natural(element)
    }
}

/// The shadow under a fab, deeper while hovered and shallower while
/// pressed, as if it moved towards the finger.
pub fn shadow(tokens: &Tokens, status: Status) -> Shadow {
    let alpha = if tokens.is_dark { 0.5 } else { 0.18 };
    let (offset, blur) = match status {
        Status::Hovered => (6.0, 16.0),
        Status::Pressed => (2.0, 6.0),
        Status::Active | Status::Disabled => (4.0, 12.0),
    };
    Shadow {
        color: fade(Color::BLACK, alpha),
        offset: Vector::new(0.0, offset),
        blur_radius: blur,
    }
}

/// The iced button style of a fab: primary colours, a large radius and a
/// soft shadow. A disabled fab is faded and flat.
pub fn style(tokens: &Tokens, status: Status) -> widget::button::Style {
    let colours = button::colours(tokens, Variant::Primary, status);
    widget::button::Style {
        background: colours.background.map(Background::Color),
        text_color: colours.foreground,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: RADIUS.into(),
        },
        shadow: if status == Status::Disabled {
            Shadow::default()
        } else {
            shadow(tokens, status)
        },
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
    fn builder_defaults() {
        let f: Fab<'_, u8> = fab(crate::lucide!(Plus));
        assert!(!f.is_enabled());
        assert!(!f.is_extended());
        assert!(fab::<u8>(crate::lucide!(Plus)).label("New").is_extended());
        assert!(fab::<u8>(crate::lucide!(Plus)).on_press(1).is_enabled());
        assert!(
            !fab::<u8>(crate::lucide!(Plus))
                .on_press(1)
                .on_press_maybe(None)
                .is_enabled()
        );
    }

    #[test]
    fn colours_are_the_primary_button_in_every_state() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in STATES {
                let style = style(&tokens, status);
                let primary = button::colours(&tokens, Variant::Primary, status);
                assert_eq!(style.background, primary.background.map(Background::Color));
                assert_eq!(style.text_color, primary.foreground);
                assert_eq!(style.border.radius, RADIUS.into());
            }
        }
    }

    #[test]
    fn hovered_and_pressed_differ_from_rest() {
        let tokens = Tokens::of(&light());
        let rest = style(&tokens, Status::Active);
        let hovered = style(&tokens, Status::Hovered);
        let pressed = style(&tokens, Status::Pressed);
        assert_ne!(rest.background, hovered.background);
        assert_ne!(hovered.background, pressed.background);
        assert!(hovered.shadow.blur_radius > rest.shadow.blur_radius);
        assert!(pressed.shadow.blur_radius < rest.shadow.blur_radius);
    }

    #[test]
    fn only_a_disabled_fab_is_flat_and_dark_shadows_are_stronger() {
        let light = Tokens::of(&light());
        let dark = Tokens::of(&dark());
        assert_eq!(style(&light, Status::Disabled).shadow, Shadow::default());
        assert!(style(&light, Status::Active).shadow.color.a > 0.0);
        assert!(
            style(&dark, Status::Active).shadow.color.a
                > style(&light, Status::Active).shadow.color.a
        );
    }
}
