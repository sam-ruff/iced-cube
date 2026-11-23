# iced-cube-rules

The date rules the iced-cube calendar uses, without the calendar: when something happens, how it repeats, and the occurrences of a repeating span in a range of time.

It depends only on `jiff`, so a command line tool or a server can list the same occurrences a window shows.

```rust
use iced_cube_rules::{Repeat, When, instant, occurrences};
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

let utc = TimeZone::UTC;
let at = |day, hour| instant(date(2026, 6, day), time(hour, 0, 0, 0), &utc);
let stand_up = When::timed(at(1, 9), at(1, 10));

let week = occurrences(stand_up, Some(Repeat::daily()), &utc, at(1, 0), at(8, 0));
assert_eq!(week.len(), 7);
```

Licensed under either of MIT or Apache-2.0, at your option.
