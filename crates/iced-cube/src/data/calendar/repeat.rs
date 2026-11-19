//! Repeating entries: how often, and expanding them into occurrences.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Span, Timestamp};

use super::entry::{Entry, Occurrence, When, midnight, next_day};

/// The most occurrences one entry gives for a single range, so a rule
/// that never ends cannot run away with a very long range.
pub const MAX_OCCURRENCES: usize = 5_000;

/// The unit an entry repeats in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl Frequency {
    pub const ALL: [Frequency; 4] = [
        Frequency::Daily,
        Frequency::Weekly,
        Frequency::Monthly,
        Frequency::Yearly,
    ];

    /// The span of `count` units.
    fn span(self, count: i64) -> Option<Span> {
        let span = Span::new();
        match self {
            Frequency::Daily => span.try_days(count),
            Frequency::Weekly => span.try_weeks(count),
            Frequency::Monthly => span.try_months(count),
            Frequency::Yearly => span.try_years(count),
        }
        .ok()
    }

    /// The longest one unit can be, in seconds. A day is 25 hours when the
    /// clocks go back.
    fn longest(self) -> i64 {
        const DAY: i64 = 25 * 60 * 60;
        match self {
            Frequency::Daily => DAY,
            Frequency::Weekly => 7 * DAY,
            Frequency::Monthly => 31 * DAY,
            Frequency::Yearly => 366 * DAY,
        }
    }
}

/// When a repeating entry stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Ends {
    #[default]
    Never,
    /// After this many occurrences, counting the first.
    After(u32),
    /// No occurrence starts after this date.
    On(Date),
}

/// How an entry repeats.
///
/// Occurrences are counted from the first, so an entry on the 31st that
/// repeats monthly lands on the last day of shorter months and returns to
/// the 31st afterwards. A timed entry keeps its local time across clock
/// changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Repeat {
    frequency: Frequency,
    interval: u32,
    ends: Ends,
}

impl Repeat {
    pub fn new(frequency: Frequency) -> Self {
        Self {
            frequency,
            interval: 1,
            ends: Ends::Never,
        }
    }

    pub fn daily() -> Self {
        Self::new(Frequency::Daily)
    }

    pub fn weekly() -> Self {
        Self::new(Frequency::Weekly)
    }

    pub fn monthly() -> Self {
        Self::new(Frequency::Monthly)
    }

    pub fn yearly() -> Self {
        Self::new(Frequency::Yearly)
    }

    /// Repeats every `interval` units, such as every 2 weeks. At least 1.
    pub fn interval(mut self, interval: u32) -> Self {
        self.interval = interval.max(1);
        self
    }

    /// Stops after `count` occurrences, counting the first.
    pub fn count(mut self, count: u32) -> Self {
        self.ends = Ends::After(count);
        self
    }

    /// Stops once an occurrence would start after `date`.
    pub fn until(mut self, date: Date) -> Self {
        self.ends = Ends::On(date);
        self
    }

    pub fn frequency(&self) -> Frequency {
        self.frequency
    }

    pub fn every(&self) -> u32 {
        self.interval
    }

    pub fn ends(&self) -> Ends {
        self.ends
    }

    /// The span from the first occurrence to occurrence `index`.
    fn offset(&self, index: u32) -> Option<Span> {
        self.frequency
            .span(i64::from(index) * i64::from(self.interval))
    }

    /// The longest `count` repetitions can take, in seconds.
    fn longest(&self, count: i64) -> i64 {
        self.frequency
            .longest()
            .saturating_mul(i64::from(self.interval))
            .saturating_mul(count)
    }
}

/// Occurrence `index` of `entry`, or `None` once the entry has stopped
/// repeating or the date is out of range.
pub fn nth(entry: &Entry, time_zone: &TimeZone, index: u32) -> Option<Occurrence> {
    let Some(repeat) = entry.repeat_of() else {
        return (index == 0).then(|| first(entry, time_zone));
    };
    if let Ends::After(count) = repeat.ends()
        && index >= count
    {
        return None;
    }
    let offset = repeat.offset(index)?;
    let (start_date, start, end) = match entry.when() {
        When::Timed { start, end } => {
            let moved = start
                .to_zoned_datetime(time_zone.clone())
                .checked_add(offset)
                .ok()?;
            let length = end.duration_since(start);
            let end = moved.timestamp().checked_add(length).ok()?;
            (moved.date(), moved.timestamp(), end)
        }
        When::AllDay { first, last } => {
            let moved = first.checked_add(offset).ok()?;
            let length = last.duration_since(first);
            let moved_last = moved.checked_add(length).ok()?;
            (
                moved,
                midnight(moved, time_zone),
                midnight(next_day(moved_last), time_zone),
            )
        }
    };
    if let Ends::On(date) = repeat.ends()
        && start_date > date
    {
        return None;
    }
    Some(Occurrence {
        entry: entry.id(),
        index,
        start,
        end,
        all_day: entry.is_all_day(),
    })
}

