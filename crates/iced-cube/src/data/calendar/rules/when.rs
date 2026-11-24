//! Spans of time, and the date arithmetic they need.

use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

/// When something happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum When {
    /// From one instant to another. `end` is not part of the span, so a
    /// span ending at 10:00 does not overlap one starting at 10:00.
    Timed { start: Timestamp, end: Timestamp },
    /// Whole days, from `first` to `last` inclusive, in whatever time zone
    /// the dates are read in.
    AllDay { first: Date, last: Date },
}

impl When {
    /// A timed span. The two instants are put in order.
    pub fn timed(start: Timestamp, end: Timestamp) -> Self {
        When::Timed {
            start: start.min(end),
            end: start.max(end),
        }
    }

    /// Whole days. The two dates are put in order.
    pub fn all_day(first: Date, last: Date) -> Self {
        When::AllDay {
            first: first.min(last),
            last: first.max(last),
        }
    }

    /// A single whole day.
    pub fn day(date: Date) -> Self {
        When::AllDay {
            first: date,
            last: date,
        }
    }

    pub fn is_all_day(&self) -> bool {
        matches!(self, When::AllDay { .. })
    }

    /// The first instant of the span in `time_zone`.
    pub fn start(&self, time_zone: &TimeZone) -> Timestamp {
        match *self {
            When::Timed { start, .. } => start,
            When::AllDay { first, .. } => midnight(first, time_zone),
        }
    }

    /// The instant the span stops at in `time_zone`. For whole days this
    /// is midnight after the last day.
    pub fn end(&self, time_zone: &TimeZone) -> Timestamp {
        match *self {
            When::Timed { end, .. } => end,
            When::AllDay { last, .. } => midnight(next_day(last), time_zone),
        }
    }
}

/// Midnight at the start of `date` in `time_zone`, or the first instant of
/// the day when a clock change skips midnight.
pub fn midnight(date: Date, time_zone: &TimeZone) -> Timestamp {
    date.to_zoned_datetime(time_zone.clone())
        .map(|zoned| zoned.timestamp())
        .unwrap_or(Timestamp::MAX)
}

/// The instant it is `time` on `date` in `time_zone`. A time a clock
/// change skips moves forward by the gap, and one that happens twice
/// takes the first.
///
/// ```
/// use iced_cube::data::calendar::instant;
/// use jiff::civil::{date, time};
/// use jiff::tz::TimeZone;
///
/// let nine = instant(date(2026, 11, 19), time(9, 0, 0, 0), &TimeZone::UTC);
/// assert_eq!(nine.to_string(), "2026-11-19T09:00:00Z");
/// ```
pub fn instant(date: Date, time: Time, time_zone: &TimeZone) -> Timestamp {
    date.to_datetime(time)
        .to_zoned_datetime(time_zone.clone())
        .map(|zoned| zoned.timestamp())
        .unwrap_or_else(|_| midnight(date, time_zone))
}

/// The calendar date of `instant` in `time_zone`.
pub fn date_of(instant: Timestamp, time_zone: &TimeZone) -> Date {
    instant.to_zoned_datetime(time_zone.clone()).date()
}

/// `date` moved by `days`, stopping at the first or last date there is.
pub fn add_days(date: Date, days: i32) -> Date {
    match Span::new().try_days(i64::from(days)) {
        Ok(span) => date.saturating_add(span),
        Err(_) if days < 0 => Date::MIN,
        Err(_) => Date::MAX,
    }
}

/// The day after `date`, or `date` itself at the end of time.
pub fn next_day(date: Date) -> Date {
    add_days(date, 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn at(day: i8, hour: i8) -> Timestamp {
        instant(
            date(2026, 11, day),
            Time::new(hour, 0, 0, 0).unwrap_or_default(),
            &TimeZone::UTC,
        )
    }

    #[test]
    fn days_are_added_and_stop_at_the_ends_of_time() {
        let day = date(2026, 11, 3);
        assert_eq!(add_days(day, 1), date(2026, 11, 4));
        assert_eq!(add_days(day, -3), date(2026, 10, 31));
        assert_eq!(add_days(day, i32::MAX), Date::MAX);
        assert_eq!(add_days(day, i32::MIN), Date::MIN);
        assert_eq!(next_day(date(2026, 12, 31)), date(2027, 1, 1));
    }

    #[test]
    fn spans_are_put_in_order_and_whole_days_run_to_the_next_midnight() {
        assert_eq!(
            When::timed(at(3, 10), at(3, 9)),
            When::Timed {
                start: at(3, 9),
                end: at(3, 10)
            }
        );
        let days = When::all_day(date(2026, 11, 4), date(2026, 11, 3));
        assert_eq!(days.start(&TimeZone::UTC), at(3, 0));
        assert_eq!(days.end(&TimeZone::UTC), at(5, 0));
        assert!(days.is_all_day());
    }
}
