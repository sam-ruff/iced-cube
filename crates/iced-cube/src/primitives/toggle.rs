//! Two-state buttons that stay pressed while an option is on.
//!
//! A toggle is a button with a pressed state, for options such as "Bold" or
//! "Show grid" that sit among other buttons rather than in a form. The app
//! owns the bool and flips it in `on_toggle`, as with a switch. For a set of
//! toggles where one or several can be on, use
//! [`toggle_group`](mod@crate::primitives::toggle_group).

use iced::keyboard::key::Named;
use iced::widget::{self, button::Status, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Shadow};

use crate::icon::{Glyph, opacity, themed};
use crate::keys::{self, Chord, Keymap};
use crate::natural::natural;
use crate::theme::{Tokens, fade, mix, radius, text_size};

/// How a toggle looks while it is off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Variant {
    /// Transparent until hovered or pressed.
    #[default]
    Default,
    /// A bordered button, so the toggle reads as a control at rest.
    Outline,
}

impl Variant {
    pub const ALL: [Variant; 2] = [Variant::Default, Variant::Outline];
}

/// Toggle dimensions.
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
                padding_x: 16.0,
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

/// A toggle builder. Convert it into an [`Element`] to render.
///
/// A toggle without [`on_toggle`](Toggle::on_toggle) is rendered disabled.
pub struct Toggle<'a, Message> {
    label: text::Fragment<'a>,
    icon: Option<Glyph>,
    pressed: bool,
    variant: Variant,
    size: Size,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

/// Creates a toggle with a text label, switched off.
pub fn toggle<'a, Message>(label: impl text::IntoFragment<'a>) -> Toggle<'a, Message> {
    Toggle {
        label: label.into_fragment(),
        icon: None,
        pressed: false,
        variant: Variant::default(),
        size: Size::default(),
        on_toggle: None,
    }
}

impl<Message> std::fmt::Debug for Toggle<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Toggle")
            .field("label", &self.label)
            .field("icon", &self.icon)
            .field("pressed", &self.pressed)
            .field("variant", &self.variant)
            .field("size", &self.size)
            .field("enabled", &self.on_toggle.is_some())
            .finish()
    }
}

impl<'a, Message> Toggle<'a, Message> {
    /// Adds an icon before the label.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.icon = Some(glyph);
        self
    }

    /// Whether the toggle is on.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Sets the message for a click, built from the new pressed state.
    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    pub fn is_enabled(&self) -> bool {
        self.on_toggle.is_some()
    }
}

impl<'a, Message: Clone + 'a> From<Toggle<'a, Message>> for Element<'a, Message> {
    fn from(toggle: Toggle<'a, Message>) -> Self {
        let Toggle {
            label,
            icon,
            pressed,
            variant,
            size,
            on_toggle,
        } = toggle;
        let message = on_toggle.map(|on_toggle| on_toggle(!pressed));
        natural(button(
            label,
            icon,
            pressed,
            size,
            message,
            move |tokens, status| style(tokens, variant, pressed, status),
        ))
    }
}

/// Renders the toggle button itself, so a group can draw its items with
/// their own borders and corners.
pub(crate) fn button<'a, Message: Clone + 'a>(
    label: impl text::IntoFragment<'a>,
    icon: Option<Glyph>,
    pressed: bool,
    size: Size,
    message: Option<Message>,
    style: impl Fn(&Tokens, Status) -> widget::button::Style + 'a,
) -> Element<'a, Message> {
    let metrics = size.metrics();
    let enabled = message.is_some();
    let resting = if enabled {
        Status::Active
    } else {
        Status::Disabled
    };

    let mut content = row![].spacing(6).align_y(Alignment::Center);
    if let Some(glyph) = icon {
        content = content.push(themed(
            glyph,
            metrics.icon,
            opacity(enabled),
            move |theme| colours(&Tokens::of(theme), pressed, resting).foreground,
        ));
    }
    content = content.push(
        text(label)
            .size(metrics.text)
            .wrapping(text::Wrapping::None),
    );

    widget::button(
        widget::container(content)
            .height(Length::Fill)
            .align_y(Alignment::Center),
    )
    .height(Length::Fixed(metrics.height))
    .padding(Padding::from([0.0, metrics.padding_x]))
    .on_press_maybe(message)
    .style(move |theme, status| style(&Tokens::of(theme), status))
    .into()
}

/// Background and foreground colours of a toggle, on or off, in a state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub background: Option<Color>,
    pub foreground: Color,
}

/// Resolves the colours of a toggle. Off, it is muted and fills on hover;
/// on, it takes the accent and darkens a little on hover, so it reads as
/// on in every state.
pub fn colours(tokens: &Tokens, pressed: bool, status: Status) -> Colours {
    let hover = matches!(status, Status::Hovered | Status::Pressed);
    let colours = match (pressed, hover) {
        (true, false) => Colours {
            background: Some(tokens.accent),
            foreground: tokens.accent_foreground,
        },
        (true, true) => Colours {
            background: Some(mix(tokens.accent, tokens.foreground, 0.08)),
            foreground: tokens.accent_foreground,
        },
        (false, true) => Colours {
            background: Some(tokens.muted),
            foreground: tokens.foreground,
        },
        (false, false) => Colours {
            background: None,
            foreground: tokens.muted_foreground,
        },
    };

    if status != Status::Disabled {
        return colours;
    }
    Colours {
        background: colours.background.map(|c| fade(c, 0.5)),
        foreground: fade(colours.foreground, 0.5),
    }
}

