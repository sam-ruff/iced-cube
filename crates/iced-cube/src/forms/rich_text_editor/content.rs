//! The content an app owns: the document, the selection and the undo
//! history, changed only through [`Content::perform`].

use std::sync::Arc;

use super::document::{
    BlockKind, Document, Mark, Position, Selection, Size, Style, normalise_link,
};
use super::{html, plain, snapshot};
use crate::editing::{self, History};

/// Where a motion moves the caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    WordLeft,
    WordRight,
    /// The start of the caret's block.
    BlockStart,
    /// The end of the caret's block.
    BlockEnd,
    DocumentStart,
    DocumentEnd,
    /// A position worked out from the layout, such as a line up or the
    /// place that was clicked.
    To(Position),
}

/// A change to the text.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Typed text, in the style at the caret.
    Insert(String),
    /// Pasted plain text. Line breaks start new blocks.
    Paste(String),
    /// A pasted copy of part of a document, with its formatting.
    PasteFormatted(Document),
    /// Starts a new block, or ends a list when the item is empty.
    Enter,
    Backspace,
    Delete,
    DeleteWordBackward,
    DeleteWordForward,
}

/// A change to the formatting of the selection, or of what is typed next
/// when nothing is selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    /// Turns a mark on for the selection, or off when all of it has it.
    Toggle(Mark),
    Size(Size),
    /// Makes the selected blocks this kind, or paragraphs again when they
    /// all already are.
    Block(BlockKind),
    /// Links the selection, the word at the caret or, with neither, the
    /// address itself as new text. Addresses go through
    /// [`normalise_link`]; one that is refused changes nothing.
    Link(String),
    /// Removes the link from the selection, or from the link at the caret.
    Unlink,
    /// Removes marks, sizes and links.
    Clear,
}

/// What an editor asks of its [`Content`].
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Move(Motion),
    /// Moves the caret and extends the selection.
    Select(Motion),
    SelectAll,
    SelectWord(Position),
    SelectBlock(Position),
    Edit(Edit),
    Format(Format),
    Undo,
    Redo,
    /// The link shortcut was pressed: the app should ask for an address.
    /// The content only counts it, see [`Content::link_requests`].
    RequestLink,
}

impl Action {
    /// Whether the action changes the document.
    pub fn is_edit(&self) -> bool {
        matches!(
            self,
            Action::Edit(_) | Action::Format(_) | Action::Undo | Action::Redo
        )
    }
}

/// The formatting at the caret or across the selection, for a toolbar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Formatting {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    /// `None` when the selection mixes sizes.
    pub size: Option<Size>,
    /// `None` when the selection spans different kinds of block.
    pub block: Option<BlockKind>,
    /// The link at the caret or the start of the selection.
    pub link: Option<Arc<str>>,
}

impl Formatting {
    pub fn has(&self, mark: Mark) -> bool {
        match mark {
            Mark::Bold => self.bold,
            Mark::Italic => self.italic,
            Mark::Underline => self.underline,
            Mark::Strikethrough => self.strikethrough,
        }
    }
}

/// A rich text document with its selection and undo history.
///
/// The app keeps a `Content` in its state, shows it with
/// [`rich_text_editor`](super::rich_text_editor) and applies each
/// [`Action`] the editor sends with [`perform`](Self::perform), as with
/// iced's own text editor.
#[derive(Debug, Clone)]
pub struct Content {
    document: Document,
    selection: Selection,
    /// The style for the next text typed at a collapsed caret.
    typing: Option<Style>,
    history: History,
    /// The document as the history stores it.
    snapshot: String,
    revision: u64,
    link_requests: u64,
}

impl Default for Content {
    fn default() -> Self {
        Self::new()
    }
}

impl Content {
    pub fn new() -> Self {
        Self::with_document(Document::new())
    }

    /// Plain text, one paragraph per line.
    pub fn with_text(text: &str) -> Self {
        Self::with_document(Document::from_text(text))
    }

    /// HTML, such as a saved draft, read with [`html::from_html`].
    pub fn with_html(html: &str) -> Self {
        Self::with_document(html::from_html(html))
    }

