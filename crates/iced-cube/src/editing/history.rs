//! Undo history for a text field, kept as the changes between steps.

use std::collections::VecDeque;

/// Steps kept for undo. The oldest is dropped beyond this.
pub const LIMIT: usize = 100;

/// An edit about to change the text, as far as grouping cares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Typed text, including a new line and text from an input method.
    Insert(String),
    /// A single character removed with Backspace or Delete.
    Delete,
    /// Anything else, such as a paste, a cut or a word deletion. It is
    /// always a step of its own.
    Other,
}

/// The text and caret, as a byte offset, that undo or redo restores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restore {
    pub text: String,
    pub caret: usize,
}

/// Groups edits into steps and undoes or redoes them.
///
/// The field reports each edit with [`record`](Self::record) before the app
/// applies it, and calls [`settle`](Self::settle) once its text shows the
/// result. Typing groups into words, consecutive deletions group together,
/// and a moved caret or any other edit starts a new step. A text changed by
/// the app rather than by the field clears the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History {
    undo: VecDeque<Change>,
    redo: Vec<Change>,
    open: Option<Open>,
    /// The text after the last settled change, to notice changes by the app.
    known: Option<String>,
    /// Edits reported since the text last settled.
    pending: bool,
    limit: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::with_limit(LIMIT)
    }
}

impl History {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            open: None,
            known: None,
            pending: false,
            limit: limit.max(1),
        }
    }

    #[cfg(test)]
    fn can_undo(&self) -> bool {
        !self.undo.is_empty() || self.open.is_some()
    }

    #[cfg(test)]
    fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Reports an edit the app has not applied yet. `replacing` is whether
    /// it replaces a selection, and `current` reads the text as it is now.
    pub fn record(&mut self, edit: Edit, replacing: bool, current: impl FnOnce() -> String) {
        self.redo.clear();
        let kind = Kind::after(&edit);

        // While earlier edits are unapplied the text is stale, so the edit
        // joins the open step rather than starting one from a wrong base.
        // An edit that would start a new step closes this one instead, so
        // the next edit after the text settles starts afresh.
        if self.pending {
            match &mut self.open {
                Some(open) if !replacing && open.kind.continues(&edit) => open.kind = kind,
                Some(open) => open.kind = Kind::Closed,
                None => {
                    let base = self.known.clone().unwrap_or_default();
                    self.open = Some(Open { kind, base });
                }
            }
            return;
        }
        self.pending = true;

        let current = current();
        if self.known.as_ref().is_some_and(|known| *known != current) {
            self.forget();
        }
        if let Some(open) = &mut self.open
            && !replacing
            && open.kind.continues(&edit)
        {
            open.kind = kind;
            return;
        }
        self.close(&current);
        self.open = Some(Open {
            kind,
            base: current.clone(),
        });
        self.known = Some(current);
    }

    /// Called once the text shows every reported edit.
    pub fn settle(&mut self, current: impl FnOnce() -> String) {
        if !self.pending {
            return;
        }
        self.pending = false;
        self.known = Some(current());
    }

    /// The caret moved, so the next edit starts a new step.
    pub fn interrupt(&mut self) {
        if let Some(open) = &mut self.open {
            open.kind = Kind::Closed;
        }
    }

    /// Steps back, returning what the field should show. `None` when there
    /// is nothing to undo, edits are still unapplied, or the app changed the
    /// text, which also clears the history.
    pub fn undo(&mut self, current: String) -> Option<Restore> {
        if !self.ready(&current) {
            return None;
        }
        self.close(&current);
        let change = self.undo.pop_back()?;
        let Some(restore) = change.revert(&current) else {
            self.forget();
            return None;
        };
        self.redo.push(change);
        Some(self.restoring(restore))
    }

    /// Steps forward again after [`undo`](Self::undo).
    pub fn redo(&mut self, current: String) -> Option<Restore> {
        if !self.ready(&current) {
            return None;
        }
        let change = self.redo.pop()?;
        let Some(restore) = change.apply(&current) else {
            self.forget();
            return None;
        };
        self.undo.push_back(change);
        Some(self.restoring(restore))
    }

    fn ready(&mut self, current: &str) -> bool {
        if self.pending {
            return false;
        }
        if self.known.as_deref().is_some_and(|known| known != current) {
            self.forget();
            return false;
        }
        true
    }

    fn restoring(&mut self, restore: Restore) -> Restore {
        self.open = None;
        self.known = Some(restore.text.clone());
        self.pending = true;
        restore
    }

    fn close(&mut self, current: &str) {
        let Some(open) = self.open.take() else {
            return;
        };
        let Some(change) = Change::between(&open.base, current) else {
            return;
        };
        self.undo.push_back(change);
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
    }

    fn forget(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.open = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Open {
    kind: Kind,
    /// The text before the step began.
    base: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Typing { after_space: bool },
    Deleting,
    Closed,
}

impl Kind {
    fn after(edit: &Edit) -> Self {
        match edit {
            Edit::Insert(text) => Kind::Typing {
                after_space: text.ends_with(char::is_whitespace),
            },
            Edit::Delete => Kind::Deleting,
            Edit::Other => Kind::Closed,
        }
    }

    fn continues(self, edit: &Edit) -> bool {
        match (self, edit) {
            // A word typed after a space is a new step.
            (Kind::Typing { after_space }, Edit::Insert(text)) => {
                !after_space || text.starts_with(char::is_whitespace)
            }
            (Kind::Deleting, Edit::Delete) => true,
            _ => false,
        }
    }
}

/// One step: `before` at byte `at` became `after`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Change {
    at: usize,
    before: String,
    after: String,
}

