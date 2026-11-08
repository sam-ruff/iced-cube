//! A document to and from a small, safe subset of HTML.
//!
//! [`to_html`] writes `div` lines, `ul`, `ol` and `blockquote` blocks, and
//! `strong`, `em`, `u`, `s`, `a` and sized `span` elements, which mail
//! clients show as they are. [`from_html`] reads that back, and also reads
//! what other editors write closely enough to keep the formatting: `p`,
//! `br`, headings, `b`, `i`, `font` sizes and inline styles. Scripts,
//! styles, images and every attribute except a safe `href` are dropped.

use std::sync::Arc;

use super::document::{Block, BlockKind, Document, Run, Size, Style, normalise_link};

/// The style of a quoted block, as mail clients draw a reply.
const QUOTE_STYLE: &str = "margin:0 0 0 .8ex;border-left:1px solid #ccc;padding-left:1ex";

/// Escapes text for HTML, attribute values included.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

fn size_css(size: Size) -> Option<&'static str> {
    match size {
        Size::Small => Some("font-size:small"),
        Size::Normal => None,
        Size::Large => Some("font-size:large"),
        Size::Huge => Some("font-size:x-large"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tag {
    Link(Arc<str>),
    Size(Size),
    Bold,
    Italic,
    Underline,
    Strike,
}

impl Tag {
    fn open(&self) -> String {
        match self {
            Tag::Link(url) => format!("<a href=\"{}\">", escape(url)),
            Tag::Size(size) => format!("<span style=\"{}\">", size_css(*size).unwrap_or_default()),
            Tag::Bold => "<strong>".into(),
            Tag::Italic => "<em>".into(),
            Tag::Underline => "<u>".into(),
            Tag::Strike => "<s>".into(),
        }
    }

    fn close(&self) -> &'static str {
        match self {
            Tag::Link(_) => "</a>",
            Tag::Size(_) => "</span>",
            Tag::Bold => "</strong>",
            Tag::Italic => "</em>",
            Tag::Underline => "</u>",
            Tag::Strike => "</s>",
        }
    }
}

/// The elements a style needs, outermost first. Links come first so that
/// one stays open across style changes inside it.
fn tags(style: &Style) -> Vec<Tag> {
    let mut tags = Vec::new();
    if let Some(link) = &style.link {
        tags.push(Tag::Link(link.clone()));
    }
    if style.size != Size::Normal {
        tags.push(Tag::Size(style.size));
    }
    for (on, tag) in [
        (style.bold, Tag::Bold),
        (style.italic, Tag::Italic),
        (style.underline, Tag::Underline),
        (style.strikethrough, Tag::Strike),
    ] {
        if on {
            tags.push(tag);
        }
    }
    tags
}

fn inline(block: &Block) -> String {
    let mut out = String::new();
    let mut open: Vec<Tag> = Vec::new();
    for run in block.runs() {
        let wanted = tags(&run.style);
        let keep = open.iter().zip(&wanted).take_while(|(a, b)| a == b).count();
        for tag in open.drain(keep..).rev() {
            out.push_str(tag.close());
        }
        for tag in wanted.into_iter().skip(keep) {
            out.push_str(&tag.open());
            open.push(tag);
        }
        if run.text.is_empty() {
            out.push_str("<br>");
        } else {
            out.push_str(&escape(&run.text));
        }
    }
    for tag in open.iter().rev() {
        out.push_str(tag.close());
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Container {
    None,
    Bullets,
    Numbers,
    Quote,
}

impl Container {
    fn of(kind: BlockKind) -> Self {
        match kind {
            BlockKind::Paragraph => Container::None,
            BlockKind::Bullet => Container::Bullets,
            BlockKind::Numbered => Container::Numbers,
            BlockKind::Quote => Container::Quote,
        }
    }

    fn open(self) -> String {
        match self {
            Container::None => String::new(),
            Container::Bullets => "<ul>".into(),
            Container::Numbers => "<ol>".into(),
            Container::Quote => format!("<blockquote style=\"{QUOTE_STYLE}\">"),
        }
    }

    fn close(self) -> &'static str {
        match self {
            Container::None => "",
            Container::Bullets => "</ul>",
            Container::Numbers => "</ol>",
            Container::Quote => "</blockquote>",
        }
    }
}

/// The document as an HTML fragment. Spaces are kept as typed.
pub fn to_html(document: &Document) -> String {
    let mut out = String::from("<div style=\"white-space:pre-wrap\">");
    let mut container = Container::None;
    for block in document.blocks() {
        let next = Container::of(block.kind());
        if next != container {
            out.push_str(container.close());
            out.push_str(&next.open());
            container = next;
        }
        let (open, close) = if block.kind().is_list() {
            ("<li>", "</li>")
        } else {
            ("<div>", "</div>")
        };
        out.push_str(open);
        out.push_str(&inline(block));
        out.push_str(close);
    }
    out.push_str(container.close());
    out.push_str("</div>");
    out
}

/// A complete HTML document around [`to_html`], for a mail body.
pub fn to_html_document(document: &Document) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"></head><body>{}</body></html>",
        to_html(document)
    )
}

