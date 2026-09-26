//! Raised surfaces that group related content.

use iced::widget::{column, container, text};
use iced::{Background, Border, Color, Element, Length, Shadow, Vector};

use crate::theme::{Tokens, fade, mix, radius, space, text_size};

/// Headings break inside a word only when the word alone is wider than the
/// card, so they never run past the border.
const HEADER_WRAPPING: text::Wrapping = text::Wrapping::WordOrGlyph;

/// A card builder. Convert it into an [`Element`] to render.
pub struct Card<'a, Message> {
    title: Option<text::Fragment<'a>>,
    description: Option<text::Fragment<'a>>,
    body: Option<Element<'a, Message>>,
    footer: Option<Element<'a, Message>>,
    width: Length,
}

impl<Message> std::fmt::Debug for Card<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Card")
            .field("title", &self.title)
            .field("description", &self.description)
            .field("body", &self.body.is_some())
            .field("footer", &self.footer.is_some())
            .field("width", &self.width)
            .finish()
    }
}

/// Creates an empty card. Add a header, body and footer with the builder.
pub fn card<'a, Message>() -> Card<'a, Message> {
    Card {
        title: None,
        description: None,
        body: None,
        footer: None,
        width: Length::Shrink,
    }
}

impl<'a, Message> Card<'a, Message> {
    /// Sets the heading.
    pub fn title(mut self, title: impl text::IntoFragment<'a>) -> Self {
        self.title = Some(title.into_fragment());
        self
    }

    /// Sets the muted line under the heading.
    pub fn description(mut self, description: impl text::IntoFragment<'a>) -> Self {
        self.description = Some(description.into_fragment());
        self
    }

    /// Sets the main content.
    pub fn body(mut self, body: impl Into<Element<'a, Message>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sets the content under the body, usually a row of buttons.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Whether the card has a title or description.
    pub fn has_header(&self) -> bool {
        self.title.is_some() || self.description.is_some()
    }
}

impl<'a, Message: 'a> From<Card<'a, Message>> for Element<'a, Message> {
    fn from(card: Card<'a, Message>) -> Self {
        let has_header = card.has_header();
        let mut sections = column![].spacing(space::LG).width(Length::Fill);

        if has_header {
            let mut header = column![].spacing(space::XS);
            if let Some(title) = card.title {
                header = header.push(
                    text(title)
                        .size(text_size::LG)
                        .font(crate::theme::semibold())
                        .wrapping(HEADER_WRAPPING),
                );
            }
            if let Some(description) = card.description {
                header = header.push(
                    text(description)
                        .size(text_size::SM)
                        .wrapping(HEADER_WRAPPING)
                        .style(|theme: &iced::Theme| text::Style {
                            color: Some(Tokens::of(theme).muted_foreground),
                        }),
                );
            }
            sections = sections.push(header);
        }
        if let Some(body) = card.body {
            sections = sections.push(body);
        }
        if let Some(footer) = card.footer {
            sections = sections.push(footer);
        }

        // Content wider than the card is cut at the border, not drawn over it.
        container(sections)
            .padding(space::XL)
            .width(card.width)
            .clip(true)
            .style(|theme| style(&Tokens::of(theme)))
            .into()
    }
}

/// Colours of a card's surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colours {
    pub background: Color,
    pub border: Color,
    pub shadow: Color,
}

/// Resolves the card colours. In dark themes the surface is lifted slightly
/// above the page background.
pub fn colours(tokens: &Tokens) -> Colours {
    if tokens.is_dark {
        let background = mix(tokens.background, tokens.foreground, 0.04);
        return Colours {
            background,
            border: mix(background, tokens.foreground, 0.1),
            shadow: Color::TRANSPARENT,
        };
    }
    Colours {
        background: tokens.background,
        border: mix(tokens.background, tokens.border, 0.8),
        shadow: fade(tokens.foreground, 0.06),
    }
}

/// The container style for a card.
pub fn style(tokens: &Tokens) -> container::Style {
    let colours = colours(tokens);
    container::Style {
        text_color: Some(tokens.foreground),
        background: Some(Background::Color(colours.background)),
        border: Border {
            color: colours.border,
            width: 1.0,
            radius: radius::LG.into(),
        },
        shadow: Shadow {
            color: colours.shadow,
            offset: Vector::new(0.0, 1.0),
            blur_radius: 3.0,
        },
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{dark, light};

    fn luminance(color: Color) -> f32 {
        0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b
    }

    #[test]
    fn default_builder_is_empty_and_shrinks() {
        let c: Card<'_, ()> = card();
        assert!(!c.has_header());
        assert!(c.body.is_none());
        assert!(c.footer.is_none());
        assert_eq!(c.width, Length::Shrink);
    }

    #[test]
    fn title_or_description_makes_a_header() {
        assert!(card::<()>().title("Team").has_header());
        assert!(card::<()>().description("Members").has_header());
    }

    #[test]
    fn dark_surface_is_lighter_than_background() {
        let tokens = Tokens::of(&dark());
        let colours = colours(&tokens);
        assert!(luminance(colours.background) > luminance(tokens.background));
    }

    #[test]
    fn light_surface_matches_background_and_casts_a_shadow() {
        let tokens = Tokens::of(&light());
        let colours = colours(&tokens);
        assert_eq!(colours.background, tokens.background);
        assert!(colours.shadow.a > 0.0);
    }

    #[test]
    fn border_is_subtle_but_visible() {
        for theme in [light(), dark()] {
            let tokens = Tokens::of(&theme);
            let colours = colours(&tokens);
            assert_ne!(colours.border, colours.background);
            let contrast = (luminance(colours.border) - luminance(colours.background)).abs();
            assert!(contrast < 0.2, "{contrast}");
            assert_eq!(style(&tokens).border.width, 1.0);
        }
    }
}
