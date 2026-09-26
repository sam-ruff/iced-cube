//! Checkboxes with checked, unchecked and indeterminate states.

use iced::keyboard::key::Named;
use iced::widget::{self, button::Status, container, mouse_area, row, space, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme, mouse};

use crate::icon::{Glyph, tinted};
use crate::keys::{self, Chord, Keymap};
use crate::theme::{Tokens, fade, mix, radius, space as spacing, text_size};

/// Side length of the check box in logical pixels.
pub const BOX_SIZE: f32 = 16.0;

const ICON_SIZE: f32 = 14.0;

/// The value a checkbox shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CheckState {
    #[default]
    Unchecked,
    Checked,
    /// Partly checked, such as a "select all" box over a mixed selection.
    Indeterminate,
}

impl CheckState {
    pub const ALL: [CheckState; 3] = [
        CheckState::Unchecked,
        CheckState::Checked,
        CheckState::Indeterminate,
    ];

    /// The state after a click. Indeterminate boxes become checked.
    pub const fn toggled(self) -> Self {
        match self {
            CheckState::Checked => CheckState::Unchecked,
            CheckState::Unchecked | CheckState::Indeterminate => CheckState::Checked,
        }
    }

    pub const fn is_checked(self) -> bool {
        matches!(self, CheckState::Checked)
    }

    /// Whether the box is drawn filled, which is true when checked or indeterminate.
    pub const fn is_filled(self) -> bool {
        !matches!(self, CheckState::Unchecked)
    }

    fn glyph(self) -> Option<Glyph> {
        match self {
            CheckState::Unchecked => None,
            CheckState::Checked => Some(crate::lucide!(Check)),
            CheckState::Indeterminate => Some(crate::lucide!(Minus)),
        }
    }
}

impl From<bool> for CheckState {
    fn from(checked: bool) -> Self {
        if checked {
            CheckState::Checked
        } else {
            CheckState::Unchecked
        }
    }
}

/// A checkbox builder. Convert it into an [`Element`] to render.
///
/// A checkbox without a message is rendered disabled.
#[derive(Debug)]
pub struct Checkbox<'a, Message> {
    state: CheckState,
    label: Option<text::Fragment<'a>>,
    on_toggle: Option<Message>,
}

/// Creates a checkbox showing `state`. Accepts a `bool` or a [`CheckState`].
pub fn checkbox<'a, Message>(state: impl Into<CheckState>) -> Checkbox<'a, Message> {
    Checkbox {
        state: state.into(),
        label: None,
        on_toggle: None,
    }
}

impl<'a, Message> Checkbox<'a, Message> {
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Sets the message for a click. It receives the new checked value, so an
    /// indeterminate box reports `true`.
    pub fn on_toggle(mut self, on_toggle: impl FnOnce(bool) -> Message) -> Self {
        self.on_toggle = Some(on_toggle(self.state.toggled().is_checked()));
        self
    }

    /// Like [`Checkbox::on_toggle`], but `None` disables the checkbox.
    pub fn on_toggle_maybe(mut self, on_toggle: Option<impl FnOnce(bool) -> Message>) -> Self {
        let next = self.state.toggled().is_checked();
        self.on_toggle = on_toggle.map(|f| f(next));
        self
    }

    pub fn state(&self) -> CheckState {
        self.state
    }

    pub fn is_enabled(&self) -> bool {
        self.on_toggle.is_some()
    }
}

impl<'a, Message: Clone + 'a> From<Checkbox<'a, Message>> for Element<'a, Message> {
    fn from(checkbox: Checkbox<'a, Message>) -> Self {
        let state = checkbox.state;
        let resting = if checkbox.on_toggle.is_some() {
            Status::Active
        } else {
            Status::Disabled
        };

        let mark: Element<'a, Message> = match state.glyph() {
            Some(glyph) => tinted(glyph, ICON_SIZE, None)
                .style(move |theme: &Theme, _| svg::Style {
                    color: Some(colours(&Tokens::of(theme), state, resting).icon),
                })
                .into(),
            None => space().into(),
        };

