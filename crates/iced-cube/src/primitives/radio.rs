//! Radio buttons for picking one option from a small set.

use iced::widget::{self, button::Status, column, container, mouse_area, row, space, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme, mouse};

use crate::theme::{Tokens, fade, mix, radius, space as spacing, text_size};

/// Diameter of the radio circle in logical pixels.
pub const CIRCLE_SIZE: f32 = 16.0;

const DOT_SIZE: f32 = 8.0;

/// How a [`RadioGroup`] lays out its options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    #[default]
    Vertical,
    Horizontal,
}

impl Direction {
    pub const ALL: [Direction; 2] = [Direction::Vertical, Direction::Horizontal];
}

/// A single radio button. Convert it into an [`Element`] to render.
///
/// A radio without a message is rendered disabled.
#[derive(Debug)]
pub struct Radio<'a, V, Message> {
    label: text::Fragment<'a>,
    value: V,
    is_selected: bool,
    on_select: Option<Message>,
}

/// Creates a radio for `value`, selected when `selected` holds the same value.
pub fn radio<'a, V, Message>(
    label: impl text::IntoFragment<'a>,
    value: V,
    selected: Option<V>,
) -> Radio<'a, V, Message>
where
    V: Copy + Eq,
{
    Radio {
        label: label.into_fragment(),
        value,
        is_selected: selected == Some(value),
        on_select: None,
    }
}

impl<'a, V: Copy, Message> Radio<'a, V, Message> {
    /// Sets the message for a click, built from this radio's value.
    pub fn on_select(mut self, on_select: impl FnOnce(V) -> Message) -> Self {
        self.on_select = Some(on_select(self.value));
        self
    }

    pub fn is_selected(&self) -> bool {
        self.is_selected
    }

    pub fn is_enabled(&self) -> bool {
        self.on_select.is_some()
    }
}

impl<'a, V, Message: Clone + 'a> From<Radio<'a, V, Message>> for Element<'a, Message> {
    fn from(radio: Radio<'a, V, Message>) -> Self {
        let selected = radio.is_selected;
        let resting = if radio.on_select.is_some() {
            Status::Active
        } else {
            Status::Disabled
        };

        let dot: Element<'a, Message> = if selected {
            container(space())
                .width(DOT_SIZE)
                .height(DOT_SIZE)
                .style(move |theme: &Theme| container::Style {
                    background: Some(Background::Color(
                        colours(&Tokens::of(theme), selected, resting).dot,
                    )),
                    border: Border::default().rounded(radius::FULL),
                    ..container::Style::default()
                })
                .into()
        } else {
            space().into()
        };

        let circle = widget::button(container(dot).center(Length::Fill))
            .width(CIRCLE_SIZE)
            .height(CIRCLE_SIZE)
            .padding(0)
            .on_press_maybe(radio.on_select.clone())
            .style(move |theme, status| style(&Tokens::of(theme), selected, status));

        let label = text(radio.label)
            .size(text_size::SM)
            .style(move |theme: &Theme| text::Style {
                color: Some(colours(&Tokens::of(theme), selected, resting).label),
            });
        let label = match radio.on_select {
            Some(message) => mouse_area(label)
                .on_press(message)
                .interaction(mouse::Interaction::Pointer),
            None => mouse_area(label),
        };

        row![circle, label]
            .spacing(spacing::SM)
            .align_y(Alignment::Center)
            .into()
    }
}

/// A set of radios for the options of one value.
///
/// A group without a message renders every radio disabled.
pub struct RadioGroup<'a, V, Message> {
    options: Vec<V>,
    selected: Option<V>,
    direction: Direction,
    on_select: Option<Box<dyn Fn(V) -> Message + 'a>>,
}

impl<V: std::fmt::Debug, Message> std::fmt::Debug for RadioGroup<'_, V, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioGroup")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("direction", &self.direction)
            .field("enabled", &self.on_select.is_some())
            .finish()
    }
}

/// Creates a group with one radio per option, labelled with its `Display` text.
pub fn radio_group<'a, V, Message>(
    options: impl IntoIterator<Item = V>,
    selected: Option<V>,
) -> RadioGroup<'a, V, Message>
where
    V: Copy + Eq + ToString,
{
    RadioGroup {
        options: options.into_iter().collect(),
        selected,
        direction: Direction::default(),
        on_select: None,
    }
}

impl<'a, V, Message> RadioGroup<'a, V, Message> {
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Sets the message emitted with the option that was clicked.
    pub fn on_select(mut self, on_select: impl Fn(V) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.on_select.is_some()
    }
}

