//! Calendar entries and the collection that holds them.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

use super::repeat::{self, Repeat};
use crate::icon::Glyph;

/// Identifies an entry. The app chooses the number, usually its own
/// record id, or takes a free one from [`Entries::next_id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryId(pub u64);

/// The colour of an entry, taken from the theme's tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tone {
    #[default]
    Primary,
    Neutral,
    Success,
    Warning,
    Destructive,
}

impl Tone {
    pub const ALL: [Tone; 5] = [
        Tone::Primary,
        Tone::Neutral,
        Tone::Success,
        Tone::Warning,
        Tone::Destructive,
    ];
}

pub use super::rules::{When, add_days, date_of, instant, midnight, next_day};

/// How strongly an entry is drawn, to show its status at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Emphasis {
    /// A wash of its tone with a solid stripe.
    #[default]
    Solid,
    /// A border in its tone on no fill, such as for a tentative or
    /// unanswered invitation.
    Outlined,
    /// Its title struck through, such as for a declined invitation.
    Struck,
    /// Drawn faintly, such as for something cancelled.
    Faded,
    /// Diagonal stripes in its tone, such as for private or busy time.
    Hatched,
}

impl Emphasis {
    pub const ALL: [Emphasis; 5] = [
        Emphasis::Solid,
        Emphasis::Outlined,
        Emphasis::Struck,
        Emphasis::Faded,
        Emphasis::Hatched,
    ];
}

/// An entry's icon and what it means.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Marker {
    glyph: Option<Glyph>,
    label: Option<String>,
}

/// Something on the calendar: a meeting, a deadline, a holiday.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    id: EntryId,
    title: String,
    when: When,
    tone: Tone,
    tag: Option<String>,
    repeat: Option<Repeat>,
    location: Option<String>,
    notes: Option<String>,
    /// Boxed, as most entries have none and events carry entries by value.
    marker: Option<Box<Marker>>,
    emphasis: Emphasis,
    locked: bool,
}

/// `text` without the space around it, or `None` when nothing is left.
fn filled(text: String) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_owned())
}

impl Entry {
    /// A timed entry from `start` up to `end`.
    pub fn new(id: EntryId, title: impl Into<String>, start: Timestamp, end: Timestamp) -> Self {
        Self::at(id, title, When::timed(start, end))
    }

    /// An entry covering whole days, `first` to `last` inclusive.
    pub fn all_day(id: EntryId, title: impl Into<String>, first: Date, last: Date) -> Self {
        Self::at(id, title, When::all_day(first, last))
    }

    /// An entry covering `when`.
    pub fn at(id: EntryId, title: impl Into<String>, when: When) -> Self {
        Self {
            id,
            title: title.into(),
            when,
            tone: Tone::default(),
            tag: None,
            repeat: None,
            location: None,
            notes: None,
            marker: None,
            emphasis: Emphasis::Solid,
            locked: false,
        }
    }

    /// A small glyph in the muted text colour, drawn before the time or
    /// title in every view and never cut off, such as a mail icon for an
    /// unanswered invitation.
    pub fn icon(mut self, glyph: Glyph) -> Self {
        self.marker.get_or_insert_default().glyph = Some(glyph);
        self
    }

    /// What the icon means, shown as a tooltip on it, such as "Team
    /// entry". Blank text clears it.
    pub fn icon_label(mut self, label: impl Into<String>) -> Self {
        self.marker.get_or_insert_default().label = filled(label.into());
        self
    }

    pub fn icon_label_of(&self) -> Option<&str> {
        self.marker.as_ref()?.label.as_deref()
    }

    pub fn emphasis(mut self, emphasis: Emphasis) -> Self {
        self.emphasis = emphasis;
        self
    }

    /// A locked entry can be selected and opened, but the calendar's own
    /// gestures and shortcuts never move, resize or remove it. The app
    /// can still change it with [`Event::Insert`](super::Event::Insert).
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Where the entry takes place. Blank text clears it.
    pub fn location(mut self, location: impl Into<String>) -> Self {
        self.location = filled(location.into());
        self
    }

