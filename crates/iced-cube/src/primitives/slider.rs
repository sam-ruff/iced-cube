//! Sliders for choosing a number within a range.

use std::ops::RangeInclusive;

use iced::keyboard::key::Named;
use iced::widget::slider::{Handle, HandleShape, Rail, Status};
use iced::widget::{self, column, row, space, text};
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::inert::inert;
use crate::keys::{self, Chord, Keymap};
use crate::theme::{Tokens, fade, mix, radius, space as spacing, text_size};

/// Radius of the handle in logical pixels.
pub const HANDLE_RADIUS: f32 = 8.0;

const RAIL_WIDTH: f32 = 6.0;

/// Numbers a slider can hold, such as `f32`, `f64`, `u8` or `i32`.
pub trait Value:
    Copy + PartialOrd + From<u8> + Into<f64> + num_traits::FromPrimitive + ToString
{
}

impl<T> Value for T where
    T: Copy + PartialOrd + From<u8> + Into<f64> + num_traits::FromPrimitive + ToString
{
}

/// A slider builder. Convert it into an [`Element`] to render.
///
/// A slider without a message is rendered disabled.
pub struct Slider<'a, T, Message> {
    range: RangeInclusive<T>,
    value: T,
    step: Option<T>,
    label: Option<text::Fragment<'a>>,
    format: Option<Box<dyn Fn(T) -> String + 'a>>,
    width: Length,
    on_change: Option<Box<dyn Fn(T) -> Message + 'a>>,
    on_release: Option<Message>,
}

impl<T: std::fmt::Debug, Message> std::fmt::Debug for Slider<'_, T, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slider")
            .field("range", &self.range)
            .field("value", &self.value)
            .field("step", &self.step)
            .field("label", &self.label)
            .field("shows_value", &self.format.is_some())
            .field("enabled", &self.on_change.is_some())
            .finish()
    }
}

/// Creates a slider over `range` showing `value`. The value is clamped to
/// the range, and a reversed range is treated as its forward equivalent.
pub fn slider<'a, T: Value, Message>(range: RangeInclusive<T>, value: T) -> Slider<'a, T, Message> {
    let range = normalise(range);
    Slider {
        value: clamp(value, &range),
        range,
        step: None,
        label: None,
        format: None,
        width: Length::Fill,
        on_change: None,
        on_release: None,
    }
}

impl<'a, T: Value, Message> Slider<'a, T, Message> {
    /// Sets the increment the value snaps to. Zero or negative steps are ignored.
    pub fn step(mut self, step: T) -> Self {
        self.step = valid_step(step, &self.range);
        self
    }

    /// Shows a label above the slider.
    pub fn label(mut self, label: impl text::IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Shows the current value above the slider, as plain text.
    pub fn show_value(self) -> Self {
        self.format_value(|value: T| value.to_string())
    }

    /// Shows the current value above the slider, formatted by `format`.
    pub fn format_value(mut self, format: impl Fn(T) -> String + 'a) -> Self {
        self.format = Some(Box::new(format));
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the message for every change while dragging.
    pub fn on_change(mut self, on_change: impl Fn(T) -> Message + 'a) -> Self {
        self.on_change = Some(Box::new(on_change));
        self
    }

    /// Sets a message emitted once the handle is released.
    pub fn on_release(mut self, message: Message) -> Self {
        self.on_release = Some(message);
        self
    }

    pub fn value(&self) -> T {
        self.value
    }

    pub fn range(&self) -> &RangeInclusive<T> {
        &self.range
    }

    pub fn is_enabled(&self) -> bool {
        self.on_change.is_some()
    }
}

impl<'a, T, Message> From<Slider<'a, T, Message>> for Element<'a, Message>
where
    T: Value + 'a,
    Message: Clone + 'a,
{
    fn from(slider: Slider<'a, T, Message>) -> Self {
        let enabled = slider.is_enabled();
        let value = slider.value;
        let step = slider.step;

        let track: Element<'a, Message> = match slider.on_change {
            Some(on_change) => {
                let mut track = widget::slider(slider.range, value, on_change)
                    .height(HANDLE_RADIUS * 2.0)
                    .style(|theme, status| style(&Tokens::of(theme), status, true));
                if let Some(step) = step {
                    track = track.step(step);
                }
                if let Some(message) = slider.on_release {
                    track = track.on_release(message);
                }
                track.into()
            }
            None => {
                let mut track = widget::slider(slider.range, value, |_| ())
                    .height(HANDLE_RADIUS * 2.0)
                    .style(|theme, status| style(&Tokens::of(theme), status, false));
                if let Some(step) = step {
                    track = track.step(step);
                }
                inert(track.into())
            }
        };

        let shown = slider.format.map(|format| format(value));
        if slider.label.is_none() && shown.is_none() {
            return widget::container(track).width(slider.width).into();
        }

        let caption = move |content: text::Fragment<'a>, muted: bool| {
            text(content)
                .size(text_size::SM)
                .style(move |theme: &Theme| {
                    let tokens = Tokens::of(theme);
                    let colour = if muted {
                        tokens.muted_foreground
                    } else {
                        tokens.foreground
                    };
                    text::Style {
                        color: Some(if enabled { colour } else { fade(colour, 0.5) }),
                    }
                })
        };

        let mut header = row![].spacing(spacing::SM);
        if let Some(label) = slider.label {
            header = header.push(caption(label, false));
        }
        header = header.push(space().width(Length::Fill));
        if let Some(shown) = shown {
            header = header.push(caption(shown.into(), true));
        }

        column![header, track]
            .spacing(spacing::SM)
            .width(slider.width)
            .into()
    }
}