impl Change {
    fn between(old: &str, new: &str) -> Option<Self> {
        if old == new {
            return None;
        }
        let prefix = common_prefix(old, new);
        let suffix = common_suffix(&old[prefix..], &new[prefix..]);
        Some(Self {
            at: prefix,
            before: old[prefix..old.len() - suffix].to_owned(),
            after: new[prefix..new.len() - suffix].to_owned(),
        })
    }

    fn revert(&self, current: &str) -> Option<Restore> {
        replace(current, self.at, &self.after, &self.before)
    }

    fn apply(&self, current: &str) -> Option<Restore> {
        replace(current, self.at, &self.before, &self.after)
    }
}

/// Replaces `old` at `at` in `text` with `new`, leaving the caret after it.
fn replace(text: &str, at: usize, old: &str, new: &str) -> Option<Restore> {
    let head = text.get(..at)?;
    let rest = text.get(at..)?;
    let tail = rest.strip_prefix(old)?;
    Some(Restore {
        text: [head, new, tail].concat(),
        caret: at + new.len(),
    })
}

fn common_prefix(a: &str, b: &str) -> usize {
    a.char_indices()
        .zip(b.chars())
        .find(|((_, x), y)| x != y)
        .map_or(a.len().min(b.len()), |((i, _), _)| i)
}

