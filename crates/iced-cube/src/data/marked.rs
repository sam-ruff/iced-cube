//! Text with some ranges marked, such as search matches. The marks are
//! painted behind the text in a theme colour, so they follow the theme
//! like everything else.

use std::ops::Range;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self as core_text, Difference, Paragraph as _, Span};
use iced::advanced::widget::{Operation, Tree, Widget, text as text_widget, tree};
use iced::advanced::{Renderer as _, Text};
use iced::alignment::Vertical;
use iced::widget::text::{self, LineHeight, Shaping, Wrapping};
use iced::{
    Background, Border, Color, Element, Font, Length, Pixels, Rectangle, Renderer, Size, Theme,
    mouse,
};

type Paragraph = <Renderer as core_text::Renderer>::Paragraph;
type Colour<'a> = Box<dyn Fn(&Theme) -> Color + 'a>;

/// Where `query` occurs in `content`, ignoring ASCII case, without
/// overlaps. An empty query matches nothing.
pub(crate) fn find(content: &str, query: &str) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let haystack = content.as_bytes();
    let needle = query.as_bytes();
    let mut found = Vec::new();
    let mut start = 0;
    while start + needle.len() <= haystack.len() {
        let window = &haystack[start..start + needle.len()];
        if window.eq_ignore_ascii_case(needle) {
            found.push(start..start + needle.len());
            start += needle.len();
        } else {
            start += 1;
        }
    }
    found
}

/// Splits `content` into runs, each marked or not, in order. Empty runs
/// are left out.
fn runs(content: &str, marks: &[Range<usize>]) -> Vec<(Range<usize>, bool)> {
    let mut runs = Vec::with_capacity(marks.len() * 2 + 1);
    let mut at = 0;
    for mark in marks {
        let (start, end) = (mark.start.max(at), mark.end.min(content.len()));
        if start >= end || !content.is_char_boundary(start) || !content.is_char_boundary(end) {
            continue;
        }
        if start > at {
            runs.push((at..start, false));
        }
        runs.push((start..end, true));
        at = end;
    }
    if at < content.len() {
        runs.push((at..content.len(), false));
    }
    runs
}

/// Text with marked ranges.
pub(crate) struct Marked<'a> {
    content: &'a str,
    marks: Vec<Range<usize>>,
    size: f32,
    line_height: LineHeight,
    font: Font,
    wrapping: Wrapping,
    colour: Colour<'a>,
    mark: Colour<'a>,
}

/// Creates text with `marks` painted behind it. Marks are byte ranges that
/// must fall on character boundaries; any that do not are ignored.
pub(crate) fn marked<'a>(
    content: &'a str,
    marks: Vec<Range<usize>>,
    colour: impl Fn(&Theme) -> Color + 'a,
    mark: impl Fn(&Theme) -> Color + 'a,
) -> Marked<'a> {
    Marked {
        content,
        marks,
        size: crate::theme::text_size::SM,
        line_height: LineHeight::default(),
        font: crate::theme::font(),
        wrapping: Wrapping::Word,
        colour: Box::new(colour),
        mark: Box::new(mark),
    }
}

impl Marked<'_> {
    pub(crate) fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub(crate) fn line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.line_height = line_height.into();
        self
    }

    pub(crate) fn wrapping(mut self, wrapping: Wrapping) -> Self {
        self.wrapping = wrapping;
        self
    }

    fn format<C>(&self, content: C, bounds: Size) -> Text<C, Font> {
        Text {
            content,
            bounds,
            size: Pixels(self.size),
            line_height: self.line_height,
            font: self.font,
            align_x: text::Alignment::Default,
            align_y: Vertical::Top,
            shaping: Shaping::Advanced,
            wrapping: self.wrapping,
        }
    }

    /// The spans to shape, and which of them are marks.
    fn spans(&self) -> (Vec<Span<'_, (), Font>>, Vec<usize>) {
        let runs = runs(self.content, &self.marks);
        let marks = runs
            .iter()
            .enumerate()
            .filter_map(|(index, (_, marked))| marked.then_some(index))
            .collect();
        let spans = runs
            .into_iter()
            .map(|(range, _)| Span::new(&self.content[range]))
            .collect();
        (spans, marks)
    }
}

/// The shaped paragraph and what it was shaped from.
#[derive(Default)]
struct State {
    paragraph: Paragraph,
    source: Option<(String, Vec<Range<usize>>)>,
    /// Which spans are marks.
    marks: Vec<usize>,
}