impl<'a, V, Message> From<RadioGroup<'a, V, Message>> for Element<'a, Message>
where
    V: Copy + Eq + ToString + 'a,
    Message: Clone + 'a,
{
    fn from(group: RadioGroup<'a, V, Message>) -> Self {
        let on_select = group.on_select;
        let radios = group.options.into_iter().map(|value| {
            let mut radio = radio(value.to_string(), value, group.selected);
            radio.on_select = on_select.as_ref().map(|f| f(value));
            Element::from(radio)
        });

        match group.direction {
            Direction::Vertical => column(radios).spacing(spacing::MD).into(),
            Direction::Horizontal => row(radios)
                .spacing(spacing::XL)
                .align_y(Alignment::Center)
                .wrap()
                .into(),
        }
    }
}

/// Colours of a radio for its selection and interaction status.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub background: Color,
    pub border: Color,
    pub dot: Color,
    pub label: Color,
}

/// Resolves the colours of a radio.
pub fn colours(tokens: &Tokens, selected: bool, status: Status) -> Colours {
    let hover = matches!(status, Status::Hovered | Status::Pressed);
    let pressed = status == Status::Pressed;

    let border = match (selected, hover) {
        (true, _) => tokens.primary,
        (false, true) => mix(tokens.border, tokens.foreground, 0.6),
        (false, false) => mix(tokens.border, tokens.foreground, 0.3),
    };
    let colours = Colours {
        background: if pressed {
            tokens.accent
        } else {
            Color::TRANSPARENT
        },
        border,
        dot: tokens.primary,
        label: tokens.foreground,
    };

    if status != Status::Disabled {
        return colours;
    }
    Colours {
        background: fade(colours.background, 0.5),
        border: fade(colours.border, 0.5),
        dot: fade(colours.dot, 0.5),
        label: fade(colours.label, 0.5),
    }
}

/// The iced button style used for the radio circle.
pub fn style(tokens: &Tokens, selected: bool, status: Status) -> widget::button::Style {
    let colours = colours(tokens, selected, status);
    widget::button::Style {
        background: Some(Background::Color(colours.background)),
        text_color: colours.label,
        border: Border {
            color: colours.border,
            width: 1.0,
            radius: radius::FULL.into(),
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
    fn radio_is_selected_only_for_its_own_value() {
        let r: Radio<'_, u8, ()> = radio("One", 1, Some(1));
        assert!(r.is_selected());
        let r: Radio<'_, u8, ()> = radio("One", 1, Some(2));
        assert!(!r.is_selected());
        let r: Radio<'_, u8, ()> = radio("One", 1, None);
        assert!(!r.is_selected());
    }

    #[test]
    fn radio_without_message_is_disabled() {
        let r: Radio<'_, u8, u8> = radio("One", 1, None);
        assert!(!r.is_enabled());
        assert_eq!(r.on_select(|v| v * 10).on_select, Some(10));
    }

    #[test]
    fn group_defaults_to_vertical_and_disabled() {
        let g: RadioGroup<'_, u8, ()> = radio_group([1, 2, 3], None);
        assert_eq!(g.direction, Direction::Vertical);
        assert_eq!(g.options, vec![1, 2, 3]);
        assert!(!g.is_enabled());
        assert!(g.on_select(|_| ()).is_enabled());
    }

    #[test]
    fn selected_border_uses_primary_in_every_enabled_status() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in [Status::Active, Status::Hovered, Status::Pressed] {
                assert_eq!(colours(&tokens, true, status).border, tokens.primary);
            }
        }
    }

    #[test]
    fn hover_strengthens_the_unselected_border() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let active = colours(&tokens, false, Status::Active);
            let hovered = colours(&tokens, false, Status::Hovered);
            assert_ne!(active.border, hovered.border);
        }
    }

    #[test]
    fn pressing_fills_the_background() {
        let tokens = Tokens::of(&light());
        for selected in [false, true] {
            assert_eq!(
                colours(&tokens, selected, Status::Pressed).background,
                tokens.accent
            );
        }
    }

    #[test]
    fn disabled_halves_alpha() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for selected in [false, true] {
                let active = colours(&tokens, selected, Status::Active);
                let disabled = colours(&tokens, selected, Status::Disabled);
                assert!((disabled.dot.a - active.dot.a * 0.5).abs() < 1e-6);
                assert!((disabled.label.a - active.label.a * 0.5).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn every_style_is_a_circle() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for selected in [false, true] {
                for status in STATES {
                    let style = style(&tokens, selected, status);
                    assert_eq!(style.border.radius, radius::FULL.into());
                    assert_eq!(style.border.width, 1.0);
                }
            }
        }
    }
}