    pub fn with_document(document: Document) -> Self {
        Self {
            snapshot: snapshot::encode(&document),
            selection: Selection::caret(document.end()),
            document,
            typing: None,
            history: History::default(),
            revision: 0,
            link_requests: 0,
        }
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// Counts every change to the document, so views can tell when to
    /// lay it out again.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// How many times the link shortcut has been pressed, so a toolbar can
    /// open its address field on a new request.
    pub fn link_requests(&self) -> u64 {
        self.link_requests
    }

    /// The document as an HTML fragment, see [`html::to_html`].
    pub fn html(&self) -> String {
        html::to_html(&self.document)
    }

    /// The plain-text alternative, see [`plain::to_plain`].
    pub fn plain_text(&self) -> String {
        plain::to_plain(&self.document)
    }

    /// The text with no formatting at all, one line per block.
    pub fn text(&self) -> String {
        self.document.text()
    }

    pub fn is_empty(&self) -> bool {
        self.document.is_empty()
    }

    /// Whether the document has any formatting, see
    /// [`Document::has_formatting`].
    pub fn has_formatting(&self) -> bool {
        self.document.has_formatting()
    }

    /// The selected part of the document.
    pub fn selected(&self) -> Document {
        self.document
            .slice(self.selection.start(), self.selection.end())
    }

    /// The selected text without formatting.
    pub fn selected_text(&self) -> String {
        self.selected().text()
    }

    /// The style text typed at the caret takes.
    pub fn typing_style(&self) -> Style {
        self.typing
            .clone()
            .unwrap_or_else(|| self.document.style_for_insert(self.selection.head))
    }

    pub fn formatting(&self) -> Formatting {
        let (start, end) = (self.selection.start(), self.selection.end());
        let link = self.document.link_at(start).map(|(_, _, url)| url);
        let block = self.block_kind(start.block, end.block);
        if self.selection.is_collapsed() {
            let style = self.typing_style();
            return Formatting {
                bold: style.bold,
                italic: style.italic,
                underline: style.underline,
                strikethrough: style.strikethrough,
                size: Some(style.size),
                block,
                link,
            };
        }
        let has = |mark| self.document.all_have(start, end, |style| style.has(mark));
        Formatting {
            bold: has(Mark::Bold),
            italic: has(Mark::Italic),
            underline: has(Mark::Underline),
            strikethrough: has(Mark::Strikethrough),
            size: self.document.size_between(start, end),
            block,
            link,
        }
    }

    fn block_kind(&self, first: usize, last: usize) -> Option<BlockKind> {
        let kinds = self.document.blocks().get(first..=last)?;
        let kind = kinds.first()?.kind();
        kinds
            .iter()
            .all(|block| block.kind() == kind)
            .then_some(kind)
    }

    /// Applies an action from the editor, a toolbar or the app itself.
    pub fn perform(&mut self, action: Action) {
        match action {
            Action::Move(motion) => {
                let to = self.motion(motion, false);
                self.select(Selection::caret(to));
            }
            Action::Select(motion) => {
                let to = self.motion(motion, true);
                self.select(Selection::new(self.selection.anchor, to));
            }
            Action::SelectAll => {
                self.select(Selection::new(self.document.start(), self.document.end()));
            }
            Action::SelectWord(at) => {
                let (start, end) = self.document.word_at(at);
                self.select(Selection::new(start, end));
            }
            Action::SelectBlock(at) => {
                let at = self.document.clamp(at);
                let len = self.document.block(at.block).map_or(0, |block| block.len());
                self.select(Selection::new(
                    Position::new(at.block, 0),
                    Position::new(at.block, len),
                ));
            }
            Action::Edit(edit) => self.edit(edit),
            Action::Format(format) => self.format(format),
            Action::Undo => {
                let restore = self.history.undo(self.snapshot.clone());
                self.restore(restore);
            }
            Action::Redo => {
                let restore = self.history.redo(self.snapshot.clone());
                self.restore(restore);
            }
            Action::RequestLink => self.link_requests += 1,
        }
    }

    fn select(&mut self, selection: Selection) {
        let selection = Selection::new(
            self.document.clamp(selection.anchor),
            self.document.clamp(selection.head),
        );
        if selection != self.selection {
            self.typing = None;
            self.history.interrupt();
        }
        self.selection = selection;
    }

    fn motion(&self, motion: Motion, extending: bool) -> Position {
        let head = self.selection.head;
        let collapsed = self.selection.is_collapsed();
        match motion {
            Motion::Left if !extending && !collapsed => self.selection.start(),
            Motion::Right if !extending && !collapsed => self.selection.end(),
            Motion::Left => self.document.previous(head),
            Motion::Right => self.document.next(head),
            Motion::WordLeft => self.document.word_left(head),
            Motion::WordRight => self.document.word_right(head),
            Motion::BlockStart => Position::new(head.block, 0),
            Motion::BlockEnd => Position::new(
                head.block,
                self.document
                    .block(head.block)
                    .map_or(0, |block| block.len()),
            ),
            Motion::DocumentStart => self.document.start(),
            Motion::DocumentEnd => self.document.end(),
            Motion::To(at) => self.document.clamp(at),
        }
    }

    /// Records a change for undo, makes it, and settles the history.
    /// `change` returns the new selection, or `None` when nothing changed.
    fn change(&mut self, edit: editing::Edit, change: impl FnOnce(&mut Self) -> Option<Selection>) {
        let replacing = !self.selection.is_collapsed();
        let snapshot = &self.snapshot;
        self.history.record(edit, replacing, || snapshot.clone());
        if let Some(selection) = change(self) {
            self.snapshot = snapshot::encode(&self.document);
            self.revision += 1;
            self.selection = Selection::new(
                self.document.clamp(selection.anchor),
                self.document.clamp(selection.head),
            );
        }
        let snapshot = &self.snapshot;
        self.history.settle(|| snapshot.clone());
    }

    fn restore(&mut self, restore: Option<editing::Restore>) {
        let Some(restore) = restore else {
            return;
        };
        let Some((document, caret)) = snapshot::decode(&restore.text, restore.caret) else {
            return;
        };
        self.document = document;
        self.snapshot = restore.text;
        self.revision += 1;
        self.typing = None;
        self.selection = Selection::caret(caret);
        let snapshot = &self.snapshot;
        self.history.settle(|| snapshot.clone());
    }

    fn edit(&mut self, edit: Edit) {
        let collapsed = self.selection.is_collapsed();
        let kind = match &edit {
            Edit::Insert(text) => editing::Edit::Insert(text.clone()),
            Edit::Enter => editing::Edit::Insert("\n".into()),
            Edit::Backspace | Edit::Delete if collapsed => editing::Edit::Delete,
            _ => editing::Edit::Other,
        };
        let typing = self.typing.take();
        self.change(kind, |content| content.apply(edit, typing))
    }

    fn apply(&mut self, edit: Edit, typing: Option<Style>) -> Option<Selection> {
        let (start, end) = (self.selection.start(), self.selection.end());
        let document = &mut self.document;
        let caret = match edit {
            Edit::Insert(text) | Edit::Paste(text) => {
                let text = text.replace("\r\n", "\n").replace('\r', "\n");
                if text.is_empty() {
                    return None;
                }
                let style = match typing {
                    Some(style) => style,
                    None if start != end => document
                        .block(start.block)
                        .and_then(|block| block.style_after(start.offset))
                        .cloned()
                        .unwrap_or_default(),
                    None => document.style_for_insert(start),
                };
                let at = document.delete(start, end);
                document.insert_text(at, &text, &style)
            }
            Edit::PasteFormatted(fragment) => {
                let at = document.delete(start, end);
                document.insert(at, &fragment)
            }
            Edit::Enter => {
                let at = document.delete(start, end);
                let block = document.block(at.block)?;
                if block.is_empty() && block.kind() != BlockKind::Paragraph {
                    document.set_kind(at.block, at.block, BlockKind::Paragraph);
                    at
                } else {
                    document.split(at)
                }
            }
            Edit::Backspace | Edit::Delete | Edit::DeleteWordBackward | Edit::DeleteWordForward
                if start != end =>
            {
                document.delete(start, end)
            }
            Edit::Backspace => {
                let kind = document.block(start.block)?.kind();
                if start.offset == 0 && kind != BlockKind::Paragraph {
                    document.set_kind(start.block, start.block, BlockKind::Paragraph);
                    start
                } else {
                    let from = document.previous(start);
                    if from == start {
                        return None;
                    }
                    document.delete(from, start)
                }
            }
            Edit::Delete => {
                let to = document.next(start);
                if to == start {
                    return None;
                }
                document.delete(start, to)
            }
            Edit::DeleteWordBackward => {
                let from = document.word_left(start);
                if from == start {
                    return None;
                }
                document.delete(from, start)
            }
            Edit::DeleteWordForward => {
                let to = document.word_right(start);
                if to == start {
                    return None;
                }
                document.delete(start, to)
            }
        };
        Some(Selection::caret(caret))
    }

    fn format(&mut self, format: Format) {
        let (start, end) = (self.selection.start(), self.selection.end());
        let collapsed = start == end;
        match format {
            Format::Toggle(mark) if collapsed => {
                let mut style = self.typing_style();
                style.set(mark, !style.has(mark));
                self.typing = Some(style);
            }
            Format::Size(size) if collapsed => {
                let mut style = self.typing_style();
                style.size = size;
                self.typing = Some(style);
            }
            Format::Clear if collapsed => {
                let link = self.typing_style().link;
                self.typing = Some(Style {
                    link,
                    ..Style::default()
                });
            }
            Format::Toggle(mark) => {
                let on = !self.document.all_have(start, end, |style| style.has(mark));
                self.restyle(start, end, move |style| style.set(mark, on));
            }
            Format::Size(size) => self.restyle(start, end, move |style| style.size = size),
            Format::Clear => self.restyle(start, end, |style| *style = Style::default()),
            Format::Block(kind) => {
                let all = self.block_kind(start.block, end.block) == Some(kind);
                let kind = if all { BlockKind::Paragraph } else { kind };
                let selection = self.selection;
                self.change(editing::Edit::Other, |content| {
                    content.document.set_kind(start.block, end.block, kind);
                    Some(selection)
                });
            }
            Format::Link(address) => self.link(&address),
            Format::Unlink => {
                let range = if collapsed {
                    self.document.link_at(start).map(|(from, to, _)| (from, to))
                } else {
                    Some((start, end))
                };
                if let Some((from, to)) = range {
                    self.restyle(from, to, |style| style.link = None);
                }
            }
        }
    }

    fn restyle(&mut self, start: Position, end: Position, change: impl Fn(&mut Style)) {
        let selection = self.selection;
        self.change(editing::Edit::Other, |content| {
            let before = content.document.clone();
            content.document.restyle(start, end, change);
            (content.document != before).then_some(selection)
        });
    }

    fn link(&mut self, address: &str) {
        let Some(url) = normalise_link(address) else {
            return;
        };
        let (start, end) = (self.selection.start(), self.selection.end());
        let range = if start != end {
            Some((start, end))
        } else {
            self.document
                .link_at(start)
                .map(|(from, to, _)| (from, to))
                .or_else(|| self.document.word_touching(start))
        };
        let link: Arc<str> = url.as_str().into();
        let Some((from, to)) = range else {
            let shown = url.strip_prefix("mailto:").unwrap_or(&url).to_owned();
            let style = self.typing_style();
            let after_word = start.offset > 0
                && self
                    .document
                    .block(start.block)
                    .and_then(|block| block.text().get(..start.offset)?.chars().next_back())
                    .is_some_and(|c| !c.is_whitespace());
            self.change(editing::Edit::Other, |content| {
                let mut at = start;
                if after_word {
                    let mut space = style.clone();
                    space.link = None;
                    at = content.document.insert_text(at, " ", &space);
                }
                let end = content
                    .document
                    .insert_text(at, &shown, &style.linked(link));
                Some(Selection::caret(end))
            });
            return;
        };
        let selection = self.selection;
        self.change(editing::Edit::Other, |content| {
            content
                .document
                .restyle(from, to, |style| style.link = Some(link.clone()));
            Some(selection)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::rich_text_editor::document::{Block, Run};

    fn at(block: usize, offset: usize) -> Position {
        Position::new(block, offset)
    }

    fn typed(content: &mut Content, text: &str) {
        for c in text.chars() {
            let edit = if c == '\n' {
                Edit::Enter
            } else {
                Edit::Insert(c.to_string())
            };
            content.perform(Action::Edit(edit));
        }
    }

    fn select(content: &mut Content, from: Position, to: Position) {
        content.perform(Action::Move(Motion::To(from)));
        content.perform(Action::Select(Motion::To(to)));
    }

    fn runs(content: &Content, block: usize) -> Vec<(String, bool)> {
        content.document().blocks()[block]
            .runs()
            .iter()
            .map(|run| (run.text.clone(), run.style.bold))
            .collect()
    }

    #[test]
    fn typing_and_moving() {
        let mut content = Content::new();
        typed(&mut content, "hello\nworld");
        assert_eq!(content.text(), "hello\nworld");
        assert_eq!(content.selection(), Selection::caret(at(1, 5)));
        content.perform(Action::Move(Motion::WordLeft));
        content.perform(Action::Move(Motion::Left));
        assert_eq!(content.selection().head, at(0, 5));
        content.perform(Action::Select(Motion::BlockStart));
        assert_eq!(content.selected_text(), "hello");
        content.perform(Action::Move(Motion::Right));
        assert_eq!(content.selection(), Selection::caret(at(0, 5)));
    }

    #[test]
    fn bold_applies_to_part_of_a_word_and_toggles_off() {
        let mut content = Content::with_text("hello");
        select(&mut content, at(0, 1), at(0, 3));
        content.perform(Action::Format(Format::Toggle(Mark::Bold)));
        assert_eq!(
            runs(&content, 0),
            [
                ("h".into(), false),
                ("el".into(), true),
                ("lo".into(), false)
            ]
        );
        assert!(content.formatting().bold);
        content.perform(Action::Format(Format::Toggle(Mark::Bold)));
        assert_eq!(runs(&content, 0), [("hello".into(), false)]);
    }

    #[test]
    fn a_mixed_selection_turns_the_mark_on() {
        let mut content = Content::with_text("abcd");
        select(&mut content, at(0, 0), at(0, 2));
        content.perform(Action::Format(Format::Toggle(Mark::Bold)));
        select(&mut content, at(0, 0), at(0, 4));
        assert!(!content.formatting().bold);
        content.perform(Action::Format(Format::Toggle(Mark::Bold)));
        assert_eq!(runs(&content, 0), [("abcd".into(), true)]);
    }

    #[test]
    fn a_mark_at_the_caret_applies_to_what_is_typed_next() {
        let mut content = Content::with_text("a");
        content.perform(Action::Format(Format::Toggle(Mark::Bold)));
        assert!(content.formatting().bold);
        assert_eq!(content.revision(), 0);
        typed(&mut content, "bc");
        content.perform(Action::Format(Format::Toggle(Mark::Bold)));
        typed(&mut content, "d");
        assert_eq!(
            runs(&content, 0),
            [
                ("a".into(), false),
                ("bc".into(), true),
                ("d".into(), false)
            ]
        );
        content.perform(Action::Format(Format::Size(Size::Huge)));
        content.perform(Action::Move(Motion::Left));
        assert_eq!(content.formatting().size, Some(Size::Normal));
    }

    #[test]
    fn enter_continues_a_list_and_leaves_it_on_an_empty_item() {
        let mut content = Content::new();
        typed(&mut content, "one");
        content.perform(Action::Format(Format::Block(BlockKind::Bullet)));
        assert_eq!(content.formatting().block, Some(BlockKind::Bullet));
        typed(&mut content, "\ntwo\n");
        let kinds: Vec<_> = content
            .document()
            .blocks()
            .iter()
            .map(Block::kind)
            .collect();
        assert_eq!(kinds, [BlockKind::Bullet; 3]);
        typed(&mut content, "\n");
        assert_eq!(content.document().blocks()[2].kind(), BlockKind::Paragraph);
        assert_eq!(content.document().blocks().len(), 3);
    }

    #[test]
    fn backspace_at_the_start_of_an_item_ends_the_list_first() {
        let mut content = Content::with_text("a\nb");
        content.perform(Action::SelectAll);
        content.perform(Action::Format(Format::Block(BlockKind::Numbered)));
        content.perform(Action::Move(Motion::To(at(1, 0))));
        content.perform(Action::Edit(Edit::Backspace));
        assert_eq!(content.document().blocks()[1].kind(), BlockKind::Paragraph);
        content.perform(Action::Edit(Edit::Backspace));
        assert_eq!(content.text(), "ab");
        assert_eq!(content.selection().head, at(0, 1));
    }

    #[test]
    fn the_list_toggles_off_when_every_block_has_it() {
        let mut content = Content::with_text("a\nb");
        content.perform(Action::SelectAll);
        content.perform(Action::Format(Format::Block(BlockKind::Bullet)));
        assert_eq!(content.formatting().block, Some(BlockKind::Bullet));
        content.perform(Action::Format(Format::Block(BlockKind::Bullet)));
        assert_eq!(content.formatting().block, Some(BlockKind::Paragraph));
    }

    #[test]
    fn undo_restores_text_and_formatting_and_redo_returns() {
        let mut content = Content::new();
        typed(&mut content, "one two");
        content.perform(Action::SelectAll);
        content.perform(Action::Format(Format::Toggle(Mark::Italic)));
        assert!(content.formatting().italic);
        content.perform(Action::Undo);
        assert!(!content.document().has_formatting());
        assert_eq!(content.text(), "one two");
        content.perform(Action::Undo);
        assert_eq!(content.text(), "one ");
        assert_eq!(content.selection(), Selection::caret(at(0, 4)));
        content.perform(Action::Redo);
        content.perform(Action::Redo);
        assert_eq!(content.text(), "one two");
        assert!(content.document().has_formatting());
        content.perform(Action::Undo);
        content.perform(Action::Undo);
        content.perform(Action::Undo);
        assert!(content.is_empty());
    }

    #[test]
    fn deleting_words_and_selections() {
        let mut content = Content::with_text("one two three");
        content.perform(Action::Edit(Edit::DeleteWordBackward));
        assert_eq!(content.text(), "one two ");
        content.perform(Action::Move(Motion::DocumentStart));
        content.perform(Action::Edit(Edit::DeleteWordForward));
        assert_eq!(content.text(), " two ");
        content.perform(Action::SelectAll);
        content.perform(Action::Edit(Edit::Delete));
        assert!(content.is_empty());
        content.perform(Action::Edit(Edit::Backspace));
        assert!(content.is_empty());
        content.perform(Action::Undo);
        assert_eq!(content.text(), " two ");
    }

    #[test]
    fn typing_over_a_selection_takes_its_style() {
        let mut content = Content::with_document(Document::from_blocks([Block::with_runs(
            BlockKind::Paragraph,
            [
                Run::plain("a "),
                Run::new("bold", Style::default().with(Mark::Bold)),
            ],
        )]));
        select(&mut content, at(0, 2), at(0, 6));
        typed(&mut content, "x");
        assert_eq!(
            runs(&content, 0),
            [("a ".into(), false), ("x".into(), true)]
        );
    }

    #[test]
    fn pasting_plain_and_formatted_text() {
        let mut content = Content::with_text("ab");
        content.perform(Action::Move(Motion::To(at(0, 1))));
        content.perform(Action::Edit(Edit::Paste("1\r\n2".into())));
        assert_eq!(content.text(), "a1\n2b");
        assert_eq!(content.selection().head, at(1, 1));
        let fragment = Document::from_blocks([Block::with_runs(
            BlockKind::Paragraph,
            [Run::new("B", Style::default().with(Mark::Bold))],
        )]);
        content.perform(Action::Edit(Edit::PasteFormatted(fragment)));
        assert_eq!(
            runs(&content, 1),
            [("2".into(), false), ("B".into(), true), ("b".into(), false)]
        );
        content.perform(Action::Undo);
        assert_eq!(content.text(), "a1\n2b");
    }

    #[test]
    fn links_apply_to_the_selection_the_word_or_new_text() {
        let mut content = Content::with_text("see here");
        content.perform(Action::Move(Motion::To(at(0, 6))));
        content.perform(Action::Format(Format::Link("example.com".into())));
        assert_eq!(
            content.formatting().link.as_deref(),
            Some("https://example.com")
        );
        let (from, to, _) = content.document().link_at(at(0, 6)).unwrap();
        assert_eq!((from, to), (at(0, 4), at(0, 8)));
        content.perform(Action::Format(Format::Link("https://other.org".into())));
        assert_eq!(
            content.formatting().link.as_deref(),
            Some("https://other.org")
        );
        content.perform(Action::Format(Format::Unlink));
        assert_eq!(content.formatting().link, None);

        content.perform(Action::Move(Motion::DocumentEnd));
        typed(&mut content, " ");
        content.perform(Action::Format(Format::Link("sam@example.com".into())));
        assert_eq!(content.text(), "see here sam@example.com");
        typed(&mut content, "!");
        assert_eq!(
            content
                .document()
                .link_at(at(0, 24))
                .map(|(_, _, url)| url.to_string()),
            Some("mailto:sam@example.com".into())
        );
        assert_eq!(content.document().style_for_insert(at(0, 25)).link, None);

        content.perform(Action::Format(Format::Link("javascript:alert(1)".into())));
        assert_eq!(content.text(), "see here sam@example.com!");
    }

    #[test]
    fn a_new_link_after_a_word_gets_a_space() {
        let mut content = Content::with_text("Notes.");
        content.perform(Action::Format(Format::Link("e.com".into())));
        assert_eq!(content.text(), "Notes. https://e.com");
        assert_eq!(content.document().style_for_insert(at(0, 7)).link, None);
        assert!(content.document().link_at(at(0, 10)).is_some());
        assert!(content.document().link_at(at(0, 6)).is_none());
    }

    #[test]
    fn clear_removes_everything_inline() {
        let mut content = Content::with_text("text");
        content.perform(Action::SelectAll);
        for mark in Mark::ALL {
            content.perform(Action::Format(Format::Toggle(mark)));
        }
        content.perform(Action::Format(Format::Size(Size::Large)));
        content.perform(Action::Format(Format::Link("e.com".into())));
        assert!(content.has_formatting());
        content.perform(Action::Format(Format::Clear));
        assert!(!content.has_formatting());
    }

    #[test]
    fn link_requests_are_counted_without_changing_anything() {
        let mut content = Content::with_text("x");
        content.perform(Action::RequestLink);
        content.perform(Action::RequestLink);
        assert_eq!(content.link_requests(), 2);
        assert_eq!(content.revision(), 0);
        assert!(!Action::RequestLink.is_edit());
        assert!(Action::Undo.is_edit());
    }

    #[test]
    fn selection_actions() {
        let mut content = Content::with_text("one two\nthree");
        content.perform(Action::SelectWord(at(0, 5)));
        assert_eq!(content.selected_text(), "two");
        content.perform(Action::SelectBlock(at(1, 2)));
        assert_eq!(content.selected_text(), "three");
        content.perform(Action::SelectAll);
        assert_eq!(content.selected_text(), "one two\nthree");
        content.perform(Action::Move(Motion::Left));
        assert_eq!(content.selection(), Selection::caret(at(0, 0)));
    }

    #[test]
    fn html_and_plain_text_come_from_the_document() {
        let mut content = Content::with_html("<div>Hi <b>there</b></div><ul><li>item</li></ul>");
        assert_eq!(content.plain_text(), "Hi *there*\n- item");
        assert!(content.html().contains("<strong>there</strong>"));
        content.perform(Action::SelectAll);
        content.perform(Action::Format(Format::Clear));
        assert_eq!(content.plain_text(), "Hi there\n- item");
    }
}