impl<Message> Widget<Message, Theme, Renderer> for Marked<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State>();
        layout::sized(limits, Length::Fill, Length::Shrink, |limits| {
            let bounds = limits.max();
            let same = state
                .source
                .as_ref()
                .is_some_and(|(content, marks)| content == self.content && *marks == self.marks);
            if same {
                match state.paragraph.compare(self.format((), bounds)) {
                    Difference::None => {}
                    Difference::Bounds => state.paragraph.resize(bounds),
                    Difference::Shape => {
                        let (spans, _) = self.spans();
                        state.paragraph =
                            Paragraph::with_spans(self.format(spans.as_slice(), bounds));
                    }
                }
            } else {
                let (spans, marks) = self.spans();
                state.paragraph = Paragraph::with_spans(self.format(spans.as_slice(), bounds));
                state.marks = marks;
                state.source = Some((self.content.to_owned(), self.marks.clone()));
            }
            state.paragraph.min_bounds()
        })
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
        let bounds = layout.bounds();
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        let state = tree.state.downcast_ref::<State>();
        let mark = (self.mark)(theme);
        let origin = bounds.position() - iced::Point::ORIGIN;
        for &index in &state.marks {
            for region in state.paragraph.span_bounds(index) {
                renderer.fill_quad(
                    Quad {
                        bounds: region + origin,
                        border: Border {
                            radius: 2.0.into(),
                            ..Border::default()
                        },
                        snap: true,
                        ..Quad::default()
                    },
                    Background::Color(mark),
                );
            }
        }
        text_widget::draw(
            renderer,
            style,
            bounds,
            &state.paragraph,
            text::Style {
                color: Some((self.colour)(theme)),
            },
            &clip,
        );
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.text(None, layout.bounds(), self.content);
    }
}

impl<'a, Message: 'a> From<Marked<'a>> for Element<'a, Message> {
    fn from(marked: Marked<'a>) -> Self {
        Element::new(marked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_ignores_ascii_case_and_never_overlaps() {
        assert_eq!(find("Error: error ERROR", "error"), [0..5, 7..12, 13..18]);
        assert_eq!(find("aaaa", "aa"), [0..2, 2..4]);
        assert_eq!(find("anything", ""), Vec::<Range<usize>>::new());
        assert_eq!(find("short", "longer query"), Vec::<Range<usize>>::new());
    }

    #[test]
    fn find_lands_on_character_boundaries() {
        let content = "caf\u{e9} au lait, Caf\u{e9}";
        let found = find(content, "caf\u{e9}");
        assert_eq!(found.len(), 2);
        for range in found {
            assert!(content.is_char_boundary(range.start));
            assert!(content.is_char_boundary(range.end));
        }
    }

    #[test]
    fn runs_alternate_between_plain_and_marked_text() {
        let content = "one two three";
        assert_eq!(
            runs(content, std::slice::from_ref(&(4..7))),
            [(0..4, false), (4..7, true), (7..13, false)]
        );
        assert_eq!(
            runs(content, std::slice::from_ref(&(0..3))),
            [(0..3, true), (3..13, false)]
        );
        assert_eq!(runs(content, &[]), [(0..13, false)]);
        assert_eq!(runs("", &[]), []);
    }

    #[test]
    fn runs_skip_marks_that_overlap_or_fall_outside() {
        let content = "abcdef";
        assert_eq!(
            runs(content, &[0..2, 1..3, 9..12]),
            [(0..2, true), (2..3, true), (3..6, false)],
            "an overlap is cut to what is left"
        );
        let accented = "\u{e9}t\u{e9}";
        assert_eq!(
            runs(accented, std::slice::from_ref(&(1..2))),
            [(0..5, false)],
            "inside a character"
        );
    }

    #[test]
    fn spans_record_which_ones_are_marks() {
        let text = marked(
            "find me here",
            find("find me here", "me"),
            |_| Color::BLACK,
            |_| Color::WHITE,
        );
        let (spans, marks) = text.spans();
        assert_eq!(spans.len(), 3);
        assert_eq!(marks, [1]);
        let text = text
            .size(12.0)
            .line_height(LineHeight::Absolute(Pixels(18.0)))
            .wrapping(Wrapping::None);
        assert_eq!(text.size, 12.0);
        assert_eq!(text.wrapping, Wrapping::None);
    }
}