    /// Free text kept with the entry, which may run over several lines.
    /// Blank text clears it.
    pub fn notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = filled(notes.into());
        self
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// A label that triggers can be limited to, such as `"work"`. See
    /// [`Scope::Tagged`](super::schedule::Scope::Tagged).
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Makes the entry repeat. Every occurrence keeps the length of the
    /// first.
    pub fn repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = Some(repeat);
        self
    }

    /// Replaces the title.
    pub fn titled(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Moves the entry. A repeating entry moves as a whole, since `when` is
    /// its first occurrence.
    pub fn rescheduled(mut self, when: When) -> Self {
        self.when = when;
        self
    }

    pub fn id(&self) -> EntryId {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    /// When the entry, or its first occurrence, happens.
    pub fn when(&self) -> When {
        self.when
    }

    pub fn tone_of(&self) -> Tone {
        self.tone
    }

    pub fn tag_of(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    pub fn repeat_of(&self) -> Option<Repeat> {
        self.repeat
    }

    pub fn location_of(&self) -> Option<&str> {
        self.location.as_deref()
    }

    pub fn notes_of(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    pub fn is_all_day(&self) -> bool {
        self.when.is_all_day()
    }

    pub fn icon_of(&self) -> Option<Glyph> {
        self.marker.as_ref()?.glyph
    }

    pub fn emphasis_of(&self) -> Emphasis {
        self.emphasis
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

/// One time an entry happens. An entry that does not repeat has a single
/// occurrence with index 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Occurrence {
    pub entry: EntryId,
    /// Which repetition this is, counted from 0.
    pub index: u32,
    pub start: Timestamp,
    /// The instant the occurrence stops at, which is not part of it.
    pub end: Timestamp,
    pub all_day: bool,
}

impl Occurrence {
    pub fn duration(&self) -> SignedDuration {
        self.end.duration_since(self.start)
    }

    /// The first and last calendar dates the occurrence touches.
    pub fn dates(&self, time_zone: &TimeZone) -> (Date, Date) {
        let first = date_of(self.start, time_zone);
        if self.end <= self.start {
            return (first, first);
        }
        let last_instant = self
            .end
            .saturating_sub(SignedDuration::from_nanos(1))
            .unwrap_or(self.end);
        (first, date_of(last_instant, time_zone).max(first))
    }
}

/// A span the user asked for by clicking or dragging on empty space. The
/// app turns it into an [`Entry`], usually after asking for a title.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Draft {
    pub when: When,
}

/// What happened to an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Change {
    Created,
    Updated,
    Removed,
}

impl Change {
    pub const ALL: [Change; 3] = [Change::Created, Change::Updated, Change::Removed];
}

/// A change made to [`Entries`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    pub change: Change,
    /// The entry as it is now, or as it was when removed.
    pub entry: Entry,
    /// The entry before an update.
    pub before: Option<Entry>,
}

/// The entries of a calendar and the time zone they are shown in.
///
/// Two collections are equal when they hold the same entries in the same
/// time zone, whatever ids they have handed out.
#[derive(Debug, Clone)]
pub struct Entries {
    items: Vec<Entry>,
    time_zone: TimeZone,
    /// One more than the highest id ever held, kept after removals.
    next: u64,
}

impl PartialEq for Entries {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items && self.time_zone == other.time_zone
    }
}

impl Eq for Entries {}

impl Default for Entries {
    fn default() -> Self {
        Self::utc()
    }
}

impl Entries {
    /// An empty collection shown in `time_zone`.
    pub fn new(time_zone: TimeZone) -> Self {
        Self {
            items: Vec::new(),
            time_zone,
            next: 0,
        }
    }

    /// An empty collection shown in UTC.
    pub fn utc() -> Self {
        Self::new(TimeZone::UTC)
    }

    /// An empty collection shown in the system's time zone, or UTC when
    /// the system does not say.
    pub fn system() -> Self {
        Self::new(TimeZone::system())
    }