        let indicator = widget::button(container(mark).center(Length::Fill))
            .width(BOX_SIZE)
            .height(BOX_SIZE)
            .padding(0)
            .on_press_maybe(checkbox.on_toggle.clone())
            .style(move |theme, status| style(&Tokens::of(theme), state, status));

        let Some(label) = checkbox.label else {
            return indicator.into();
        };

        let label = text(label)
            .size(text_size::SM)
            .style(move |theme: &Theme| text::Style {
                color: Some(colours(&Tokens::of(theme), state, resting).label),
            });
        let label = match checkbox.on_toggle {
            Some(message) => mouse_area(label)
                .on_press(message)
                .interaction(mouse::Interaction::Pointer),
            None => mouse_area(label),
        };

        row![indicator, label]
            .spacing(spacing::SM)
            .align_y(Alignment::Center)
            .into()
    }
}

/// Colours of a checkbox in a given state and interaction status.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub background: Color,
    pub border: Color,
    pub icon: Color,
    pub label: Color,
}

/// Resolves the colours of a checkbox for a value and interaction status.
pub fn colours(tokens: &Tokens, state: CheckState, status: Status) -> Colours {
    let hover = matches!(status, Status::Hovered | Status::Pressed);
    let pressed = status == Status::Pressed;

    let colours = if state.is_filled() {
        let target = if tokens.is_dark {
            Color::BLACK
        } else {
            Color::WHITE
        };
        let amount = match (hover, pressed) {
            (_, true) => 0.2,
            (true, false) => 0.1,
            _ => 0.0,
        };
        let fill = mix(tokens.primary, target, amount);
        Colours {
            background: fill,
            border: Color::TRANSPARENT,
            icon: tokens.primary_foreground,
            label: tokens.foreground,
        }
    } else {
        Colours {
            background: if pressed {
                tokens.accent
            } else {
                Color::TRANSPARENT
            },
            border: mix(
                tokens.border,
                tokens.foreground,
                if hover { 0.6 } else { 0.3 },
            ),
            icon: tokens.primary_foreground,
            label: tokens.foreground,
        }
    };

    if status != Status::Disabled {
        return colours;
    }
    Colours {
        background: fade(colours.background, 0.5),
        border: fade(colours.border, 0.5),
        icon: fade(colours.icon, 0.5),
        label: fade(colours.label, 0.5),
    }
}

