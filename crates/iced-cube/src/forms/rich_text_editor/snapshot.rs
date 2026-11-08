//! A document written as one string, so the shared undo history can keep
//! the changes between steps as it does for plain text.
//!
//! Text is written as it is, and private-use characters mark where each
//! block and run starts. The same document always gives the same string.

use super::document::{Block, BlockKind, Document, Position, Run, Size, Style};

const BLOCK: u32 = 0xF_0000;
const STYLE: u32 = 0xF_0100;
const LINK_START: u32 = 0xF_0200;
const LINK_END: u32 = 0xF_0201;
const ESCAPE: u32 = 0xF_0300;
/// Text characters from here on are escaped, so they never read as marks.
const RESERVED: u32 = 0xF_0000;

fn mark(code: u32) -> char {
    char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
}

fn style_code(style: &Style) -> u32 {
    u32::from(style.bold)
        | u32::from(style.italic) << 1
        | u32::from(style.underline) << 2
        | u32::from(style.strikethrough) << 3
        | style.size.code() << 4
}

fn push_escaped(out: &mut String, text: &str) {
    for c in text.chars() {
        if u32::from(c) >= RESERVED {
            out.push(mark(ESCAPE));
        }
        out.push(c);
    }
}

pub fn encode(document: &Document) -> String {
    let mut out = String::new();
    for block in document.blocks() {
        out.push(mark(BLOCK + block.kind().code()));
        for run in block.runs() {
            out.push(mark(STYLE + style_code(&run.style)));
            if let Some(link) = &run.style.link {
                out.push(mark(LINK_START));
                push_escaped(&mut out, link);
                out.push(mark(LINK_END));
            }
            push_escaped(&mut out, &run.text);
        }
    }
    out
}

/// Reads a string from [`encode`] back, with the document position of the
/// byte offset `caret` in it.
pub fn decode(encoded: &str, caret: usize) -> Option<(Document, Position)> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut runs: Vec<Run> = Vec::new();
    let mut kind = None;
    let mut found = None;
    let mut link: Option<String> = None;
    let mut chars = encoded.char_indices().peekable();

    let finish = |kind: Option<BlockKind>, runs: &mut Vec<Run>, blocks: &mut Vec<Block>| {
        if let Some(kind) = kind {
            blocks.push(Block::with_runs(kind, runs.drain(..)));
        }
    };
    let offset = |runs: &[Run]| runs.iter().map(|run| run.text.len()).sum::<usize>();

    while let Some((at, c)) = chars.next() {
        if found.is_none() && at >= caret && kind.is_some() {
            found = Some(Position::new(blocks.len(), offset(&runs)));
        }
        let code = u32::from(c);
        let literal = if code == ESCAPE {
            chars.next().map(|(_, c)| c)?
        } else if (BLOCK..BLOCK + 4).contains(&code) {
            finish(kind, &mut runs, &mut blocks);
            kind = Some(BlockKind::from_code(code - BLOCK));
            continue;
        } else if (STYLE..STYLE + 64).contains(&code) {
            let bits = code - STYLE;
            runs.push(Run::new(
                "",
                Style {
                    bold: bits & 1 != 0,
                    italic: bits & 2 != 0,
                    underline: bits & 4 != 0,
                    strikethrough: bits & 8 != 0,
                    size: Size::from_code(bits >> 4),
                    link: None,
                },
            ));
            continue;
        } else if code == LINK_START {
            link = Some(String::new());
            continue;
        } else if code == LINK_END {
            let run = runs.last_mut()?;
            run.style.link = link.take().map(Into::into);
            continue;
        } else {
            c
        };
        match &mut link {
            Some(address) => address.push(literal),
            None => runs.last_mut()?.text.push(literal),
        }
    }
    let end = Position::new(blocks.len(), offset(&runs));
    finish(kind, &mut runs, &mut blocks);
    let document = Document::from_blocks(blocks);
    let caret = document.clamp(found.unwrap_or(end));
    Some((document, caret))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::rich_text_editor::document::Mark;

    fn sample() -> Document {
        Document::from_blocks([
            Block::with_runs(
                BlockKind::Paragraph,
                [
                    Run::plain("Hi "),
                    Run::new(
                        "there",
                        Style::default().with(Mark::Bold).sized(Size::Large),
                    ),
                ],
            ),
            Block::new(BlockKind::Bullet),
            Block::with_runs(
                BlockKind::Numbered,
                [Run::new(
                    "link",
                    Style::default().linked("https://e.com/\u{F0001}"),
                )],
            ),
            Block::paragraph("odd \u{F0100}\u{10FFFF} text"),
        ])
    }

    #[test]
    fn documents_round_trip() {
        let document = sample();
        let encoded = encode(&document);
        let (decoded, _) = decode(&encoded, 0).unwrap();
        assert_eq!(decoded, document);
        assert_eq!(encode(&decoded), encoded);
        let empty = Document::new();
        assert_eq!(decode(&encode(&empty), 0).unwrap().0, empty);
    }

    #[test]
    fn the_caret_maps_to_a_position() {
        let document = Document::from_text("ab\ncd");
        let encoded = encode(&document);
        let c = encoded.find('c').unwrap();
        assert_eq!(decode(&encoded, c).unwrap().1, Position::new(1, 0));
        assert_eq!(decode(&encoded, c + 1).unwrap().1, Position::new(1, 1));
        assert_eq!(
            decode(&encoded, encoded.len()).unwrap().1,
            Position::new(1, 2)
        );
        assert_eq!(decode(&encoded, 0).unwrap().1, Position::new(0, 0));
    }

    #[test]
    fn broken_input_is_refused() {
        assert!(decode("\u{F0300}", 0).is_none());
        assert!(decode("\u{F0000}text", 0).is_none());
    }
}
