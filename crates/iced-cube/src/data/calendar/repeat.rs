//! Repeating entries: how often, and expanding them into occurrences.
//!
//! The rules live in the `iced-cube-rules` crate, which has no GUI
//! dependency, so tools outside the window can expand entries the same
//! way. This module applies them to an [`Entry`].

use jiff::Timestamp;
use jiff::tz::TimeZone;

pub use iced_cube_rules::{Ends, Frequency, Instance, MAX_OCCURRENCES, Repeat};

use super::entry::{Entry, Occurrence};

fn occurrence(entry: &Entry, instance: Instance) -> Occurrence {
    Occurrence {
        entry: entry.id(),
        index: instance.index,
        start: instance.start,
        end: instance.end,
        all_day: instance.all_day,
    }
}

/// Occurrence `index` of `entry`, or `None` once the entry has stopped
/// repeating or the date is out of range.
pub fn nth(entry: &Entry, time_zone: &TimeZone, index: u32) -> Option<Occurrence> {
    iced_cube_rules::nth(entry.when(), entry.repeat_of(), time_zone, index)
        .map(|instance| occurrence(entry, instance))
}

/// The occurrences of `entry` that overlap the time from `from` up to
/// `to`, earliest first. At most [`MAX_OCCURRENCES`] are returned.
pub fn occurrences(
    entry: &Entry,
    time_zone: &TimeZone,
    from: Timestamp,
    to: Timestamp,
) -> Vec<Occurrence> {
    iced_cube_rules::occurrences(entry.when(), entry.repeat_of(), time_zone, from, to)
        .into_iter()
        .map(|instance| occurrence(entry, instance))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::calendar::entry::EntryId;
    use jiff::SignedDuration;
    use jiff::civil::{Date, date, time};

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
            first.map(|o| (o.entry, o.index, o.start, o.end)),
            Some((EntryId(1), 0, start, end))
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
