//! The pure rules behind the open list: moving the highlight, finding an
//! option by typing its first letters, and the scroll and scrollbar
//! geometry.

use iced::keyboard::key::Named;
use iced::time::{Duration, Instant};

/// The position highlighted after `key` among `count` shown rows, from
/// `current`. Arrows move one row and Page Up and Page Down move `page`
/// rows, stopping at the ends; Home and End jump to them.
pub(crate) fn step(current: Option<usize>, count: usize, page: usize, key: Named) -> Option<usize> {
    let last = count.checked_sub(1)?;
    let page = page.max(1);
    let next = match (key, current) {
        (Named::ArrowDown | Named::PageDown, None) | (Named::Home, _) => 0,
        (Named::ArrowUp | Named::PageUp, None) | (Named::End, _) => last,
        (Named::ArrowDown, Some(index)) => index + 1,
        (Named::ArrowUp, Some(index)) => index.saturating_sub(1),
        (Named::PageDown, Some(index)) => index.saturating_add(page),
        (Named::PageUp, Some(index)) => index.saturating_sub(page),
        _ => return current.map(|index| index.min(last)),
    };
    Some(next.min(last))
}

/// Whether a named key moves the highlight.
pub(crate) fn moves(key: Named) -> bool {
    matches!(
        key,
        Named::ArrowDown
            | Named::ArrowUp
            | Named::PageDown
            | Named::PageUp
            | Named::Home
            | Named::End
    )
}

/// The letters typed into a list with neither search nor custom values,
/// which move the highlight to the first option starting with them.
///
/// Key presses carry no time, so the list never reads the clock: the last
/// letter is [stamped](Self::stamp) with the time of the next frame drawn,
/// and the frame asked for a pause later tells the list the pause is over.
/// Without any frame times, as in tests, letters always add up.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TypeAhead {
    buffer: String,
    last: Option<Instant>,
    unstamped: bool,
}

impl TypeAhead {
    /// Whether a letter typed when the latest frame was drawn at `now`
    /// adds to the current buffer.
    pub(crate) fn is_typing(&self, now: Option<Instant>, pause: Duration) -> bool {
        if self.buffer.is_empty() {
            return false;
        }
        match (self.last, now) {
            (Some(last), Some(now)) if !self.unstamped => {
                now.saturating_duration_since(last) < pause
            }
            _ => true,
        }
    }

    /// Gives the last letter the time of the frame drawn after it. Returns
    /// the time it was stamped with, once per letter.
    pub(crate) fn stamp(&mut self, now: Instant) -> Option<Instant> {
        if !self.unstamped {
            return None;
        }
        self.unstamped = false;
        self.last = Some(now);
        Some(now)
    }

    /// Adds `text` and returns the position in `labels` (lowercase, as
    /// shown) to highlight, if any starts with the letters. Typing the same
    /// letter again cycles through the options starting with it. A pause
    /// of `pause` or more starts a new search.
    pub(crate) fn type_text(
        &mut self,
        text: &str,
        now: Option<Instant>,
        pause: Duration,
        labels: &[&str],
        current: Option<usize>,
    ) -> Option<usize> {
        if !self.is_typing(now, pause) {
            self.buffer.clear();
        }
        self.buffer.push_str(&text.to_lowercase());
        self.unstamped = true;

        let mut letters = self.buffer.chars();
        let first = letters.next()?;
        let repeated = letters.all(|letter| letter == first);
        let (prefix, start) = if repeated {
            (first.to_string(), current.map_or(0, |index| index + 1))
        } else {
            (self.buffer.clone(), current.unwrap_or(0))
        };
        let count = labels.len();
        (0..count)
            .map(|offset| (start + offset) % count)
            .find(|&index| labels[index].starts_with(&prefix))
    }

    pub(crate) fn clear(&mut self) {
        self.buffer.clear();
        self.last = None;
        self.unstamped = false;
    }
}

/// `offset` kept between the top of the content and the last full view.
pub(crate) fn clamp_offset(offset: f32, content: f32, viewport: f32) -> f32 {
    offset.min((content - viewport).max(0.0)).max(0.0)
}