/// Orders the bounds of a range so the start is never above the end.
pub fn normalise<T: PartialOrd + Copy>(range: RangeInclusive<T>) -> RangeInclusive<T> {
    let (start, end) = (*range.start(), *range.end());
    if start > end { end..=start } else { range }
}

/// Clamps `value` into `range`.
pub fn clamp<T: PartialOrd + Copy>(value: T, range: &RangeInclusive<T>) -> T {
    if value < *range.start() {
        return *range.start();
    }
    if value > *range.end() {
        return *range.end();
    }
    value
}

/// Returns `step` if it is positive, capped at the width of `range`.
pub fn valid_step<T: Value>(step: T, range: &RangeInclusive<T>) -> Option<T> {
    if step <= T::from(0) {
        return None;
    }
    let span: f64 = (*range.end()).into() - (*range.start()).into();
    if span > 0.0 && step.into() > span {
        return T::from_f64(span);
    }
    Some(step)
}

/// Moves `value` by `steps` whole steps within `range`, landing on the
/// step grid that starts at the range's start. Without a valid `step` it
/// moves by 1, as the widget does. The range end is always reachable.
pub fn stepped<T: Value>(value: T, range: RangeInclusive<T>, step: Option<T>, steps: i32) -> T {
    let range = normalise(range);
    let size: f64 = step
        .and_then(|step| valid_step(step, &range))
        .unwrap_or_else(|| T::from(1))
        .into();
    let start: f64 = (*range.start()).into();
    let end: f64 = (*range.end()).into();
    let current: f64 = clamp(value, &range).into();
    if steps == 0 || size <= 0.0 {
        return clamp(value, &range);
    }

    // Snap an off-grid value to the grid point in the direction of travel.
    let position = (current - start) / size;
    let tolerance = 1e-9;
    let index = if steps > 0 {
        (position + tolerance).floor() + f64::from(steps)
    } else {
        (position - tolerance).ceil() + f64::from(steps)
    };
    let target = (start + index * size).clamp(start, end);
    T::from_f64(target).map_or(value, |target| clamp(target, &range))
}

/// What a slider keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Increase,
    Decrease,
    Min,
    Max,
}

impl Action {
    /// The value after this action, to send through the slider's
    /// `on_change` message. `step` is the one given to [`Slider::step`].
    pub fn apply<T: Value>(self, value: T, range: RangeInclusive<T>, step: Option<T>) -> T {
        let range = normalise(range);
        match self {
            Action::Increase => stepped(value, range, step, 1),
            Action::Decrease => stepped(value, range, step, -1),
            Action::Min => *range.start(),
            Action::Max => *range.end(),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[Action::Increase, Action::Decrease, Action::Min, Action::Max];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Increase => "Increase",
            Action::Decrease => "Decrease",
            Action::Min => "Min",
            Action::Max => "Max",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Increase => "Raises the value by one step.",
            Action::Decrease => "Lowers the value by one step.",
            Action::Min => "Sets the value to the start of the range.",
            Action::Max => "Sets the value to the end of the range.",
        }
    }
}