/// The iced button style used for the check box itself.
pub fn style(tokens: &Tokens, state: CheckState, status: Status) -> widget::button::Style {
    let colours = colours(tokens, state, status);
    widget::button::Style {
        background: Some(Background::Color(colours.background)),
        text_color: colours.icon,
        border: Border {
            color: colours.border,
            width: 1.0,
            radius: radius::SM.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// What a checkbox keyboard shortcut does. Useful when the app tracks
/// which checkbox is current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Toggle,
}

impl Action {
    /// The checked value to send through the checkbox's `on_toggle`
    /// message, matching a click.
    pub fn apply(self, state: CheckState) -> bool {
        match self {
            Action::Toggle => state.toggled().is_checked(),
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
        "Checks or unchecks the current checkbox. An indeterminate box becomes checked."
    }
}

/// The default checkbox shortcuts:
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
    fn toggle_matches_a_click() {
        assert!(Action::Toggle.apply(CheckState::Unchecked));
        assert!(!Action::Toggle.apply(CheckState::Checked));
        assert!(Action::Toggle.apply(CheckState::Indeterminate));
    }

    #[test]
    fn default_keymap_and_overrides() {
        let keymap = default_keymap();
        assert_eq!(
            keymap.resolve(
                &iced::keyboard::Key::Named(Named::Space),
                Default::default()
            ),
            Some(Action::Toggle)
        );
        let custom = keymap
            .clear()
            .bind("Enter".parse().unwrap(), Action::Toggle);
        assert_eq!(custom.len(), 1);
        assert_eq!(
            custom.resolve(
                &iced::keyboard::Key::Named(Named::Enter),
                Default::default()
            ),
            Some(Action::Toggle)
        );
    }

    const STATES: [Status; 4] = [
        Status::Active,
        Status::Hovered,
        Status::Pressed,
        Status::Disabled,
    ];

    #[test]
    fn default_builder_is_unchecked_unlabelled_and_disabled() {
        let c: Checkbox<'_, ()> = checkbox(false);
        assert_eq!(c.state(), CheckState::Unchecked);
        assert!(c.label.is_none());
        assert!(!c.is_enabled());
    }

    #[test]
    fn bool_converts_to_two_states() {
        assert_eq!(CheckState::from(true), CheckState::Checked);
        assert_eq!(CheckState::from(false), CheckState::Unchecked);
    }

    #[test]
    fn toggling_cycles_between_checked_and_unchecked() {
        assert_eq!(CheckState::Unchecked.toggled(), CheckState::Checked);
        assert_eq!(CheckState::Checked.toggled(), CheckState::Unchecked);
        assert_eq!(CheckState::Indeterminate.toggled(), CheckState::Checked);
    }

    #[test]
    fn on_toggle_receives_the_next_value() {
        assert_eq!(checkbox(false).on_toggle(|v| v).on_toggle, Some(true));
        assert_eq!(checkbox(true).on_toggle(|v| v).on_toggle, Some(false));
        let mixed = checkbox(CheckState::Indeterminate).on_toggle(|v| v);
        assert_eq!(mixed.on_toggle, Some(true));
    }

    #[test]
    fn on_toggle_maybe_none_disables() {
        let c = checkbox(true).on_toggle_maybe(None::<fn(bool) -> bool>);
        assert!(!c.is_enabled());
    }

    #[test]
    fn only_filled_states_draw_a_mark() {
        assert!(CheckState::Unchecked.glyph().is_none());
        assert_eq!(CheckState::Checked.glyph().map(Glyph::name), Some("check"));
        assert_eq!(
            CheckState::Indeterminate.glyph().map(Glyph::name),
            Some("minus")
        );
    }

    #[test]
    fn filled_states_use_primary_and_unchecked_is_transparent() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let checked = colours(&tokens, CheckState::Checked, Status::Active);
            let mixed = colours(&tokens, CheckState::Indeterminate, Status::Active);
            let unchecked = colours(&tokens, CheckState::Unchecked, Status::Active);
            assert_eq!(checked.background, tokens.primary);
            assert_eq!(checked, mixed);
            assert_eq!(unchecked.background, Color::TRANSPARENT);
        }
    }

    #[test]
    fn hover_and_press_change_the_box_in_every_state() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for state in CheckState::ALL {
                let active = colours(&tokens, state, Status::Active);
                let hovered = colours(&tokens, state, Status::Hovered);
                let pressed = colours(&tokens, state, Status::Pressed);
                assert_ne!(active, hovered, "{state:?}");
                assert_ne!(hovered, pressed, "{state:?}");
            }
        }
    }

    #[test]
    fn disabled_halves_alpha_everywhere() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for state in CheckState::ALL {
                let active = colours(&tokens, state, Status::Active);
                let disabled = colours(&tokens, state, Status::Disabled);
                assert!((disabled.border.a - active.border.a * 0.5).abs() < 1e-6);
                assert!((disabled.label.a - active.label.a * 0.5).abs() < 1e-6);
                assert!((disabled.icon.a - active.icon.a * 0.5).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn every_style_has_a_rounded_one_pixel_border() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for state in CheckState::ALL {
                for status in STATES {
                    let style = style(&tokens, state, status);
                    assert_eq!(style.border.width, 1.0);
                    assert!(style.background.is_some());
                }
            }
        }
    }
}
