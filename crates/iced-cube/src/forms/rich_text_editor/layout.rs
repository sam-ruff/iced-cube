//! Each block laid out as one paragraph of styled spans, with the caret,
//! hit testing and selection worked out from the shaped glyphs.

use iced::advanced::graphics::text::cosmic_text::{Buffer, LayoutGlyph, LayoutRun};
use iced::advanced::text::{self, Paragraph as _, Span};
use iced::alignment;
use iced::font::Style as FontStyle;
use iced::widget::text::{LineHeight, Shaping, Wrapping};
use iced::{Color, Font, Pixels, Point, Renderer, Size};

use super::document::{Block, BlockKind, Document, Position, Style};
use crate::theme;

/// The renderer's paragraph: the shaped text of a backend, or nothing when
/// the library is built without one.
pub type Paragraph = <Renderer as text::Renderer>::Paragraph;

/// Access to the glyphs a paragraph was shaped into.
pub trait Shaped {
    fn shaped(&self) -> Option<&Buffer>;
}

impl Shaped for () {
    fn shaped(&self) -> Option<&Buffer> {
        None
    }
}

impl Shaped for iced::advanced::graphics::text::Paragraph {
    fn shaped(&self) -> Option<&Buffer> {
        Some(self.buffer())
    }
}

/// Line height as a multiple of each span's text size.
pub const LINE_HEIGHT: f32 = 1.5;

/// A zero-width character that keeps a word whole but makes the shaper
/// split it where the face changes. Without it a word that is bold only in
/// part is drawn in the face of its first letter.
const JOINER: &str = "\u{2060}";

/// The font for a style: the registered base font, semibold for bold.
pub fn font(style: &Style) -> Font {
    let base = if style.bold {
        theme::semibold()
    } else {
        theme::font()
    };
    Font {
        style: if style.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        ..base
    }
}

/// The text size of a style in pixels.
pub fn pixels(style: &Style, base: f32) -> f32 {
    (base * style.size.scale()).round()
}

/// How one span is decorated, by its index in the paragraph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decoration {
    pub underline: bool,
    pub strikethrough: bool,
    pub link: bool,
    pub size: f32,
}

/// A laid out block.
#[derive(Debug, Clone)]
pub struct Laid {
    pub block: Block,
    pub number: Option<usize>,
    pub paragraph: Paragraph,
    /// The same text with only its links showing, when it has any.
    pub links: Option<Paragraph>,
    /// Model offsets where a joiner was put into the drawn text.
    joiners: Vec<usize>,
    pub decorations: Vec<Decoration>,
    /// The bullet or number, drawn in the gutter.
    pub marker: Option<Paragraph>,
    /// How far below the block's top the marker is drawn.
    pub marker_y: f32,
    /// Left inset of the text within the content.
    pub indent: f32,
    /// Top of the block within the content.
    pub y: f32,
    pub height: f32,
    /// The text size of the block's first run.
    pub first_size: f32,
}

/// The gutter for list markers and quote bars.
pub fn indent(kind: BlockKind, base: f32) -> f32 {
    match kind {
        BlockKind::Paragraph => 0.0,
        BlockKind::Bullet | BlockKind::Numbered => (base * 1.8).round(),
        BlockKind::Quote => (base * 0.9).round(),
    }
}

type Spans = Vec<Span<'static, (), Font>>;

