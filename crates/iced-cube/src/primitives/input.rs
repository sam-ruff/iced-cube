//! Single-line text inputs with an optional leading icon.
//!
//! Besides iced's own editing keys (select all, copy, cut, paste, moving by
//! word, Home and End), inputs undo and redo, delete a word with
//! Ctrl+Backspace and Ctrl+Delete, and keep Escape to themselves: see
//! [`Shortcut`] and [`default_keymap`]. Pass a [`Shortcuts`] set to
//! [`Input::shortcuts`] to switch some or all of them off, and a keymap to
//! [`Input::keymap`] to rebind them. Textareas and every component with a
//! search or filter field take the same set.
//!
//! ```no_run
//! use iced_cube::input;
//! use iced_cube::primitives::input::{Shortcut, Shortcuts};
//!
//! #[derive(Debug, Clone)]
//! enum Message { Name(String) }
//!
//! // The app has its own undo, so the field leaves Ctrl+Z and Ctrl+Y alone.
//! let view: iced::Element<'_, Message> = input("Name", "Ada")
//!     .shortcuts(Shortcuts::all().without(Shortcut::Undo).without(Shortcut::Redo))
//!     .on_input(Message::Name)
//!     .into();
//! ```

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use iced::widget::text::LineHeight;
use iced::widget::text_input::{Status, Style};
use iced::widget::{self, Id, container, stack, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding};

pub use crate::editing::{Shortcut, Shortcuts, default_keymap};

use crate::editing::{self, Editing, Limit};
use crate::icon::{Glyph, opacity, themed};
use crate::keys::Keymap;
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
    shortcuts: Shortcuts,
    keymap: Keymap<Shortcut>,
    on_clear: Option<Message>,
    max_length: Option<usize>,
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
        shortcuts: Shortcuts::all(),
        keymap: default_keymap(),
        on_clear: None,
        max_length: None,
    }
}

/// The id of the clear button of a [clearable](Input::clearable) input,
/// to find or press it in tests. Every clearable input shares it.
pub fn clear_id() -> Id {
    Id::new("iced-cube-input-clear")
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

    /// Sets which [`Shortcut`]s the input handles. All of them by default;
    /// pass [`Shortcuts::none`] to turn them all off. The keys of a shortcut
    /// that is off reach the app, and iced's own editing keys always work.
    pub fn shortcuts(mut self, shortcuts: Shortcuts) -> Self {
        self.shortcuts = shortcuts;
        self
    }

    /// Replaces the [`default_keymap`], to bind the shortcuts to other keys.
    pub fn keymap(mut self, keymap: Keymap<Shortcut>) -> Self {
        self.keymap = keymap;
        self
    }

    /// Draws an x button at the end of the field while it holds text and is
    /// enabled. Pressing it emits `message`; the app empties the value.
    pub fn clearable(mut self, message: Message) -> Self {
        self.on_clear = Some(message);
        self
    }

    /// Stops edits from growing the value past `max_length` characters.
    /// Typed text that does not fit is dropped and pasted text is cut short.
    /// A longer value set by the app is shown as it is and can only shrink.
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_input.is_some()
    }

    /// Whether the clear button shows: the input is clearable, enabled and
    /// not empty.
    pub fn shows_clear(&self) -> bool {
        self.on_clear.is_some() && self.is_enabled() && !self.value.is_empty()
    }
}

/// Builds an iced text input from `field` with `on_input` wired up and the
/// editing shortcuts around it.
#[cfg(any(feature = "combobox", feature = "command", feature = "select"))]
pub(crate) fn with_shortcuts<'a, Message: Clone + 'a>(
    field: text_input::TextInput<'a, Message>,
    value: &str,
    secure: bool,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    keymap: Keymap<Shortcut>,
    shortcuts: Shortcuts,
) -> Element<'a, Message> {
    limited(field, value, secure, on_input, keymap, shortcuts, None)
}

/// Builds the field with its editing shortcuts, keeping the value within
/// `max_length` characters when it is set.
fn limited<'a, Message: Clone + 'a>(
    field: text_input::TextInput<'a, Message>,
    value: &str,
    secure: bool,
    on_input: Option<Box<dyn Fn(String) -> Message + 'a>>,
    keymap: Keymap<Shortcut>,
    shortcuts: Shortcuts,
    max_length: Option<usize>,
) -> Element<'a, Message> {
    let Some(on_input) = on_input else {
        return field.into();
    };
    let on_input: Rc<dyn Fn(String) -> Message + 'a> = Rc::from(on_input);
    let edited = Rc::new(Cell::new(false));
    let limit = max_length.map(|max| Rc::new(Limit::new(max, value)));
    let field = field.on_input({
        let (on_input, edited, limit) = (Rc::clone(&on_input), Rc::clone(&edited), limit.clone());
        move |value| {
            edited.set(true);
            match &limit {
                Some(limit) => on_input(limit.apply(value)),
                None => on_input(value),
            }
        }
    });
    Editing::new(
        field,
        editing::Input::new(value, secure, on_input).limit(limit),
        keymap,
        shortcuts,
        edited,
    )
    .into()
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
            .field("shortcuts", &self.shortcuts)
            .field("keymap", &self.keymap)
            .field("on_clear", &self.on_clear)
            .field("max_length", &self.max_length)
            .finish()
    }
}

