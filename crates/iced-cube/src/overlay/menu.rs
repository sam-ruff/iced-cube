//! The look every menu and list shares: dropdown and context menus, the
//! command list, and the combobox and select lists.
//!
//! Rows are 32 pixels tall: 14 pixel text on a 20 pixel line, 6 pixels of
//! padding above and below and 8 at the sides, inset 4 pixels from the
//! surface edge. [`row_style`] resolves their colours for a [`RowStatus`].

use iced::widget::text::LineHeight;
use iced::widget::{Row, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Theme};

use crate::icon::{self, Glyph, themed};
use crate::theme::{Tokens, fade, radius, space, text_size};

/// Padding inside a row: 6 pixels above and below, 8 at the sides.
pub const ROW_PADDING: Padding = Padding {
    top: 6.0,
    bottom: 6.0,
    left: space::SM,
    right: space::SM,
};
/// Line height of a row's label.
pub const LINE_HEIGHT: f32 = 20.0;
/// Line height of a group label.
pub const LABEL_LINE_HEIGHT: f32 = 16.0;
/// Size of leading icons, check marks and chevrons.
pub const ICON_SIZE: f32 = 16.0;

/// How a row is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RowStatus {
    #[default]
    Idle,
    /// Under the pointer or the keyboard highlight.
    Highlighted,
    Disabled,
}

impl RowStatus {
    pub const ALL: [RowStatus; 3] = [RowStatus::Idle, RowStatus::Highlighted, RowStatus::Disabled];
}

/// The colours of one row. Icons are drawn with an opaque tint; a disabled
/// row fades them with [`icon::opacity`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStyle {
    pub background: Option<Color>,
    pub text: Color,
    /// The leading icon.
    pub icon: Color,
    /// The shortcut hint.
    pub hint: Color,
}

/// Resolves a row's colours. Destructive rows keep the destructive colour
/// for their text and icon, and highlight with a tint of it.
pub fn row_style(tokens: &Tokens, status: RowStatus, destructive: bool) -> RowStyle {
    match (status, destructive) {
        (RowStatus::Disabled, _) => RowStyle {
            background: None,
            text: tokens.muted_foreground,
            icon: tokens.muted_foreground,
            hint: fade(tokens.muted_foreground, 0.5),
        },
        (RowStatus::Idle, false) => RowStyle {
            background: None,
            text: tokens.foreground,
            icon: tokens.muted_foreground,
            hint: tokens.muted_foreground,
        },
        (RowStatus::Highlighted, false) => RowStyle {
            background: Some(tokens.accent),
            text: tokens.accent_foreground,
            icon: tokens.muted_foreground,
            hint: tokens.muted_foreground,
        },
        (RowStatus::Idle, true) => RowStyle {
            background: None,
            text: tokens.destructive,
            icon: tokens.destructive,
            hint: tokens.muted_foreground,
        },
        (RowStatus::Highlighted, true) => RowStyle {
            background: Some(fade(
                tokens.destructive,
                if tokens.is_dark { 0.2 } else { 0.1 },
            )),
            text: tokens.destructive,
            icon: tokens.destructive,
            hint: tokens.muted_foreground,
        },
    }
}

/// The container style of a row: its background, text colour and radius.
pub fn row_container_style(style: RowStyle) -> container::Style {
    container::Style {
        background: style.background.map(Background::Color),
        text_color: Some(style.text),
        border: Border {
            radius: radius::SM.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The group label style.
pub fn label_style(tokens: &Tokens) -> text::Style {
    text::Style {
        color: Some(tokens.muted_foreground),
    }
}

/// The separator line.
pub fn separator_style(tokens: &Tokens) -> container::Style {
    container::Style {
        background: Some(Background::Color(tokens.border)),
        ..container::Style::default()
    }
}

/// Space a label or row leaves for the check mark slot in a menu with
/// checkbox or radio items, so every label lines up.
#[cfg(any(feature = "command", feature = "dropdown-menu"))]
const INDICATOR_WIDTH: f32 = ICON_SIZE + space::SM;

/// What sits in front of a row's label, in menus with check marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Leading {
    None,
    #[cfg(feature = "dropdown-menu")]
    Empty,
    #[cfg(feature = "dropdown-menu")]
    Check,
    #[cfg(feature = "dropdown-menu")]
    Dot,
}

/// What sits at the end of a row, after any shortcut hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Trailing {
    None,
    /// The selected option in a single-choice list.
    #[cfg(any(feature = "combobox", feature = "select"))]
    Check,
    /// A row that opens a submenu.
    #[cfg(feature = "dropdown-menu")]
    Chevron,
}

/// The parts of a row, laid out by [`content`].
pub(crate) struct Parts<'a> {
    pub label: &'a str,
    pub icon: Option<Glyph>,
    pub hint: Option<&'a str>,
    pub leading: Leading,
    pub trailing: Trailing,
}

