//! Repeating spans: how often, and expanding them into occurrences.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Span, Timestamp};

use crate::when::{When, midnight, next_day};

/// The most occurrences one span gives for a single range, so a rule that
/// never ends cannot run away with a very long range.
pub const MAX_OCCURRENCES: usize = 5_000;

/// The unit a span repeats in.
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

/// When a repeating span stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Ends {
    #[default]
    Never,
    /// After this many occurrences, counting the first.
    After(u32),
    /// No occurrence starts after this date.
    On(Date),
}

/// How a span repeats.
///
/// Occurrences are counted from the first, so a span on the 31st that
/// repeats monthly lands on the last day of shorter months and returns to
/// the 31st afterwards. A timed span keeps its local time across clock
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

    /// The longest one repetition can take, in seconds.
    fn longest(&self) -> i64 {
        self.frequency
            .longest()
            .saturating_mul(i64::from(self.interval))
    }
}

/// One time a span happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Instance {
    /// Which repetition this is, counted from 0.
    pub index: u32,
    pub start: Timestamp,
    /// The instant the occurrence stops at, which is not part of it.
    pub end: Timestamp,
    pub all_day: bool,
}

/// Occurrence `index` of `when` repeating by `repeat`, or `None` once it
/// has stopped repeating or the date is out of range. A span that does
/// not repeat has only occurrence 0.
pub fn nth(
    when: When,
    repeat: Option<Repeat>,
    time_zone: &TimeZone,
    index: u32,
) -> Option<Instance> {
    let Some(repeat) = repeat else {
        return (index == 0).then(|| first(when, time_zone));
    };
    if let Ends::After(count) = repeat.ends()
        && index >= count
    {
        return None;
    }
    let offset = repeat.offset(index)?;
    let (start_date, start, end) = match when {
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
    Some(Instance {
        index,
        start,
        end,
        all_day: when.is_all_day(),
    })
}

fn first(when: When, time_zone: &TimeZone) -> Instance {
    Instance {
        index: 0,
        start: when.start(time_zone),
        end: when.end(time_zone),
        all_day: when.is_all_day(),
    }
}

/// Whether `instance` overlaps the time from `from` up to `to`. One with no
/// length counts when it starts inside the range.
fn overlaps(instance: &Instance, from: Timestamp, to: Timestamp) -> bool {
    if instance.end <= instance.start {
        return instance.start >= from && instance.start < to;
    }
    instance.start < to && instance.end > from
}

/// The occurrences of `when` repeating by `repeat` that overlap the time
/// from `from` up to `to`, earliest first. At most [`MAX_OCCURRENCES`] are
/// returned.
pub fn occurrences(
    when: When,
    repeat: Option<Repeat>,
    time_zone: &TimeZone,
    from: Timestamp,
    to: Timestamp,
) -> Vec<Instance> {
    let Some(rule) = repeat else {
        let only = first(when, time_zone);
        return if overlaps(&only, from, to) {
            vec![only]
        } else {
            Vec::new()
        };
    };

    let mut found = Vec::new();
    let mut index = first_candidate(when, rule, time_zone, from);
    for _ in 0..2 * MAX_OCCURRENCES {
        if found.len() == MAX_OCCURRENCES {
            break;
        }
        let Some(instance) = nth(when, repeat, time_zone, index) else {
            break;
        };
        if instance.start >= to {
            break;
        }
        if overlaps(&instance, from, to) {
            found.push(instance);
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

/// An index no later than the first occurrence that can reach `from`,
/// found by jumping ahead by the longest each repetition can be.
fn first_candidate(when: When, repeat: Repeat, time_zone: &TimeZone, from: Timestamp) -> u32 {
    let at = |index| nth(when, Some(repeat), time_zone, index);
    let mut index: u32 = 0;
    // Each jump lands short of `from`, and a handful get close enough.
    for _ in 0..JUMPS {
        let Some(instance) = at(index) else {
            break;
        };
        let short_by = from.duration_since(instance.end);
        if short_by <= SignedDuration::ZERO {
            break;
        }
        let jump = short_by.as_secs() / repeat.longest().max(1);
        let mut jump = u32::try_from(jump).unwrap_or(u32::MAX - index);
        // A jump past the end of the series is halved until it lands on it.
        while jump > 0 && index.checked_add(jump).and_then(at).is_none() {
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
    use crate::when::instant;
    use jiff::civil::{date, time};

    fn utc(year: i16, month: i8, day: i8, hour: i8) -> Timestamp {
        instant(date(year, month, day), time(hour, 0, 0, 0), &TimeZone::UTC)
    }

    fn hour_from(start: Timestamp) -> When {
        let end = start
            .checked_add(SignedDuration::from_hours(1))
            .unwrap_or(start);
        When::timed(start, end)
    }

    #[test]
    fn a_span_that_does_not_repeat_has_one_occurrence() {
        let when = hour_from(utc(2026, 6, 1, 9));
        assert_eq!(
            nth(when, None, &TimeZone::UTC, 0).map(|i| i.start),
            Some(utc(2026, 6, 1, 9))
        );
        assert_eq!(nth(when, None, &TimeZone::UTC, 1), None);
        let all = occurrences(
            when,
            None,
            &TimeZone::UTC,
            utc(2026, 1, 1, 0),
            utc(2027, 1, 1, 0),
        );
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn monthly_on_the_31st_uses_the_last_day_of_short_months() {
        let when = hour_from(utc(2026, 1, 31, 9));
        let starts: Vec<Date> = (0..4)
            .filter_map(|index| nth(when, Some(Repeat::monthly()), &TimeZone::UTC, index))
            .map(|instance| instance.start.to_zoned_datetime(TimeZone::UTC).date())
            .collect();
        assert_eq!(
            starts,
            vec![
                date(2026, 1, 31),
                date(2026, 2, 28),
                date(2026, 3, 31),
                date(2026, 4, 30)
            ]
        );
    }

    #[test]
    fn whole_days_keep_their_length_and_count_stops_the_series() {
        let when = When::all_day(date(2026, 6, 1), date(2026, 6, 2));
        let rule = Repeat::weekly().count(2);
        let all = occurrences(
            when,
            Some(rule),
            &TimeZone::UTC,
            utc(2026, 1, 1, 0),
            utc(2027, 1, 1, 0),
        );
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|instance| instance.all_day));
        assert_eq!(all[1].start, utc(2026, 6, 8, 0));
        assert_eq!(all[1].end, utc(2026, 6, 10, 0));
    }

    #[test]
    fn the_first_candidate_never_skips_an_occurrence() {
        for repeat in [
            Repeat::daily(),
            Repeat::weekly().interval(3),
            Repeat::monthly(),
            Repeat::yearly(),
        ] {
            let when = hour_from(utc(2001, 1, 31, 9));
            let from = utc(2026, 6, 15, 0);
            let index = first_candidate(when, repeat, &TimeZone::UTC, from);
            if index == 0 {
                continue;
            }
            let Some(previous) = nth(when, Some(repeat), &TimeZone::UTC, index - 1) else {
                panic!("{repeat:?}");
            };
            assert!(previous.end <= from, "{repeat:?}");
        }
    }

    #[test]
    fn a_range_with_too_many_occurrences_is_capped() {
        let when = hour_from(utc(2000, 1, 1, 9));
        let found = occurrences(
            when,
            Some(Repeat::daily()),
            &TimeZone::UTC,
            utc(2000, 1, 1, 0),
            utc(2100, 1, 1, 0),
        );
        assert_eq!(found.len(), MAX_OCCURRENCES);
    }
}
