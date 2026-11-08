//! The document model: blocks of styled runs, and the edits made on them.
//!
//! Everything here is pure, so it can be tested and used without a window.
//! Offsets are byte offsets into a block's text and always fall on
//! character boundaries.

use std::fmt;
use std::sync::Arc;

use unicode_segmentation::UnicodeSegmentation;

/// A text size, relative to the editor's base size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum Size {
    Small,
    #[default]
    Normal,
    Large,
    Huge,
}

impl Size {
    pub const ALL: [Size; 4] = [Size::Small, Size::Normal, Size::Large, Size::Huge];

    pub const fn label(self) -> &'static str {
        match self {
            Size::Small => "Small",
            Size::Normal => "Normal",
            Size::Large => "Large",
            Size::Huge => "Huge",
        }
    }

    /// The text size as a multiple of the base size.
    pub const fn scale(self) -> f32 {
        match self {
            Size::Small => 0.85,
            Size::Normal => 1.0,
            Size::Large => 1.3,
            Size::Huge => 1.7,
        }
    }

    pub(crate) const fn code(self) -> u32 {
        match self {
            Size::Small => 0,
            Size::Normal => 1,
            Size::Large => 2,
            Size::Huge => 3,
        }
    }

    pub(crate) const fn from_code(code: u32) -> Self {
        match code {
            0 => Size::Small,
            2 => Size::Large,
            3 => Size::Huge,
            _ => Size::Normal,
        }
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// An inline mark that is either on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mark {
    Bold,
    Italic,
    Underline,
    Strikethrough,
}

impl Mark {
    pub const ALL: [Mark; 4] = [
        Mark::Bold,
        Mark::Italic,
        Mark::Underline,
        Mark::Strikethrough,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Mark::Bold => "Bold",
            Mark::Italic => "Italic",
            Mark::Underline => "Underline",
            Mark::Strikethrough => "Strikethrough",
        }
    }
}

/// How a run of text looks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub size: Size,
    /// The address the text links to.
    pub link: Option<Arc<str>>,
}

impl Style {
    pub fn has(&self, mark: Mark) -> bool {
        match mark {
            Mark::Bold => self.bold,
            Mark::Italic => self.italic,
            Mark::Underline => self.underline,
            Mark::Strikethrough => self.strikethrough,
        }
    }

    pub fn set(&mut self, mark: Mark, on: bool) {
        match mark {
            Mark::Bold => self.bold = on,
            Mark::Italic => self.italic = on,
            Mark::Underline => self.underline = on,
            Mark::Strikethrough => self.strikethrough = on,
        }
    }

    pub fn with(mut self, mark: Mark) -> Self {
        self.set(mark, true);
        self
    }

    pub fn sized(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn linked(mut self, url: impl Into<Arc<str>>) -> Self {
        self.link = Some(url.into());
        self
    }

    /// Whether the run looks like plain text.
    pub fn is_plain(&self) -> bool {
        *self == Style::default()
    }

    /// Whether the run is drawn with a different face than `other`, which
    /// matters for shaping.
    pub(crate) fn same_face(&self, other: &Style) -> bool {
        self.bold == other.bold && self.italic == other.italic
    }
}

/// Text in one style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub style: Style,
}

impl Run {
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self::new(text, Style::default())
    }
}

/// What a block is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlockKind {
    #[default]
    Paragraph,
    /// An item of a bulleted list.
    Bullet,
    /// An item of a numbered list. Adjacent items count up from one.
    Numbered,
    /// Quoted text, such as an earlier message in a reply.
    Quote,
}

impl BlockKind {
    pub(crate) const fn code(self) -> u32 {
        match self {
            BlockKind::Paragraph => 0,
            BlockKind::Bullet => 1,
            BlockKind::Numbered => 2,
            BlockKind::Quote => 3,
        }
    }

    pub(crate) const fn from_code(code: u32) -> Self {
        match code {
            1 => BlockKind::Bullet,
            2 => BlockKind::Numbered,
            3 => BlockKind::Quote,
            _ => BlockKind::Paragraph,
        }
    }

    pub fn is_list(self) -> bool {
        matches!(self, BlockKind::Bullet | BlockKind::Numbered)
    }
}

/// One line of the document: a paragraph, a list item or a quoted line.
///
/// A block always has at least one run. An empty block keeps one empty run
/// so that the style typed into it next is remembered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    kind: BlockKind,
    runs: Vec<Run>,
}

impl Default for Block {
    fn default() -> Self {
        Self::new(BlockKind::Paragraph)
    }
}

impl Block {
    /// An empty block.
    pub fn new(kind: BlockKind) -> Self {
        Self {
            kind,
            runs: vec![Run::plain("")],
        }
    }