impl<'a> Parts<'a> {
    pub fn label(label: &'a str) -> Self {
        Self {
            label,
            icon: None,
            hint: None,
            leading: Leading::None,
            trailing: Trailing::None,
        }
    }
}

/// An icon tinted by a row's colours: `pick` chooses the colour.
fn glyph<'a, Message: 'a>(
    glyph: Glyph,
    colours: impl Fn(&Theme) -> RowStyle + 'a,
    pick: fn(&RowStyle) -> Color,
    opacity: f32,
) -> Element<'a, Message> {
    themed(glyph, ICON_SIZE, opacity, move |theme| {
        pick(&colours(theme))
    })
    .into()
}

/// A row's content, without its padding or background. The label takes
/// the text colour of whatever holds the row.
pub(crate) fn content<'a, Message: 'a>(
    parts: Parts<'_>,
    status: RowStatus,
    destructive: bool,
) -> Row<'a, Message> {
    let colours = move |theme: &Theme| row_style(&Tokens::of(theme), status, destructive);
    let opacity = icon::opacity(status != RowStatus::Disabled);

    let mut content = row![].spacing(space::SM).align_y(Alignment::Center);
    let leading: Option<Element<'a, Message>> = match parts.leading {
        Leading::None => None,
        #[cfg(feature = "dropdown-menu")]
        Leading::Empty => Some(iced::widget::space().into()),
        #[cfg(feature = "dropdown-menu")]
        Leading::Check => Some(glyph(
            crate::lucide!(Check),
            colours,
            |style| style.text,
            opacity,
        )),
        #[cfg(feature = "dropdown-menu")]
        Leading::Dot => Some(dot(colours)),
    };
    if let Some(leading) = leading {
        content = content.push(container(leading).center_x(ICON_SIZE).center_y(ICON_SIZE));
    }
    if let Some(icon) = parts.icon {
        content = content.push(glyph(icon, colours, |style| style.icon, opacity));
    }
    content = content.push(
        text(parts.label.to_owned())
            .size(text_size::SM)
            .line_height(LineHeight::Absolute(LINE_HEIGHT.into()))
            .width(Length::Fill),
    );
    if let Some(hint) = parts.hint {
        content = content.push(
            text(hint.to_owned())
                .size(text_size::XS)
                .style(move |theme| text::Style {
                    color: Some(colours(theme).hint),
                }),
        );
    }
    let trailing = match parts.trailing {
        Trailing::None => None,
        #[cfg(any(feature = "combobox", feature = "select"))]
        Trailing::Check => Some(crate::lucide!(Check)),
        #[cfg(feature = "dropdown-menu")]
        Trailing::Chevron => Some(crate::lucide!(ChevronRight)),
    };
    if let Some(trailing) = trailing {
        content = content.push(glyph(trailing, colours, |style| style.text, opacity));
    }
    content
}

/// The dot in front of a checked radio item.
#[cfg(feature = "dropdown-menu")]
fn dot<'a, Message: 'a>(colours: impl Fn(&Theme) -> RowStyle + 'a) -> Element<'a, Message> {
    container(iced::widget::space())
        .width(8)
        .height(8)
        .style(move |theme| container::Style {
            background: Some(Background::Color(colours(theme).text)),
            border: Border {
                radius: radius::FULL.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// A full row: the content with its padding and background, highlighted
/// when the pointer enters and activated when it is pressed. Without
/// messages the row is inert.
#[cfg(any(feature = "combobox", feature = "command", feature = "dropdown-menu"))]
pub(crate) fn item<'a, Message: Clone + 'a>(
    parts: Parts<'_>,
    status: RowStatus,
    destructive: bool,
    messages: Option<(Message, Message)>,
) -> Element<'a, Message> {
    let body = container(content(parts, status, destructive))
        .padding(ROW_PADDING)
        .width(Length::Fill)
        .style(move |theme| {
            row_container_style(row_style(&Tokens::of(theme), status, destructive))
        });
    let Some((highlight, activate)) = messages else {
        return inset(body);
    };
    inset(
        iced::widget::mouse_area(body)
            .on_enter(highlight)
            .on_press(activate)
            .interaction(iced::mouse::Interaction::Pointer),
    )
}

/// A heading for the rows below it. `indented` lines it up with the labels
/// of rows that have a check mark slot.
#[cfg(any(feature = "command", feature = "dropdown-menu"))]
pub(crate) fn group_label<'a, Message: 'a>(label: &str, indented: bool) -> Element<'a, Message> {
    let left = ROW_PADDING.left + if indented { INDICATOR_WIDTH } else { 0.0 };
    inset(
        container(
            text(label.to_owned())
                .size(text_size::XS)
                .line_height(LineHeight::Absolute(LABEL_LINE_HEIGHT.into()))
                .font(crate::theme::semibold())
                .style(|theme| label_style(&Tokens::of(theme))),
        )
        .padding(ROW_PADDING.left(left))
        .width(Length::Fill),
    )
}

/// A line between groups, running the full width of the surface.
#[cfg(any(feature = "command", feature = "dropdown-menu"))]
pub(crate) fn separator<'a, Message: 'a>() -> Element<'a, Message> {
    container(
        container(iced::widget::space())
            .width(Length::Fill)
            .height(1)
            .style(|theme| separator_style(&Tokens::of(theme))),
    )
    .padding([space::XS, 0.0])
    .into()
}

/// Muted text in place of rows, such as "No results found."
#[cfg(any(feature = "combobox", feature = "command"))]
pub(crate) fn empty<'a, Message: 'a>(message: &str) -> Element<'a, Message> {
    container(
        text(message.to_owned())
            .size(text_size::SM)
            .style(|theme| label_style(&Tokens::of(theme))),
    )
    .padding([space::XL, space::SM])
    .center_x(Length::Fill)
    .into()
}

/// Rows sit inside the surface's horizontal padding; separators do not, so
/// they run the full width.
pub(crate) fn inset<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(content)
        .padding([0.0, space::XS])
        .width(Length::Fill)
        .into()
}