impl<'a, Message: Clone + 'a> From<Input<'a, Message>> for Element<'a, Message> {
    fn from(input: Input<'a, Message>) -> Self {
        let metrics = input.size.metrics();
        let invalid = input.invalid;
        let enabled = input.is_enabled();
        let clearable = input.on_clear.is_some() && enabled;
        let clear = input
            .shows_clear()
            .then_some(input.on_clear.clone())
            .flatten();

        let mut padding = metrics.padding(input.icon.is_some());
        // The room stays while the field is empty, so the text does not
        // reflow when the button appears.
        if clearable {
            padding.right = metrics.padding_x + metrics.icon + space::SM;
        }
        let mut field = text_input(&input.placeholder, &input.value)
            .secure(input.secure)
            .size(metrics.text)
            .line_height(LineHeight::Relative(LINE_HEIGHT))
            .padding(padding)
            .width(Length::Fill)
            .on_submit_maybe(input.on_submit.filter(|_| enabled))
            .style(move |theme, status| style(&Tokens::of(theme), status, invalid));
        if let Some(id) = input.id {
            field = field.id(id);
        }
        let field = limited(
            field,
            &input.value,
            input.secure,
            input.on_input,
            input.keymap,
            input.shortcuts,
            input.max_length,
        );

        // A clearable field keeps the same widget tree whether or not the
        // button shows, so it stays focused as the first character is typed.
        if input.icon.is_none() && !clearable {
            return container(field).width(input.width).into();
        }

        let status = if enabled {
            Status::Active
        } else {
            Status::Disabled
        };
        let icon = input.icon.map(|glyph| {
            let icon = themed(glyph, metrics.icon, opacity(enabled), move |theme| {
                style(&Tokens::of(theme), status, invalid).icon
            });
            container(icon)
                .padding(Padding::ZERO.left(metrics.padding_x))
                .height(Length::Fill)
                .align_y(Alignment::Center)
        });
        let clear = clear.map(|message| clear_button(metrics, message));

        stack![field]
            .push(icon)
            .push(clear)
            .width(input.width)
            .into()
    }
}

/// Padding around the clear button's icon.
const CLEAR_PADDING: f32 = 2.0;

/// The x button at the end of a clearable input, lined up with the text
/// padding.
fn clear_button<'a, Message: Clone + 'a>(
    metrics: Metrics,
    message: Message,
) -> Element<'a, Message> {
    let icon = themed(crate::lucide!(X), metrics.icon, 1.0, |theme| {
        Tokens::of(theme).muted_foreground
    });
    let button = widget::button(icon)
        .padding(CLEAR_PADDING)
        .on_press(message)
        .style(|theme, status| clear_style(&Tokens::of(theme), status));
    container(container(button).id(clear_id()))
        .padding(Padding::ZERO.right(metrics.padding_x - CLEAR_PADDING))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .align_y(Alignment::Center)
        .into()
}

/// The clear button: bare at rest, on the accent surface under the pointer.
pub fn clear_style(tokens: &Tokens, status: widget::button::Status) -> widget::button::Style {
    use widget::button::Status as Button;
    let background = matches!(status, Button::Hovered | Button::Pressed)
        .then_some(Background::Color(tokens.accent));
    widget::button::Style {
        background,
        text_color: tokens.muted_foreground,
        border: Border {
            radius: radius::SM.into(),
            ..Border::default()
        },
        ..widget::button::Style::default()
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

    // Transparent so the field takes the colour of whatever surface it sits
    // on, such as a dialog or popover.
    let active = Style {
        background: Background::Color(Color::TRANSPARENT),
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
        background: Background::Color(tokens.disabled_field()),
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
        assert_eq!(i.shortcuts, Shortcuts::all());
        assert_eq!(i.keymap, default_keymap());
        assert!(i.on_clear.is_none() && i.max_length.is_none());
        assert!(!i.shows_clear());
    }

    #[test]
    fn the_clear_button_shows_only_for_an_enabled_field_with_text() {
        let shows = |value: &str, enabled: bool| {
            input::<String>("", value)
                .clearable(String::new())
                .on_input_maybe(enabled.then_some(|s| s))
                .shows_clear()
        };
        assert!(shows("text", true));
        assert!(!shows("", true));
        assert!(!shows("text", false));
        assert!(!input::<String>("", "text").on_input(|s| s).shows_clear());
    }

    #[test]
    fn max_length_is_kept() {
        let i: Input<'_, ()> = input("", "").max_length(8);
        assert_eq!(i.max_length, Some(8));
    }

    #[test]
    fn the_clear_button_highlights_under_the_pointer() {
        use widget::button::Status as Button;
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(clear_style(&tokens, Button::Active).background, None);
            for status in [Button::Hovered, Button::Pressed] {
                assert_eq!(
                    clear_style(&tokens, status).background,
                    Some(Background::Color(tokens.accent))
                );
            }
            assert_eq!(
                clear_style(&tokens, Button::Active).text_color,
                tokens.muted_foreground
            );
        }
    }

    #[test]
    fn shortcuts_and_keymap_can_be_changed() {
        let i: Input<'_, ()> = input("", "")
            .shortcuts(Shortcuts::none())
            .keymap(Keymap::new());
        assert_eq!(i.shortcuts, Shortcuts::none());
        assert!(i.keymap.is_empty());
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
            assert_eq!(
                disabled.background,
                Background::Color(tokens.disabled_field())
            );
        }
    }

    #[test]
    fn enabled_fields_take_the_colour_of_their_surface() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in STATES.into_iter().filter(|s| *s != Status::Disabled) {
                for invalid in [false, true] {
                    let style = style(&tokens, status, invalid);
                    assert_eq!(style.background, Background::Color(Color::TRANSPARENT));
                }
            }
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
