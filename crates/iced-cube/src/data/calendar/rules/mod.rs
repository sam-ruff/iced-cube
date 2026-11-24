//! The calendar's date rules, with no GUI dependency: when something
//! happens ([`When`]), how it repeats ([`Repeat`]), and the occurrences of
//! a repeating span in a range of time ([`occurrences`]).
//!
//! Nothing here uses iced, only `jiff`, so the module can be split into a
//! crate of its own for tools that list occurrences without a window.
//!
//! ```
//! use iced_cube::data::calendar::rules::{Repeat, When, instant, occurrences};
//! use jiff::civil::{date, time};
//! use jiff::tz::TimeZone;
//!
//! let utc = TimeZone::UTC;
//! let at = |day, hour| instant(date(2026, 6, day), time(hour, 0, 0, 0), &utc);
//! let stand_up = When::timed(at(1, 9), at(1, 10));
//!
//! // Every other day, from 1 June: the 1st, 3rd, 5th and 7th.
//! let rule = Repeat::daily().interval(2);
//! let week = occurrences(stand_up, Some(rule), &utc, at(1, 0), at(8, 0));
//! assert_eq!(week.len(), 4);
//! assert_eq!(week[1].start, at(3, 9));
//! ```

mod repeat;
mod when;

pub use repeat::{Ends, Frequency, Instance, MAX_OCCURRENCES, Repeat, nth, occurrences};
pub use when::{When, add_days, date_of, instant, midnight, next_day};