    /// A block of `runs`, with neighbouring runs of the same style merged.
    pub fn with_runs(kind: BlockKind, runs: impl IntoIterator<Item = Run>) -> Self {
        let mut block = Self {
            kind,
            runs: runs.into_iter().collect(),
        };
        block.normalise();
        block
    }

    pub fn paragraph(text: impl Into<String>) -> Self {
        Self::with_runs(BlockKind::Paragraph, [Run::plain(text)])
    }

    pub fn kind(&self) -> BlockKind {
        self.kind
    }

    pub fn set_kind(&mut self, kind: BlockKind) {
        self.kind = kind;
    }

    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    pub fn text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }

    /// The length of the text in bytes.
    pub fn len(&self) -> usize {
        self.runs.iter().map(|run| run.text.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The style of the character just before `offset`, or of the first
    /// character at the start.
    pub fn style_before(&self, offset: usize) -> &Style {
        let mut start = 0;
        for run in &self.runs {
            let end = start + run.text.len();
            if offset <= end && (offset > start || start == 0) && !run.text.is_empty() {
                return &run.style;
            }
            start = end;
        }
        self.runs
            .first()
            .map_or_else(|| empty_style(), |run| &run.style)
    }

    /// The style of the character at `offset`, if there is one.
    pub fn style_after(&self, offset: usize) -> Option<&Style> {
        let mut start = 0;
        for run in &self.runs {
            let end = start + run.text.len();
            if offset >= start && offset < end {
                return Some(&run.style);
            }
            start = end;
        }
        None
    }

    /// The runs covering `start..end`, cut at both ends.
    pub fn slice(&self, start: usize, end: usize) -> Vec<Run> {
        let mut out = Vec::new();
        let mut at = 0;
        for run in &self.runs {
            let run_end = at + run.text.len();
            let from = start.max(at);
            let to = end.min(run_end);
            if from < to {
                out.push(Run::new(&run.text[from - at..to - at], run.style.clone()));
            }
            at = run_end;
        }
        if out.is_empty() {
            out.push(Run::new("", self.style_before(start).clone()));
        }
        out
    }

    fn normalise(&mut self) {
        let fallback = self.runs.first().map(|run| run.style.clone());
        let mut runs: Vec<Run> = Vec::with_capacity(self.runs.len());
        for run in self.runs.drain(..) {
            if run.text.is_empty() {
                continue;
            }
            match runs.last_mut() {
                Some(last) if last.style == run.style => last.text.push_str(&run.text),
                _ => runs.push(run),
            }
        }
        if runs.is_empty() {
            runs.push(Run::new("", fallback.unwrap_or_default()));
        }
        self.runs = runs;
    }

    /// Splits the runs at `offset`, so that a run starts there. Returns the
    /// index of that run.
    fn cut(&mut self, offset: usize) -> usize {
        let mut at = 0;
        for index in 0..self.runs.len() {
            let len = self.runs[index].text.len();
            if offset == at {
                return index;
            }
            if offset < at + len {
                let tail = self.runs[index].text.split_off(offset - at);
                let style = self.runs[index].style.clone();
                self.runs.insert(index + 1, Run::new(tail, style));
                return index + 1;
            }
            at += len;
        }
        self.runs.len()
    }

    /// Removes and returns everything from `offset` on.
    fn split_off(&mut self, offset: usize) -> Vec<Run> {
        let style = self.style_before(offset).clone();
        let index = self.cut(offset);
        let mut tail = self.runs.split_off(index);
        if tail.is_empty() {
            tail.push(Run::new("", style.clone()));
        }
        if self.runs.is_empty() {
            self.runs.push(Run::new("", style));
        }
        self.normalise();
        tail
    }

    fn insert_runs(&mut self, offset: usize, runs: Vec<Run>) {
        let tail = self.split_off(offset);
        if self.is_empty() {
            self.runs.clear();
        }
        self.runs.extend(runs);
        self.runs
            .extend(tail.into_iter().filter(|run| !run.text.is_empty()));
        self.normalise();
    }

    fn remove(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let keep = self
            .style_after(start)
            .unwrap_or_else(|| self.style_before(start))
            .clone();
        let mut tail = self.split_off(end);
        self.split_off(start);
        if self.is_empty() {
            self.runs = vec![Run::new("", keep)];
        }
        self.runs.append(&mut tail);
        self.normalise();
    }

    fn restyle(&mut self, start: usize, end: usize, change: &impl Fn(&mut Style)) {
        if self.is_empty() {
            if let Some(run) = self.runs.first_mut() {
                change(&mut run.style);
            }
            return;
        }
        if start >= end {
            return;
        }
        let first = self.cut(start);
        let last = self.cut(end);
        for run in &mut self.runs[first..last] {
            change(&mut run.style);
        }
        self.normalise();
    }

    /// The text from `offset` up to the end of the grapheme there.
    fn next_boundary(&self, text: &str, offset: usize) -> Option<usize> {
        let rest = text.get(offset..)?;
        let grapheme = rest.graphemes(true).next()?;
        Some(offset + grapheme.len())
    }

    fn previous_boundary(&self, text: &str, offset: usize) -> Option<usize> {
        let head = text.get(..offset)?;
        head.grapheme_indices(true).next_back().map(|(at, _)| at)
    }

    /// The range of the link at `offset`, joining neighbouring runs with
    /// the same address.
    fn link_range(&self, offset: usize) -> Option<(usize, usize, Arc<str>)> {
        let link = self
            .style_after(offset)
            .and_then(|style| style.link.clone())
            .or_else(|| {
                (offset > 0)
                    .then(|| self.style_before(offset).link.clone())
                    .flatten()
            })?;
        let mut ranges = Vec::new();
        let mut at = 0;
        for run in &self.runs {
            let end = at + run.text.len();
            ranges.push((at, end, run.style.link.as_ref() == Some(&link)));
            at = end;
        }
        let index = ranges
            .iter()
            .position(|(start, end, linked)| *linked && *start <= offset && offset <= *end)?;
        let mut first = index;
        while first > 0 && ranges[first - 1].2 {
            first -= 1;
        }
        let mut last = index;
        while last + 1 < ranges.len() && ranges[last + 1].2 {
            last += 1;
        }
        Some((ranges[first].0, ranges[last].1, link))
    }
}

fn empty_style() -> &'static Style {
    static PLAIN: std::sync::OnceLock<Style> = std::sync::OnceLock::new();
    PLAIN.get_or_init(Style::default)
}

