//! On/off switches for settings that apply immediately.

use iced::keyboard::key::Named;
use iced::widget::{self, toggler::Status};
use iced::{Background, Color, Element};

use crate::keys::{self, Chord, Keymap};
use crate::theme::{Tokens, fade, mix, space as spacing, text_size};

/// Height of the switch track in logical pixels. The track is twice as wide.
pub const TRACK_HEIGHT: f32 = 20.0;

/// A switch builder. Convert it into an [`Element`] to render.
///
/// A switch without a message is rendered disabled.
pub struct Switch<'a, Message> {
    is_on: bool,
    label: Option<widget::text::Fragment<'a>>,
    on_toggle: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

impl<Message> std::fmt::Debug for Switch<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Switch")
            .field("is_on", &self.is_on)
            .field("label", &self.label)
            .field("enabled", &self.on_toggle.is_some())
            .finish()
    }
}

/// Creates a switch that is on when `is_on` is true.
pub fn switch<'a, Message>(is_on: bool) -> Switch<'a, Message> {
    Switch {
        is_on,
        label: None,
        on_toggle: None,
    }
}

impl<'a, Message> Switch<'a, Message> {
    pub fn label(mut self, label: impl widget::text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Sets the message for a click. It receives the new value.
    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    /// Like [`Switch::on_toggle`], but `None` disables the switch.
    pub fn on_toggle_maybe(mut self, on_toggle: Option<impl Fn(bool) -> Message + 'a>) -> Self {
        self.on_toggle = on_toggle.map(|f| Box::new(f) as Box<dyn Fn(bool) -> Message + 'a>);
        self
    }

    pub fn is_on(&self) -> bool {
        self.is_on
    }

    pub fn is_enabled(&self) -> bool {
        self.on_toggle.is_some()
    }
}

impl<'a, Message: 'a> From<Switch<'a, Message>> for Element<'a, Message> {
    fn from(switch: Switch<'a, Message>) -> Self {
        let mut toggler = widget::toggler(switch.is_on)
            .size(TRACK_HEIGHT)
            .text_size(text_size::SM)
            .spacing(spacing::SM)
            .style(|theme, status| style(&Tokens::of(theme), status));
        if let Some(label) = switch.label {
            toggler = toggler.label(label);
        }
        if let Some(on_toggle) = switch.on_toggle {
            toggler = toggler.on_toggle(on_toggle);
        }
        toggler.into()
    }
}

/// Track, thumb and label colours of a switch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub track: Color,
    pub thumb: Color,
    pub label: Color,
}

/// Resolves the colours of a switch for an interaction status.
pub fn colours(tokens: &Tokens, status: Status) -> Colours {
    let (is_on, hover, disabled) = match status {
        Status::Active { is_toggled } => (is_toggled, false, false),
        Status::Hovered { is_toggled } => (is_toggled, true, false),
        Status::Disabled { is_toggled } => (is_toggled, false, true),
    };

    let off_track = mix(tokens.border, tokens.foreground, 0.1);
    let track = match (is_on, hover) {
        (true, false) => tokens.primary,
        (true, true) => mix(tokens.primary, tokens.background, 0.15),
        (false, false) => off_track,
        (false, true) => mix(off_track, tokens.foreground, 0.15),
    };
    let colours = Colours {
        track,
        thumb: tokens.background,
        label: tokens.foreground,
    };

    if !disabled {
        return colours;
    }
    Colours {
        track: fade(colours.track, 0.5),
        thumb: colours.thumb,
        label: fade(colours.label, 0.5),
    }
}

/// The iced toggler style for a status.
pub fn style(tokens: &Tokens, status: Status) -> widget::toggler::Style {
    let colours = colours(tokens, status);
    widget::toggler::Style {
        background: Background::Color(colours.track),
        background_border_width: 0.0,
        background_border_color: Color::TRANSPARENT,
        foreground: Background::Color(colours.thumb),
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: Some(colours.label),
        border_radius: None,
        padding_ratio: 0.1,
    }
}

/// What a switch keyboard shortcut does. Useful when the app tracks
/// which switch is current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Toggle,
}

impl Action {
    /// The value to send through the switch's `on_toggle` message.
    pub fn apply(self, is_on: bool) -> bool {
        match self {
            Action::Toggle => !is_on,
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Toggle];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        "Toggle"
    }

    fn description(self) -> &'static str {
        "Turns the current switch on or off."
    }
}

/// The default switch shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `Space` | [`Action::Toggle`] |
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new().bind(Chord::named(Named::Space), Action::Toggle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn toggle_flips_the_value() {
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
        let custom = keymap
            .unbind(&Chord::named(Named::Space))
            .bind("T".parse().unwrap(), Action::Toggle);
        assert_eq!(custom.resolve(&space, Default::default()), None);
        assert_eq!(
            custom.resolve(
                &iced::keyboard::Key::Character("t".into()),
                Default::default()
            ),
            Some(Action::Toggle)
        );
    }

    fn statuses(is_toggled: bool) -> [Status; 3] {
        [
            Status::Active { is_toggled },
            Status::Hovered { is_toggled },
            Status::Disabled { is_toggled },
        ]
    }

    #[test]
    fn default_builder_is_unlabelled_and_disabled() {
        let s: Switch<'_, ()> = switch(true);
        assert!(s.is_on());
        assert!(s.label.is_none());
        assert!(!s.is_enabled());
    }

    #[test]
    fn on_toggle_enables_and_maybe_none_disables() {
        assert!(switch::<bool>(false).on_toggle(|v| v).is_enabled());
        let s = switch::<bool>(false).on_toggle_maybe(None::<fn(bool) -> bool>);
        assert!(!s.is_enabled());
    }

    #[test]
    fn on_track_is_primary_and_off_track_is_not() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let on = colours(&tokens, Status::Active { is_toggled: true });
            let off = colours(&tokens, Status::Active { is_toggled: false });
            assert_eq!(on.track, tokens.primary);
            assert_ne!(off.track, tokens.primary);
            assert_eq!(on.thumb, tokens.background);
        }
    }

    #[test]
    fn hover_changes_the_track() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for is_toggled in [false, true] {
                let active = colours(&tokens, Status::Active { is_toggled });
                let hovered = colours(&tokens, Status::Hovered { is_toggled });
                assert_ne!(active.track, hovered.track);
            }
        }
    }

    #[test]
    fn disabled_fades_track_and_label() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for is_toggled in [false, true] {
                let active = colours(&tokens, Status::Active { is_toggled });
                let disabled = colours(&tokens, Status::Disabled { is_toggled });
                assert!((disabled.track.a - active.track.a * 0.5).abs() < 1e-6);
                assert!((disabled.label.a - active.label.a * 0.5).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn every_status_has_a_round_track_and_label_colour() {
        let tokens = Tokens::of(&dark());
        for status in statuses(true).into_iter().chain(statuses(false)) {
            let style = style(&tokens, status);
            assert!(style.border_radius.is_none());
            assert!(style.text_color.is_some());
        }
    }
}
