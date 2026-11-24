//! A character limit for single-line fields.

use std::cell::Cell;

use iced::widget::text_input::Value;

/// Keeps edits to a field within `max` characters.
///
/// The field's message closure passes every edited value through
/// [`Limit::apply`]. Before each event the wrapper records which part of
/// the value an edit would replace, so an insertion that does not fit is
/// cut short rather than pushing text off the end, and afterwards it moves
/// the caret to the end of what was kept.
#[derive(Debug)]
pub(crate) struct Limit {
    max: usize,
    value: String,
    /// Byte range of `value` the next edit replaces: the selection, or an
    /// empty range at the caret.
    range: Cell<(usize, usize)>,
    /// Where the caret belongs after a clamped edit, in graphemes.
    caret: Cell<Option<usize>>,
}

impl Limit {
    pub fn new(max: usize, value: &str) -> Self {
        Self {
            max,
            value: value.to_owned(),
            range: Cell::new((value.len(), value.len())),
            caret: Cell::new(None),
        }
    }

    /// Cuts `text` to the limit, for values that do not come from an edit
    /// at the caret, such as an undo.
    pub fn truncate(&self, text: String) -> String {
        truncate(text, self.max)
    }

    /// Records the part of the value the next edit replaces, from the
    /// field's selection or caret in graphemes.
    pub fn replacing(&self, start: usize, end: usize) {
        let value = Value::new(&self.value);
        let byte = |index: usize| value.select(0, index).to_string().len();
        self.range.set((byte(start), byte(end)));
    }

    /// The edited value, clamped to the limit.
    pub fn apply(&self, edited: String) -> String {
        let clamped = clamp(&self.value, self.range.get(), edited, self.max);
        if let Some(caret) = clamped.caret {
            let head = clamped.text.get(..caret).unwrap_or_default();
            self.caret.set(Some(Value::new(head).len()));
        }
        clamped.text
    }

    /// Where the caret belongs after the last clamped edit, once.
    pub fn take_caret(&self) -> Option<usize> {
        self.caret.take()
    }
}

/// A value after clamping, with the caret's byte offset when an insertion
/// was cut short.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Clamped {
    pub text: String,
    pub caret: Option<usize>,
}

/// Clamps `edited`, the result of replacing the byte range `start..end` of
/// `value`, to `max` characters. An edit that does not grow the value past
/// the limit is kept as it is, so a longer value set by the app can still
/// be shortened. Otherwise the inserted text is cut to what fits.
pub(crate) fn clamp(
    value: &str,
    (start, end): (usize, usize),
    edited: String,
    max: usize,
) -> Clamped {
    let count = edited.chars().count();
    if count <= max || count <= value.chars().count() {
        return Clamped {
            text: edited,
            caret: None,
        };
    }
    let (Some(head), Some(tail)) = (value.get(..start), value.get(end..)) else {
        return cut(edited, max);
    };
    let inserted = edited
        .strip_prefix(head)
        .and_then(|rest| rest.strip_suffix(tail));
    let Some(inserted) = inserted else {
        return cut(edited, max);
    };
    let room = max.saturating_sub(head.chars().count() + tail.chars().count());
    let kept: String = inserted.chars().take(room).collect();
    Clamped {
        caret: Some(head.len() + kept.len()),
        text: format!("{head}{kept}{tail}"),
    }
}

fn cut(edited: String, max: usize) -> Clamped {
    let text = truncate(edited, max);
    Clamped {
        caret: Some(text.len()),
        text,
    }
}

fn truncate(mut text: String, max: usize) -> String {
    if let Some((index, _)) = text.char_indices().nth(max) {
        text.truncate(index);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(value: &str, caret: usize) -> (usize, usize) {
        let byte = value
            .char_indices()
            .nth(caret)
            .map_or(value.len(), |(i, _)| i);
        (byte, byte)
    }

    #[test]
    fn edits_within_the_limit_pass_through() {
        let clamped = clamp("abc", at("abc", 3), "abcd".into(), 5);
        assert_eq!(
            clamped,
            Clamped {
                text: "abcd".into(),
                caret: None
            }
        );
    }

    #[test]
    fn typing_at_the_limit_changes_nothing() {
        let clamped = clamp("abcde", at("abcde", 2), "abXcde".into(), 5);
        assert_eq!(clamped.text, "abcde");
        assert_eq!(clamped.caret, Some(2));
    }

    #[test]
    fn a_paste_is_cut_to_what_fits_and_keeps_the_text_after_the_caret() {
        let clamped = clamp("abc", at("abc", 1), "a12345bc".into(), 5);
        assert_eq!(clamped.text, "a12bc");
        assert_eq!(clamped.caret, Some(3));
    }

    #[test]
    fn a_paste_over_a_selection_frees_its_room() {
        let clamped = clamp("abcde", (1, 4), "a123456e".into(), 5);
        assert_eq!(clamped.text, "a123e");
        assert_eq!(clamped.caret, Some(4));
    }

    #[test]
    fn the_limit_counts_characters_not_bytes() {
        let clamped = clamp("é", at("é", 1), "éàü".into(), 2);
        assert_eq!(clamped.text, "éà");
        assert_eq!(clamped.caret, Some("éà".len()));
    }

    #[test]
    fn a_longer_value_from_the_app_can_shrink_but_not_grow() {
        let deleted = clamp("abcdefg", at("abcdefg", 7), "abcdef".into(), 3);
        assert_eq!(deleted.text, "abcdef");
        let typed = clamp("abcdefg", at("abcdefg", 7), "abcdefgh".into(), 3);
        assert_eq!(typed.text, "abcdefg");
    }

    #[test]
    fn an_edit_that_does_not_match_the_range_is_truncated() {
        let clamped = clamp("abc", at("abc", 0), "zzzzzz".into(), 4);
        assert_eq!(clamped.text, "zzzz");
        assert_eq!(clamped.caret, Some(4));
    }

    #[test]
    fn a_limit_of_zero_keeps_the_field_empty() {
        assert_eq!(clamp("", (0, 0), "a".into(), 0).text, "");
    }

    #[test]
    fn apply_records_the_caret_in_graphemes() {
        let limit = Limit::new(4, "aéb");
        limit.replacing(1, 1);
        assert_eq!(limit.apply("aXYZéb".into()), "aXéb");
        assert_eq!(limit.take_caret(), Some(2));
        assert_eq!(limit.take_caret(), None);
        assert_eq!(limit.truncate("123456".into()), "1234");
    }
}