/// A style change made by one element.
#[derive(Debug, Clone, Default)]
struct Change {
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
    strikethrough: Option<bool>,
    size: Option<Size>,
    link: Option<Option<Arc<str>>>,
    preserve: bool,
}

impl Change {
    fn apply(&self, style: &mut Style) {
        let set = |target: &mut bool, value: Option<bool>| {
            if let Some(value) = value {
                *target = value;
            }
        };
        set(&mut style.bold, self.bold);
        set(&mut style.italic, self.italic);
        set(&mut style.underline, self.underline);
        set(&mut style.strikethrough, self.strikethrough);
        if let Some(size) = self.size {
            style.size = size;
        }
        if let Some(link) = &self.link {
            style.link = link.clone();
        }
    }

    fn read_css(&mut self, css: &str) {
        for declaration in css.split(';') {
            let Some((property, value)) = declaration.split_once(':') else {
                continue;
            };
            let property = property.trim().to_ascii_lowercase();
            let value = value.trim().to_ascii_lowercase();
            match property.as_str() {
                "font-weight" => {
                    self.bold = match value.as_str() {
                        "bold" | "bolder" => Some(true),
                        "normal" | "lighter" => Some(false),
                        number => number.parse::<u32>().ok().map(|weight| weight >= 600),
                    };
                }
                "font-style" => self.italic = Some(value != "normal"),
                "text-decoration" | "text-decoration-line" => {
                    if value.contains("none") {
                        self.underline = Some(false);
                        self.strikethrough = Some(false);
                    }
                    if value.contains("underline") {
                        self.underline = Some(true);
                    }
                    if value.contains("line-through") {
                        self.strikethrough = Some(true);
                    }
                }
                "font-size" => self.size = css_size(&value).or(self.size),
                "white-space" => {
                    self.preserve = value.starts_with("pre") || value == "break-spaces"
                }
                _ => {}
            }
        }
    }
}

/// The nearest size for a CSS `font-size`, taking 16 pixels as normal.
fn css_size(value: &str) -> Option<Size> {
    let keyword = match value {
        "xx-small" | "x-small" | "small" | "smaller" => Some(Size::Small),
        "medium" => Some(Size::Normal),
        "large" | "larger" => Some(Size::Large),
        "x-large" | "xx-large" | "xxx-large" => Some(Size::Huge),
        _ => None,
    };
    if keyword.is_some() {
        return keyword;
    }
    let (number, unit) = value
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .map_or((value, ""), |at| value.split_at(at));
    let number: f32 = number.parse().ok()?;
    let pixels = match unit.trim() {
        "" | "px" => number,
        "pt" => number * 4.0 / 3.0,
        "em" | "rem" => number * 16.0,
        "%" => number * 0.16,
        _ => return None,
    };
    Some(if pixels < 12.5 {
        Size::Small
    } else if pixels <= 16.5 {
        Size::Normal
    } else if pixels <= 22.0 {
        Size::Large
    } else {
        Size::Huge
    })
}

/// The size of a `<font size>` attribute, 1 to 7 with 3 as normal.
fn font_size(value: &str) -> Option<Size> {
    let value = value.trim();
    let size: i32 = match value.strip_prefix('+') {
        Some(step) => 3 + step.parse::<i32>().ok()?,
        None => match value.strip_prefix('-') {
            Some(step) => 3 - step.parse::<i32>().ok()?,
            None => value.parse().ok()?,
        },
    };
    Some(match size {
        ..=2 => Size::Small,
        3 => Size::Normal,
        4 | 5 => Size::Large,
        _ => Size::Huge,
    })
}