/// The spans of a block, and the same spans showing only link text, to be
/// drawn over them in the link colour, which is only known when drawing.
fn spans(block: &Block, base: f32) -> (Spans, Option<Spans>, Vec<Decoration>, Vec<usize>) {
    let mut spans: Spans = Vec::new();
    let mut decorations = Vec::new();
    let mut joiners = Vec::new();
    let mut offset = 0;
    let mut previous: Option<(&Style, char)> = None;
    for run in block.runs() {
        let mut text = String::with_capacity(run.text.len() + JOINER.len());
        let first = run.text.chars().next();
        if let (Some((style, last)), Some(first)) = (previous, first)
            && !style.same_face(&run.style)
            && !last.is_whitespace()
            && !first.is_whitespace()
        {
            text.push_str(JOINER);
            joiners.push(offset);
        }
        text.push_str(&run.text);
        let size = pixels(&run.style, base);
        spans.push(
            Span::new(text)
                .size(size)
                .line_height(LineHeight::Relative(LINE_HEIGHT))
                .font(font(&run.style)),
        );
        decorations.push(Decoration {
            underline: run.style.underline,
            strikethrough: run.style.strikethrough,
            link: run.style.link.is_some(),
            size,
        });
        offset += run.text.len();
        if let Some(last) = run.text.chars().next_back() {
            previous = Some((&run.style, last));
        }
    }
    let any_link = decorations.iter().any(|decoration| decoration.link);
    let links = any_link.then(|| {
        spans
            .iter()
            .zip(&decorations)
            .map(|(span, decoration)| {
                if decoration.link {
                    span.clone()
                } else {
                    span.clone().color(Color::TRANSPARENT)
                }
            })
            .collect()
    });
    if any_link {
        for (span, decoration) in spans.iter_mut().zip(&decorations) {
            if decoration.link {
                span.color = Some(Color::TRANSPARENT);
            }
        }
    }
    (spans, links, decorations, joiners)
}

impl Laid {
    pub fn new(document: &Document, index: usize, width: f32, base: f32) -> Option<Self> {
        let block = document.block(index)?;
        let first_size = block
            .runs()
            .first()
            .map_or(base, |run| pixels(&run.style, base));
        let indent = indent(block.kind(), base);
        let (spans, links, decorations, joiners) = spans(block, base);
        let first_font = font(
            &block
                .runs()
                .first()
                .map(|run| run.style.clone())
                .unwrap_or_default(),
        );
        let shape = |spans: &[Span<'static, (), Font>]| {
            Paragraph::with_spans(text::Text {
                content: spans,
                bounds: Size::new((width - indent).max(1.0), f32::INFINITY),
                size: Pixels(first_size),
                line_height: LineHeight::Relative(LINE_HEIGHT),
                font: first_font,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                shaping: Shaping::Advanced,
                wrapping: Wrapping::WordOrGlyph,
            })
        };
        let paragraph = shape(&spans);
        let links = links.map(|links| shape(&links));
        let number = document.number(index);
        let marker = match block.kind() {
            BlockKind::Bullet => Some("\u{2022}".to_owned()),
            BlockKind::Numbered => number.map(|number| format!("{number}.")),
            BlockKind::Paragraph | BlockKind::Quote => None,
        }
        .map(|label| {
            Paragraph::with_text(text::Text {
                content: label.as_str(),
                bounds: Size::new((indent - base * 0.4).max(1.0), f32::INFINITY),
                size: Pixels(base),
                line_height: LineHeight::Relative(LINE_HEIGHT),
                font: theme::font(),
                align_x: text::Alignment::Right,
                align_y: alignment::Vertical::Top,
                shaping: Shaping::Advanced,
                wrapping: Wrapping::None,
            })
        });
        let line = (first_size * LINE_HEIGHT).round();
        let height = paragraph.min_bounds().height.max(line);
        let baseline = |paragraph: &Paragraph| {
            paragraph
                .shaped()
                .and_then(|buffer| buffer.layout_runs().next().map(|run| run.line_y))
        };
        // The marker sits on the baseline of the first line, which is lower
        // when that line holds larger text.
        let marker_y = match (baseline(&paragraph), marker.as_ref().and_then(baseline)) {
            (Some(text), Some(marker)) => (text - marker).max(0.0),
            _ => 0.0,
        };
        Some(Self {
            block: block.clone(),
            number,
            paragraph,
            links,
            joiners,
            decorations,
            marker,
            marker_y,
            indent,
            y: 0.0,
            height,
            first_size,
        })
    }

    /// Whether this layout still shows `block`.
    fn shows(&self, block: &Block, number: Option<usize>) -> bool {
        *block == self.block && number == self.number
    }

    pub fn len(&self) -> usize {
        self.block.len()
    }

    fn to_drawn(&self, offset: usize) -> usize {
        offset + JOINER.len() * self.joiners.iter().take_while(|at| **at <= offset).count()
    }