/// The iced button style of a toggle.
pub fn style(
    tokens: &Tokens,
    variant: Variant,
    pressed: bool,
    status: Status,
) -> widget::button::Style {
    let colours = colours(tokens, pressed, status);
    let outline = variant == Variant::Outline;
    let border = if status == Status::Disabled {
        fade(tokens.border, 0.5)
    } else {
        tokens.border
    };
    widget::button::Style {
        background: colours.background.map(Background::Color),
        text_color: colours.foreground,
        border: Border {
            color: if outline { border } else { Color::TRANSPARENT },
            width: if outline { 1.0 } else { 0.0 },
            radius: radius::MD.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// What a toggle keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Toggle,
}

impl Action {
    /// The pressed state to send through the toggle's `on_toggle` message.
    pub fn apply(self, pressed: bool) -> bool {
        match self {
            Action::Toggle => !pressed,
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Toggle];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Toggle => "Toggle",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Toggle => "Switches the toggle on or off.",
        }
    }
}

/// The default toggle shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Space` | [`Action::Toggle`] |
///
/// Shortcuts are app-wide, so an app with several toggles binds each one
/// its own chord, such as `Ctrl+B` for a bold toggle.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new().bind(Chord::named(Named::Space), Action::Toggle)
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
    fn defaults_to_an_off_medium_toggle_that_is_disabled() {
        let t: Toggle<'_, bool> = toggle("Bold");
        assert_eq!(t.variant, Variant::Default);
        assert_eq!(t.size, Size::Md);
        assert!(t.icon.is_none());
        assert!(!t.is_pressed());
        assert!(!t.is_enabled());
    }

    #[test]
    fn builder_sets_every_option() {
        let t = toggle("Grid")
            .icon(crate::lucide!(Grid3x3))
            .pressed(true)
            .variant(Variant::Outline)
            .size(Size::Lg)
            .on_toggle(|on| on);
        assert_eq!(t.icon.map(Glyph::name), Some("grid-3x3"));
        assert!(t.is_pressed());
        assert!(t.is_enabled());
        assert_eq!(t.variant, Variant::Outline);
        assert_eq!(t.size, Size::Lg);
    }

    #[test]
    fn sizes_grow_monotonically() {
        let heights = Size::ALL.map(|size| size.metrics().height);
        assert!(heights.windows(2).all(|w| w[0] < w[1]));
        let icons = Size::ALL.map(|size| size.metrics().icon);
        assert!(icons.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn on_takes_the_accent_and_off_is_transparent_at_rest() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let on = colours(&tokens, true, Status::Active);
            let off = colours(&tokens, false, Status::Active);
            assert_eq!(on.background, Some(tokens.accent));
            assert_eq!(on.foreground, tokens.accent_foreground);
            assert_eq!(off.background, None);
            assert_eq!(off.foreground, tokens.muted_foreground);
        }
    }

    #[test]
    fn hover_changes_the_background_on_and_off() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for pressed in [false, true] {
                let rest = colours(&tokens, pressed, Status::Active);
                for status in [Status::Hovered, Status::Pressed] {
                    let hovered = colours(&tokens, pressed, status);
                    assert_ne!(hovered.background, rest.background, "{pressed} {status:?}");
                    assert!(hovered.background.is_some());
                }
            }
        }
    }

    #[test]
    fn on_and_off_differ_in_every_enabled_state() {
        let tokens = Tokens::of(&light());
        for status in [Status::Active, Status::Hovered, Status::Pressed] {
            assert_ne!(
                colours(&tokens, true, status),
                colours(&tokens, false, status),
                "{status:?}"
            );
        }
    }

    #[test]
    fn disabled_keeps_the_look_at_half_alpha() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for pressed in [false, true] {
                let rest = colours(&tokens, pressed, Status::Active);
                let disabled = colours(&tokens, pressed, Status::Disabled);
                assert!((disabled.foreground.a - rest.foreground.a * 0.5).abs() < 1e-6);
                assert_eq!(
                    disabled.background.map(|c| c.a),
                    rest.background.map(|c| c.a * 0.5)
                );
            }
        }
    }

    #[test]
    fn only_outline_has_a_border_in_every_state() {
        let tokens = Tokens::of(&dark());
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

    #[test]
    fn toggle_action_flips_the_state() {
        assert!(Action::Toggle.apply(false));
        assert!(!Action::Toggle.apply(true));
    }

    #[test]
    fn default_keymap_and_overrides() {
        let space = iced::keyboard::Key::Named(Named::Space);
        let keymap = default_keymap();
        assert_eq!(
            keymap.resolve(&space, Default::default()),
            Some(Action::Toggle)
        );
        assert_eq!(Keymap::<Action>::defaults(), keymap);

        let bold: Chord = "Ctrl+B".parse().unwrap();
        let custom = keymap
            .unbind(&Chord::named(Named::Space))
            .bind(bold.clone(), Action::Toggle);
        assert_eq!(custom.resolve(&space, Default::default()), None);
        assert_eq!(
            custom.resolve(bold.key(), bold.modifiers()),
            Some(Action::Toggle)
        );
        assert!(custom.unbind_action(&Action::Toggle).is_empty());
    }
}
