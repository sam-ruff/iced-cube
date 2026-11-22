//! Where things go: the weeks of a month, the lanes of bars across days
//! and the side by side columns of overlapping entries in a day.

use iced::{Point, Rectangle, Size};
use jiff::Timestamp;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;

use super::entry::{Occurrence, add_days, midnight};

/// Minutes in a day on the time grid.
pub const DAY_MINUTES: u16 = 24 * 60;

/// The shortest a block is drawn, so a five minute entry can still be
/// read and grabbed. Blocks also keep clear of each other by this much.
pub const MIN_BLOCK_MINUTES: u16 = 30;

/// The seven dates of the week holding `date`, starting on `week_start`.
pub fn week_of(date: Date, week_start: Weekday) -> [Date; 7] {
    let back = i32::from(week_start.until(date.weekday()).rem_euclid(7));
    let first = add_days(date, -back);
    std::array::from_fn(|day| add_days(first, day as i32))
}

/// The weeks that cover the month of `date`, four to six of them.
pub fn month_weeks(date: Date, week_start: Weekday) -> Vec<[Date; 7]> {
    let last = date.last_of_month();
    let mut weeks = vec![week_of(date.first_of_month(), week_start)];
    // No month needs more than six, which also stops at the last date
    // there is, where the days no longer move on.
    while let Some(week) = weeks.last()
        && week[6] < last
        && weeks.len() < 6
    {
        weeks.push(week_of(add_days(week[6], 1), week_start));
    }
    weeks
}

/// The seven weekdays in order, starting on `week_start`.
pub fn weekdays(week_start: Weekday) -> [Weekday; 7] {
    std::array::from_fn(|day| week_start.wrapping_add(day as i64))
}

/// An occurrence laid across a row of days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    pub occurrence: Occurrence,
    /// The first and last day it covers, as indices into the row.
    pub first: usize,
    pub last: usize,
    /// The line it sits on, counted from 0 at the top.
    pub lane: usize,
    /// Whether it began before the row, or carries on after it.
    pub starts_before: bool,
    pub ends_after: bool,
}

impl Bar {
    pub fn covers(&self, day: usize) -> bool {
        (self.first..=self.last).contains(&day)
    }

    /// How many days of the row it covers.
    pub fn days(&self) -> usize {
        self.last - self.first + 1
    }
}

/// Lays `occurrences` across `days`, which are consecutive dates. Longer
/// bars go first so they stay in one piece near the top, and each bar
/// takes the highest lane that is free for all its days.
pub fn bars(days: &[Date], occurrences: &[Occurrence], time_zone: &TimeZone) -> Vec<Bar> {
    let (Some(&row_first), Some(&row_last)) = (days.first(), days.last()) else {
        return Vec::new();
    };
    let index = |date: Date| days.iter().position(|day| *day == date);

    let mut placed: Vec<Bar> = occurrences
        .iter()
        .filter_map(|occurrence| {
            let (first, last) = occurrence.dates(time_zone);
            if last < row_first || first > row_last {
                return None;
            }
            Some(Bar {
                occurrence: *occurrence,
                first: index(first.max(row_first))?,
                last: index(last.min(row_last))?,
                lane: 0,
                starts_before: first < row_first,
                ends_after: last > row_last,
            })
        })
        .collect();
    placed.sort_by_key(|bar| {
        (
            bar.first,
            std::cmp::Reverse(bar.days()),
            !bar.occurrence.all_day,
            bar.occurrence.start,
            bar.occurrence.entry,
        )
    });

    // The last day taken in each lane.
    let mut lanes: Vec<usize> = Vec::new();
    for bar in &mut placed {
        let free = lanes.iter().position(|taken| *taken < bar.first);
        bar.lane = free.unwrap_or(lanes.len());
        match free {
            Some(lane) => lanes[lane] = bar.last,
            None => lanes.push(bar.last),
        }
    }
    placed
}

/// Bars cut down to the lanes there is room for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fitted {
    pub bars: Vec<Bar>,
    /// How many bars each day has that are not shown.
    pub hidden: Vec<usize>,
}

