//! Labels, and fields that stack a label, a control and helper text.
//!
//! ```no_run
//! use iced_cube::{field, input};
//!
//! #[derive(Debug, Clone)]
//! enum Message { Email(String) }
//!
//! let email = "";
//! let view: iced::Element<'_, Message> = field("Email", input("you@example.com", email).on_input(Message::Email))
//!     .description("We never share it.")
//!     .required(true)
//!     .into();
//! ```

use std::fmt;

use iced::widget::{Column, column, row, text};
use iced::{Color, Element, Length};

use crate::theme::{Tokens, fade, space, text_size};

/// Tone of a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum State {
    #[default]
    Normal,
    Disabled,
    Invalid,
}

impl State {
    pub const ALL: [State; 3] = [State::Normal, State::Disabled, State::Invalid];
}

/// A label builder. Convert it into an [`Element`] to render.
#[derive(Debug)]
pub struct Label<'a> {
    text: text::Fragment<'a>,
    state: State,
    required: bool,
}

/// Creates a label naming a control.
pub fn label<'a>(text: impl text::IntoFragment<'a>) -> Label<'a> {
    Label {
        text: text.into_fragment(),
        state: State::default(),
        required: false,
    }
}

impl<'a> Label<'a> {
    pub fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }

    /// Mutes the label to match a disabled control.
    pub fn disabled(self, disabled: bool) -> Self {
        self.toggle(State::Disabled, disabled)
    }

    /// Colours the label to match an invalid control.
    pub fn invalid(self, invalid: bool) -> Self {
        self.toggle(State::Invalid, invalid)
    }

    /// Appends a required marker.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    fn toggle(mut self, state: State, on: bool) -> Self {
        if on {
            self.state = state;
        } else if self.state == state {
            self.state = State::Normal;
        }
        self
    }
}

impl<'a, Message: 'a> From<Label<'a>> for Element<'a, Message> {
    fn from(label: Label<'a>) -> Self {
        let state = label.state;
        let name = text(label.text)
            .size(text_size::SM)
            .style(move |theme| text::Style {
                color: Some(colour(&Tokens::of(theme), state)),
            });

        if !label.required {
            return name.into();
        }

        let marker = text("*")
            .size(text_size::SM)
            .style(move |theme| text::Style {
                color: Some(required_colour(&Tokens::of(theme), state)),
            });
        row![name, marker].spacing(2).into()
    }
}

/// A labelled control with optional description and error text.
pub struct Field<'a, Message> {
    label: Label<'a>,
    control: Element<'a, Message>,
    description: Option<text::Fragment<'a>>,
    error: Option<text::Fragment<'a>>,
    width: Length,
}

impl<Message> fmt::Debug for Field<'_, Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Field")
            .field("label", &self.label)
            .field("description", &self.description)
            .field("error", &self.error)
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

/// Stacks a label above `control`. Add helper text with
/// [`Field::description`] and [`Field::error`].
pub fn field<'a, Message>(
    label_text: impl text::IntoFragment<'a>,
    control: impl Into<Element<'a, Message>>,
) -> Field<'a, Message> {
    Field {
        label: label(label_text),
        control: control.into(),
        description: None,
        error: None,
        width: Length::Fill,
    }
}

impl<'a, Message> Field<'a, Message> {
    /// Adds muted helper text below the control.
    pub fn description(mut self, description: impl text::IntoFragment<'a>) -> Self {
        self.description = Some(description.into_fragment());
        self
    }

    /// Shows an error below the control and colours the label to match.
    pub fn error(mut self, error: impl text::IntoFragment<'a>) -> Self {
        self.error = Some(error.into_fragment());
        self
    }

    pub fn error_maybe(mut self, error: Option<impl text::IntoFragment<'a>>) -> Self {
        self.error = error.map(text::IntoFragment::into_fragment);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.label = self.label.required(required);
        self
    }

    /// Mutes the label to match a disabled control.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.label = self.label.disabled(disabled);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
}