fn decode_entity(name: &str) -> Option<char> {
    if let Some(number) = name.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };
        return char::from_u32(code).filter(|c| *c != '\0');
    }
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        // Kept apart from spaces until whitespace has collapsed.
        "nbsp" => '\u{A0}',
        "ndash" => '\u{2013}',
        "mdash" => '\u{2014}',
        "hellip" => '\u{2026}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        "copy" => '\u{A9}',
        "reg" => '\u{AE}',
        "trade" => '\u{2122}',
        "euro" => '\u{20AC}',
        "pound" => '\u{A3}',
        _ => return None,
    })
}

/// Replaces character references in `text`.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let entity = rest[1..]
            .find(';')
            .filter(|end| *end <= 10)
            .and_then(|end| decode_entity(&rest[1..=end]).map(|c| (c, end + 2)));
        match entity {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[derive(Debug)]
enum Token<'a> {
    Text(&'a str),
    Start {
        name: String,
        attributes: Vec<(String, String)>,
        closed: bool,
    },
    End(String),
}

/// Splits HTML into text and tags, skipping comments and declarations.
fn tokens(html: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut rest = html;
    while !rest.is_empty() {
        let Some(at) = rest.find('<') else {
            out.push(Token::Text(rest));
            break;
        };
        if at > 0 {
            out.push(Token::Text(&rest[..at]));
        }
        rest = &rest[at..];
        if let Some(comment) = rest.strip_prefix("<!--") {
            rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
            continue;
        }
        let next = rest[1..].chars().next();
        let is_tag =
            next.is_some_and(|c| c.is_ascii_alphabetic() || c == '/' || c == '!' || c == '?');
        let Some(end) = rest.find('>').filter(|_| is_tag) else {
            out.push(Token::Text(&rest[..1]));
            rest = &rest[1..];
            continue;
        };
        let inner = &rest[1..end];
        rest = &rest[end + 1..];
        if inner.starts_with('!') || inner.starts_with('?') {
            continue;
        }
        if let Some(name) = inner.strip_prefix('/') {
            out.push(Token::End(name.trim().to_ascii_lowercase()));
            continue;
        }
        let closed = inner.ends_with('/');
        let inner = inner.trim_end_matches('/');
        let name_end = inner
            .find(|c: char| c.is_whitespace())
            .unwrap_or(inner.len());
        out.push(Token::Start {
            name: inner[..name_end].to_ascii_lowercase(),
            attributes: attributes(&inner[name_end..]),
            closed,
        });
    }
    out
}

fn attributes(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        let name_end = rest
            .find(|c: char| c.is_whitespace() || c == '=')
            .unwrap_or(rest.len());
        let name = rest[..name_end].to_ascii_lowercase();
        rest = rest[name_end..].trim_start();
        let mut value = String::new();
        if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            let (raw, remaining) = match after.chars().next() {
                Some(quote @ ('"' | '\'')) => {
                    let body = &after[1..];
                    let end = body.find(quote).unwrap_or(body.len());
                    (&body[..end], body.get(end + 1..).unwrap_or(""))
                }
                _ => {
                    let end = after.find(char::is_whitespace).unwrap_or(after.len());
                    (&after[..end], &after[end..])
                }
            };
            value = unescape(raw);
            rest = remaining.trim_start();
        }
        if !name.is_empty() {
            out.push((name, value));
        }
    }
    out
}

fn attribute<'a>(attributes: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

const VOID: [&str; 11] = [
    "br", "img", "hr", "meta", "link", "input", "wbr", "col", "area", "base", "source",
];
const SKIPPED: [&str; 7] = [
    "head", "style", "script", "title", "template", "noscript", "iframe",
];
const BLOCKS: [&str; 22] = [
    "p",
    "div",
    "li",
    "ul",
    "ol",
    "blockquote",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "pre",
    "table",
    "tr",
    "section",
    "article",
    "header",
    "footer",
    "address",
    "dl",
    "dt",
];