    fn to_model(&self, drawn: usize) -> usize {
        let mut shift = 0;
        for (index, at) in self.joiners.iter().enumerate() {
            let start = at + index * JOINER.len();
            if drawn >= start + JOINER.len() {
                shift += JOINER.len();
            } else if drawn > start {
                return *at;
            } else {
                break;
            }
        }
        (drawn - shift).min(self.len())
    }

    fn runs(&self) -> Vec<LayoutRun<'_>> {
        self.paragraph
            .shaped()
            .map(|buffer| buffer.layout_runs().collect())
            .unwrap_or_default()
    }

    /// Where the caret for `offset` is drawn, relative to the block's text:
    /// its x, the top of its line and the line's height.
    pub fn caret(&self, offset: usize) -> (f32, f32, f32) {
        let index = self.to_drawn(offset);
        let runs = self.runs();
        for (position, run) in runs.iter().enumerate() {
            let (Some(first), Some(last)) = (run.glyphs.first(), run.glyphs.last()) else {
                continue;
            };
            if index < first.start || index > last.end {
                continue;
            }
            // At a wrap the caret belongs at the start of the next line.
            let next_starts_here = runs
                .get(position + 1)
                .and_then(|next| next.glyphs.first())
                .is_some_and(|glyph| glyph.start == index);
            if index == last.end && next_starts_here {
                continue;
            }
            let x = run
                .glyphs
                .iter()
                .find(|glyph| glyph.start <= index && index < glyph.end)
                .map_or(last.x + last.w, |glyph| inside(run, glyph, index));
            return (x, run.line_top, run.line_height);
        }
        let line = (self.first_size * LINE_HEIGHT).round();
        match runs.last() {
            Some(run) => (
                run.glyphs.last().map_or(0.0, |glyph| glyph.x + glyph.w),
                run.line_top,
                run.line_height.max(1.0),
            ),
            None => (0.0, 0.0, line),
        }
    }

    /// The offset nearest to `point`, relative to the block's text.
    pub fn hit(&self, point: Point) -> usize {
        let y = point.y.clamp(0.0, (self.height - 1.0).max(0.0));
        let Some(cursor) = self
            .paragraph
            .shaped()
            .and_then(|buffer| buffer.hit(point.x, y))
        else {
            return if point.y <= 0.0 { 0 } else { self.len() };
        };
        let mut index = cursor.index;
        let runs = self.runs();
        // Past the end of a wrapped line, stay on that line.
        for (position, run) in runs.iter().enumerate() {
            let Some(last) = run.glyphs.last() else {
                continue;
            };
            let wraps = position + 1 < runs.len();
            if wraps
                && index == last.end
                && y >= run.line_top
                && y < run.line_top + run.line_height
                && run.text[last.start..last.end].trim().is_empty()
            {
                index = last.start;
            }
        }
        self.to_model(index)
    }

    /// The offsets where the visual line holding `offset` starts and ends.
    pub fn line_bounds(&self, offset: usize) -> (usize, usize) {
        let (_, top, _) = self.caret(offset);
        let runs = self.runs();
        let Some(run) = runs
            .iter()
            .find(|run| (run.line_top - top).abs() < 0.5)
            .or_else(|| runs.last())
        else {
            return (0, self.len());
        };
        let start = run.glyphs.first().map_or(0, |glyph| glyph.start);
        let last = run.glyphs.last();
        let wraps = runs
            .last()
            .is_some_and(|final_run| (final_run.line_top - run.line_top).abs() > 0.5);
        let end = match last {
            Some(glyph) if wraps && run.text[glyph.start..glyph.end].trim().is_empty() => {
                glyph.start
            }
            Some(glyph) => glyph.end,
            None => 0,
        };
        (self.to_model(start), self.to_model(end))
    }

    /// The highlighted rectangles for `start..end`, relative to the block's
    /// text, as x, y, width and height. `past_end` adds a sliver for the
    /// line break when the selection goes on into the next block.
    pub fn selection(&self, start: usize, end: usize, past_end: bool) -> Vec<[f32; 4]> {
        let (from, to) = (self.to_drawn(start), self.to_drawn(end));
        let runs = self.runs();
        let mut out = Vec::new();
        let count = runs.len();
        for (position, run) in runs.iter().enumerate() {
            let mut left = f32::INFINITY;
            let mut right = f32::NEG_INFINITY;
            for glyph in run.glyphs {
                if glyph.start >= from && glyph.end <= to && glyph.end > glyph.start {
                    left = left.min(glyph.x);
                    right = right.max(glyph.x + glyph.w);
                }
            }
            let last_line = position + 1 == count;
            if past_end && last_line {
                let end_x = run.glyphs.last().map_or(0.0, |glyph| glyph.x + glyph.w);
                let reaches = run.glyphs.last().is_none_or(|glyph| glyph.end <= to);
                if reaches && from <= run.glyphs.last().map_or(0, |glyph| glyph.end) {
                    left = left.min(end_x);
                    right = right.max(end_x + self.first_size * 0.4);
                }
            }
            if left < right {
                out.push([left, run.line_top, right - left, run.line_height]);
            }
        }
        out
    }

    /// Underline and strikethrough lines, relative to the block's text, as
    /// x, y, width, thickness and whether each belongs to a link.
    pub fn lines(&self) -> Vec<([f32; 4], bool)> {
        let mut out = Vec::new();
        for run in self.runs() {
            let mut start = 0;
            while start < run.glyphs.len() {
                let metadata = run.glyphs[start].metadata;
                let mut end = start;
                while end < run.glyphs.len() && run.glyphs[end].metadata == metadata {
                    end += 1;
                }
                let mut glyphs = &run.glyphs[start..end];
                let ends_line = end == run.glyphs.len();
                start = end;
                let Some(decoration) = self.decorations.get(metadata) else {
                    continue;
                };
                // Spaces at the end of a line are not underlined.
                while ends_line
                    && let Some((last, rest)) = glyphs.split_last()
                    && run.text[last.start..last.end].trim().is_empty()
                {
                    glyphs = rest;
                }
                let (Some(first), Some(last)) = (glyphs.first(), glyphs.last()) else {
                    continue;
                };
                let x = first.x;
                let width = last.x + last.w - x;
                let thickness = (decoration.size / 14.0).max(1.0).round();
                if decoration.underline || decoration.link {
                    let y = run.line_y + (decoration.size * 0.12).max(1.0).round();
                    out.push(([x, y, width, thickness], decoration.link));
                }
                if decoration.strikethrough {
                    let y = run.line_y - (decoration.size * 0.3).round();
                    out.push(([x, y, width, thickness], false));
                }
            }
        }
        out
    }
}

