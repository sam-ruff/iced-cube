//! The plain-text alternative of a document, for a mail's text part.
//!
//! Bold becomes `*bold*`, italic and underline become `_text_`, sizes and
//! strikethrough are dropped, a link is followed by its address in angle
//! brackets unless the text is the address, list items start with `- ` or
//! their number, and quoted lines start with `> `.

use std::sync::Arc;

use super::document::{Block, BlockKind, Document, Run};

/// Where a marker goes in a line, and what sorts it among others at the
/// same offset: closing markers come first, then link addresses, then
/// opening markers.
struct Insert {
    offset: usize,
    order: u8,
    text: String,
}

/// The byte ranges of the line where `test` holds, trimmed of spaces.
fn ranges(runs: &[Run], text: &str, test: impl Fn(&Run) -> bool) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut at = 0;
    for run in runs {
        let end = at + run.text.len();
        if test(run) && !run.text.is_empty() {
            match out.last_mut() {
                Some(last) if last.1 == at => last.1 = end,
                _ => out.push((at, end)),
            }
        }
        at = end;
    }
    out.into_iter()
        .filter_map(|(start, end)| {
            let slice = &text[start..end];
            let lead = slice.len() - slice.trim_start().len();
            let trail = slice.len() - slice.trim_end().len();
            (start + lead < end - trail).then_some((start + lead, end - trail))
        })
        .collect()
}

fn links(runs: &[Run]) -> Vec<(usize, usize, Arc<str>)> {
    let mut out: Vec<(usize, usize, Arc<str>)> = Vec::new();
    let mut at = 0;
    for run in runs {
        let end = at + run.text.len();
        if let Some(link) = &run.style.link {
            match out.last_mut() {
                Some(last) if last.1 == at && last.2 == *link => last.1 = end,
                _ => out.push((at, end, link.clone())),
            }
        }
        at = end;
    }
    out
}

fn line(block: &Block) -> String {
    let text = block.text();
    let runs = block.runs();
    let mut inserts = Vec::new();
    for (marker, open, close, test) in [
        ("*", 3, 1, (|run: &Run| run.style.bold) as fn(&Run) -> bool),
        ("_", 4, 0, |run: &Run| {
            run.style.italic || run.style.underline
        }),
    ] {
        for (start, end) in ranges(runs, &text, test) {
            inserts.push(Insert {
                offset: start,
                order: open,
                text: marker.into(),
            });
            inserts.push(Insert {
                offset: end,
                order: close,
                text: marker.into(),
            });
        }
    }
    for (start, end, link) in links(runs) {
        let label = &text[start..end];
        let shown = link.strip_prefix("mailto:").unwrap_or(&link);
        if label.trim() != shown && label.trim() != &*link {
            inserts.push(Insert {
                offset: end,
                order: 2,
                text: format!(" <{shown}>"),
            });
        }
    }
    inserts.sort_by_key(|insert| (insert.offset, insert.order));
    let mut out = String::with_capacity(text.len() + inserts.len() * 2);
    let mut at = 0;
    for insert in inserts {
        out.push_str(&text[at..insert.offset]);
        out.push_str(&insert.text);
        at = insert.offset;
    }
    out.push_str(&text[at..]);
    out
}

/// The document as plain text, one line per block.
pub fn to_plain(document: &Document) -> String {
    document
        .blocks()
        .iter()
        .enumerate()
        .map(|(index, block)| {
            let prefix = match block.kind() {
                BlockKind::Paragraph => String::new(),
                BlockKind::Bullet => "- ".into(),
                BlockKind::Numbered => format!("{}. ", document.number(index).unwrap_or(1)),
                BlockKind::Quote => "> ".into(),
            };
            format!("{prefix}{}", line(block))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::rich_text_editor::document::{Mark, Size, Style};

    fn block(kind: BlockKind, runs: impl IntoIterator<Item = Run>) -> Block {
        Block::with_runs(kind, runs)
    }

    #[test]
    fn marks_become_conventional_emphasis() {
        let document = Document::from_blocks([block(
            BlockKind::Paragraph,
            [
                Run::plain("a "),
                Run::new("bold ", Style::default().with(Mark::Bold)),
                Run::new("it", Style::default().with(Mark::Italic)),
                Run::plain(" "),
                Run::new("under", Style::default().with(Mark::Underline)),
                Run::new("struck", Style::default().with(Mark::Strikethrough)),
                Run::new("big", Style::default().sized(Size::Huge)),
            ],
        )]);
        assert_eq!(to_plain(&document), "a *bold* _it_ _under_struckbig");
    }

    #[test]
    fn overlapping_marks_nest() {
        let bold = Style::default().with(Mark::Bold);
        let document = Document::from_blocks([block(
            BlockKind::Paragraph,
            [
                Run::new("both", bold.clone().with(Mark::Italic)),
                Run::new(" bold", bold),
            ],
        )]);
        assert_eq!(to_plain(&document), "*_both_ bold*");
    }

    #[test]
    fn links_show_their_address_once() {
        let link = |url: &str| Style::default().linked(url);
        let document = Document::from_blocks([
            block(
                BlockKind::Paragraph,
                [
                    Run::plain("see "),
                    Run::new("the site", link("https://e.com")),
                    Run::plain("."),
                ],
            ),
            block(
                BlockKind::Paragraph,
                [Run::new("https://e.com", link("https://e.com"))],
            ),
            block(
                BlockKind::Paragraph,
                [Run::new("sam@e.com", link("mailto:sam@e.com"))],
            ),
            block(
                BlockKind::Paragraph,
                [Run::new(
                    "mail me",
                    link("mailto:sam@e.com").with(Mark::Bold),
                )],
            ),
        ]);
        assert_eq!(
            to_plain(&document),
            "see the site <https://e.com>.\nhttps://e.com\nsam@e.com\n*mail me* <sam@e.com>"
        );
    }

    #[test]
    fn lists_and_quotes_keep_their_prefixes() {
        let document = Document::from_blocks([
            block(BlockKind::Bullet, [Run::plain("one")]),
            block(BlockKind::Bullet, [Run::plain("two")]),
            block(BlockKind::Numbered, [Run::plain("first")]),
            block(BlockKind::Numbered, [Run::plain("second")]),
            Block::new(BlockKind::Paragraph),
            block(BlockKind::Quote, [Run::plain("quoted")]),
        ]);
        assert_eq!(
            to_plain(&document),
            "- one\n- two\n1. first\n2. second\n\n> quoted"
        );
    }

    #[test]
    fn plain_text_is_unchanged() {
        let document = Document::from_text("Hello\n\n  spaced *stars* stay");
        assert_eq!(to_plain(&document), "Hello\n\n  spaced *stars* stay");
    }
}