fn first(entry: &Entry, time_zone: &TimeZone) -> Occurrence {
    let when = entry.when();
    Occurrence {
        entry: entry.id(),
        index: 0,
        start: when.start(time_zone),
        end: when.end(time_zone),
        all_day: when.is_all_day(),
    }
}

/// Whether `occurrence` overlaps the time from `from` up to `to`. An
/// occurrence with no length counts when it starts inside the range.
fn overlaps(occurrence: &Occurrence, from: Timestamp, to: Timestamp) -> bool {
    if occurrence.end <= occurrence.start {
        return occurrence.start >= from && occurrence.start < to;
    }
    occurrence.start < to && occurrence.end > from
}

/// The occurrences of `entry` that overlap the time from `from` up to
/// `to`, earliest first. At most [`MAX_OCCURRENCES`] are returned.
pub fn occurrences(
    entry: &Entry,
    time_zone: &TimeZone,
    from: Timestamp,
    to: Timestamp,
) -> Vec<Occurrence> {
    let Some(repeat) = entry.repeat_of() else {
        let only = first(entry, time_zone);
        return if overlaps(&only, from, to) {
            vec![only]
        } else {
            Vec::new()
        };
    };

    let mut found = Vec::new();
    let mut index = first_candidate(entry, &repeat, time_zone, from);
    for _ in 0..2 * MAX_OCCURRENCES {
        if found.len() == MAX_OCCURRENCES {
            break;
        }
        let Some(occurrence) = nth(entry, time_zone, index) else {
            break;
        };
        if occurrence.start >= to {
            break;
        }
        if overlaps(&occurrence, from, to) {
            found.push(occurrence);
        }
        let Some(next) = index.checked_add(1) else {
            break;
        };
        index = next;
    }
    found
}

/// How many times [`first_candidate`] jumps ahead at most.
const JUMPS: usize = 48;

fn landing(entry: &Entry, time_zone: &TimeZone, index: u32, jump: u32) -> Option<Occurrence> {
    nth(entry, time_zone, index.checked_add(jump)?)
}