struct Builder {
    blocks: Vec<Block>,
    runs: Vec<Run>,
    kind: Option<BlockKind>,
    open: Vec<(String, Change)>,
    lists: Vec<BlockKind>,
    items: Vec<BlockKind>,
    quotes: usize,
    skipping: usize,
    /// A collapsed space waiting for the next word, in its style.
    space_pending: Option<Style>,
}

impl Builder {
    fn style(&self) -> Style {
        let mut style = Style::default();
        for (_, change) in &self.open {
            change.apply(&mut style);
        }
        style
    }

    fn preserving(&self) -> bool {
        self.open.iter().any(|(_, change)| change.preserve)
    }

    fn context(&self) -> BlockKind {
        match self.items.last() {
            Some(kind) => *kind,
            None if self.quotes > 0 => BlockKind::Quote,
            None => BlockKind::Paragraph,
        }
    }

    fn push_text(&mut self, text: &str) {
        let text = text.replace('\u{A0}', " ");
        let text = text.as_str();
        let style = self.style();
        if self.kind.is_none() {
            self.kind = Some(self.context());
        }
        match self.runs.last_mut() {
            Some(run) if run.style == style => run.text.push_str(text),
            _ => self.runs.push(Run::new(text, style)),
        }
    }

    fn text(&mut self, raw: &str) {
        if self.skipping > 0 {
            return;
        }
        let text = unescape(raw);
        if self.preserving() {
            let mut lines = text.split('\n');
            if let Some(first) = lines.next() {
                self.push_text(first);
            }
            for line in lines {
                self.end_line(true);
                self.push_text(line);
            }
            return;
        }
        for (index, word) in text.split(|c: char| c.is_ascii_whitespace()).enumerate() {
            if index > 0 && self.space_pending.is_none() {
                self.space_pending = Some(self.style());
            }
            if word.is_empty() {
                continue;
            }
            // A collapsed space keeps the style it was written in.
            if let Some(style) = self.space_pending.take()
                && self.kind.is_some()
            {
                match self.runs.last_mut() {
                    Some(run) if run.style == style => run.text.push(' '),
                    _ => self.runs.push(Run::new(" ", style)),
                }
            }
            self.push_text(word);
        }
    }

    /// Ends the line being built. A forced end, from a line break, keeps
    /// an empty line.
    fn end_line(&mut self, force: bool) {
        self.space_pending = None;
        if self.kind.is_none() && !force {
            return;
        }
        let kind = self.kind.take().unwrap_or_else(|| self.context());
        let mut runs = std::mem::take(&mut self.runs);
        if !self.preserving()
            && let Some(last) = runs.last_mut()
        {
            let trimmed = last.text.trim_end_matches(' ').len();
            last.text.truncate(trimmed);
        }
        if runs.iter().all(|run| run.text.is_empty()) {
            runs = vec![Run::new("", self.style())];
        }
        self.blocks.push(Block::with_runs(kind, runs));
    }

    fn start(&mut self, name: &str, attributes: &[(String, String)], closed: bool) {
        if SKIPPED.contains(&name) {
            if !closed {
                self.skipping += 1;
            }
            return;
        }
        if self.skipping > 0 {
            return;
        }
        if name == "br" {
            self.end_line(true);
            return;
        }
        if BLOCKS.contains(&name) || name == "hr" {
            self.end_line(false);
        }
        if VOID.contains(&name) || closed {
            return;
        }
        match name {
            "ul" => self.lists.push(BlockKind::Bullet),
            "ol" => self.lists.push(BlockKind::Numbered),
            "li" => {
                let kind = self.lists.last().copied().unwrap_or(BlockKind::Bullet);
                self.items.push(kind);
            }
            "blockquote" => self.quotes += 1,
            _ => {}
        }
        let mut change = Change::default();
        match name {
            "b" | "strong" => change.bold = Some(true),
            "i" | "em" | "cite" | "var" | "dfn" => change.italic = Some(true),
            "u" | "ins" => change.underline = Some(true),
            "s" | "strike" | "del" => change.strikethrough = Some(true),
            "small" => change.size = Some(Size::Small),
            "big" => change.size = Some(Size::Large),
            "h1" => {
                change.bold = Some(true);
                change.size = Some(Size::Huge);
            }
            "h2" => {
                change.bold = Some(true);
                change.size = Some(Size::Large);
            }
            "h3" | "h4" | "h5" | "h6" | "th" | "dt" => change.bold = Some(true),
            "pre" => change.preserve = true,
            "a" => {
                change.link = Some(
                    attribute(attributes, "href")
                        .and_then(normalise_link)
                        .map(Into::into),
                );
            }
            "font" => change.size = attribute(attributes, "size").and_then(font_size),
            _ => {}
        }
        if let Some(css) = attribute(attributes, "style") {
            change.read_css(css);
        }
        self.open.push((name.to_owned(), change));
    }

