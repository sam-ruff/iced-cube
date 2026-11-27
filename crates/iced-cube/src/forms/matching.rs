//! Matching typed text against option labels, shared by the lists that
//! filter as you type.

/// Labels lowercased once, ready for [`filter`].
pub(crate) fn lowercase<T: ToString>(options: &[T]) -> Vec<String> {
    options
        .iter()
        .map(|option| option.to_string().to_lowercase())
        .collect()
}

/// The indices of the `labels` (already lowercase) that contain `query`,
/// ignoring case and surrounding spaces, in their original order.
pub(crate) fn filter(labels: &[String], query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    labels
        .iter()
        .enumerate()
        .filter(|(_, label)| label.contains(&query))
        .map(|(index, _)| index)
        .collect()
}

/// The index of the label equal to `text`, ignoring case and surrounding
/// spaces.
#[cfg(feature = "select")]
pub(crate) fn exact(labels: &[String], text: &str) -> Option<usize> {
    let text = text.trim().to_lowercase();
    labels.iter().position(|label| *label == text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_by_substring_ignoring_case_and_spaces() {
        let labels = lowercase(&["Apple", "Banana", "Blueberry", "Cherry"]);
        assert_eq!(filter(&labels, "b"), vec![1, 2]);
        assert_eq!(filter(&labels, "  RR "), vec![2, 3]);
        assert_eq!(filter(&labels, ""), vec![0, 1, 2, 3]);
        assert!(filter(&labels, "kiwi").is_empty());
    }

    #[cfg(feature = "select")]
    #[test]
    fn exact_matches_a_whole_label() {
        let labels = lowercase(&["09:00", "09:15"]);
        assert_eq!(exact(&labels, " 09:15 "), Some(1));
        assert_eq!(exact(&labels, "09:1"), None);
    }
}