    /// Adds entries while building. An entry with an id already present
    /// replaces the earlier one.
    pub fn with(mut self, entries: impl IntoIterator<Item = Entry>) -> Self {
        for entry in entries {
            let _ = self.insert(entry);
        }
        self
    }

    pub fn time_zone(&self) -> &TimeZone {
        &self.time_zone
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.items.iter()
    }

    pub fn get(&self, id: EntryId) -> Option<&Entry> {
        self.items.iter().find(|entry| entry.id == id)
    }

    /// An id no entry in this collection has ever used, including entries
    /// since removed, so ids kept outside the calendar never point at a
    /// different entry. Ids only go up.
    pub fn next_id(&self) -> EntryId {
        EntryId(self.next)
    }

    /// Adds `entry`, or replaces the entry with the same id. Returns what
    /// changed, or `None` when an identical entry was already there.
    pub fn insert(&mut self, entry: Entry) -> Option<Changed> {
        self.next = self.next.max(entry.id.0.saturating_add(1));
        let Some(existing) = self.items.iter_mut().find(|item| item.id == entry.id) else {
            self.items.push(entry.clone());
            return Some(Changed {
                change: Change::Created,
                entry,
                before: None,
            });
        };
        if *existing == entry {
            return None;
        }
        let before = std::mem::replace(existing, entry.clone());
        Some(Changed {
            change: Change::Updated,
            entry,
            before: Some(before),
        })
    }

    /// Removes the entry with `id`, if there is one.
    pub fn remove(&mut self, id: EntryId) -> Option<Changed> {
        let index = self.items.iter().position(|entry| entry.id == id)?;
        Some(Changed {
            change: Change::Removed,
            entry: self.items.remove(index),
            before: None,
        })
    }

    /// The calendar date of `instant` in the collection's time zone.
    pub fn date_of(&self, instant: Timestamp) -> Date {
        date_of(instant, &self.time_zone)
    }

    /// Midnight at the start of `date` in the collection's time zone.
    pub fn midnight(&self, date: Date) -> Timestamp {
        midnight(date, &self.time_zone)
    }

    /// Every occurrence that overlaps the time from `from` up to `to`,
    /// earliest first. Repeating entries are expanded.
    pub fn occurrences(&self, from: Timestamp, to: Timestamp) -> Vec<Occurrence> {
        let mut found: Vec<Occurrence> = self
            .items
            .iter()
            .flat_map(|entry| repeat::occurrences(entry, &self.time_zone, from, to))
            .collect();
        found.sort_by_key(|occurrence| (occurrence.start, occurrence.entry));
        found
    }

    /// Every occurrence that touches a date from `first` to `last`
    /// inclusive.
    pub fn between(&self, first: Date, last: Date) -> Vec<Occurrence> {
        self.occurrences(self.midnight(first), self.midnight(next_day(last)))
    }