/// The offset that puts a row spanning `top` to `bottom` in the middle of
/// the viewport, as far as the ends of the content allow.
pub(crate) fn centred_offset(top: f32, bottom: f32, content: f32, viewport: f32) -> f32 {
    let middle = (top + bottom) / 2.0;
    clamp_offset(middle - viewport / 2.0, content, viewport)
}

/// The shortest the scrollbar's thumb gets, so it stays easy to grab.
pub(crate) const MIN_THUMB: f32 = 24.0;

/// Where the thumb sits along a `track` beside the viewport: its top and
/// height. `None` when everything fits and there is no scrollbar.
pub(crate) fn thumb(offset: f32, content: f32, viewport: f32, track: f32) -> Option<(f32, f32)> {
    if content <= viewport || viewport <= 0.0 || track <= 0.0 {
        return None;
    }
    let height = (track * viewport / content).clamp(MIN_THUMB.min(track), track);
    let travel = track - height;
    let scrollable = content - viewport;
    let top = travel * clamp_offset(offset, content, viewport) / scrollable;
    Some((top, height))
}

/// The offset for a thumb whose top is dragged to `top` along the track.
pub(crate) fn offset_for_thumb(top: f32, content: f32, viewport: f32, track: f32) -> f32 {
    let Some((_, height)) = thumb(0.0, content, viewport, track) else {
        return 0.0;
    };
    let travel = track - height;
    if travel <= 0.0 {
        return 0.0;
    }
    clamp_offset(top / travel * (content - viewport), content, viewport)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_and_pages_stop_at_the_ends() {
        assert_eq!(step(None, 10, 4, Named::ArrowDown), Some(0));
        assert_eq!(step(None, 10, 4, Named::ArrowUp), Some(9));
        assert_eq!(step(Some(9), 10, 4, Named::ArrowDown), Some(9));
        assert_eq!(step(Some(0), 10, 4, Named::ArrowUp), Some(0));
        assert_eq!(step(Some(2), 10, 4, Named::PageDown), Some(6));
        assert_eq!(step(Some(8), 10, 4, Named::PageDown), Some(9));
        assert_eq!(step(Some(6), 10, 4, Named::PageUp), Some(2));
        assert_eq!(step(Some(2), 10, 4, Named::PageUp), Some(0));
        assert_eq!(step(None, 10, 4, Named::PageDown), Some(0));
        assert_eq!(step(None, 10, 4, Named::PageUp), Some(9));
        assert_eq!(step(Some(5), 10, 4, Named::Home), Some(0));
        assert_eq!(step(Some(5), 10, 4, Named::End), Some(9));
    }

    #[test]
    fn other_keys_and_empty_lists_leave_the_highlight() {
        assert_eq!(step(Some(1), 3, 4, Named::Tab), Some(1));
        assert_eq!(step(Some(7), 3, 4, Named::Tab), Some(2), "kept in range");
        assert_eq!(step(None, 0, 4, Named::ArrowDown), None);
        assert_eq!(
            step(Some(1), 10, 0, Named::PageDown),
            Some(2),
            "a page is a row at least"
        );
        assert!(moves(Named::PageUp) && !moves(Named::Enter));
    }

    const DAYS: [&str; 7] = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ];

    fn at(seconds: u64) -> Option<Instant> {
        // Fixed points after one start, so the test never reads the clock twice.
        thread_local!(static START: Instant = Instant::now());
        Some(START.with(|start| *start + Duration::from_secs(seconds)))
    }

    const PAUSE: Duration = Duration::from_secs(1);

    fn stamp(typed: &mut TypeAhead, now: Option<Instant>) {
        if let Some(now) = now {
            let _ = typed.stamp(now);
        }
    }

    #[test]
    fn letters_typed_together_find_the_first_option_starting_with_them() {
        let mut typed = TypeAhead::default();
        assert_eq!(typed.type_text("T", at(0), PAUSE, &DAYS, None), Some(1));
        stamp(&mut typed, at(0));
        assert_eq!(typed.type_text("h", at(0), PAUSE, &DAYS, Some(1)), Some(3));
        assert!(typed.is_typing(at(0), PAUSE), "until stamped");
    }

    #[test]
    fn without_frame_times_letters_always_add_up() {
        let mut typed = TypeAhead::default();
        assert_eq!(typed.type_text("t", None, PAUSE, &DAYS, None), Some(1));
        assert_eq!(typed.type_text("h", None, PAUSE, &DAYS, Some(1)), Some(3));
    }

    #[test]
    fn a_letter_is_stamped_once_by_the_next_frame() {
        let mut typed = TypeAhead::default();
        let _ = typed.type_text("t", at(0), PAUSE, &DAYS, None);
        assert!(
            typed.is_typing(at(9), PAUSE),
            "a frame from before it is ignored"
        );
        let Some(frame) = at(3) else {
            panic!("a fixed time");
        };
        assert_eq!(typed.stamp(frame), Some(frame));
        assert_eq!(typed.stamp(frame), None);
    }

    #[test]
    fn the_same_letter_again_cycles_and_wraps() {
        let mut typed = TypeAhead::default();
        assert_eq!(typed.type_text("s", at(0), PAUSE, &DAYS, None), Some(5));
        assert_eq!(typed.type_text("s", at(0), PAUSE, &DAYS, Some(5)), Some(6));
        assert_eq!(typed.type_text("s", at(0), PAUSE, &DAYS, Some(6)), Some(5));
    }

    #[test]
    fn a_pause_starts_a_new_search_and_no_match_keeps_the_highlight() {
        let mut typed = TypeAhead::default();
        assert_eq!(typed.type_text("t", at(0), PAUSE, &DAYS, None), Some(1));
        stamp(&mut typed, at(0));
        assert!(!typed.is_typing(at(1), PAUSE), "a whole pause has passed");
        assert_eq!(typed.type_text("f", at(5), PAUSE, &DAYS, Some(1)), Some(4));
        assert_eq!(typed.type_text("x", at(5), PAUSE, &DAYS, Some(4)), None);
        typed.clear();
        assert!(!typed.is_typing(at(5), PAUSE));
        assert_eq!(typed.type_text("x", at(9), PAUSE, &[], None), None);
    }

    #[test]
    fn offsets_stay_within_the_content() {
        assert_eq!(clamp_offset(-5.0, 1000.0, 200.0), 0.0);
        assert_eq!(clamp_offset(900.0, 1000.0, 200.0), 800.0);
        assert_eq!(clamp_offset(50.0, 100.0, 200.0), 0.0, "content that fits");
    }

    #[test]
    fn centring_puts_the_row_in_the_middle_unless_near_an_end() {
        assert_eq!(centred_offset(480.0, 512.0, 3072.0, 256.0), 368.0);
        assert_eq!(centred_offset(0.0, 32.0, 3072.0, 256.0), 0.0);
        assert_eq!(centred_offset(3040.0, 3072.0, 3072.0, 256.0), 2816.0);
    }

    #[test]
    fn the_thumb_shows_how_much_is_in_view_and_where() {
        assert_eq!(thumb(0.0, 200.0, 200.0, 200.0), None);
        assert_eq!(thumb(0.0, 400.0, 200.0, 200.0), Some((0.0, 100.0)));
        assert_eq!(thumb(200.0, 400.0, 200.0, 200.0), Some((100.0, 100.0)));
        assert_eq!(thumb(200.0, 400.0, 200.0, 100.0), Some((50.0, 50.0)));
        let (_, height) = thumb(0.0, 100_000.0, 200.0, 200.0).unwrap_or_default();
        assert_eq!(height, MIN_THUMB, "never thinner than the minimum");
    }

    #[test]
    fn dragging_the_thumb_maps_back_to_an_offset() {
        assert_eq!(offset_for_thumb(50.0, 400.0, 200.0, 200.0), 100.0);
        assert_eq!(offset_for_thumb(500.0, 400.0, 200.0, 200.0), 200.0);
        assert_eq!(offset_for_thumb(-10.0, 400.0, 200.0, 200.0), 0.0);
        assert_eq!(
            offset_for_thumb(50.0, 100.0, 200.0, 200.0),
            0.0,
            "no scrollbar"
        );
    }
}