/// A place in the document: a block and a byte offset into its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Position {
    pub block: usize,
    pub offset: usize,
}

impl Position {
    pub const fn new(block: usize, offset: usize) -> Self {
        Self { block, offset }
    }
}

/// The caret and, when text is selected, the other end of the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Selection {
    /// Where the selection started.
    pub anchor: Position,
    /// Where the caret is drawn.
    pub head: Position,
}

impl Selection {
    pub const fn caret(at: Position) -> Self {
        Self {
            anchor: at,
            head: at,
        }
    }

    pub const fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    pub fn start(&self) -> Position {
        self.anchor.min(self.head)
    }

    pub fn end(&self) -> Position {
        self.anchor.max(self.head)
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }
}

/// A rich text document. It always has at least one block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    blocks: Vec<Block>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    /// An empty document of one empty paragraph.
    pub fn new() -> Self {
        Self {
            blocks: vec![Block::default()],
        }
    }

    pub fn from_blocks(blocks: impl IntoIterator<Item = Block>) -> Self {
        let blocks: Vec<Block> = blocks.into_iter().collect();
        if blocks.is_empty() {
            return Self::new();
        }
        Self { blocks }
    }

    /// Plain paragraphs, one per line of `text`.
    pub fn from_text(text: &str) -> Self {
        Self::from_blocks(
            text.split('\n')
                .map(|line| Block::paragraph(line.strip_suffix('\r').unwrap_or(line))),
        )
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn block(&self, index: usize) -> Option<&Block> {
        self.blocks.get(index)
    }

    /// The text without formatting, one line per block.
    pub fn text(&self) -> String {
        self.blocks
            .iter()
            .map(Block::text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Whether the document is a single empty paragraph.
    pub fn is_empty(&self) -> bool {
        matches!(self.blocks.as_slice(), [block] if block.is_empty() && block.kind == BlockKind::Paragraph)
    }

    /// Whether anything is styled, listed or quoted, so that sending the
    /// document as plain text would lose something.
    pub fn has_formatting(&self) -> bool {
        self.blocks.iter().any(|block| {
            block.kind != BlockKind::Paragraph
                || block
                    .runs
                    .iter()
                    .any(|run| !run.text.is_empty() && !run.style.is_plain())
        })
    }

    pub fn start(&self) -> Position {
        Position::default()
    }

    pub fn end(&self) -> Position {
        let block = self.blocks.len().saturating_sub(1);
        Position::new(block, self.blocks.get(block).map_or(0, Block::len))
    }

    /// The nearest valid position to `at`.
    pub fn clamp(&self, at: Position) -> Position {
        let Some(block) = self.blocks.get(at.block) else {
            return self.end();
        };
        let text = block.text();
        let mut offset = at.offset.min(text.len());
        while !text.is_char_boundary(offset) {
            offset -= 1;
        }
        Position::new(at.block, offset)
    }

    /// The number shown before a numbered item, counting the numbered items
    /// directly above it.
    pub fn number(&self, block: usize) -> Option<usize> {
        if self.blocks.get(block)?.kind != BlockKind::Numbered {
            return None;
        }
        let above = self.blocks[..block]
            .iter()
            .rev()
            .take_while(|block| block.kind == BlockKind::Numbered)
            .count();
        Some(above + 1)
    }

    /// The style text typed at `at` takes: that of the character before,
    /// without a link unless the text on both sides has the same link.
    pub fn style_for_insert(&self, at: Position) -> Style {
        let Some(block) = self.blocks.get(at.block) else {
            return Style::default();
        };
        let mut style = block.style_before(at.offset).clone();
        let inside = at.offset > 0
            && block
                .style_after(at.offset)
                .and_then(|after| after.link.as_ref())
                == style.link.as_ref();
        if !inside {
            style.link = None;
        }
        style
    }

    /// A copy of the document between `start` and `end`.
    pub fn slice(&self, start: Position, end: Position) -> Document {
        let (start, end) = (start.min(end), start.max(end));
        let mut blocks = Vec::new();
        for index in start.block..=end.block.min(self.blocks.len().saturating_sub(1)) {
            let block = &self.blocks[index];
            let from = if index == start.block {
                start.offset
            } else {
                0
            };
            let to = if index == end.block {
                end.offset
            } else {
                block.len()
            };
            blocks.push(Block::with_runs(block.kind, block.slice(from, to)));
        }
        Self::from_blocks(blocks)
    }

    /// Deletes the text between `start` and `end`, joining the blocks at
    /// either end. Returns where the text was.
    pub fn delete(&mut self, start: Position, end: Position) -> Position {
        let (start, end) = (self.clamp(start.min(end)), self.clamp(start.max(end)));
        if start == end {
            return start;
        }
        if start.block == end.block {
            self.blocks[start.block].remove(start.offset, end.offset);
            return start;
        }
        let tail = self.blocks[end.block].split_off(end.offset);
        let head = &mut self.blocks[start.block];
        head.split_off(start.offset);
        let at = head.len();
        head.insert_runs(at, tail);
        self.blocks.drain(start.block + 1..=end.block);
        start
    }

    /// Inserts `text` in `style` at `at`, starting a new block at each line
    /// break. Returns the end of the inserted text.
    pub fn insert_text(&mut self, at: Position, text: &str, style: &Style) -> Position {
        let lines = text.split('\n').map(|line| {
            Block::with_runs(
                BlockKind::Paragraph,
                [Run::new(
                    line.strip_suffix('\r').unwrap_or(line),
                    style.clone(),
                )],
            )
        });
        self.insert(at, &Document::from_blocks(lines))
    }

    /// Inserts a fragment at `at` and returns its end. The first block of
    /// the fragment joins the block at `at`, and the text after `at` joins
    /// the last. Pasted paragraphs take the kind of the block they land in.
    pub fn insert(&mut self, at: Position, fragment: &Document) -> Position {
        let at = self.clamp(at);
        let target = self.blocks[at.block].kind;
        let pasted = |kind: BlockKind| {
            if kind == BlockKind::Paragraph {
                target
            } else {
                kind
            }
        };
        let [first, rest @ ..] = fragment.blocks.as_slice() else {
            return at;
        };
        let first_runs: Vec<Run> = first.runs.clone();
        let Some((last, middle)) = rest.split_last() else {
            let block = &mut self.blocks[at.block];
            let before = block.len();
            block.insert_runs(at.offset, first_runs);
            return Position::new(at.block, at.offset + block.len() - before);
        };
        let block = &mut self.blocks[at.block];
        if block.is_empty() && first.kind != BlockKind::Paragraph {
            block.kind = first.kind;
        }
        let tail = block.split_off(at.offset);
        let end = block.len();
        block.insert_runs(end, first_runs);
        let mut new_blocks: Vec<Block> = middle
            .iter()
            .map(|block| Block::with_runs(pasted(block.kind), block.runs.clone()))
            .collect();
        let mut closing = Block::with_runs(pasted(last.kind), last.runs.clone());
        let offset = closing.len();
        let closing_len = closing.len();
        closing.insert_runs(closing_len, tail);
        new_blocks.push(closing);
        let index = at.block + new_blocks.len();
        self.blocks.splice(at.block + 1..at.block + 1, new_blocks);
        Position::new(index, offset)
    }

    /// Splits the block at `at`. The new block keeps the kind, so a list
    /// goes on. Returns the start of the new block.
    pub fn split(&mut self, at: Position) -> Position {
        let at = self.clamp(at);
        let block = &mut self.blocks[at.block];
        let tail = block.split_off(at.offset);
        let next = Block::with_runs(block.kind, tail);
        self.blocks.insert(at.block + 1, next);
        Position::new(at.block + 1, 0)
    }

    /// Changes the style of the text between `start` and `end`. An empty
    /// block in the range keeps the style for what is typed into it.
    pub fn restyle(&mut self, start: Position, end: Position, change: impl Fn(&mut Style)) {
        let (start, end) = (self.clamp(start.min(end)), self.clamp(start.max(end)));
        for index in start.block..=end.block {
            let block = &mut self.blocks[index];
            let from = if index == start.block {
                start.offset
            } else {
                0
            };
            let to = if index == end.block {
                end.offset
            } else {
                block.len()
            };
            if from < to || block.is_empty() {
                block.restyle(from, to, &change);
            }
        }
    }

    /// Sets the kind of every block from `first` to `last`.
    pub fn set_kind(&mut self, first: usize, last: usize, kind: BlockKind) {
        for block in self.blocks.iter_mut().take(last + 1).skip(first) {
            block.kind = kind;
        }
    }

    /// Whether every character between `start` and `end` has `mark`. An
    /// empty range reads the style typed there.
    pub fn all_have(&self, start: Position, end: Position, test: impl Fn(&Style) -> bool) -> bool {
        let (start, end) = (start.min(end), start.max(end));
        if start == end {
            return test(&self.style_for_insert(start));
        }
        let mut any = false;
        for (style, _) in self.styles_between(start, end) {
            any = true;
            if !test(style) {
                return false;
            }
        }
        any || test(&self.style_for_insert(start))
    }

    /// The styles of the non-empty runs between `start` and `end`, with
    /// how many bytes of each are in the range.
    fn styles_between(&self, start: Position, end: Position) -> Vec<(&Style, usize)> {
        let mut styles = Vec::new();
        for index in start.block..=end.block.min(self.blocks.len().saturating_sub(1)) {
            let block = &self.blocks[index];
            let from = if index == start.block {
                start.offset
            } else {
                0
            };
            let to = if index == end.block {
                end.offset
            } else {
                block.len()
            };
            let mut at = 0;
            for run in &block.runs {
                let run_end = at + run.text.len();
                let overlap = to.min(run_end).saturating_sub(from.max(at));
                if overlap > 0 {
                    styles.push((&run.style, overlap));
                }
                at = run_end;
            }
        }
        styles
    }

    /// The size used throughout the range, or `None` when it is mixed.
    pub fn size_between(&self, start: Position, end: Position) -> Option<Size> {
        let (start, end) = (start.min(end), start.max(end));
        if start == end {
            return Some(self.style_for_insert(start).size);
        }
        let styles = self.styles_between(start, end);
        let first = styles.first().map(|(style, _)| style.size)?;
        styles
            .iter()
            .all(|(style, _)| style.size == first)
            .then_some(first)
    }

    /// The link at `at`, and the range it covers.
    pub fn link_at(&self, at: Position) -> Option<(Position, Position, Arc<str>)> {
        let block = self.blocks.get(at.block)?;
        let (start, end, link) = block.link_range(at.offset)?;
        Some((
            Position::new(at.block, start),
            Position::new(at.block, end),
            link,
        ))
    }

    /// One grapheme before `at`, moving to the end of the previous block
    /// at the start of a block.
    pub fn previous(&self, at: Position) -> Position {
        let at = self.clamp(at);
        let block = &self.blocks[at.block];
        if at.offset == 0 {
            if at.block == 0 {
                return at;
            }
            return Position::new(at.block - 1, self.blocks[at.block - 1].len());
        }
        let text = block.text();
        let offset = block.previous_boundary(&text, at.offset).unwrap_or(0);
        Position::new(at.block, offset)
    }

    /// One grapheme after `at`, moving to the start of the next block at
    /// the end of a block.
    pub fn next(&self, at: Position) -> Position {
        let at = self.clamp(at);
        let block = &self.blocks[at.block];
        if at.offset >= block.len() {
            if at.block + 1 >= self.blocks.len() {
                return at;
            }
            return Position::new(at.block + 1, 0);
        }
        let text = block.text();
        let offset = block.next_boundary(&text, at.offset).unwrap_or(text.len());
        Position::new(at.block, offset)
    }

    /// The start of the word before `at`, skipping spaces.
    pub fn word_left(&self, at: Position) -> Position {
        let at = self.clamp(at);
        if at.offset == 0 {
            return self.previous(at);
        }
        let text = self.blocks[at.block].text();
        let head = &text[..at.offset];
        let start = head
            .split_word_bound_indices()
            .rev()
            .find(|(_, word)| !word.trim().is_empty())
            .map_or(0, |(start, _)| start);
        Position::new(at.block, start)
    }

    /// The end of the word after `at`, skipping spaces.
    pub fn word_right(&self, at: Position) -> Position {
        let at = self.clamp(at);
        let text = self.blocks[at.block].text();
        if at.offset >= text.len() {
            return self.next(at);
        }
        let end = text[at.offset..]
            .split_word_bound_indices()
            .find(|(_, word)| !word.trim().is_empty())
            .map_or(text.len(), |(start, word)| at.offset + start + word.len());
        Position::new(at.block, end)
    }

    /// The word, or run of spaces or punctuation, around `at`.
    pub fn word_at(&self, at: Position) -> (Position, Position) {
        let at = self.clamp(at);
        let text = self.blocks[at.block].text();
        let mut found = None;
        for (start, word) in text.split_word_bound_indices() {
            let end = start + word.len();
            if start <= at.offset && at.offset < end {
                found = Some((start, end));
                break;
            }
            if end == at.offset {
                found = Some((start, end));
            }
        }
        let (start, end) = found.unwrap_or((at.offset, at.offset));
        (Position::new(at.block, start), Position::new(at.block, end))
    }

    /// The word around `at`, if `at` is inside or at either end of one.
    pub fn word_touching(&self, at: Position) -> Option<(Position, Position)> {
        let at = self.clamp(at);
        let text = self.blocks.get(at.block)?.text();
        let mut found = None;
        for (start, word) in text.split_word_bound_indices() {
            let end = start + word.len();
            if start > at.offset {
                break;
            }
            if at.offset <= end && word.chars().any(char::is_alphanumeric) {
                found = Some((start, end));
                if at.offset < end {
                    break;
                }
            }
        }
        let (start, end) = found?;
        Some((Position::new(at.block, start), Position::new(at.block, end)))
    }
}

/// Makes `address` safe to link to: http and https addresses, and mailto
/// for something that looks like an email address. An address without a
/// scheme gets `https://`. Anything else, such as script, is refused.
pub fn normalise_link(address: &str) -> Option<String> {
    let address = address.trim();
    if address.is_empty()
        || address
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"'))
    {
        return None;
    }
    let lower = address.to_ascii_lowercase();
    for scheme in ["https://", "http://", "mailto:"] {
        if lower.starts_with(scheme) {
            return (address.len() > scheme.len()).then(|| address.to_owned());
        }
    }
    if lower.contains("://") {
        return None;
    }
    // Any other scheme, such as `javascript:`, is refused; a host with a
    // port, such as `localhost:8080`, is not a scheme.
    if let Some((_, rest)) = lower.split_once(':')
        && !rest.starts_with(|c: char| c.is_ascii_digit())
    {
        return None;
    }
    if address.contains('@') && !address.contains('/') {
        return Some(format!("mailto:{address}"));
    }
    // A bare word is more likely a typo than a host.
    let host = address.split(['/', '?', '#']).next().unwrap_or_default();
    if !host.contains(['.', ':']) {
        return None;
    }
    Some(format!("https://{address}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold() -> Style {
        Style::default().with(Mark::Bold)
    }

    fn doc(lines: &[&str]) -> Document {
        Document::from_text(&lines.join("\n"))
    }

    fn at(block: usize, offset: usize) -> Position {
        Position::new(block, offset)
    }

    #[test]
    fn a_new_document_is_one_empty_paragraph() {
        let document = Document::new();
        assert!(document.is_empty());
        assert_eq!(document.blocks().len(), 1);
        assert_eq!(document.text(), "");
        assert!(!document.has_formatting());
        assert_eq!(Document::from_blocks([]), document);
    }

    #[test]
    fn runs_of_the_same_style_merge_and_empty_runs_go() {
        let block = Block::with_runs(
            BlockKind::Paragraph,
            [
                Run::plain("a"),
                Run::plain("b"),
                Run::new("", bold()),
                Run::new("c", bold()),
            ],
        );
        assert_eq!(block.runs(), [Run::plain("ab"), Run::new("c", bold())]);
        let empty = Block::with_runs(BlockKind::Bullet, [Run::new("", bold())]);
        assert_eq!(empty.runs(), [Run::new("", bold())]);
        assert!(empty.is_empty());
    }

    #[test]
    fn styling_part_of_a_word_splits_its_run() {
        let mut document = doc(&["hello"]);
        document.restyle(at(0, 1), at(0, 3), |style| style.bold = true);
        assert_eq!(
            document.blocks()[0].runs(),
            [Run::plain("h"), Run::new("el", bold()), Run::plain("lo")]
        );
        document.restyle(at(0, 0), at(0, 5), |style| style.bold = true);
        assert_eq!(document.blocks()[0].runs(), [Run::new("hello", bold())]);
    }

    #[test]
    fn restyling_across_blocks_touches_each() {
        let mut document = doc(&["one", "two", "three"]);
        document.restyle(at(0, 1), at(2, 2), |style| style.italic = true);
        let italic = Style::default().with(Mark::Italic);
        assert_eq!(
            document.blocks()[0].runs(),
            [Run::plain("o"), Run::new("ne", italic.clone())]
        );
        assert_eq!(
            document.blocks()[1].runs(),
            [Run::new("two", italic.clone())]
        );
        assert_eq!(
            document.blocks()[2].runs(),
            [Run::new("th", italic), Run::plain("ree")]
        );
    }

    #[test]
    fn deleting_within_and_across_blocks() {
        let mut document = doc(&["hello", "world"]);
        assert_eq!(document.delete(at(0, 1), at(0, 3)), at(0, 1));
        assert_eq!(document.text(), "hlo\nworld");
        assert_eq!(document.delete(at(1, 2), at(0, 2)), at(0, 2));
        assert_eq!(document.text(), "hlrld");
        assert_eq!(document.blocks().len(), 1);
    }

    #[test]
    fn deleting_everything_keeps_the_style_for_typing() {
        let mut document = Document::from_blocks([Block::with_runs(
            BlockKind::Paragraph,
            [Run::new("bold", bold())],
        )]);
        document.delete(at(0, 0), at(0, 4));
        assert_eq!(document.blocks()[0].runs(), [Run::new("", bold())]);
        assert_eq!(document.style_for_insert(at(0, 0)), bold());
    }

    #[test]
    fn inserting_text_with_line_breaks_makes_blocks() {
        let mut document = doc(&["ab"]);
        let end = document.insert_text(at(0, 1), "1\n2\n3", &bold());
        assert_eq!(end, at(2, 1));
        assert_eq!(document.text(), "a1\n2\n3b");
        assert_eq!(
            document.blocks()[0].runs(),
            [Run::plain("a"), Run::new("1", bold())]
        );
        assert_eq!(
            document.blocks()[2].runs(),
            [Run::new("3", bold()), Run::plain("b")]
        );
    }

    #[test]
    fn pasted_paragraphs_join_the_list_they_land_in() {
        let mut document =
            Document::from_blocks([Block::with_runs(BlockKind::Bullet, [Run::plain("item")])]);
        document.insert_text(at(0, 4), "\nnext\nlast", &Style::default());
        let kinds: Vec<_> = document.blocks().iter().map(Block::kind).collect();
        assert_eq!(kinds, [BlockKind::Bullet; 3]);
    }

    #[test]
    fn splitting_keeps_the_kind_and_the_style() {
        let mut document = Document::from_blocks([Block::with_runs(
            BlockKind::Numbered,
            [Run::new("onetwo", bold())],
        )]);
        assert_eq!(document.split(at(0, 3)), at(1, 0));
        assert_eq!(document.blocks()[1].kind(), BlockKind::Numbered);
        assert_eq!(document.blocks()[1].runs(), [Run::new("two", bold())]);
        assert_eq!(document.split(at(1, 3)), at(2, 0));
        assert_eq!(document.blocks()[2].runs(), [Run::new("", bold())]);
        assert_eq!(document.number(2), Some(3));
    }

    #[test]
    fn slices_cut_runs_at_both_ends() {
        let mut document = doc(&["hello", "there"]);
        document.restyle(at(0, 0), at(0, 2), |style| style.bold = true);
        let slice = document.slice(at(0, 1), at(1, 2));
        assert_eq!(slice.text(), "ello\nth");
        assert_eq!(
            slice.blocks()[0].runs(),
            [Run::new("e", bold()), Run::plain("llo")]
        );
    }

    #[test]
    fn numbers_count_the_items_above() {
        let mut document = doc(&["a", "b", "c", "d"]);
        document.set_kind(0, 1, BlockKind::Numbered);
        document.set_kind(3, 3, BlockKind::Numbered);
        assert_eq!(document.number(0), Some(1));
        assert_eq!(document.number(1), Some(2));
        assert_eq!(document.number(2), None);
        assert_eq!(document.number(3), Some(1));
    }

    #[test]
    fn typing_after_a_link_does_not_extend_it() {
        let link = Style::default().linked("https://example.com");
        let document = Document::from_blocks([Block::with_runs(
            BlockKind::Paragraph,
            [
                Run::plain("see "),
                Run::new("here", link.clone()),
                Run::plain("."),
            ],
        )]);
        assert_eq!(document.style_for_insert(at(0, 6)).link, link.link);
        assert_eq!(document.style_for_insert(at(0, 8)).link, None);
        assert_eq!(document.style_for_insert(at(0, 4)).link, None);
        let (start, end, url) = document.link_at(at(0, 8)).unwrap();
        assert_eq!(
            (start, end, &*url),
            (at(0, 4), at(0, 8), "https://example.com")
        );
        assert!(document.link_at(at(0, 2)).is_none());
    }

    #[test]
    fn graphemes_move_as_one() {
        let document = doc(&["e\u{301}x", "y"]);
        assert_eq!(document.next(at(0, 0)), at(0, 3));
        assert_eq!(document.previous(at(0, 3)), at(0, 0));
        assert_eq!(document.next(at(0, 4)), at(1, 0));
        assert_eq!(document.previous(at(1, 0)), at(0, 4));
        assert_eq!(document.previous(at(0, 0)), at(0, 0));
        assert_eq!(document.next(at(1, 1)), at(1, 1));
    }

    #[test]
    fn words_move_and_select() {
        let document = doc(&["one two  three", "four"]);
        assert_eq!(document.word_left(at(0, 9)), at(0, 4));
        assert_eq!(document.word_left(at(0, 4)), at(0, 0));
        assert_eq!(document.word_left(at(1, 0)), at(0, 14));
        assert_eq!(document.word_right(at(0, 0)), at(0, 3));
        assert_eq!(document.word_right(at(0, 3)), at(0, 7));
        assert_eq!(document.word_right(at(0, 14)), at(1, 0));
        assert_eq!(document.word_at(at(0, 5)), (at(0, 4), at(0, 7)));
        assert_eq!(document.word_at(at(0, 7)), (at(0, 7), at(0, 9)));
        assert_eq!(document.word_touching(at(0, 7)), Some((at(0, 4), at(0, 7))));
        assert_eq!(document.word_touching(at(0, 8)), None);
    }

    #[test]
    fn marks_and_sizes_across_a_range() {
        let mut document = doc(&["abcd"]);
        document.restyle(at(0, 0), at(0, 2), |style| style.bold = true);
        assert!(document.all_have(at(0, 0), at(0, 2), |s| s.bold));
        assert!(!document.all_have(at(0, 0), at(0, 3), |s| s.bold));
        assert!(document.all_have(at(0, 1), at(0, 1), |s| s.bold));
        assert_eq!(
            document.size_between(at(0, 0), at(0, 4)),
            Some(Size::Normal)
        );
        document.restyle(at(0, 3), at(0, 4), |style| style.size = Size::Huge);
        assert_eq!(document.size_between(at(0, 0), at(0, 4)), None);
        assert_eq!(document.size_between(at(0, 3), at(0, 4)), Some(Size::Huge));
    }

    #[test]
    fn clamping_keeps_positions_on_characters() {
        let document = doc(&["añb"]);
        assert_eq!(document.clamp(at(0, 2)), at(0, 1));
        assert_eq!(document.clamp(at(0, 99)), at(0, 4));
        assert_eq!(document.clamp(at(5, 0)), at(0, 4));
    }

    #[test]
    fn has_formatting_notices_styles_and_lists() {
        let mut document = doc(&["plain"]);
        assert!(!document.has_formatting());
        document.set_kind(0, 0, BlockKind::Bullet);
        assert!(document.has_formatting());
        let mut styled = doc(&["x"]);
        styled.restyle(at(0, 0), at(0, 1), |style| style.size = Size::Large);
        assert!(styled.has_formatting());
    }

    #[test]
    fn links_are_made_safe() {
        assert_eq!(
            normalise_link("example.com").as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            normalise_link(" https://a.b/c?d=1 ").as_deref(),
            Some("https://a.b/c?d=1")
        );
        assert_eq!(normalise_link("HTTP://a.b").as_deref(), Some("HTTP://a.b"));
        assert_eq!(
            normalise_link("sam@example.com").as_deref(),
            Some("mailto:sam@example.com")
        );
        assert_eq!(
            normalise_link("mailto:sam@example.com").as_deref(),
            Some("mailto:sam@example.com")
        );
        assert_eq!(
            normalise_link("localhost:8080").as_deref(),
            Some("https://localhost:8080")
        );
        assert_eq!(normalise_link("javascript:alert(1)"), None);
        assert_eq!(normalise_link("data:text/html,x"), None);
        assert_eq!(normalise_link("https://"), None);
        assert_eq!(normalise_link("a b"), None);
        assert_eq!(normalise_link("abc"), None);
        assert_eq!(normalise_link("abc/x.html"), None);
        assert_eq!(normalise_link(""), None);
    }

    #[test]
    fn size_codes_round_trip() {
        for size in Size::ALL {
            assert_eq!(Size::from_code(size.code()), size);
        }
        for kind in [
            BlockKind::Paragraph,
            BlockKind::Bullet,
            BlockKind::Numbered,
            BlockKind::Quote,
        ] {
            assert_eq!(BlockKind::from_code(kind.code()), kind);
        }
    }
}