/// The floating surface around a list of rows, with its vertical padding.
#[cfg(any(feature = "combobox", feature = "dropdown-menu", feature = "select"))]
pub(crate) fn surface<'a, Message: 'a>(
    rows: impl Into<Element<'a, Message>>,
) -> container::Container<'a, Message> {
    container(rows)
        .padding([space::XS, 0.0])
        .style(|theme| crate::overlay::anchored::surface_style(&Tokens::of(theme)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    #[test]
    fn row_metrics_add_up_to_32_pixels() {
        assert_eq!(ROW_PADDING.top + LINE_HEIGHT + ROW_PADDING.bottom, 32.0);
        assert_eq!(ROW_PADDING.left, 8.0);
        assert_eq!(ICON_SIZE, 16.0);
    }

    #[test]
    fn highlighted_rows_use_the_accent_in_both_themes() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let idle = row_style(&tokens, RowStatus::Idle, false);
            let highlighted = row_style(&tokens, RowStatus::Highlighted, false);
            assert_eq!(idle.background, None);
            assert_eq!(idle.text, tokens.foreground);
            assert_eq!(highlighted.background, Some(tokens.accent));
            assert_eq!(highlighted.text, tokens.accent_foreground);
            assert_eq!(idle.icon, tokens.muted_foreground);
            assert_eq!(idle.hint, tokens.muted_foreground);
        }
    }

    #[test]
    fn disabled_rows_are_muted_with_opaque_faded_icons() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for destructive in [false, true] {
                let style = row_style(&tokens, RowStatus::Disabled, destructive);
                assert_eq!(style.background, None);
                assert_eq!(style.text, tokens.muted_foreground);
                assert_eq!(style.icon.a, tokens.muted_foreground.a, "icons stay opaque");
                assert!(style.hint.a < tokens.muted_foreground.a);
            }
        }
    }

    #[test]
    fn destructive_rows_keep_their_colour_and_highlight_with_a_tint() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for status in [RowStatus::Idle, RowStatus::Highlighted] {
                let style = row_style(&tokens, status, true);
                assert_eq!(style.text, tokens.destructive);
                assert_eq!(style.icon, tokens.destructive);
            }
            let tint = row_style(&tokens, RowStatus::Highlighted, true).background;
            let expected = if tokens.is_dark { 0.2 } else { 0.1 };
            assert!(tint.is_some_and(|colour| (colour.a - expected).abs() < 1e-6));
        }
    }

    #[test]
    fn every_status_resolves_distinctly() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            for destructive in [false, true] {
                let styles = RowStatus::ALL.map(|status| row_style(&tokens, status, destructive));
                for (i, a) in styles.iter().enumerate() {
                    assert!(styles[i + 1..].iter().all(|b| a != b));
                }
            }
        }
    }

    #[test]
    fn row_containers_carry_the_background_text_and_small_radius() {
        let tokens = Tokens::of(&light());
        let style = row_container_style(row_style(&tokens, RowStatus::Highlighted, false));
        assert_eq!(style.background, Some(Background::Color(tokens.accent)));
        assert_eq!(style.text_color, Some(tokens.accent_foreground));
        assert_eq!(style.border.radius, radius::SM.into());
        let idle = row_container_style(row_style(&tokens, RowStatus::Idle, false));
        assert_eq!(idle.background, None);
    }

    #[test]
    fn label_and_separator_styles() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            assert_eq!(label_style(&tokens).color, Some(tokens.muted_foreground));
            assert_eq!(
                separator_style(&tokens).background,
                Some(Background::Color(tokens.border))
            );
        }
    }
}