/// Keeps the bars that fit in `lanes` lines over `days` days. A day with
/// more bars than lines gives its last line to a count of the rest, so a
/// bar on that line is hidden if any of its days is that full.
pub fn fit(bars: Vec<Bar>, days: usize, lanes: usize) -> Fitted {
    if days == 0 {
        return Fitted {
            bars: Vec::new(),
            hidden: Vec::new(),
        };
    }
    let depth = |day: usize| {
        bars.iter()
            .filter(|bar| bar.covers(day))
            .map(|bar| bar.lane + 1)
            .max()
            .unwrap_or(0)
    };
    let crowded: Vec<bool> = (0..days).map(|day| depth(day) > lanes).collect();
    let shown = |bar: &Bar| {
        let over_a_crowded_day = (bar.first..=bar.last).any(|day| crowded[day.min(days - 1)]);
        bar.lane + 1 < lanes || (bar.lane + 1 == lanes && !over_a_crowded_day)
    };

    let hidden = (0..days)
        .map(|day| {
            bars.iter()
                .filter(|bar| bar.covers(day) && !shown(bar))
                .count()
        })
        .collect();
    let bars = bars.iter().copied().filter(shown).collect();
    Fitted { bars, hidden }
}

/// How many lanes `bars` use.
pub fn lane_count(bars: &[Bar]) -> usize {
    bars.iter().map(|bar| bar.lane + 1).max().unwrap_or(0)
}

/// A timed occurrence in one day of the time grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub occurrence: Occurrence,
    /// Minutes from midnight where it starts and stops on this day.
    pub top: u16,
    pub bottom: u16,
    /// Its place among the entries it overlaps, and how many share the
    /// width.
    pub column: usize,
    pub columns: usize,
    /// Whether it began on an earlier day, or carries on to a later one.
    pub starts_before: bool,
    pub ends_after: bool,
}

impl Block {
    /// Where the block is drawn from: its top, moved up when it starts
    /// too close to midnight to fit [`MIN_BLOCK_MINUTES`] below it.
    pub fn drawn_top(&self) -> u16 {
        self.top.min(DAY_MINUTES - MIN_BLOCK_MINUTES)
    }

    /// Where the block is drawn down to, which is at least
    /// [`MIN_BLOCK_MINUTES`] below where it is drawn from.
    pub fn drawn_bottom(&self) -> u16 {
        self.bottom.max(self.drawn_top() + MIN_BLOCK_MINUTES)
    }
}

/// The minute of `date` that `instant` falls on: 0 when it is before the
/// day and [`DAY_MINUTES`] when it is after.
pub fn minute_on(date: Date, instant: Timestamp, time_zone: &TimeZone) -> u16 {
    let zoned = instant.to_zoned_datetime(time_zone.clone());
    match zoned.date().cmp(&date) {
        std::cmp::Ordering::Less => 0,
        std::cmp::Ordering::Greater => DAY_MINUTES,
        std::cmp::Ordering::Equal => {
            u16::try_from(i32::from(zoned.hour()) * 60 + i32::from(zoned.minute())).unwrap_or(0)
        }
    }
}

/// The instant `minute` minutes into `date`.
pub fn instant_at(date: Date, minute: u16, time_zone: &TimeZone) -> Timestamp {
    if minute >= DAY_MINUTES {
        return midnight(add_days(date, 1), time_zone);
    }
    let hour = i8::try_from(minute / 60).unwrap_or(0);
    let minute = i8::try_from(minute % 60).unwrap_or(0);
    date.at(hour, minute, 0, 0)
        .to_zoned_datetime(time_zone.clone())
        .map(|zoned| zoned.timestamp())
        .unwrap_or_else(|_| midnight(date, time_zone))
}