/// The default slider shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `ArrowRight`, `ArrowUp` | [`Action::Increase`] |
/// | `ArrowLeft`, `ArrowDown` | [`Action::Decrease`] |
/// | `Home` | [`Action::Min`] |
/// | `End` | [`Action::Max`] |
///
/// Shortcuts are app-wide, so an app with several sliders routes them to
/// the one it considers current.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::ArrowRight), Action::Increase)
        .bind(Chord::named(Named::ArrowUp), Action::Increase)
        .bind(Chord::named(Named::ArrowLeft), Action::Decrease)
        .bind(Chord::named(Named::ArrowDown), Action::Decrease)
        .bind(Chord::named(Named::Home), Action::Min)
        .bind(Chord::named(Named::End), Action::Max)
}

/// Rail and handle colours of a slider.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub filled: Color,
    pub empty: Color,
    pub handle: Color,
    pub handle_border: Color,
}

/// Resolves the colours of a slider for an interaction status.
pub fn colours(tokens: &Tokens, status: Status, enabled: bool) -> Colours {
    let handle_border = match status {
        Status::Active => tokens.primary,
        Status::Hovered => mix(tokens.primary, tokens.background, 0.15),
        Status::Dragged => mix(tokens.primary, tokens.background, 0.3),
    };
    let colours = Colours {
        filled: tokens.primary,
        empty: mix(tokens.muted, tokens.border, 0.5),
        handle: tokens.background,
        handle_border,
    };

    if enabled {
        return colours;
    }
    Colours {
        filled: fade(colours.filled, 0.5),
        empty: fade(colours.empty, 0.5),
        handle: colours.handle,
        handle_border: fade(tokens.primary, 0.5),
    }
}