/// The x of `index` inside a glyph that covers several characters, such
/// as a ligature.
fn inside(run: &LayoutRun<'_>, glyph: &LayoutGlyph, index: usize) -> f32 {
    let cluster = &run.text[glyph.start..glyph.end];
    let total = cluster.chars().count().max(1);
    let before = run.text[glyph.start..index].chars().count();
    glyph.x + glyph.w * before as f32 / total as f32
}

/// Every block laid out and stacked.
#[derive(Debug, Clone, Default)]
pub struct Stack {
    pub blocks: Vec<Laid>,
    pub width: f32,
    pub base: f32,
    pub font: Option<Font>,
    pub revision: Option<u64>,
}

impl Stack {
    /// Lays out what changed since the last call.
    pub fn update(&mut self, document: &Document, revision: u64, width: f32, base: f32) {
        let font = theme::font();
        let same_frame = self.width == width && self.base == base && self.font == Some(font);
        if same_frame
            && self.revision == Some(revision)
            && self.blocks.len() == document.blocks().len()
        {
            return;
        }
        if !same_frame {
            self.blocks.clear();
        }
        let mut reuse: Vec<Option<Laid>> = std::mem::take(&mut self.blocks)
            .into_iter()
            .map(Some)
            .collect();
        let mut blocks = Vec::with_capacity(document.blocks().len());
        let mut number = 0;
        let mut y = 0.0;
        // Blocks move by at most a few places in one edit, so only the
        // old layouts near each index are tried.
        let shift = document.blocks().len() as isize - reuse.len() as isize;
        for (index, block) in document.blocks().iter().enumerate() {
            number = if block.kind() == BlockKind::Numbered {
                number + 1
            } else {
                0
            };
            let shown = (number > 0).then_some(number);
            let candidates = [
                index as isize,
                index as isize - shift,
                index as isize - 1,
                index as isize + 1,
            ];
            let found = candidates.into_iter().find_map(|candidate| {
                let slot = reuse.get_mut(usize::try_from(candidate).ok()?)?;
                slot.as_ref()
                    .is_some_and(|laid| laid.shows(block, shown))
                    .then(|| slot.take())
                    .flatten()
            });
            let Some(mut laid) = found.or_else(|| Laid::new(document, index, width, base)) else {
                continue;
            };
            laid.y = y;
            y += laid.height;
            blocks.push(laid);
        }
        self.blocks = blocks;
        self.width = width;
        self.base = base;
        self.font = Some(font);
        self.revision = Some(revision);
    }