/// The timed occurrences on `date` as blocks. Entries that overlap share
/// the width in columns; a run of entries that touch one another uses as
/// many columns as its busiest moment needs.
pub fn blocks(date: Date, occurrences: &[Occurrence], time_zone: &TimeZone) -> Vec<Block> {
    let mut placed: Vec<Block> = occurrences
        .iter()
        .filter(|occurrence| !occurrence.all_day)
        .filter_map(|occurrence| {
            let (first, last) = occurrence.dates(time_zone);
            if date < first || date > last {
                return None;
            }
            let top = minute_on(date, occurrence.start, time_zone);
            let bottom = minute_on(date, occurrence.end, time_zone).max(top);
            Some(Block {
                occurrence: *occurrence,
                top,
                bottom,
                column: 0,
                columns: 1,
                starts_before: first < date,
                ends_after: last > date,
            })
        })
        .collect();
    placed.sort_by_key(|block| {
        (
            block.drawn_top(),
            std::cmp::Reverse(block.bottom),
            block.occurrence.entry,
        )
    });

    let mut run_start = 0;
    let mut run_end = 0;
    // Where the last block in each column of the current run stops.
    let mut columns: Vec<u16> = Vec::new();
    for index in 0..placed.len() {
        let (top, bottom) = (placed[index].drawn_top(), placed[index].drawn_bottom());
        if index > run_start && top >= run_end {
            close_run(&mut placed[run_start..index], columns.len());
            run_start = index;
            run_end = 0;
            columns.clear();
        }
        let free = columns.iter().position(|taken| *taken <= top);
        placed[index].column = free.unwrap_or(columns.len());
        match free {
            Some(column) => columns[column] = bottom,
            None => columns.push(bottom),
        }
        run_end = run_end.max(bottom);
    }
    close_run(&mut placed[run_start..], columns.len());
    placed
}

fn close_run(run: &mut [Block], columns: usize) {
    for block in run {
        block.columns = columns.max(1);
    }
}

/// `minutes` rounded to the nearest `step`, kept within the day.
pub fn snap(minutes: f32, step: u16) -> u16 {
    let step = f32::from(step.max(1));
    let snapped = (minutes / step).round() * step;
    snapped.clamp(0.0, f32::from(DAY_MINUTES)) as u16
}

/// The geometry of a time grid: day columns beside a gutter of hour
/// labels, one row per hour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// The width of everything, gutter included.
    pub width: f32,
    /// The width of the hour labels on the left.
    pub gutter: f32,
    pub days: usize,
    pub hour_height: f32,
    /// The hours on show: from the start of `first_hour` to the start of
    /// `last_hour`, so 0 and 24 for the whole day.
    pub first_hour: u8,
    pub last_hour: u8,
}

impl Grid {
    /// The first and last minute on show.
    pub fn minutes(&self) -> (u16, u16) {
        let first = u16::from(self.first_hour.min(23)) * 60;
        let last = (u16::from(self.last_hour) * 60).clamp(first + 60, DAY_MINUTES);
        (first, last)
    }

    pub fn height(&self) -> f32 {
        let (first, last) = self.minutes();
        f32::from(last - first) / 60.0 * self.hour_height
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.height())
    }

    pub fn day_width(&self) -> f32 {
        ((self.width - self.gutter) / self.days.max(1) as f32).max(0.0)
    }

    /// The left edge of a day's column.
    pub fn day_x(&self, day: usize) -> f32 {
        self.gutter + day as f32 * self.day_width()
    }

    /// The day column under `x`, if any.
    pub fn day_at(&self, x: f32) -> Option<usize> {
        let width = self.day_width();
        if x < self.gutter || width <= 0.0 {
            return None;
        }
        let day = ((x - self.gutter) / width) as usize;
        (day < self.days).then_some(day)
    }

    /// How far down `minute` is. Negative for a minute before the first
    /// hour on show.
    pub fn y_of(&self, minute: u16) -> f32 {
        let (first, _) = self.minutes();
        (f32::from(minute) - f32::from(first)) / 60.0 * self.hour_height
    }

    /// The minute at `y`, not rounded, kept within the hours on show.
    pub fn minutes_at(&self, y: f32) -> f32 {
        let (first, last) = self.minutes();
        if self.hour_height <= 0.0 {
            return f32::from(first);
        }
        (f32::from(first) + y / self.hour_height * 60.0).clamp(f32::from(first), f32::from(last))
    }

    /// Where `block` is drawn in column `day`, cut to the hours on show,
    /// or `None` when it is outside them. Blocks sharing a moment split
    /// the column, less a sliver on the right that stays free for dragging
    /// out a new entry.
    pub fn block(&self, day: usize, block: &Block) -> Option<Rectangle> {
        let (first, last) = self.minutes();
        let (top, bottom) = (block.drawn_top(), block.drawn_bottom());
        if bottom <= first || top >= last {
            return None;
        }
        let lane = (self.day_width() - BLOCK_GUTTER).max(0.0);
        let width = lane / block.columns.max(1) as f32;
        let y = self.y_of(top.max(first));
        Some(Rectangle::new(
            Point::new(self.day_x(day) + block.column as f32 * width, y),
            Size::new(
                (width - BLOCK_GAP).max(0.0),
                (self.y_of(bottom.min(last)) - y - BLOCK_GAP).max(0.0),
            ),
        ))
    }
}