    fn end(&mut self, name: &str) {
        if SKIPPED.contains(&name) {
            self.skipping = self.skipping.saturating_sub(1);
            return;
        }
        if self.skipping > 0 {
            return;
        }
        if BLOCKS.contains(&name) {
            self.end_line(false);
        }
        if let Some(index) = self.open.iter().rposition(|(open, _)| open == name) {
            self.open.truncate(index);
        }
        match name {
            "ul" | "ol" => {
                self.lists.pop();
            }
            "li" => {
                self.items.pop();
            }
            "blockquote" => self.quotes = self.quotes.saturating_sub(1),
            _ => {}
        }
    }
}

/// Reads HTML into a document. Unknown elements keep their text, and
/// nested lists are flattened.
pub fn from_html(html: &str) -> Document {
    let mut builder = Builder {
        blocks: Vec::new(),
        runs: Vec::new(),
        kind: None,
        open: Vec::new(),
        lists: Vec::new(),
        items: Vec::new(),
        quotes: 0,
        skipping: 0,
        space_pending: None,
    };
    for token in tokens(html) {
        match token {
            Token::Text(text) => builder.text(text),
            Token::Start {
                name,
                attributes,
                closed,
            } => builder.start(&name, &attributes, closed),
            Token::End(name) => builder.end(&name),
        }
    }
    builder.end_line(false);
    Document::from_blocks(builder.blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::rich_text_editor::document::Mark;

    fn bold() -> Style {
        Style::default().with(Mark::Bold)
    }

    fn sample() -> Document {
        Document::from_blocks([
            Block::with_runs(
                BlockKind::Paragraph,
                [
                    Run::plain("Hello "),
                    Run::new("bold", bold()),
                    Run::new(" and ", Style::default()),
                    Run::new("big", bold().with(Mark::Italic).sized(Size::Huge)),
                    Run::plain("  spaced"),
                ],
            ),
            Block::new(BlockKind::Paragraph),
            Block::with_runs(
                BlockKind::Bullet,
                [Run::new("one", Style::default().with(Mark::Underline))],
            ),
            Block::with_runs(
                BlockKind::Bullet,
                [Run::new("two", Style::default().with(Mark::Strikethrough))],
            ),
            Block::with_runs(BlockKind::Numbered, [Run::plain("first")]),
            Block::with_runs(
                BlockKind::Paragraph,
                [
                    Run::plain("see "),
                    Run::new(
                        "the ",
                        Style::default().linked("https://example.com/?a=1&b=2"),
                    ),
                    Run::new("site", bold().linked("https://example.com/?a=1&b=2")),
                    Run::new("small", Style::default().sized(Size::Small)),
                    Run::new("<tag> & \"quotes\"", Style::default().sized(Size::Large)),
                ],
            ),
            Block::with_runs(BlockKind::Quote, [Run::plain("quoted")]),
            Block::with_runs(BlockKind::Paragraph, [Run::new("", bold())]),
        ])
    }

    #[test]
    fn documents_round_trip_through_html() {
        let document = sample();
        let html = to_html(&document);
        assert_eq!(from_html(&html), document);
        assert_eq!(from_html(&to_html_document(&document)), document);
    }

    #[test]
    fn html_output_is_small_and_nested() {
        let html = to_html(&sample());
        assert!(html.starts_with(
            "<div style=\"white-space:pre-wrap\"><div>Hello <strong>bold</strong> and "
        ));
        assert!(
            html.contains("<span style=\"font-size:x-large\"><strong><em>big</em></strong></span>")
        );
        assert!(html.contains("<div><br></div>"));
        assert!(
            html.contains("<ul><li><u>one</u></li><li><s>two</s></li></ul><ol><li>first</li></ol>")
        );
        assert!(html.contains(
            "<a href=\"https://example.com/?a=1&amp;b=2\">the <strong>site</strong></a>"
        ));
        assert!(html.contains("&lt;tag&gt; &amp; &quot;quotes&quot;"));
        assert!(html.contains("<blockquote style="));
        assert!(html.contains("<div><strong><br></strong></div>"));
    }

    #[test]
    fn other_editors_html_keeps_its_formatting() {
        let document = from_html(
            "<html><head><style>p{color:red}</style><title>x</title></head><body>\
             <p>One <b>bold</b> <i>it</i> <span style=\"font-weight:700;text-decoration:underline\">u</span></p>\
             <p>Two<br>lines<br><br>after</p>\
             <h1>Title</h1>\
             <ul><li>a<ul><li>nested</li></ul></li></ul>\
             <font size=\"1\">tiny</font><font size=\"+3\">huge</font>\
             <script>alert(1)</script><!-- comment -->\
             <a href=\"javascript:alert(1)\">bad</a> <a href='www.ok.com'>ok</a>\
             </body></html>",
        );
        let lines: Vec<String> = document.blocks().iter().map(Block::text).collect();
        assert_eq!(
            lines,
            [
                "One bold it u",
                "Two",
                "lines",
                "",
                "after",
                "Title",
                "a",
                "nested",
                "tinyhugebad ok"
            ]
        );
        let first = &document.blocks()[0];
        assert!(
            first
                .runs()
                .iter()
                .any(|run| run.text == "bold" && run.style.bold)
        );
        assert!(
            first
                .runs()
                .iter()
                .any(|run| run.text == "it" && run.style.italic)
        );
        assert!(
            first
                .runs()
                .iter()
                .any(|run| run.text == "u" && run.style.bold && run.style.underline)
        );
        let title = &document.blocks()[5].runs()[0].style;
        assert!(title.bold && title.size == Size::Huge);
        assert_eq!(document.blocks()[7].kind(), BlockKind::Bullet);
        let last = document.blocks()[8].runs();
        assert_eq!(last[0].style.size, Size::Small);
        assert_eq!(last[1].style.size, Size::Huge);
        let links: Vec<_> = last
            .iter()
            .filter_map(|run| run.style.link.clone())
            .collect();
        assert_eq!(links, [Arc::from("https://www.ok.com")]);
    }

    #[test]
    fn whitespace_collapses_outside_preserved_text() {
        let document = from_html("<div>  a \n  b  </div><pre>x  y\nz</pre>");
        let lines: Vec<String> = document.blocks().iter().map(Block::text).collect();
        assert_eq!(lines, ["a b", "x  y", "z"]);
    }

    #[test]
    fn entities_and_stray_brackets_read_as_text() {
        let document = from_html("a &lt; b &amp;&amp; c &#x41;&#66; &nbsp;&unknown; 1 < 2");
        assert_eq!(document.text(), "a < b && c AB  &unknown; 1 < 2");
    }

    #[test]
    fn empty_html_is_an_empty_document() {
        assert!(from_html("").is_empty());
        assert!(from_html("<html><body></body></html>").is_empty());
        assert!(to_html(&Document::new()).contains("<div><br></div>"));
    }

    #[test]
    fn css_sizes_map_to_the_nearest_size() {
        assert_eq!(css_size("small"), Some(Size::Small));
        assert_eq!(css_size("12px"), Some(Size::Small));
        assert_eq!(css_size("16px"), Some(Size::Normal));
        assert_eq!(css_size("13.5pt"), Some(Size::Large));
        assert_eq!(css_size("2em"), Some(Size::Huge));
        assert_eq!(css_size("100%"), Some(Size::Normal));
        assert_eq!(css_size("x-large"), Some(Size::Huge));
        assert_eq!(css_size("calc(1px)"), None);
        assert_eq!(font_size("7"), Some(Size::Huge));
        assert_eq!(font_size("-1"), Some(Size::Small));
        assert_eq!(font_size("x"), None);
    }
}