/// An index no later than the first occurrence that can reach `from`,
/// found by jumping ahead by the longest each repetition can be.
fn first_candidate(entry: &Entry, repeat: &Repeat, time_zone: &TimeZone, from: Timestamp) -> u32 {
    let mut index: u32 = 0;
    // Each jump lands short of `from`, and a handful get close enough.
    for _ in 0..JUMPS {
        let Some(occurrence) = nth(entry, time_zone, index) else {
            break;
        };
        let short_by = from.duration_since(occurrence.end);
        if short_by <= SignedDuration::ZERO {
            break;
        }
        let jump = short_by.as_secs() / repeat.longest(1).max(1);
        let mut jump = u32::try_from(jump).unwrap_or(u32::MAX - index);
        // A jump past the end of the series is halved until it lands on it.
        while jump > 0 && landing(entry, time_zone, index, jump).is_none() {
            jump /= 2;
        }
        if jump == 0 {
            break;
        }
        index = index.saturating_add(jump);
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::calendar::entry::EntryId;
    use jiff::civil::{date, time};

    fn utc(year: i16, month: i8, day: i8, hour: i8) -> Timestamp {
        zoned(&TimeZone::UTC, year, month, day, hour)
    }

    fn zoned(time_zone: &TimeZone, year: i16, month: i8, day: i8, hour: i8) -> Timestamp {
        date(year, month, day)
            .at(hour, 0, 0, 0)
            .to_zoned_datetime(time_zone.clone())
            .map(|zoned| zoned.timestamp())
            .unwrap_or_default()
    }

    fn timed(start: Timestamp, repeat: Repeat) -> Entry {
        let end = start
            .checked_add(SignedDuration::from_hours(1))
            .unwrap_or(start);
        Entry::new(EntryId(1), "Repeats", start, end).repeat(repeat)
    }

    fn start_dates(entry: &Entry, time_zone: &TimeZone, count: u32) -> Vec<Date> {
        (0..count)
            .filter_map(|index| nth(entry, time_zone, index))
            .map(|occurrence| occurrence.start.to_zoned_datetime(time_zone.clone()).date())
            .collect()
    }

    #[test]
    fn builder_defaults_and_options() {
        for frequency in Frequency::ALL {
            let repeat = Repeat::new(frequency);
            assert_eq!(repeat.frequency(), frequency);
            assert_eq!(repeat.every(), 1);
            assert_eq!(repeat.ends(), Ends::Never);
        }
        assert_eq!(Repeat::daily().frequency(), Frequency::Daily);
        assert_eq!(Repeat::weekly().frequency(), Frequency::Weekly);
        assert_eq!(Repeat::monthly().frequency(), Frequency::Monthly);
        assert_eq!(Repeat::yearly().frequency(), Frequency::Yearly);
        assert_eq!(Repeat::daily().interval(0).every(), 1);
        assert_eq!(Repeat::daily().interval(3).every(), 3);
        assert_eq!(Repeat::daily().count(4).ends(), Ends::After(4));
        let stop = date(2026, 12, 1);
        assert_eq!(Repeat::daily().until(stop).ends(), Ends::On(stop));
    }

    #[test]
    fn an_entry_that_does_not_repeat_has_one_occurrence() {
        let start = utc(2026, 11, 3, 9);
        let end = utc(2026, 11, 3, 10);
        let entry = Entry::new(EntryId(1), "Once", start, end);
        let first = nth(&entry, &TimeZone::UTC, 0);
        assert_eq!(
            first.map(|o| (o.index, o.start, o.end)),
            Some((0, start, end))
        );
        assert_eq!(nth(&entry, &TimeZone::UTC, 1), None);
    }

    #[test]
    fn daily_and_weekly_step_by_their_interval() {
        let daily = timed(utc(2026, 11, 3, 9), Repeat::daily().interval(2));
        assert_eq!(
            start_dates(&daily, &TimeZone::UTC, 3),
            vec![date(2026, 11, 3), date(2026, 11, 5), date(2026, 11, 7)]
        );
        let weekly = timed(utc(2026, 11, 3, 9), Repeat::weekly());
        assert_eq!(
            start_dates(&weekly, &TimeZone::UTC, 3),
            vec![date(2026, 11, 3), date(2026, 11, 10), date(2026, 11, 17)]
        );
    }

    #[test]
    fn monthly_on_the_31st_uses_the_last_day_of_short_months() {
        let entry = timed(utc(2027, 1, 31, 9), Repeat::monthly());
        assert_eq!(
            start_dates(&entry, &TimeZone::UTC, 4),
            vec![
                date(2027, 1, 31),
                date(2027, 2, 28),
                date(2027, 3, 31),
                date(2027, 4, 30)
            ]
        );
    }

    #[test]
    fn yearly_on_a_leap_day_falls_back_to_the_28th() {
        let entry = Entry::all_day(EntryId(1), "Leap", date(2028, 2, 29), date(2028, 2, 29))
            .repeat(Repeat::yearly());
        assert_eq!(
            start_dates(&entry, &TimeZone::UTC, 5),
            vec![
                date(2028, 2, 29),
                date(2029, 2, 28),
                date(2030, 2, 28),
                date(2031, 2, 28),
                date(2032, 2, 29)
            ]
        );
    }

    #[test]
    fn a_timed_entry_keeps_its_local_time_across_a_clock_change() {
        let Ok(london) = TimeZone::get("Europe/London") else {
            panic!("the time zone database is bundled for tests");
        };
        // The clocks go forward on 28 March 2027.
        let entry = timed(zoned(&london, 2027, 3, 27, 9), Repeat::daily());
        for index in 0..3 {
            let Some(occurrence) = nth(&entry, &london, index) else {
                panic!("occurrence {index}");
            };
            let local = occurrence.start.to_zoned_datetime(london.clone());
            assert_eq!(local.time(), time(9, 0, 0, 0), "{index}");
            assert_eq!(occurrence.duration(), SignedDuration::from_hours(1));
        }
        let before = nth(&entry, &london, 0).map(|o| o.start);
        let after = nth(&entry, &london, 1).map(|o| o.start);
        let (Some(before), Some(after)) = (before, after) else {
            panic!("two occurrences");
        };
        assert_eq!(after.duration_since(before), SignedDuration::from_hours(23));
    }

    #[test]
    fn count_and_until_stop_the_series() {
        let counted = timed(utc(2026, 11, 3, 9), Repeat::daily().count(3));
        assert!(nth(&counted, &TimeZone::UTC, 2).is_some());
        assert_eq!(nth(&counted, &TimeZone::UTC, 3), None);

        let until = timed(
            utc(2026, 11, 3, 9),
            Repeat::daily().until(date(2026, 11, 5)),
        );
        assert_eq!(
            start_dates(&until, &TimeZone::UTC, 10),
            vec![date(2026, 11, 3), date(2026, 11, 4), date(2026, 11, 5)]
        );

        let never = timed(utc(2026, 11, 3, 9), Repeat::daily().count(0));
        assert_eq!(nth(&never, &TimeZone::UTC, 0), None);
    }

    #[test]
    fn whole_day_occurrences_keep_their_length() {
        let entry = Entry::all_day(EntryId(1), "Sprint", date(2026, 11, 2), date(2026, 11, 4))
            .repeat(Repeat::weekly().interval(2));
        let Some(second) = nth(&entry, &TimeZone::UTC, 1) else {
            panic!("second occurrence");
        };
        assert!(second.all_day);
        assert_eq!(second.start, utc(2026, 11, 16, 0));
        assert_eq!(second.end, utc(2026, 11, 19, 0));
    }

    #[test]
    fn a_range_only_gets_the_occurrences_that_overlap_it() {
        let entry = timed(utc(2026, 11, 3, 9), Repeat::daily());
        let found = occurrences(
            &entry,
            &TimeZone::UTC,
            utc(2026, 11, 10, 0),
            utc(2026, 11, 12, 0),
        );
        let indices: Vec<_> = found.iter().map(|o| o.index).collect();
        assert_eq!(indices, vec![7, 8]);

        let before_it_starts = occurrences(
            &entry,
            &TimeZone::UTC,
            utc(2026, 10, 1, 0),
            utc(2026, 11, 1, 0),
        );
        assert!(before_it_starts.is_empty());
    }

    #[test]
    fn a_range_far_from_the_start_is_found_without_walking_there() {
        let entry = timed(utc(1990, 1, 1, 9), Repeat::daily());
        let found = occurrences(
            &entry,
            &TimeZone::UTC,
            utc(2026, 11, 3, 0),
            utc(2026, 11, 4, 0),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].start, utc(2026, 11, 3, 9));

        let monthly = timed(utc(1990, 1, 31, 9), Repeat::monthly());
        let found = occurrences(
            &monthly,
            &TimeZone::UTC,
            utc(2026, 11, 1, 0),
            utc(2026, 12, 1, 0),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].start, utc(2026, 11, 30, 9));
    }

    #[test]
    fn the_first_candidate_never_skips_an_occurrence() {
        for repeat in [
            Repeat::daily(),
            Repeat::weekly().interval(3),
            Repeat::monthly(),
            Repeat::yearly(),
        ] {
            let entry = timed(utc(2001, 1, 31, 9), repeat);
            let from = utc(2026, 6, 15, 0);
            let index = first_candidate(&entry, &repeat, &TimeZone::UTC, from);
            if index == 0 {
                continue;
            }
            let Some(previous) = nth(&entry, &TimeZone::UTC, index - 1) else {
                panic!("{repeat:?}");
            };
            assert!(previous.end <= from, "{repeat:?}");
        }
    }

    #[test]
    fn a_range_with_too_many_occurrences_is_capped() {
        let entry = timed(utc(2000, 1, 1, 9), Repeat::daily());
        let found = occurrences(
            &entry,
            &TimeZone::UTC,
            utc(2000, 1, 1, 0),
            utc(2100, 1, 1, 0),
        );
        assert_eq!(found.len(), MAX_OCCURRENCES);
    }

    #[test]
    fn a_finished_series_gives_nothing_later() {
        let entry = timed(utc(2026, 11, 3, 9), Repeat::weekly().count(2));
        let found = occurrences(
            &entry,
            &TimeZone::UTC,
            utc(2026, 12, 1, 0),
            utc(2027, 1, 1, 0),
        );
        assert!(found.is_empty());
    }
}