fn common_suffix(a: &str, b: &str) -> usize {
    a.chars()
        .rev()
        .zip(b.chars().rev())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x.len_utf8())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives a history the way a field does: each edit is applied and
    /// settled before the next.
    struct Field {
        text: String,
        history: History,
    }

    impl Field {
        fn new(text: &str) -> Self {
            Self {
                text: text.to_owned(),
                history: History::default(),
            }
        }

        fn edit(&mut self, edit: Edit, result: &str) {
            let text = self.text.clone();
            self.history.record(edit, false, || text);
            self.text = result.to_owned();
            self.history.settle(|| result.to_owned());
        }

        fn type_text(&mut self, typed: &str) {
            for c in typed.chars() {
                let result = format!("{}{c}", self.text);
                self.edit(Edit::Insert(c.to_string()), &result);
            }
        }

        fn backspace(&mut self) {
            let mut result = self.text.clone();
            result.pop();
            self.edit(Edit::Delete, &result);
        }

        fn undo(&mut self) -> Option<usize> {
            let restore = self.history.undo(self.text.clone())?;
            self.text = restore.text.clone();
            self.history.settle(|| restore.text);
            Some(restore.caret)
        }

        fn redo(&mut self) -> Option<usize> {
            let restore = self.history.redo(self.text.clone())?;
            self.text = restore.text.clone();
            self.history.settle(|| restore.text);
            Some(restore.caret)
        }
    }

    #[test]
    fn typing_groups_into_words() {
        let mut field = Field::new("");
        field.type_text("one two  three");
        assert!(field.undo().is_some());
        assert_eq!(field.text, "one two  ");
        field.undo();
        assert_eq!(field.text, "one ");
        field.undo();
        assert_eq!(field.text, "");
        assert_eq!(field.undo(), None);
    }

    #[test]
    fn redo_walks_forward_and_returns_the_caret() {
        let mut field = Field::new("");
        field.type_text("ab cd");
        field.undo();
        field.undo();
        assert_eq!(field.redo(), Some(3));
        assert_eq!(field.text, "ab ");
        assert_eq!(field.redo(), Some(5));
        assert_eq!(field.text, "ab cd");
        assert_eq!(field.redo(), None);
    }

    #[test]
    fn deletions_group_and_split_from_typing() {
        let mut field = Field::new("");
        field.type_text("abcd");
        field.backspace();
        field.backspace();
        field.type_text("x");
        assert_eq!(field.text, "abx");
        field.undo();
        assert_eq!(field.text, "ab");
        field.undo();
        assert_eq!(field.text, "abcd");
        field.undo();
        assert_eq!(field.text, "");
    }

    #[test]
    fn other_edits_are_steps_of_their_own() {
        let mut field = Field::new("");
        field.type_text("a");
        field.edit(Edit::Other, "a pasted");
        field.edit(Edit::Other, "a pasted twice");
        field.undo();
        assert_eq!(field.text, "a pasted");
        field.undo();
        assert_eq!(field.text, "a");
    }

    #[test]
    fn replacing_a_selection_starts_a_step() {
        let mut field = Field::new("");
        field.type_text("abc");
        let text = field.text.clone();
        field
            .history
            .record(Edit::Insert("x".into()), true, || text);
        field.text = "x".into();
        field.history.settle(|| "x".into());
        field.undo();
        assert_eq!(field.text, "abc");
    }

    #[test]
    fn interrupting_starts_a_step() {
        let mut field = Field::new("");
        field.type_text("ab");
        field.history.interrupt();
        field.type_text("c");
        field.undo();
        assert_eq!(field.text, "ab");
    }

    #[test]
    fn a_new_edit_clears_redo() {
        let mut field = Field::new("");
        field.type_text("ab cd");
        field.undo();
        field.type_text("x");
        assert_eq!(field.redo(), None);
        assert!(!field.history.can_redo());
    }

    #[test]
    fn a_change_by_the_app_clears_the_history() {
        let mut field = Field::new("");
        field.type_text("draft");
        field.text = "loaded".into();
        assert_eq!(field.undo(), None);
        assert!(!field.history.can_undo());
        field.type_text("!");
        field.undo();
        assert_eq!(field.text, "loaded");
    }

    #[test]
    fn unapplied_edits_join_the_open_step_and_block_undo() {
        let mut history = History::default();
        history.record(Edit::Insert("a".into()), false, String::new);
        // The text is not read again until the first edit settles.
        history.record(Edit::Insert(" ".into()), false, || "stale".into());
        history.record(Edit::Insert("b".into()), false, || "stale".into());
        assert_eq!(history.undo("a b".into()), None);
        history.settle(|| "a b".into());
        let restore = history.undo("a b".into());
        assert_eq!(restore.map(|r| r.text), Some(String::new()));
    }

    #[test]
    fn words_typed_in_one_frame_do_not_swallow_the_next_frame() {
        let mut history = History::default();
        for c in "one two".chars() {
            history.record(Edit::Insert(c.to_string()), false, String::new);
        }
        history.settle(|| "one two".into());
        for c in " three".chars() {
            history.record(Edit::Insert(c.to_string()), false, || "one two".into());
        }
        history.settle(|| "one two three".into());
        let restore = history.undo("one two three".into());
        assert_eq!(restore.map(|r| r.text).as_deref(), Some("one two"));
    }

    #[test]
    fn a_frame_of_one_word_still_joins_the_word_before() {
        let mut history = History::default();
        history.record(Edit::Insert("o".into()), false, String::new);
        history.settle(|| "o".into());
        for c in "ne".chars() {
            history.record(Edit::Insert(c.to_string()), false, || "o".into());
        }
        history.settle(|| "one".into());
        let restore = history.undo("one".into());
        assert_eq!(restore.map(|r| r.text).as_deref(), Some(""));
    }

    #[test]
    fn the_history_is_bounded() {
        let mut field = Field::new("");
        field.history = History::with_limit(3);
        field.type_text("a b c d e ");
        let mut steps = 0;
        while field.undo().is_some() {
            steps += 1;
        }
        assert_eq!(steps, 3);
        assert_eq!(field.text, "a b ");
    }

    #[test]
    fn changes_keep_only_the_edited_region() {
        let change = Change::between("hello world", "hello brave world");
        assert_eq!(
            change,
            Some(Change {
                at: 6,
                before: String::new(),
                after: "brave ".into(),
            })
        );
        assert_eq!(Change::between("same", "same"), None);
    }

    #[test]
    fn changes_respect_multibyte_characters() {
        let Some(change) = Change::between("café", "cafè") else {
            panic!("a change");
        };
        assert_eq!(change.before, "é");
        assert_eq!(change.after, "è");
        assert_eq!(
            change.revert("cafè"),
            Some(Restore {
                text: "café".into(),
                caret: 5,
            })
        );
    }

    #[test]
    fn a_change_that_no_longer_fits_is_refused() {
        let change = Change {
            at: 10,
            before: "x".into(),
            after: "y".into(),
        };
        assert_eq!(change.revert("short"), None);
        assert_eq!(change.apply("short"), None);
    }
}