impl<'a, Message: 'a> From<Field<'a, Message>> for Element<'a, Message> {
    fn from(field: Field<'a, Message>) -> Self {
        let invalid = field.error.is_some();
        let label = if invalid {
            field.label.state(State::Invalid)
        } else {
            field.label
        };

        let helper = |content: text::Fragment<'a>, tone: fn(&Tokens) -> Color| {
            text(content)
                .size(text_size::XS)
                .style(move |theme| text::Style {
                    color: Some(tone(&Tokens::of(theme))),
                })
        };

        let content: Column<'a, Message> = column![label, field.control]
            .push(field.description.map(|d| helper(d, description_colour)))
            .push(field.error.map(|e| helper(e, error_colour)))
            .spacing(space::SM)
            .width(field.width);
        content.into()
    }
}

/// Label colour for a state.
pub fn colour(tokens: &Tokens, state: State) -> Color {
    match state {
        State::Normal => tokens.foreground,
        State::Disabled => fade(tokens.foreground, 0.5),
        State::Invalid => tokens.destructive,
    }
}

/// Colour of the required marker.
pub fn required_colour(tokens: &Tokens, state: State) -> Color {
    match state {
        State::Disabled => fade(tokens.destructive, 0.5),
        State::Normal | State::Invalid => tokens.destructive,
    }
}

/// Colour of a field's description.
pub fn description_colour(tokens: &Tokens) -> Color {
    tokens.muted_foreground
}

/// Colour of a field's error message.
pub fn error_colour(tokens: &Tokens) -> Color {
    tokens.destructive
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};
    use iced::widget::Space;

    fn control<'a>() -> Element<'a, ()> {
        Space::new().into()
    }

    #[test]
    fn default_label_is_normal_and_optional() {
        let l = label("Name");
        assert_eq!(l.state, State::Normal);
        assert!(!l.required);
        assert_eq!(&*l.text, "Name");
    }

    #[test]
    fn toggles_set_and_clear_their_own_state() {
        assert_eq!(label("x").disabled(true).state, State::Disabled);
        assert_eq!(label("x").invalid(true).state, State::Invalid);
        assert_eq!(
            label("x").disabled(true).disabled(false).state,
            State::Normal
        );
        assert_eq!(
            label("x").invalid(true).disabled(false).state,
            State::Invalid
        );
    }

    #[test]
    fn states_have_distinct_colours_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let colours = State::ALL.map(|s| colour(&tokens, s));
            assert_ne!(colours[0], colours[1]);
            assert_ne!(colours[0], colours[2]);
            assert_ne!(colours[1], colours[2]);
            assert!((colours[1].a - colours[0].a * 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn required_marker_is_destructive_and_fades_when_disabled() {
        let tokens = Tokens::of(&dark());
        assert_eq!(required_colour(&tokens, State::Normal), tokens.destructive);
        assert_eq!(required_colour(&tokens, State::Invalid), tokens.destructive);
        assert!(required_colour(&tokens, State::Disabled).a < tokens.destructive.a);
    }

    #[test]
    fn helper_text_colours_come_from_tokens() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(description_colour(&tokens), tokens.muted_foreground);
            assert_eq!(error_colour(&tokens), tokens.destructive);
        }
    }

    #[test]
    fn field_defaults_have_no_helper_text() {
        let f = field("Email", control());
        assert!(f.description.is_none());
        assert!(!f.has_error());
        assert_eq!(f.width, Length::Fill);
    }

    #[test]
    fn field_builder_forwards_to_label() {
        let f = field("Email", control()).required(true).disabled(true);
        assert!(f.label.required);
        assert_eq!(f.label.state, State::Disabled);
    }

    #[test]
    fn error_maybe_none_clears_the_error() {
        let f = field("Email", control())
            .error("Required")
            .error_maybe(None::<&str>);
        assert!(!f.has_error());
        let f = field("Email", control()).error_maybe(Some("Required"));
        assert!(f.has_error());
    }
}
