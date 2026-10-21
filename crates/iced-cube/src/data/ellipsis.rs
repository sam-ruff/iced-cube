//! Single-line text that ends in an ellipsis when it does not fit, so a
//! long label never wraps inside a row.

use std::borrow::Cow;

use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self as core_text, Paragraph, paragraph};
use iced::advanced::widget::{Operation, Tree, Widget, text as text_widget, tree};
use iced::advanced::{Text, renderer};
use iced::alignment::Vertical;
use iced::widget::text::{self, LineHeight, Shaping, Wrapping};
use iced::{Color, Element, Font, Length, Pixels, Rectangle, Renderer, Size, Theme, mouse};

use crate::theme::{self, text_size};

/// The character appended to shortened text.
pub(crate) const MARK: char = '\u{2026}';

/// `content` as it fits in `max` logical pixels, cut at a character
/// boundary and ended with [`MARK`] when it is too wide. `measure` returns
/// the width of a string.
pub(crate) fn fit<'a>(content: &'a str, max: f32, measure: impl Fn(&str) -> f32) -> Cow<'a, str> {
    if measure(content) <= max + 0.5 {
        return Cow::Borrowed(content);
    }
    let ends: Vec<usize> = content
        .char_indices()
        .map(|(index, _)| index)
        .skip(1)
        .collect();
    let shortened = |count: usize| {
        let end = ends.get(count).copied().unwrap_or(content.len());
        let mut cut = content[..end].trim_end().to_owned();
        cut.push(MARK);
        cut
    };

    // The longest prefix that still fits, found by bisection.
    let (mut low, mut high) = (0, ends.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        if measure(&shortened(middle)) <= max + 0.5 {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let best = shortened(low);
    if low == 0 && measure(&best) > max + 0.5 {
        return Cow::Owned(MARK.to_string());
    }
    Cow::Owned(best)
}

type Colour<'a> = Box<dyn Fn(&Theme) -> Color + 'a>;

/// Text on one line, shortened with an ellipsis to the width it is given.
pub(crate) struct Ellipsis<'a> {
    content: Cow<'a, str>,
    size: f32,
    line_height: LineHeight,
    font: Font,
    width: Length,
    align_x: text::Alignment,
    colour: Option<Colour<'a>>,
}

/// Creates small body text that ends in an ellipsis when it does not fit.
pub(crate) fn ellipsis<'a>(content: impl Into<Cow<'a, str>>) -> Ellipsis<'a> {
    Ellipsis {
        content: content.into(),
        size: text_size::SM,
        line_height: LineHeight::Absolute(Pixels(20.0)),
        font: theme::font(),
        width: Length::Fill,
        align_x: text::Alignment::Default,
        colour: None,
    }
}

impl<'a> Ellipsis<'a> {
    #[cfg(feature = "data-table")]
    pub(crate) fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    #[cfg(feature = "data-table")]
    pub(crate) fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    #[cfg(feature = "data-table")]
    pub(crate) fn align_x(mut self, align: impl Into<text::Alignment>) -> Self {
        self.align_x = align.into();
        self
    }

    pub(crate) fn colour(mut self, colour: impl Fn(&Theme) -> Color + 'a) -> Self {
        self.colour = Some(Box::new(colour));
        self
    }

    fn measure(&self, content: &str) -> f32 {
        <Renderer as core_text::Renderer>::Paragraph::with_text(Text {
            content,
            bounds: Size::INFINITE,
            size: Pixels(self.size),
            line_height: self.line_height,
            font: self.font,
            align_x: text::Alignment::Default,
            align_y: Vertical::Top,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::None,
        })
        .min_width()
    }
}

/// What the widget remembers between layouts: the paragraph, and the
/// shortened text for the last content and width.
#[derive(Default)]
struct State {
    paragraph: paragraph::Plain<<Renderer as core_text::Renderer>::Paragraph>,
    fitted: Option<(String, f32, String)>,
}

impl<Message> Widget<Message, Theme, Renderer> for Ellipsis<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        let max = limits.max().width;
        let cached = state
            .fitted
            .as_ref()
            .filter(|(source, width, _)| source == self.content.as_ref() && *width == max);
        let shown = match cached {
            Some((_, _, shown)) => shown.clone(),
            None => {
                let shown = fit(&self.content, max, |value| self.measure(value)).into_owned();
                state.fitted = Some((self.content.to_string(), max, shown.clone()));
                shown
            }
        };
        text_widget::layout(
            &mut state.paragraph,
            renderer,
            limits,
            &shown,
            text_widget::Format {
                width: self.width,
                height: Length::Shrink,
                size: Some(Pixels(self.size)),
                font: Some(self.font),
                line_height: self.line_height,
                align_x: self.align_x,
                align_y: Vertical::Top,
                shaping: Shaping::Advanced,
                wrapping: Wrapping::None,
            },
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        text_widget::draw(
            renderer,
            style,
            layout.bounds(),
            state.paragraph.raw(),
            text::Style {
                color: self.colour.as_ref().map(|colour| colour(theme)),
            },
            viewport,
        );
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.text(None, layout.bounds(), &self.content);
    }
}

impl<'a, Message: 'a> From<Ellipsis<'a>> for Element<'a, Message> {
    fn from(ellipsis: Ellipsis<'a>) -> Self {
        Element::new(ellipsis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character is ten pixels wide.
    fn width(value: &str) -> f32 {
        value.chars().count() as f32 * 10.0
    }

    #[test]
    fn text_that_fits_is_unchanged() {
        assert_eq!(fit("Invoices", 80.0, width), "Invoices");
        assert!(matches!(fit("Invoices", 200.0, width), Cow::Borrowed(_)));
    }

    #[test]
    fn long_text_keeps_the_longest_prefix_that_fits_with_the_mark() {
        let shown = fit("Quarterly report", 60.0, width);
        assert_eq!(shown, "Quart\u{2026}");
        assert!(width(&shown) <= 60.0);
    }

    #[test]
    fn trailing_spaces_are_dropped_before_the_mark() {
        assert_eq!(fit("Tax year 2026", 50.0, width), "Tax\u{2026}");
    }

    #[test]
    fn multibyte_characters_are_cut_on_boundaries() {
        assert_eq!(
            fit("\u{e9}t\u{e9} r\u{e9}sum\u{e9}", 30.0, width),
            "\u{e9}t\u{2026}"
        );
    }

    #[test]
    fn no_room_leaves_only_the_mark() {
        assert_eq!(fit("Report", 5.0, width), "\u{2026}");
        assert_eq!(fit("", 0.0, width), "");
    }
}