    pub fn height(&self) -> f32 {
        self.blocks.last().map_or(0.0, |laid| laid.y + laid.height)
    }

    /// The caret for `at` in content coordinates: x, top and height.
    pub fn caret(&self, at: Position) -> Option<(f32, f32, f32)> {
        let laid = self.blocks.get(at.block)?;
        let (x, top, height) = laid.caret(at.offset);
        Some((laid.indent + x, laid.y + top, height))
    }

    /// The position nearest to `point` in content coordinates.
    pub fn hit(&self, point: Point) -> Position {
        let Some(last) = self.blocks.len().checked_sub(1) else {
            return Position::default();
        };
        if point.y < 0.0 {
            return Position::new(
                0,
                self.blocks[0].hit(Point::new(point.x - self.blocks[0].indent, 0.0)),
            );
        }
        let index = self
            .blocks
            .iter()
            .position(|laid| point.y < laid.y + laid.height)
            .unwrap_or(last);
        let laid = &self.blocks[index];
        let local = Point::new(point.x - laid.indent, point.y - laid.y);
        let offset = if point.y >= laid.y + laid.height {
            laid.len()
        } else {
            laid.hit(local)
        };
        Position::new(index, offset)
    }

    /// The position a line above or below `at`, keeping to `x`.
    pub fn vertical(&self, at: Position, x: f32, lines: f32) -> Position {
        let Some((_, top, height)) = self.caret(at) else {
            return at;
        };
        let y = if lines < 0.0 {
            top - 1.0 + (lines + 1.0) * height
        } else {
            top + height + 1.0 + (lines - 1.0) * height
        };
        if y < 0.0 {
            return Position::default();
        }
        if y >= self.height() {
            let last = self.blocks.len().saturating_sub(1);
            return Position::new(last, self.blocks.get(last).map_or(0, Laid::len));
        }
        self.hit(Point::new(x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::rich_text_editor::document::{Mark, Run};

    fn laid(runs: Vec<Run>, width: f32) -> Laid {
        let document = Document::from_blocks([Block::with_runs(BlockKind::Paragraph, runs)]);
        let Some(laid) = Laid::new(&document, 0, width, 14.0) else {
            panic!("a block to lay out");
        };
        laid
    }

    #[test]
    fn a_face_change_inside_a_word_gets_a_joiner() {
        let bold = Style::default().with(Mark::Bold);
        let laid = laid(
            vec![
                Run::plain("ab"),
                Run::new("cd", bold.clone()),
                Run::plain(" e"),
                Run::new("f", bold),
            ],
            400.0,
        );
        // Inside "abcd" and "ef", but not at the space before "e".
        assert_eq!(laid.joiners, [2, 6]);
        assert_eq!(laid.to_drawn(2), 5);
        assert_eq!(laid.to_drawn(1), 1);
        assert_eq!(laid.to_model(5), 2);
        assert_eq!(laid.to_model(3), 2);
        assert_eq!(laid.to_model(2), 2);
        assert_eq!(laid.to_model(6), 3);
        let underlined = Style::default().with(Mark::Underline);
        let same_face = self::laid(vec![Run::plain("ab"), Run::new("cd", underlined)], 400.0);
        assert!(same_face.joiners.is_empty());
    }

    #[test]
    fn the_caret_moves_right_along_the_text() {
        let laid = laid(vec![Run::plain("hello")], 400.0);
        let xs: Vec<f32> = (0..=5).map(|offset| laid.caret(offset).0).collect();
        assert_eq!(xs[0], 0.0);
        assert!(xs.windows(2).all(|pair| pair[1] > pair[0]), "{xs:?}");
        assert_eq!(laid.hit(Point::new(xs[3] + 0.1, 5.0)), 3);
        assert_eq!(laid.hit(Point::new(1000.0, 5.0)), 5);
        assert_eq!(laid.hit(Point::new(-5.0, 5.0)), 0);
    }

    #[test]
    fn wrapped_lines_stack_and_the_caret_follows() {
        let laid = laid(vec![Run::plain("one two three four five six seven")], 60.0);
        let (_, first_top, height) = laid.caret(0);
        let (_, last_top, _) = laid.caret(laid.len());
        assert!(last_top > first_top);
        assert!(laid.height >= last_top + height);
        let (start, end) = laid.line_bounds(0);
        assert_eq!(start, 0);
        assert!(end < laid.len());
        let rects = laid.selection(0, laid.len(), false);
        assert!(rects.len() > 1);
    }

    #[test]
    fn an_empty_block_has_a_line_and_a_caret() {
        let laid = laid(vec![Run::plain("")], 200.0);
        let (x, top, height) = laid.caret(0);
        assert_eq!((x, top), (0.0, 0.0));
        assert!(height > 0.0 && laid.height >= height);
        assert_eq!(laid.hit(Point::new(30.0, 3.0)), 0);
        assert_eq!(laid.selection(0, 0, true).len(), 1);
    }

    #[test]
    fn list_markers_sit_on_the_first_baseline() {
        let item = |runs: Vec<Run>| {
            let document = Document::from_blocks([Block::with_runs(BlockKind::Bullet, runs)]);
            Laid::new(&document, 0, 300.0, 14.0)
        };
        let normal = item(vec![Run::plain("item")]).map(|laid| laid.marker_y);
        let large = item(vec![
            Run::plain("a "),
            Run::new(
                "big",
                Style::default().sized(crate::forms::rich_text_editor::Size::Huge),
            ),
        ])
        .map(|laid| laid.marker_y);
        assert_eq!(normal, Some(0.0));
        assert!(large.is_some_and(|y| y > 2.0), "{large:?}");
    }

    #[test]
    fn decorations_follow_their_spans() {
        let laid = laid(
            vec![
                Run::plain("a "),
                Run::new("under", Style::default().with(Mark::Underline)),
                Run::new(" link", Style::default().linked("https://e.com")),
                Run::new(" gone", Style::default().with(Mark::Strikethrough)),
            ],
            400.0,
        );
        let lines = laid.lines();
        assert_eq!(lines.len(), 3);
        assert!(!lines[0].1 && lines[1].1 && !lines[2].1);
        assert!(lines[2].0[1] < lines[0].0[1]);
    }

    #[test]
    fn the_stack_reuses_unchanged_blocks_and_hits_between_them() {
        let mut document = Document::from_text("one\ntwo\nthree");
        let mut stack = Stack::default();
        stack.update(&document, 0, 300.0, 14.0);
        assert_eq!(stack.blocks.len(), 3);
        let second = stack.blocks[1].y;
        assert!(second > 0.0);
        assert_eq!(
            stack.hit(Point::new(0.0, second + 1.0)),
            Position::new(1, 0)
        );
        assert_eq!(stack.hit(Point::new(0.0, -10.0)), Position::new(0, 0));
        assert_eq!(stack.hit(Point::new(0.0, 1e6)), Position::new(2, 5));
        assert_eq!(
            stack.vertical(Position::new(0, 0), 0.0, 1.0),
            Position::new(1, 0)
        );
        assert_eq!(
            stack.vertical(Position::new(1, 0), 0.0, -1.0),
            Position::new(0, 0)
        );
        assert_eq!(
            stack.vertical(Position::new(0, 0), 0.0, -1.0),
            Position::new(0, 0)
        );
        document.set_kind(2, 2, BlockKind::Bullet);
        stack.update(&document, 1, 300.0, 14.0);
        assert!(stack.blocks[2].marker.is_some());
        assert!(stack.blocks[2].indent > 0.0);
    }
}