/// The part of a day column blocks leave free on the right.
pub const BLOCK_GUTTER: f32 = 8.0;
/// The space between neighbouring blocks.
pub const BLOCK_GAP: f32 = 2.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::calendar::entry::{Entries, Entry, EntryId};
    use jiff::civil::date;

    fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
        date(2026, 11, day)
            .at(hour, minute, 0, 0)
            .to_zoned_datetime(TimeZone::UTC)
            .map(|zoned| zoned.timestamp())
            .unwrap_or_default()
    }

    fn timed(id: u64, from: (i8, i8, i8), to: (i8, i8, i8)) -> Entry {
        Entry::new(
            EntryId(id),
            "Entry",
            at(from.0, from.1, from.2),
            at(to.0, to.1, to.2),
        )
    }

    fn days(id: u64, first: i8, last: i8) -> Entry {
        Entry::all_day(
            EntryId(id),
            "Days",
            date(2026, 11, first),
            date(2026, 11, last),
        )
    }

    /// Monday 2 to Sunday 8 November 2026.
    fn week() -> [Date; 7] {
        week_of(date(2026, 11, 4), Weekday::Monday)
    }

    fn week_bars(entries: &Entries) -> Vec<Bar> {
        let week = week();
        bars(
            &week,
            &entries.between(week[0], week[6]),
            entries.time_zone(),
        )
    }

    fn lanes(bars: &[Bar]) -> Vec<(u64, usize, usize, usize)> {
        let mut lanes: Vec<_> = bars
            .iter()
            .map(|bar| (bar.occurrence.entry.0, bar.first, bar.last, bar.lane))
            .collect();
        lanes.sort();
        lanes
    }

    #[test]
    fn a_week_starts_on_the_chosen_day() {
        let monday = week_of(date(2026, 11, 4), Weekday::Monday);
        assert_eq!(monday[0], date(2026, 11, 2));
        assert_eq!(monday[6], date(2026, 11, 8));
        let sunday = week_of(date(2026, 11, 4), Weekday::Sunday);
        assert_eq!(sunday[0], date(2026, 11, 1));
        assert_eq!(sunday[6], date(2026, 11, 7));
        assert_eq!(week_of(date(2026, 11, 2), Weekday::Monday), monday);
        assert_eq!(week_of(date(2026, 11, 8), Weekday::Monday), monday);
    }

    #[test]
    fn weekdays_start_on_the_chosen_day() {
        assert_eq!(weekdays(Weekday::Monday)[0], Weekday::Monday);
        assert_eq!(weekdays(Weekday::Monday)[6], Weekday::Sunday);
        assert_eq!(weekdays(Weekday::Sunday)[0], Weekday::Sunday);
        assert_eq!(weekdays(Weekday::Sunday)[6], Weekday::Saturday);
    }

    #[test]
    fn a_month_is_covered_by_four_to_six_whole_weeks() {
        // November 2026 starts on a Sunday and ends on a Monday.
        let weeks = month_weeks(date(2026, 11, 19), Weekday::Monday);
        assert_eq!(weeks.len(), 6);
        assert_eq!(weeks[0][0], date(2026, 10, 26));
        assert_eq!(weeks[5][6], date(2026, 12, 6));

        let weeks = month_weeks(date(2026, 11, 19), Weekday::Sunday);
        assert_eq!(weeks.len(), 5);
        assert_eq!(weeks[0][0], date(2026, 11, 1));

        // The first and last months there are still give whole weeks.
        for edge in [Date::MIN, Date::MAX] {
            let weeks = month_weeks(edge, Weekday::Monday);
            assert!((4..=6).contains(&weeks.len()), "{edge}");
        }

        // February 2027 is exactly four weeks from a Monday.
        let weeks = month_weeks(date(2027, 2, 10), Weekday::Monday);
        assert_eq!(weeks.len(), 4);
        assert_eq!(weeks[0][0], date(2027, 2, 1));
        assert_eq!(weeks[3][6], date(2027, 2, 28));
    }

    #[test]
    fn bars_take_the_first_free_lane() {
        let entries = Entries::utc().with([
            days(1, 2, 4),
            days(2, 3, 3),
            days(3, 5, 6),
            timed(4, (3, 9, 0), (3, 10, 0)),
        ]);
        assert_eq!(
            lanes(&week_bars(&entries)),
            vec![(1, 0, 2, 0), (2, 1, 1, 1), (3, 3, 4, 0), (4, 1, 1, 2)]
        );
        assert_eq!(lane_count(&week_bars(&entries)), 3);
        assert_eq!(lane_count(&[]), 0);
    }

    #[test]
    fn bars_are_clipped_to_the_row_and_say_so() {
        let entries = Entries::utc().with([
            Entry::all_day(EntryId(1), "Long", date(2026, 10, 30), date(2026, 11, 12)),
            days(2, 8, 9),
            days(3, 20, 21),
        ]);
        let bars = week_bars(&entries);
        assert_eq!(bars.len(), 2);
        let long = bars[0];
        assert_eq!((long.first, long.last, long.days()), (0, 6, 7));
        assert!(long.starts_before && long.ends_after);
        let tail = bars[1];
        assert_eq!((tail.first, tail.last), (6, 6));
        assert!(!tail.starts_before && tail.ends_after);
        assert!(tail.covers(6) && !tail.covers(5));
    }

    #[test]
    fn a_timed_entry_past_midnight_covers_both_days() {
        let entries = Entries::utc().with([timed(1, (3, 22, 0), (4, 2, 0))]);
        assert_eq!(lanes(&week_bars(&entries)), vec![(1, 1, 2, 0)]);
    }

    #[test]
    fn no_days_no_bars() {
        let entries = Entries::utc().with([days(1, 2, 4)]);
        let all = entries.between(date(2026, 11, 2), date(2026, 11, 8));
        assert!(bars(&[], &all, entries.time_zone()).is_empty());
        let fitted = fit(Vec::new(), 0, 3);
        assert!(fitted.bars.is_empty() && fitted.hidden.is_empty());
    }

    #[test]
    fn everything_fits_when_there_are_enough_lanes() {
        let entries = Entries::utc().with([days(1, 2, 4), days(2, 3, 3), days(3, 3, 3)]);
        let fitted = fit(week_bars(&entries), 7, 3);
        assert_eq!(fitted.bars.len(), 3);
        assert_eq!(fitted.hidden, vec![0; 7]);
    }

    #[test]
    fn a_crowded_day_gives_its_last_lane_to_the_count() {
        let entries = Entries::utc().with([
            days(1, 2, 4),
            days(2, 3, 3),
            days(3, 3, 3),
            days(4, 3, 3),
            days(5, 5, 5),
        ]);
        let fitted = fit(week_bars(&entries), 7, 3);
        let shown: Vec<_> = fitted
            .bars
            .iter()
            .map(|bar| bar.occurrence.entry.0)
            .collect();
        assert_eq!(shown, vec![1, 2, 5]);
        assert_eq!(fitted.hidden, vec![0, 2, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn a_bar_on_the_last_lane_hides_if_any_of_its_days_is_crowded() {
        let entries = Entries::utc().with([
            days(1, 2, 2),
            days(2, 2, 2),
            days(3, 2, 2),
            days(4, 3, 3),
            // The longer bar goes first, so it has the top lane on Tuesday
            // and Wednesday.
            days(5, 3, 4),
        ]);
        let bars = week_bars(&entries);
        let two = fit(bars.clone(), 7, 2);
        assert_eq!(two.hidden, vec![2, 0, 0, 0, 0, 0, 0]);
        assert_eq!(two.bars.len(), 3);

        let none = fit(bars.clone(), 7, 0);
        assert!(none.bars.is_empty());
        assert_eq!(none.hidden, vec![3, 2, 1, 0, 0, 0, 0]);

        // With one lane Tuesday is crowded, so the bar that also covers
        // Wednesday is counted there too.
        let one = fit(bars, 7, 1);
        assert!(one.bars.is_empty());
        assert_eq!(one.hidden, vec![3, 2, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn minutes_and_instants_convert_both_ways() {
        let day = date(2026, 11, 3);
        assert_eq!(minute_on(day, at(3, 9, 30), &TimeZone::UTC), 570);
        assert_eq!(minute_on(day, at(2, 23, 0), &TimeZone::UTC), 0);
        assert_eq!(minute_on(day, at(4, 0, 0), &TimeZone::UTC), DAY_MINUTES);
        assert_eq!(instant_at(day, 570, &TimeZone::UTC), at(3, 9, 30));
        assert_eq!(instant_at(day, DAY_MINUTES, &TimeZone::UTC), at(4, 0, 0));
        assert_eq!(instant_at(day, 0, &TimeZone::UTC), at(3, 0, 0));
    }

    #[test]
    fn snapping_rounds_to_the_step_within_the_day() {
        assert_eq!(snap(7.0, 15), 0);
        assert_eq!(snap(8.0, 15), 15);
        assert_eq!(snap(577.0, 15), 570);
        assert_eq!(snap(-20.0, 15), 0);
        assert_eq!(snap(5000.0, 15), DAY_MINUTES);
        assert_eq!(snap(61.4, 0), 61);
    }

    fn day_blocks(entries: &Entries) -> Vec<Block> {
        let day = date(2026, 11, 3);
        blocks(day, &entries.on(day), entries.time_zone())
    }

    fn columns(blocks: &[Block]) -> Vec<(u64, usize, usize)> {
        let mut columns: Vec<_> = blocks
            .iter()
            .map(|block| (block.occurrence.entry.0, block.column, block.columns))
            .collect();
        columns.sort();
        columns
    }

    #[test]
    fn entries_apart_take_the_whole_width() {
        let entries = Entries::utc().with([
            timed(1, (3, 9, 0), (3, 10, 0)),
            timed(2, (3, 10, 0), (3, 11, 0)),
            days(3, 3, 3),
        ]);
        let blocks = day_blocks(&entries);
        assert_eq!(columns(&blocks), vec![(1, 0, 1), (2, 0, 1)]);
        assert_eq!((blocks[0].top, blocks[0].bottom), (540, 600));
    }

    #[test]
    fn overlapping_entries_share_the_width() {
        let entries = Entries::utc().with([
            timed(1, (3, 9, 0), (3, 11, 0)),
            timed(2, (3, 9, 30), (3, 10, 30)),
            timed(3, (3, 10, 30), (3, 11, 30)),
            timed(4, (3, 14, 0), (3, 15, 0)),
        ]);
        assert_eq!(
            columns(&day_blocks(&entries)),
            vec![(1, 0, 2), (2, 1, 2), (3, 1, 2), (4, 0, 1)]
        );
    }

    #[test]
    fn short_entries_are_drawn_tall_enough_and_kept_apart() {
        let entries = Entries::utc().with([
            timed(1, (3, 9, 0), (3, 9, 5)),
            timed(2, (3, 9, 10), (3, 9, 15)),
            timed(3, (3, 23, 50), (3, 23, 55)),
        ]);
        let blocks = day_blocks(&entries);
        assert_eq!(columns(&blocks), vec![(1, 0, 2), (2, 1, 2), (3, 0, 1)]);
        assert_eq!(blocks[0].drawn_bottom(), 570);
        assert_eq!(
            (blocks[2].drawn_top(), blocks[2].drawn_bottom()),
            (1410, DAY_MINUTES),
            "moved up to keep its height before midnight"
        );
    }

    #[test]
    fn an_entry_across_midnight_is_cut_at_the_day() {
        let entries = Entries::utc().with([
            timed(1, (2, 22, 0), (3, 2, 0)),
            timed(2, (3, 22, 0), (4, 2, 0)),
            timed(3, (2, 22, 0), (3, 0, 0)),
        ]);
        let blocks = day_blocks(&entries);
        assert_eq!(blocks.len(), 2, "one ending at midnight is not on the day");
        let morning = blocks[0];
        assert_eq!((morning.top, morning.bottom), (0, 120));
        assert!(morning.starts_before && !morning.ends_after);
        let night = blocks[1];
        assert_eq!((night.top, night.bottom), (1320, DAY_MINUTES));
        assert!(!night.starts_before && night.ends_after);
    }

    fn grid() -> Grid {
        Grid {
            width: 756.0,
            gutter: 56.0,
            days: 7,
            hour_height: 48.0,
            first_hour: 0,
            last_hour: 24,
        }
    }

    #[test]
    fn a_grid_can_show_part_of_the_day() {
        let grid = Grid {
            first_hour: 8,
            last_hour: 18,
            ..grid()
        };
        assert_eq!(grid.minutes(), (480, 1080));
        assert_eq!(grid.height(), 480.0);
        assert_eq!(grid.y_of(540), 48.0);
        assert_eq!(grid.y_of(420), -48.0);
        assert_eq!(grid.minutes_at(48.0), 540.0);
        assert_eq!(grid.minutes_at(-100.0), 480.0);
        assert_eq!(grid.minutes_at(9000.0), 1080.0);

        let entries = Entries::utc().with([
            timed(1, (3, 6, 0), (3, 7, 0)),
            timed(2, (3, 7, 0), (3, 9, 0)),
            timed(3, (3, 17, 0), (3, 20, 0)),
        ]);
        let blocks = day_blocks(&entries);
        assert_eq!(grid.block(0, &blocks[0]), None, "before the first hour");
        let early = grid.block(0, &blocks[1]);
        assert_eq!(early.map(|r| (r.y, r.height)), Some((0.0, 46.0)));
        let late = grid.block(0, &blocks[2]);
        assert_eq!(late.map(|r| (r.y, r.height)), Some((432.0, 46.0)));
    }

    #[test]
    fn hours_out_of_order_still_show_an_hour() {
        let grid = Grid {
            first_hour: 30,
            last_hour: 2,
            ..grid()
        };
        assert_eq!(grid.minutes(), (1380, 1440));
        assert_eq!(grid.height(), 48.0);
    }

    #[test]
    fn the_grid_maps_points_to_days_and_minutes() {
        let grid = grid();
        assert_eq!(grid.height(), 1152.0);
        assert_eq!(grid.size(), Size::new(756.0, 1152.0));
        assert_eq!(grid.day_width(), 100.0);
        assert_eq!(grid.day_x(2), 256.0);
        assert_eq!(grid.day_at(10.0), None);
        assert_eq!(grid.day_at(56.0), Some(0));
        assert_eq!(grid.day_at(355.0), Some(2));
        assert_eq!(grid.day_at(800.0), None);
        assert_eq!(grid.y_of(90), 72.0);
        assert_eq!(grid.minutes_at(72.0), 90.0);
        assert_eq!(grid.minutes_at(-5.0), 0.0);
        assert_eq!(grid.minutes_at(5000.0), 1440.0);
    }

    #[test]
    fn a_grid_with_no_room_does_not_divide_by_zero() {
        let grid = Grid {
            width: 20.0,
            gutter: 56.0,
            days: 0,
            hour_height: 0.0,
            first_hour: 0,
            last_hour: 24,
        };
        assert_eq!(grid.day_width(), 0.0);
        assert_eq!(grid.day_at(60.0), None);
        assert_eq!(grid.minutes_at(10.0), 0.0);
    }

    #[test]
    fn blocks_split_the_column_and_leave_a_sliver_free() {
        let entries = Entries::utc().with([
            timed(1, (3, 9, 0), (3, 11, 0)),
            timed(2, (3, 9, 30), (3, 10, 30)),
        ]);
        let blocks = day_blocks(&entries);
        let grid = grid();
        let (Some(first), Some(second)) = (grid.block(1, &blocks[0]), grid.block(1, &blocks[1]))
        else {
            panic!("both blocks are on show");
        };
        assert_eq!(first.x, 156.0);
        assert_eq!(first.width, 44.0);
        assert_eq!(first.y, 432.0);
        assert_eq!(first.height, 94.0);
        assert_eq!(second.x, 202.0);
        assert!(second.x + second.width <= 256.0 - BLOCK_GUTTER);
    }
}