/// The iced slider style for a status.
pub fn style(tokens: &Tokens, status: Status, enabled: bool) -> widget::slider::Style {
    let colours = colours(tokens, status, enabled);
    let border_width = match (enabled, status) {
        (true, Status::Hovered | Status::Dragged) => 2.5,
        _ => 1.5,
    };
    widget::slider::Style {
        rail: Rail {
            backgrounds: (
                Background::Color(colours.filled),
                Background::Color(colours.empty),
            ),
            width: RAIL_WIDTH,
            border: Border::default().rounded(radius::FULL),
        },
        handle: Handle {
            shape: HandleShape::Circle {
                radius: HANDLE_RADIUS,
            },
            background: Background::Color(colours.handle),
            border_width,
            border_color: colours.handle_border,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    const STATES: [Status; 3] = [Status::Active, Status::Hovered, Status::Dragged];

    #[test]
    fn default_builder_fills_width_and_is_disabled() {
        let s: Slider<'_, f32, ()> = slider(0.0..=1.0, 0.5);
        assert_eq!(s.width, Length::Fill);
        assert!(s.step.is_none());
        assert!(s.format.is_none());
        assert!(!s.is_enabled());
        assert!(s.on_change(|_| ()).is_enabled());
    }

    #[test]
    fn value_is_clamped_to_the_range() {
        let s: Slider<'_, i32, ()> = slider(0..=10, 42);
        assert_eq!(s.value(), 10);
        let s: Slider<'_, i32, ()> = slider(0..=10, -3);
        assert_eq!(s.value(), 0);
        let s: Slider<'_, i32, ()> = slider(0..=10, 7);
        assert_eq!(s.value(), 7);
    }

    #[test]
    fn reversed_range_is_normalised() {
        let s: Slider<'_, f64, ()> = slider(10.0..=0.0, 4.0);
        assert_eq!(s.range(), &(0.0..=10.0));
        assert_eq!(s.value(), 4.0);
    }

    #[test]
    fn non_positive_steps_are_ignored() {
        assert_eq!(valid_step(0.0_f32, &(0.0..=1.0)), None);
        assert_eq!(valid_step(-0.5_f32, &(0.0..=1.0)), None);
        assert_eq!(valid_step(0.25_f32, &(0.0..=1.0)), Some(0.25));
    }

    #[test]
    fn step_is_capped_at_the_range_width() {
        assert_eq!(valid_step(50_u8, &(0..=10)), Some(10));
        assert_eq!(valid_step(5_i32, &(3..=3)), Some(5));
    }

    #[test]
    fn builder_step_uses_validation() {
        let s: Slider<'_, f32, ()> = slider(0.0..=1.0, 0.5).step(0.0);
        assert!(s.step.is_none());
        let s: Slider<'_, f32, ()> = slider(0.0..=100.0, 0.5).step(5.0);
        assert_eq!(s.step, Some(5.0));
    }

    #[test]
    fn stepped_moves_along_the_grid() {
        assert_eq!(stepped(40_u8, 0..=100, None, 1), 41);
        assert_eq!(stepped(40_u8, 0..=100, Some(5), 1), 45);
        assert_eq!(stepped(40_u8, 0..=100, Some(5), -2), 30);
        assert_eq!(stepped(0.5_f32, 0.0..=1.0, Some(0.25), 1), 0.75);
        assert_eq!(stepped(7_i32, 0..=10, Some(2), 0), 7);
    }

    #[test]
    fn stepped_snaps_off_grid_values_in_the_direction_of_travel() {
        assert_eq!(stepped(42_u8, 0..=100, Some(5), 1), 45);
        assert_eq!(stepped(42_u8, 0..=100, Some(5), -1), 40);
        assert_eq!(stepped(12_i32, 10..=20, Some(4), 1), 14);
    }

    #[test]
    fn stepped_clamps_and_reaches_the_end() {
        assert_eq!(stepped(98_u8, 0..=100, Some(5), 1), 100);
        assert_eq!(stepped(100_u8, 0..=100, Some(5), 1), 100);
        assert_eq!(stepped(0_u8, 0..=100, Some(5), -1), 0);
        assert_eq!(stepped(9_i32, 0..=10, Some(4), 1), 10);
        assert_eq!(stepped(5_i32, RangeInclusive::new(10, 0), None, -1), 4);
    }

    #[test]
    fn stepped_ignores_invalid_steps() {
        assert_eq!(stepped(5_i32, 0..=10, Some(0), 1), 6);
        assert_eq!(stepped(5_i32, 0..=10, Some(-3), -1), 4);
    }

    #[test]
    fn actions_apply_to_the_value() {
        assert_eq!(Action::Increase.apply(40_u8, 0..=100, Some(10)), 50);
        assert_eq!(Action::Decrease.apply(40_u8, 0..=100, Some(10)), 30);
        assert_eq!(Action::Min.apply(40_u8, 20..=100, None), 20);
        assert_eq!(
            Action::Max.apply(40_u8, RangeInclusive::new(100, 20), None),
            100
        );
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let chord: Chord = chord.parse().unwrap();
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_and_overrides() {
        let keymap = default_keymap();
        assert_eq!(press(&keymap, "ArrowRight"), Some(Action::Increase));
        assert_eq!(press(&keymap, "ArrowUp"), Some(Action::Increase));
        assert_eq!(press(&keymap, "ArrowLeft"), Some(Action::Decrease));
        assert_eq!(press(&keymap, "ArrowDown"), Some(Action::Decrease));
        assert_eq!(press(&keymap, "Home"), Some(Action::Min));
        assert_eq!(press(&keymap, "End"), Some(Action::Max));

        let custom = keymap
            .unbind(&"ArrowUp".parse().unwrap())
            .bind("=".parse().unwrap(), Action::Increase);
        assert_eq!(press(&custom, "ArrowUp"), None);
        assert_eq!(press(&custom, "="), Some(Action::Increase));
        assert!(custom.clear().is_empty());
    }

    #[test]
    fn show_value_formats_with_display() {
        let s: Slider<'_, u8, ()> = slider(0..=100, 30).show_value();
        let shown = s.format.as_ref().map(|format| format(s.value));
        assert_eq!(shown.as_deref(), Some("30"));
    }

    #[test]
    fn rail_fills_with_primary() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in STATES {
                let colours = colours(&tokens, status, true);
                assert_eq!(colours.filled, tokens.primary);
                assert_ne!(colours.filled, colours.empty);
            }
        }
    }

    #[test]
    fn interaction_changes_the_handle() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let active = style(&tokens, Status::Active, true);
            let hovered = style(&tokens, Status::Hovered, true);
            let dragged = style(&tokens, Status::Dragged, true);
            assert_ne!(active.handle, hovered.handle);
            assert_ne!(hovered.handle, dragged.handle);
        }
    }

    #[test]
    fn disabled_fades_and_ignores_interaction() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let active = colours(&tokens, Status::Active, true);
            for status in STATES {
                let disabled = colours(&tokens, status, false);
                assert!((disabled.filled.a - active.filled.a * 0.5).abs() < 1e-6);
                assert_eq!(disabled, colours(&tokens, Status::Active, false));
            }
        }
    }
}