    /// Every occurrence that touches `date`.
    pub fn on(&self, date: Date) -> Vec<Occurrence> {
        self.between(date, date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn at(day: i8, hour: i8) -> Timestamp {
        date(2026, 11, day)
            .at(hour, 0, 0, 0)
            .to_zoned_datetime(TimeZone::UTC)
            .map(|zoned| zoned.timestamp())
            .unwrap_or_default()
    }

    fn meeting(id: u64, day: i8, hour: i8) -> Entry {
        Entry::new(EntryId(id), "Meeting", at(day, hour), at(day, hour + 1))
    }

    #[test]
    fn days_are_added_and_stop_at_the_ends_of_time() {
        let day = date(2026, 11, 3);
        assert_eq!(add_days(day, 1), date(2026, 11, 4));
        assert_eq!(add_days(day, -3), date(2026, 10, 31));
        assert_eq!(add_days(day, 0), day);
        assert_eq!(add_days(day, i32::MAX), Date::MAX);
        assert_eq!(add_days(day, i32::MIN), Date::MIN);
        assert_eq!(add_days(Date::MAX, 1), Date::MAX);
        assert_eq!(next_day(date(2026, 12, 31)), date(2027, 1, 1));
    }

    #[test]
    fn spans_are_put_in_order() {
        assert_eq!(
            When::timed(at(3, 10), at(3, 9)),
            When::Timed {
                start: at(3, 9),
                end: at(3, 10)
            }
        );
        assert_eq!(
            When::all_day(date(2026, 11, 5), date(2026, 11, 3)),
            When::AllDay {
                first: date(2026, 11, 3),
                last: date(2026, 11, 5)
            }
        );
    }

    #[test]
    fn whole_days_run_from_midnight_to_the_midnight_after_the_last_day() {
        let when = When::all_day(date(2026, 11, 3), date(2026, 11, 4));
        assert!(when.is_all_day());
        assert_eq!(when.start(&TimeZone::UTC), at(3, 0));
        assert_eq!(when.end(&TimeZone::UTC), at(5, 0));
        assert_eq!(When::day(date(2026, 11, 3)).end(&TimeZone::UTC), at(4, 0));
    }

    #[test]
    fn whole_days_follow_the_time_zone() {
        let Ok(tokyo) = TimeZone::get("Asia/Tokyo") else {
            panic!("the time zone database is bundled for tests");
        };
        let when = When::day(date(2026, 11, 3));
        // Midnight in Tokyo is 15:00 the day before in UTC.
        assert_eq!(when.start(&tokyo), at(2, 15));
        assert_eq!(date_of(at(2, 15), &tokyo), date(2026, 11, 3));
        assert_eq!(date_of(at(2, 14), &tokyo), date(2026, 11, 2));
    }

    #[test]
    fn entry_builder_defaults_and_options() {
        let entry = meeting(1, 3, 9);
        assert_eq!(entry.id(), EntryId(1));
        assert_eq!(entry.title(), "Meeting");
        assert_eq!(entry.tone_of(), Tone::Primary);
        assert_eq!(entry.tag_of(), None);
        assert_eq!(entry.repeat_of(), None);
        assert!(!entry.is_all_day());

        let entry = entry
            .tone(Tone::Warning)
            .tag("work")
            .repeat(Repeat::daily())
            .titled("Review")
            .rescheduled(When::day(date(2026, 11, 9)));
        assert_eq!(entry.tone_of(), Tone::Warning);
        assert_eq!(entry.tag_of(), Some("work"));
        assert_eq!(entry.repeat_of(), Some(Repeat::daily()));
        assert_eq!(entry.title(), "Review");
        assert!(entry.is_all_day());
    }

    #[test]
    fn location_and_notes_are_trimmed_and_blank_clears_them() {
        let entry = meeting(1, 3, 9);
        assert_eq!(entry.location_of(), None);
        assert_eq!(entry.notes_of(), None);

        let entry = entry
            .location("  Room 4 ")
            .notes("Bring the plan\nand the figures\n");
        assert_eq!(entry.location_of(), Some("Room 4"));
        assert_eq!(entry.notes_of(), Some("Bring the plan\nand the figures"));

        let entry = entry.location("   ").notes("");
        assert_eq!(entry.location_of(), None);
        assert_eq!(entry.notes_of(), None);
    }

    #[test]
    fn insert_reports_created_updated_and_unchanged() {
        let mut entries = Entries::utc();
        let created = entries.insert(meeting(1, 3, 9));
        assert_eq!(created.map(|changed| changed.change), Some(Change::Created));

        assert_eq!(entries.insert(meeting(1, 3, 9)), None);

        let updated = entries.insert(meeting(1, 4, 9));
        let Some(updated) = updated else {
            panic!("a moved entry is an update");
        };
        assert_eq!(updated.change, Change::Updated);
        assert_eq!(updated.before, Some(meeting(1, 3, 9)));
        assert_eq!(updated.entry, meeting(1, 4, 9));
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn remove_hands_back_the_entry() {
        let mut entries = Entries::utc().with([meeting(1, 3, 9), meeting(2, 4, 9)]);
        let removed = entries.remove(EntryId(1));
        assert_eq!(
            removed.map(|changed| (changed.change, changed.entry)),
            Some((Change::Removed, meeting(1, 3, 9)))
        );
        assert_eq!(entries.remove(EntryId(1)), None);
        assert_eq!(entries.len(), 1);
        assert!(entries.get(EntryId(2)).is_some());
    }

    #[test]
    fn next_id_is_unused() {
        let mut entries = Entries::utc();
        assert!(entries.is_empty());
        assert_eq!(entries.next_id(), EntryId(0));
        let _ = entries.insert(meeting(7, 3, 9));
        assert_eq!(entries.next_id(), EntryId(8));
        let _ = entries.insert(meeting(u64::MAX, 3, 9));
        assert_eq!(entries.next_id(), EntryId(u64::MAX));
    }

    #[test]
    fn a_removed_entry_never_gives_its_id_back() {
        let mut entries = Entries::utc().with([meeting(1, 3, 9), meeting(2, 3, 10)]);
        assert_eq!(entries.next_id(), EntryId(3));
        let _ = entries.remove(EntryId(2));
        assert_eq!(entries.next_id(), EntryId(3), "2 was used");
        let _ = entries.remove(EntryId(1));
        assert_eq!(entries.next_id(), EntryId(3));
        // Collections holding the same entries are equal whatever they handed out.
        assert_eq!(entries, Entries::utc());
    }

    #[test]
    fn icon_emphasis_and_lock_default_off_and_can_be_set() {
        let entry = meeting(1, 3, 9);
        assert_eq!(entry.icon_of(), None);
        assert_eq!(entry.icon_label_of(), None);
        assert_eq!(entry.emphasis_of(), Emphasis::Solid);
        assert!(!entry.is_locked());
        let entry = entry
            .icon(crate::lucide!(Users))
            .icon_label("  Team entry ")
            .emphasis(Emphasis::Outlined)
            .locked(true);
        assert_eq!(entry.icon_of().map(Glyph::name), Some("users"));
        assert_eq!(entry.icon_label_of(), Some("Team entry"));
        assert_eq!(entry.emphasis_of(), Emphasis::Outlined);
        assert!(entry.is_locked());
        assert_eq!(entry.icon_label("").icon_label_of(), None);
        assert_eq!(Emphasis::ALL.len(), 5);
    }

    #[test]
    fn occurrences_overlap_the_range_and_come_in_order() {
        let entries = Entries::utc().with([
            meeting(1, 5, 9),
            meeting(2, 3, 14),
            meeting(3, 3, 9),
            Entry::all_day(EntryId(4), "Trip", date(2026, 11, 2), date(2026, 11, 3)),
        ]);
        let found = entries.on(date(2026, 11, 3));
        let ids: Vec<_> = found.iter().map(|occurrence| occurrence.entry.0).collect();
        assert_eq!(ids, vec![4, 3, 2]);
        assert!(found[0].all_day);
        assert!(!found[1].all_day);

        let week = entries.between(date(2026, 11, 2), date(2026, 11, 8));
        assert_eq!(week.len(), 4);
        assert!(entries.on(date(2026, 11, 4)).is_empty());
    }

    #[test]
    fn an_entry_ending_at_midnight_does_not_touch_the_next_day() {
        let entries = Entries::utc().with([Entry::new(EntryId(1), "Late", at(3, 22), at(4, 0))]);
        assert_eq!(entries.on(date(2026, 11, 3)).len(), 1);
        assert!(entries.on(date(2026, 11, 4)).is_empty());
        let occurrence = entries.on(date(2026, 11, 3))[0];
        assert_eq!(
            occurrence.dates(&TimeZone::UTC),
            (date(2026, 11, 3), date(2026, 11, 3))
        );
        assert_eq!(occurrence.duration(), SignedDuration::from_hours(2));
    }

    #[test]
    fn an_entry_with_no_length_sits_on_its_day() {
        let entries =
            Entries::utc().with([Entry::new(EntryId(1), "Deadline", at(3, 17), at(3, 17))]);
        let found = entries.on(date(2026, 11, 3));
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].dates(&TimeZone::UTC),
            (date(2026, 11, 3), date(2026, 11, 3))
        );
    }
}
