//! A form for creating or changing an entry: its title, whether it lasts
//! all day, when it starts and ends, how it repeats and when the repeats
//! stop, its colour, where it is and any notes.
//!
//! The form is as tall as its fields, and grows when a repeat is chosen.
//! Where room is short, put it in a scroll area.
//!
//! The form has no buttons of its own. Put it in a dialog, a popover or a
//! side panel with whatever actions the app wants, start it from the
//! [`Draft`] the calendar asked for or from an existing [`Entry`], and
//! turn it back into an entry with [`State::entry`]:
//!
//! ```
//! use iced_cube::data::calendar::editor;
//! use iced_cube::data::calendar::{Draft, EntryId, Repeat, When, instant};
//! use jiff::civil::{date, time};
//! use jiff::tz::TimeZone;
//!
//! let utc = TimeZone::UTC;
//! let at = |hour| instant(date(2026, 6, 19), time(hour, 0, 0, 0), &utc);
//! let draft = Draft { when: When::timed(at(11), at(12)) };
//!
//! let mut form = editor::State::new(&draft, &utc);
//! form.update(editor::Event::Title(String::from("Retrospective")));
//! // Starting an hour later keeps the entry an hour long.
//! form.update(editor::Event::Time(editor::Field::Start, 12 * 60));
//!
//! // Every two weeks, six times in all.
//! form.update(editor::Event::Repeats(editor::Repeats::Weekly));
//! form.update(editor::Event::Interval(String::from("2")));
//! form.update(editor::Event::Stops(editor::Stops::After));
//! form.update(editor::Event::Count(String::from("6")));
//! form.update(editor::Event::Location(String::from("Room 4")));
//!
//! let entry = form.entry(EntryId(7));
//! assert_eq!(entry.title(), "Retrospective");
//! assert_eq!(entry.when(), When::timed(at(12), at(13)));
//! assert_eq!(entry.repeat_of(), Some(Repeat::weekly().interval(2).count(6)));
//! assert_eq!(entry.location_of(), Some("Room 4"));
//! ```

use iced::widget::button::Status;
use iced::widget::text_editor::{Action, Content};
use iced::widget::{self, Column, Row, Space, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};

use super::entry::{Draft, Entry, EntryId, Tone, When, add_days, date_of};
use super::layout::{DAY_MINUTES, instant_at, minute_on, month_weeks};
use super::parts::weekday_label;
use super::repeat::{Ends, Frequency, Repeat};
use super::style;
use crate::forms::select::select;
use crate::forms::textarea::textarea;
use crate::lucide;
use crate::overlay::anchored::Align;
use crate::overlay::popover::popover;
use crate::primitives::button::{self, Size, button};
use crate::primitives::checkbox::checkbox;
use crate::primitives::icon_button::{self, icon_button};
use crate::primitives::input::{Shortcuts, input};
use crate::theme::{self, Tokens, space, text_size};

/// The step between the times offered, in minutes.
pub const TIME_STEP: u16 = 15;
/// The title given to an entry saved without one.
pub const UNTITLED: &str = "(No title)";
/// How long a new timed entry is when the draft was for whole days.
pub const DEFAULT_MINUTES: u16 = 60;

/// How many occurrences a new "after" ending starts at.
pub const DEFAULT_COUNT: u32 = 10;
/// The most digits the interval and the count take.
pub const NUMBER_DIGITS: usize = 3;

/// The side of a day in the date picker.
const CELL: f32 = 32.0;
/// The width of a time's menu.
const TIME_WIDTH: f32 = 96.0;
/// The width of the names beside the fields.
const NAME_WIDTH: f32 = 48.0;
/// The width of a field for a small number.
const NUMBER_WIDTH: f32 = 64.0;
/// The height the notes start at, about two lines.
const NOTES_HEIGHT: f32 = 64.0;

/// The id of the notes field, for focusing it.
pub fn notes_id() -> widget::Id {
    widget::Id::new("iced-cube-calendar-notes")
}

/// The start or the end of the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    Start,
    End,
}

/// A time of day in minutes from midnight, shown as "09:30".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimeOfDay(pub u16);

impl TimeOfDay {
    /// Every time offered, a day's worth at [`TIME_STEP`] apart.
    pub fn all() -> Vec<TimeOfDay> {
        (0..DAY_MINUTES)
            .step_by(usize::from(TIME_STEP))
            .map(TimeOfDay)
            .collect()
    }

    /// Every time offered, starting the list at `first` and carrying on
    /// round midnight, so a menu opens near the time it is about rather
    /// than at the start of the day.
    pub fn from(first: u16) -> Vec<TimeOfDay> {
        let mut times = Self::all();
        let first = (first.min(DAY_MINUTES - 1) / TIME_STEP) * TIME_STEP;
        let split = times.iter().position(|time| time.0 >= first).unwrap_or(0);
        times.rotate_left(split);
        times
    }

    /// Reads a time typed as "9", "09", "9:40", "09.40", "940" or "0940".
    /// A bare hour is on the hour. `None` for anything else or a time
    /// outside the day.
    pub fn parse(text: &str) -> Option<TimeOfDay> {
        let text = text.trim();
        let separator = |c: char| c == ':' || c == '.';
        if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit() || separator(c)) {
            return None;
        }
        let (hours, minutes) = match text.split_once(separator) {
            Some(parts) => parts,
            None if (1..=2).contains(&text.len()) => (text, "00"),
            None if (3..=4).contains(&text.len()) => text.split_at(text.len() - 2),
            None => return None,
        };
        if !(1..=2).contains(&hours.len()) || minutes.len() != 2 {
            return None;
        }
        let (hours, minutes) = (hours.parse::<u16>().ok()?, minutes.parse::<u16>().ok()?);
        (hours < 24 && minutes < 60).then_some(TimeOfDay(hours * 60 + minutes))
    }
}

impl std::fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.0 / 60, self.0 % 60)
    }
}

/// Whether the entry repeats and in what unit, as the menu offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Repeats {
    #[default]
    Never,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl Repeats {
    /// The choices in the menu.
    pub const ALL: [Repeats; 5] = [
        Repeats::Never,
        Repeats::Daily,
        Repeats::Weekly,
        Repeats::Monthly,
        Repeats::Yearly,
    ];

    /// The choice that stands for `frequency`.
    pub fn of(frequency: Option<Frequency>) -> Self {
        match frequency {
            None => Repeats::Never,
            Some(Frequency::Daily) => Repeats::Daily,
            Some(Frequency::Weekly) => Repeats::Weekly,
            Some(Frequency::Monthly) => Repeats::Monthly,
            Some(Frequency::Yearly) => Repeats::Yearly,
        }
    }

    /// The unit the entry repeats in, or `None` when it does not.
    pub fn frequency(self) -> Option<Frequency> {
        match self {
            Repeats::Never => None,
            Repeats::Daily => Some(Frequency::Daily),
            Repeats::Weekly => Some(Frequency::Weekly),
            Repeats::Monthly => Some(Frequency::Monthly),
            Repeats::Yearly => Some(Frequency::Yearly),
        }
    }

    /// The name of the unit after a number: "week" after 1, "weeks" after
    /// any other.
    pub fn unit(self, interval: u32) -> &'static str {
        let one = interval == 1;
        match self {
            Repeats::Never => "",
            Repeats::Daily if one => "day",
            Repeats::Daily => "days",
            Repeats::Weekly if one => "week",
            Repeats::Weekly => "weeks",
            Repeats::Monthly if one => "month",
            Repeats::Monthly => "months",
            Repeats::Yearly if one => "year",
            Repeats::Yearly => "years",
        }
    }
}

impl std::fmt::Display for Repeats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Repeats::Never => "Does not repeat",
            Repeats::Daily => "Daily",
            Repeats::Weekly => "Weekly",
            Repeats::Monthly => "Monthly",
            Repeats::Yearly => "Yearly",
        })
    }
}

/// How a repeating entry stops, as the menu offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Stops {
    #[default]
    Never,
    /// On a date picked beside the menu.
    On,
    /// After a number of occurrences typed beside the menu.
    After,
}

impl Stops {
    /// The choices in the menu.
    pub const ALL: [Stops; 3] = [Stops::Never, Stops::On, Stops::After];
}

impl std::fmt::Display for Stops {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Stops::Never => "Never",
            Stops::On => "On a date",
            Stops::After => "After",
        })
    }
}

/// A [`Tone`] with a name for the colour menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Colour(pub Tone);

impl std::fmt::Display for Colour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            Tone::Primary => "Default",
            Tone::Neutral => "Neutral",
            Tone::Success => "Green",
            Tone::Warning => "Amber",
            Tone::Destructive => "Red",
        })
    }
}

/// Changes to the form.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Title(String),
    AllDay(bool),
    /// Sets the start or end date, and closes the date picker.
    Date(Field, Date),
    /// Sets the start or end time, in minutes from midnight.
    Time(Field, u16),
    Repeats(Repeats),
    /// The text of the "every" field. Only its digits are kept.
    Interval(String),
    Stops(Stops),
    /// Sets the last date of the repeat, and closes the date picker.
    Until(Date),
    /// The text of the "after" field. Only its digits are kept.
    Count(String),
    Tone(Tone),
    Location(String),
    /// An edit to the notes.
    Notes(Action),
    /// Opens the date picker for a field, or closes whichever is open
    /// with `None`.
    Picker(Option<Field>),
    /// Opens or closes the date picker for the last date of the repeat.
    UntilPicker(bool),
    /// Moves the date picker by this many months.
    PickerMonth(i32),
}

/// A date the form picks from a month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Field(Field),
    Until,
}

/// A date picker that is open: whose it is, and the month it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Picker {
    target: Target,
    month: Date,
}

/// What the form holds. The end is kept at or after the start: moving the
/// start moves the end with it, so the entry keeps its length. The last
/// date of a repeat is kept on or after the start.
#[derive(Debug, Clone)]
pub struct State {
    id: Option<EntryId>,
    title: String,
    all_day: bool,
    start: (Date, u16),
    end: (Date, u16),
    /// The instants the form opened with, kept while their field still
    /// shows them so seconds are not lost.
    opened: Option<(Timestamp, Timestamp)>,
    /// Whether the end was put on the next day because its time was not
    /// after the start's, rather than by picking its date.
    rolled: bool,
    repeats: Repeats,
    interval: String,
    stops: Stops,
    until: Date,
    count: String,
    tone: Tone,
    tag: Option<String>,
    location: String,
    notes: Content,
    /// The notes the form opened with, returned as they were until the
    /// notes are edited.
    opened_notes: Option<String>,
    time_zone: TimeZone,
    week_start: Weekday,
    picker: Option<Picker>,
}

/// The digits of `text`, up to [`NUMBER_DIGITS`] of them, without leading
/// zeros, so the field never shows a 0 the form would read as 1.
fn digits(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_digit)
        .skip_while(|c| *c == '0')
        .take(NUMBER_DIGITS)
        .collect()
}

/// The number a field holds, at least 1. An empty field counts as 1.
fn number(text: &str) -> u32 {
    text.parse::<u32>().unwrap_or(1).max(1)
}

impl State {
    /// A form for a new entry over the span the calendar asked for.
    pub fn new(draft: &Draft, time_zone: &TimeZone) -> Self {
        Self::over(draft.when, time_zone)
    }

    /// A form for changing `entry`.
    pub fn edit(entry: &Entry, time_zone: &TimeZone) -> Self {
        let mut form = Self {
            id: Some(entry.id()),
            title: entry.title().to_owned(),
            tone: entry.tone_of(),
            tag: entry.tag_of().map(str::to_owned),
            location: entry.location_of().unwrap_or_default().to_owned(),
            notes: Content::with_text(entry.notes_of().unwrap_or_default()),
            opened_notes: entry.notes_of().map(str::to_owned),
            ..Self::over(entry.when(), time_zone)
        };
        let Some(repeat) = entry.repeat_of() else {
            return form;
        };
        form.repeats = Repeats::of(Some(repeat.frequency()));
        form.interval = repeat.every().to_string();
        match repeat.ends() {
            Ends::Never => {}
            Ends::On(date) => {
                form.stops = Stops::On;
                form.until = date;
            }
            Ends::After(count) => {
                form.stops = Stops::After;
                form.count = count.to_string();
            }
        }
        form
    }

    fn over(when: When, time_zone: &TimeZone) -> Self {
        let nine = 9 * 60;
        let (all_day, start, end, opened) = match when {
            When::AllDay { first, last } => {
                (true, (first, nine), (last, nine + DEFAULT_MINUTES), None)
            }
            When::Timed { start, end } => (
                false,
                local(start, time_zone),
                local(end.max(start), time_zone),
                Some((start, end.max(start))),
            ),
        };
        Self {
            id: None,
            title: String::new(),
            all_day,
            start,
            end,
            opened,
            rolled: false,
            repeats: Repeats::Never,
            interval: String::from("1"),
            stops: Stops::Never,
            until: start.0.checked_add(1.month()).unwrap_or(start.0),
            count: DEFAULT_COUNT.to_string(),
            tone: Tone::default(),
            tag: None,
            location: String::new(),
            notes: Content::new(),
            opened_notes: None,
            time_zone: time_zone.clone(),
            week_start: Weekday::Monday,
            picker: None,
        }
    }

    /// The day weeks start on in the date picker. Defaults to Monday.
    pub fn with_week_start(mut self, week_start: Weekday) -> Self {
        self.week_start = week_start;
        self
    }

    /// The day weeks start on in the date picker.
    pub fn week_start(&self) -> Weekday {
        self.week_start
    }

    /// The entry being changed, or `None` for a new one.
    pub fn id(&self) -> Option<EntryId> {
        self.id
    }

    pub fn is_new(&self) -> bool {
        self.id.is_none()
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn is_all_day(&self) -> bool {
        self.all_day
    }

    pub fn repeats(&self) -> Repeats {
        self.repeats
    }

    /// How many units pass between repeats, at least 1.
    pub fn interval(&self) -> u32 {
        number(&self.interval)
    }

    pub fn stops(&self) -> Stops {
        self.stops
    }

    /// The last date of the repeat, used when it stops on a date.
    pub fn until(&self) -> Date {
        self.until
    }

    /// How many occurrences there are, used when the repeat stops after
    /// a number of them. At least 1.
    pub fn count(&self) -> u32 {
        number(&self.count)
    }

    /// The repeat rule the form describes, or `None` when the entry does
    /// not repeat.
    pub fn repeat(&self) -> Option<Repeat> {
        let rule = Repeat::new(self.repeats.frequency()?).interval(self.interval());
        Some(match self.stops {
            Stops::Never => rule,
            Stops::On => rule.until(self.until),
            Stops::After => rule.count(self.count()),
        })
    }

    pub fn tone(&self) -> Tone {
        self.tone
    }

    pub fn location(&self) -> &str {
        &self.location
    }

    /// The notes as text. Notes the form opened with come back exactly as
    /// they were until they are edited; edited notes lose the space around
    /// them.
    pub fn notes(&self) -> String {
        if let Some(notes) = &self.opened_notes {
            return notes.clone();
        }
        self.notes.text().trim().to_owned()
    }

    /// The notes buffer, for drawing the notes in a text area of the
    /// app's own.
    pub fn notes_content(&self) -> &Content {
        &self.notes
    }

    /// The month the open date picker shows, as its first day, or `None`
    /// when no picker is open.
    pub fn picker_month(&self) -> Option<Date> {
        self.picker.map(|picker| picker.month)
    }

    /// The text of the "every" field as typed, which may be blank.
    pub fn interval_text(&self) -> &str {
        &self.interval
    }

    /// The text of the "after" field as typed, which may be blank.
    pub fn count_text(&self) -> &str {
        &self.count
    }

    /// Whether the date picker for the last date of the repeat is open.
    pub fn is_picking_until(&self) -> bool {
        self.picker
            .is_some_and(|picker| picker.target == Target::Until)
    }

    /// The date of a field.
    pub fn date(&self, field: Field) -> Date {
        self.at(field).0
    }

    /// The time of a field, in minutes from midnight.
    pub fn time(&self, field: Field) -> u16 {
        self.at(field).1
    }

    /// The times a field's menu lists. The start's is the whole day in
    /// order. The end's begins just after the start when both are on the
    /// same day and carries on round midnight, so the times before the
    /// start, which would end the entry the next day, come last. The menu
    /// opens with the field's time in the middle, filters as you type and
    /// takes a typed time off the [`TIME_STEP`] grid.
    pub fn times(&self, field: Field) -> Vec<TimeOfDay> {
        match field {
            Field::End if self.end.0 == self.start.0 || self.rolled => {
                TimeOfDay::from((self.start.1 + TIME_STEP) % DAY_MINUTES)
            }
            _ => TimeOfDay::all(),
        }
    }

    /// The start or end whose date picker is open.
    pub fn picker(&self) -> Option<Field> {
        match self.picker?.target {
            Target::Field(field) => Some(field),
            Target::Until => None,
        }
    }

    /// The span the form describes.
    pub fn when(&self) -> When {
        if self.all_day {
            return When::all_day(self.start.0, self.end.0);
        }
        let (opened_start, opened_end) = self.opened.unzip();
        When::timed(
            self.instant(self.start, opened_start),
            self.instant(self.end, opened_end),
        )
    }

    /// The instant a field stands for: the one the form opened with while
    /// the field still shows it, so its seconds are kept.
    fn instant(&self, field: (Date, u16), opened: Option<Timestamp>) -> Timestamp {
        match opened {
            Some(instant) if local(instant, &self.time_zone) == field => instant,
            _ => instant_at(field.0, field.1, &self.time_zone),
        }
    }

    /// The entry the form describes. A new entry takes `new_id`; a changed
    /// one keeps its own. A blank title becomes [`UNTITLED`].
    pub fn entry(&self, new_id: EntryId) -> Entry {
        let title = match self.title.trim() {
            "" => UNTITLED,
            title => title,
        };
        let mut entry = Entry::at(self.id.unwrap_or(new_id), title, self.when())
            .tone(self.tone)
            .location(self.location.clone())
            .notes(self.notes());
        if let Some(tag) = &self.tag {
            entry = entry.tag(tag.clone());
        }
        if let Some(rule) = self.repeat() {
            entry = entry.repeat(rule);
        }
        entry
    }

    pub fn update(&mut self, event: Event) {
        match event {
            Event::Title(title) => self.title = title,
            Event::AllDay(all_day) => self.set_all_day(all_day),
            Event::Date(field, date) => {
                self.picker = None;
                if field == Field::End {
                    self.rolled = false;
                }
                self.set(field, (date, self.time(field)));
            }
            Event::Time(field, minute) => {
                let mut date = self.date(field);
                // A rolled end goes back to the start's day first, so a
                // later time can end the entry on the day it starts.
                if field == Field::End && self.rolled {
                    date = self.start.0;
                    self.rolled = false;
                }
                self.set(field, (date, minute.min(DAY_MINUTES - 1)));
            }
            Event::Repeats(repeats) => {
                self.repeats = repeats;
                if repeats == Repeats::Never && self.is_picking_until() {
                    self.picker = None;
                }
            }
            Event::Interval(text) => self.interval = digits(&text),
            Event::Stops(stops) => {
                self.stops = stops;
                if stops != Stops::On && self.is_picking_until() {
                    self.picker = None;
                }
            }
            Event::Until(date) => {
                self.picker = None;
                self.until = date.max(self.start.0);
            }
            Event::Count(text) => self.count = digits(&text),
            Event::Tone(tone) => self.tone = tone,
            Event::Location(location) => self.location = location,
            Event::Notes(action) => {
                if action.is_edit() {
                    self.opened_notes = None;
                }
                self.notes.perform(action);
            }
            Event::Picker(field) => {
                self.picker = field.map(|field| Picker {
                    target: Target::Field(field),
                    month: self.date(field).first_of_month(),
                });
            }
            Event::UntilPicker(open) => {
                self.picker = open.then(|| Picker {
                    target: Target::Until,
                    month: self.until.first_of_month(),
                });
            }
            Event::PickerMonth(months) => {
                if let Some(picker) = &mut self.picker {
                    picker.month = picker
                        .month
                        .checked_add(i64::from(months.clamp(-1200, 1200)).months())
                        .unwrap_or(picker.month);
                }
            }
        }
    }

    fn at(&self, field: Field) -> (Date, u16) {
        match field {
            Field::Start => self.start,
            Field::End => self.end,
        }
    }

    /// Sets a field. A new start carries the end along, keeping the
    /// length; a new end is kept from going before the start.
    fn set(&mut self, field: Field, to: (Date, u16)) {
        match field {
            Field::Start => {
                let days = days_between(self.start.0, self.end.0);
                let minutes = i32::from(self.end.1) - i32::from(self.start.1);
                self.start = to;
                let end = i32::from(to.1) + minutes;
                self.end = (
                    add_days(to.0, days + end.div_euclid(i32::from(DAY_MINUTES))),
                    u16::try_from(end.rem_euclid(i32::from(DAY_MINUTES))).unwrap_or(0),
                );
            }
            Field::End => self.end = to,
        }
        self.until = self.until.max(self.start.0);
        self.settle(field);
    }

    /// The date a picker is for.
    fn target_date(&self, target: Target) -> Date {
        match target {
            Target::Field(field) => self.date(field),
            Target::Until => self.until,
        }
    }

    /// Switches between whole days and times. A timed end at midnight is
    /// the end of the day before, so turning whole days on takes the day
    /// before as the last, and turning them off gives it back.
    fn set_all_day(&mut self, all_day: bool) {
        if self.all_day == all_day {
            return;
        }
        self.all_day = all_day;
        let midnight_end = self.end.1 == 0;
        match all_day {
            true if midnight_end && self.end.0 > self.start.0 => {
                self.end.0 = add_days(self.end.0, -1);
            }
            false if midnight_end => self.end.0 = add_days(self.end.0, 1),
            _ => {}
        }
        self.rolled = false;
        self.settle(Field::End);
    }

    /// Puts the end back at or after the start. Whole days may start and
    /// end on the same day; a timed entry is at least one step long.
    fn settle(&mut self, changed: Field) {
        if self.all_day {
            if self.end.0 < self.start.0 {
                self.end.0 = self.start.0;
            }
            return;
        }
        if self.end > self.start {
            return;
        }
        // An end time picked on the start's day at or before the start
        // means the next day, as in "22:00 to 01:00".
        if changed == Field::End && self.end.0 == self.start.0 {
            self.end.0 = add_days(self.start.0, 1);
            self.rolled = true;
            return;
        }
        let end = self.start.1 + TIME_STEP;
        self.end = if end >= DAY_MINUTES {
            (add_days(self.start.0, 1), end - DAY_MINUTES)
        } else {
            (self.start.0, end)
        };
    }
}

/// The date and minute of `instant` in `time_zone`.
fn local(instant: Timestamp, time_zone: &TimeZone) -> (Date, u16) {
    let date = date_of(instant, time_zone);
    (date, minute_on(date, instant, time_zone))
}

fn days_between(from: Date, to: Date) -> i32 {
    i32::try_from(to.duration_since(from).as_hours() / 24).unwrap_or(0)
}

/// A part of the form, for leaving it out with [`Editor::without`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Part {
    Title,
    AllDay,
    Starts,
    Ends,
    /// The repeat menu, and the "every" and "stops" rows under it.
    Repeat,
    Colour,
    Location,
    Notes,
}

impl Part {
    pub const ALL: [Part; 8] = [
        Part::Title,
        Part::AllDay,
        Part::Starts,
        Part::Ends,
        Part::Repeat,
        Part::Colour,
        Part::Location,
        Part::Notes,
    ];
}

/// An entry form builder. Convert it into an [`Element`] to render the
/// whole form, or call [`fields`](Editor::fields) to lay out its parts.
pub struct Editor<'a, Message> {
    state: &'a State,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
    on_submit: Option<Message>,
    shortcuts: Shortcuts,
    without: Vec<Part>,
}

/// Renders the form for `state`. Without [`on_event`](Editor::on_event)
/// every control renders disabled.
pub fn editor<Message>(state: &State) -> Editor<'_, Message> {
    Editor {
        state,
        on_event: None,
        on_submit: None,
        shortcuts: Shortcuts::all(),
        without: Vec::new(),
    }
}

impl<Message> std::fmt::Debug for Editor<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Editor")
            .field("state", self.state)
            .field("enabled", &self.on_event.is_some())
            .field("submits", &self.on_submit.is_some())
            .field("shortcuts", &self.shortcuts)
            .field("without", &self.without)
            .finish()
    }
}

/// The form's parts as separate elements, for an app that lays them out
/// itself. A part left out with [`Editor::without`] is `None`, and so are
/// `every` and `stops` while the entry does not repeat.
pub struct Fields<'a, Message> {
    pub title: Option<Element<'a, Message>>,
    pub all_day: Option<Element<'a, Message>>,
    pub starts: Option<Element<'a, Message>>,
    pub ends: Option<Element<'a, Message>>,
    pub repeat: Option<Element<'a, Message>>,
    /// "Every 2 weeks".
    pub every: Option<Element<'a, Message>>,
    /// "Stops on a date" or "after 6 times".
    pub stops: Option<Element<'a, Message>>,
    pub colour: Option<Element<'a, Message>>,
    pub location: Option<Element<'a, Message>>,
    pub notes: Option<Element<'a, Message>>,
}

impl<Message> std::fmt::Debug for Fields<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fields")
            .field("title", &self.title.is_some())
            .field("all_day", &self.all_day.is_some())
            .field("starts", &self.starts.is_some())
            .field("ends", &self.ends.is_some())
            .field("repeat", &self.repeat.is_some())
            .field("every", &self.every.is_some())
            .field("stops", &self.stops.is_some())
            .field("colour", &self.colour.is_some())
            .field("location", &self.location.is_some())
            .field("notes", &self.notes.is_some())
            .finish()
    }
}

impl<'a, Message> Editor<'a, Message> {
    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// Sent when Enter is pressed in the title or the location, usually
    /// the same message as the Save button.
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// The editing shortcuts of the form's text fields. Defaults to all
    /// of them.
    pub fn shortcuts(mut self, shortcuts: Shortcuts) -> Self {
        self.shortcuts = shortcuts;
        self
    }

    /// Leaves a part out of the form. What the entry holds for it is kept.
    pub fn without(mut self, part: Part) -> Self {
        if !self.without.contains(&part) {
            self.without.push(part);
        }
        self
    }

    /// Whether the form draws `part`.
    pub fn shows(&self, part: Part) -> bool {
        !self.without.contains(&part)
    }
}

impl<'a, Message: Clone + 'a> Editor<'a, Message> {
    /// The form's parts as separate elements.
    pub fn fields(self) -> Fields<'a, Message> {
        let shows = |part| !self.without.contains(&part);
        let Editor {
            state,
            on_event,
            on_submit,
            shortcuts,
            ..
        } = self;
        let on_event: Handler<'a, Message> = on_event.map(std::rc::Rc::from);
        let repeating = state.repeats() != Repeats::Never;

        let title = shows(Part::Title).then(|| {
            input("Add a title", state.title())
                .shortcuts(shortcuts)
                .on_input_maybe(
                    on_event
                        .clone()
                        .map(|handler| move |title| handler(Event::Title(title))),
                )
                .on_submit_maybe(on_submit.clone())
                .into()
        });
        // A check box, since the form only applies on save.
        let all_day = shows(Part::AllDay).then(|| {
            checkbox(state.is_all_day())
                .label("All day")
                .on_toggle_maybe(
                    on_event
                        .clone()
                        .map(|handler| move |on| handler(Event::AllDay(on))),
                )
                .into()
        });
        let repeat = shows(Part::Repeat).then(|| {
            select(&Repeats::ALL[..], Some(state.repeats()))
                .width(Length::Fill)
                .on_select_maybe(
                    on_event
                        .clone()
                        .map(|handler| move |repeats| handler(Event::Repeats(repeats))),
                )
                .into()
        });
        let colour = shows(Part::Colour).then(|| colour_menu(state, &on_event));
        let location = shows(Part::Location).then(|| {
            input("Add a location", state.location())
                .icon(lucide!(MapPin))
                .shortcuts(shortcuts)
                .on_input_maybe(
                    on_event
                        .clone()
                        .map(|handler| move |location| handler(Event::Location(location))),
                )
                .on_submit_maybe(on_submit)
                .into()
        });
        let notes = shows(Part::Notes).then(|| {
            textarea(&state.notes)
                .id(notes_id())
                .placeholder("Add notes")
                .min_height(NOTES_HEIGHT)
                .shortcuts(shortcuts)
                .on_action_maybe(
                    on_event
                        .clone()
                        .map(|handler| move |action| handler(Event::Notes(action))),
                )
                .into()
        });

        Fields {
            title,
            all_day,
            starts: shows(Part::Starts).then(|| moment(state, Field::Start, "Starts", &on_event)),
            ends: shows(Part::Ends).then(|| moment(state, Field::End, "Ends", &on_event)),
            repeat,
            every: (shows(Part::Repeat) && repeating).then(|| every(state, shortcuts, &on_event)),
            stops: (shows(Part::Repeat) && repeating).then(|| stops(state, shortcuts, &on_event)),
            colour,
            location,
            notes,
        }
    }
}

/// The colour menu, each colour with a swatch before its name.
fn colour_menu<'a, Message: Clone + 'a>(
    state: &State,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let colours: Vec<Colour> = Tone::ALL.into_iter().map(Colour).collect();
    select(colours, Some(Colour(state.tone())))
        .width(Length::Fill)
        .leading(|colour: &Colour| {
            let tone = colour.0;
            Some(theme::swatch(move |theme: &Theme| style::tone(&Tokens::of(theme), tone)).into())
        })
        .on_select_maybe(
            on_event
                .clone()
                .map(|handler| move |colour: Colour| handler(Event::Tone(colour.0))),
        )
        .into()
}

type Handler<'a, Message> = Option<std::rc::Rc<dyn Fn(Event) -> Message + 'a>>;

fn send<Message>(handler: &Handler<'_, Message>, event: Event) -> Option<Message> {
    handler.as_ref().map(|handler| handler(event))
}

impl<'a, Message: Clone + 'a> From<Editor<'a, Message>> for Element<'a, Message> {
    fn from(editor: Editor<'a, Message>) -> Self {
        let Fields {
            title,
            all_day,
            starts,
            ends,
            repeat,
            every,
            stops,
            colour,
            location,
            notes,
        } = editor.fields();

        let menus: Vec<Element<'a, Message>> = [
            repeat.map(|menu| container(menu).width(Length::FillPortion(3)).into()),
            colour.map(|menu| container(menu).width(Length::FillPortion(2)).into()),
        ]
        .into_iter()
        .flatten()
        .collect();
        let menus =
            (!menus.is_empty()).then(|| Row::with_children(menus).spacing(space::SM).into());

        Column::with_children(
            [
                title, all_day, starts, ends, menus, every, stops, location, notes,
            ]
            .into_iter()
            .flatten(),
        )
        .spacing(space::MD)
        .width(Length::Fill)
        .into()
    }
}

/// The name beside a field.
fn name<'a, Message: 'a>(name: &'a str) -> Element<'a, Message> {
    let label = text(name)
        .size(text_size::SM)
        .wrapping(text::Wrapping::None)
        .style(|theme: &Theme| text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        });
    container(label).width(Length::Fixed(NAME_WIDTH)).into()
}

/// A field for a small whole number.
fn number_field<'a, Message: Clone + 'a>(
    value: &str,
    shortcuts: Shortcuts,
    on_event: &Handler<'a, Message>,
    event: fn(String) -> Event,
) -> Element<'a, Message> {
    input("1", value)
        .width(NUMBER_WIDTH)
        .shortcuts(shortcuts)
        .on_input_maybe(
            on_event
                .clone()
                .map(|handler| move |text| handler(event(text))),
        )
        .into()
}

/// How many units pass between repeats: "Every 2 weeks".
fn every<'a, Message: Clone + 'a>(
    state: &'a State,
    shortcuts: Shortcuts,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let unit = text(state.repeats().unit(state.interval()))
        .size(text_size::SM)
        .wrapping(text::Wrapping::None);
    row![
        name("Every"),
        number_field(&state.interval, shortcuts, on_event, Event::Interval),
        unit,
    ]
    .spacing(space::SM)
    .align_y(Alignment::Center)
    .into()
}

/// How the repeat stops: never, on a date or after a number of times.
fn stops<'a, Message: Clone + 'a>(
    state: &'a State,
    shortcuts: Shortcuts,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let handler = on_event.clone();
    let choice = select(&Stops::ALL[..], Some(state.stops()))
        .width(Length::Fill)
        .on_select_maybe(handler.map(|handler| move |stops| handler(Event::Stops(stops))));
    let mut controls = Row::new()
        .spacing(space::SM)
        .align_y(Alignment::Center)
        .push(name("Stops"))
        .push(choice);
    match state.stops() {
        Stops::Never => {}
        Stops::On => controls = controls.push(date_field(state, Target::Until, on_event)),
        Stops::After => {
            let times = text(if state.count() == 1 { "time" } else { "times" })
                .size(text_size::SM)
                .wrapping(text::Wrapping::None);
            controls = controls
                .push(number_field(
                    &state.count,
                    shortcuts,
                    on_event,
                    Event::Count,
                ))
                .push(times);
        }
    }
    controls.into()
}

/// One end of the entry: its name, its date and, unless it lasts all day,
/// its time.
fn moment<'a, Message: Clone + 'a>(
    state: &'a State,
    field: Field,
    label: &'a str,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let mut controls = Row::new()
        .spacing(space::SM)
        .align_y(Alignment::Center)
        .push(name(label))
        .push(date_field(state, Target::Field(field), on_event));
    if !state.is_all_day() {
        let handler = on_event.clone();
        controls = controls.push(
            select(state.times(field), Some(TimeOfDay(state.time(field))))
                .width(TIME_WIDTH)
                .searchable(true)
                .search_placeholder("hh:mm")
                .on_custom({
                    let handler = handler.clone();
                    move |text| {
                        let handler = handler.as_ref()?;
                        let time = TimeOfDay::parse(&text)?;
                        Some(handler(Event::Time(field, time.0)))
                    }
                })
                .on_select_maybe(
                    handler
                        .map(|handler| move |time: TimeOfDay| handler(Event::Time(field, time.0))),
                ),
        );
    }
    controls.into()
}

/// A date as a button that opens a month to pick another from.
fn date_field<'a, Message: Clone + 'a>(
    state: &'a State,
    target: Target,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let open = state.picker.filter(|picker| picker.target == target);
    let toggle = match target {
        Target::Field(field) => Event::Picker(open.is_none().then_some(field)),
        Target::Until => Event::UntilPicker(open.is_none()),
    };
    let date = state.target_date(target);
    let trigger = button(date.strftime("%a %-d %b %Y").to_string())
        .variant(button::Variant::Outline)
        .icon(lucide!(Calendar))
        .align_x(Alignment::Start)
        .width(Length::Fill)
        .on_press_maybe(send(on_event, toggle));

    let content: Element<'a, Message> = match open {
        Some(picker) => month_picker(state, picker, on_event),
        None => Space::new().into(),
    };
    let mut date = popover(trigger, content)
        .open(open.is_some())
        .align(Align::Start)
        .width(Length::Shrink)
        .padding(space::MD);
    if let Some(close) = send(on_event, Event::Picker(None)) {
        date = date.on_dismiss(close);
    }
    // A narrow column cuts the date short rather than letting it run
    // under the time beside it.
    container(date).width(Length::Fill).clip(true).into()
}

/// A month of days to pick from, with arrows to the months either side.
fn month_picker<'a, Message: Clone + 'a>(
    state: &State,
    picker: Picker,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let step = |glyph, label: &'static str, months: i32| {
        icon_button(glyph)
            .label(label)
            .variant(icon_button::Variant::Ghost)
            .size(Size::Sm)
            .on_press_maybe(send(on_event, Event::PickerMonth(months)))
    };
    let name = text(picker.month.strftime("%B %Y").to_string())
        .size(text_size::SM)
        .font(theme::semibold())
        .wrapping(text::Wrapping::None);
    let header = row![
        step(lucide!(ChevronLeft), "Previous month", -1),
        container(name).center_x(Length::Fill),
        step(lucide!(ChevronRight), "Next month", 1),
    ]
    .align_y(Alignment::Center);

    let chosen = state.target_date(picker.target);
    let target = picker.target;
    let pick = on_event.clone().map(|handler| {
        move |date: Date| match target {
            Target::Field(field) => handler(Event::Date(field, date)),
            Target::Until => handler(Event::Until(date)),
        }
    });
    let grid = month_grid(picker.month, state.week_start, Some(chosen), pick);
    column![header, grid]
        .spacing(space::SM)
        .width(Length::Fixed(7.0 * CELL))
        .into()
}

/// The days of `month` under their weekday names, as the form's date
/// picker draws them, with `chosen` marked. Pressing a day sends
/// `on_pick` with it; without `on_pick` the days are disabled. Days of the
/// neighbouring months are left blank.
pub fn month_grid<'a, Message: Clone + 'a>(
    month: Date,
    week_start: Weekday,
    chosen: Option<Date>,
    on_pick: Option<impl Fn(Date) -> Message + 'a>,
) -> Element<'a, Message> {
    let weeks = month_weeks(month, week_start);
    let on_pick = on_pick.map(std::rc::Rc::new);
    let mut grid = Column::new().spacing(2.0);
    if let Some(week) = weeks.first() {
        grid = grid.push(Row::with_children(week.iter().map(|date| {
            container(text(weekday_label(*date, true)).size(text_size::XS).style(
                |theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                },
            ))
            .center(Length::Fixed(CELL))
            .into()
        })));
    }
    for week in &weeks {
        grid = grid.push(Row::with_children(week.iter().map(|date| {
            if date.month() != month.month() {
                return Space::new()
                    .width(Length::Fixed(CELL))
                    .height(Length::Fixed(CELL))
                    .into();
            }
            let day = style::Day {
                today: false,
                selected: Some(*date) == chosen,
                in_month: true,
            };
            let number = text(date.day().to_string())
                .size(text_size::XS)
                .wrapping(text::Wrapping::None);
            widget::button(container(number).center(Length::Fill))
                .width(Length::Fixed(CELL))
                .height(Length::Fixed(CELL))
                .padding(0)
                .on_press_maybe(on_pick.as_ref().map(|pick| pick(*date)))
                .style(move |theme, status: Status| {
                    style::year_day(&Tokens::of(theme), day, status)
                })
                .into()
        })));
    }
    grid.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    const UTC: TimeZone = TimeZone::UTC;

    /// A time on 19 June 2026.
    fn at(hour: u16, minute: u16) -> Timestamp {
        instant_at(date(2026, 6, 19), hour * 60 + minute, &UTC)
    }

    fn timed() -> State {
        State::new(
            &Draft {
                when: When::timed(at(11, 0), at(12, 30)),
            },
            &UTC,
        )
    }

    fn whole_day() -> State {
        State::new(
            &Draft {
                when: When::day(date(2026, 6, 19)),
            },
            &UTC,
        )
    }

    #[test]
    fn a_new_form_starts_from_the_draft() {
        let form = timed();
        assert!(form.is_new());
        assert_eq!(form.id(), None);
        assert_eq!(form.title(), "");
        assert!(!form.is_all_day());
        assert_eq!(form.date(Field::Start), date(2026, 6, 19));
        assert_eq!((form.time(Field::Start), form.time(Field::End)), (660, 750));
        assert_eq!(form.repeats(), Repeats::Never);
        assert_eq!(form.tone(), Tone::Primary);
        assert_eq!(form.picker(), None);
        assert_eq!(form.when(), When::timed(at(11, 0), at(12, 30)));

        let form = whole_day();
        assert!(form.is_all_day());
        assert_eq!(form.when(), When::day(date(2026, 6, 19)));
    }

    #[test]
    fn editing_starts_from_the_entry_and_keeps_what_the_form_does_not_show() {
        let rule = Repeat::weekly().interval(2);
        let entry = Entry::new(EntryId(4), "Review", at(15, 0), at(16, 0))
            .tone(Tone::Success)
            .tag("work")
            .repeat(rule)
            .location("Room 4")
            .notes("Bring the plan\nand the figures");
        let mut form = State::edit(&entry, &UTC);
        assert_eq!(form.id(), Some(EntryId(4)));
        assert!(!form.is_new());
        assert_eq!(form.title(), "Review");
        assert_eq!(form.tone(), Tone::Success);
        assert_eq!(form.repeats(), Repeats::Weekly);
        assert_eq!(form.interval(), 2);
        assert_eq!(form.stops(), Stops::Never);
        assert_eq!(form.location(), "Room 4");
        assert_eq!(form.notes(), "Bring the plan\nand the figures");
        assert_eq!(form.entry(EntryId(99)), entry, "unchanged, with its own id");

        form.update(Event::Title(String::from("Retro")));
        let saved = form.entry(EntryId(99));
        assert_eq!(saved.id(), EntryId(4));
        assert_eq!(saved.title(), "Retro");
        assert_eq!(saved.tag_of(), Some("work"));
        assert_eq!(saved.repeat_of(), Some(rule));
        assert_eq!(saved.location_of(), Some("Room 4"));
    }

    #[test]
    fn a_blank_title_is_saved_as_untitled_and_a_new_entry_takes_the_id_given() {
        let mut form = timed();
        form.update(Event::Title(String::from("   ")));
        let entry = form.entry(EntryId(7));
        assert_eq!(entry.id(), EntryId(7));
        assert_eq!(entry.title(), UNTITLED);
        form.update(Event::Title(String::from("  Retro ")));
        assert_eq!(form.entry(EntryId(7)).title(), "Retro");
    }

    #[test]
    fn moving_the_start_keeps_the_length() {
        let mut form = timed();
        form.update(Event::Time(Field::Start, 14 * 60));
        assert_eq!((form.time(Field::Start), form.time(Field::End)), (840, 930));

        form.update(Event::Date(Field::Start, date(2026, 6, 22)));
        assert_eq!(form.date(Field::End), date(2026, 6, 22));
        assert_eq!(
            form.when(),
            When::timed(
                instant_at(date(2026, 6, 22), 840, &UTC),
                instant_at(date(2026, 6, 22), 930, &UTC)
            )
        );

        // Late enough, the end rolls into the next day.
        form.update(Event::Time(Field::Start, 23 * 60));
        assert_eq!(form.date(Field::End), date(2026, 6, 23));
        assert_eq!(form.time(Field::End), 30);
    }

    #[test]
    fn the_end_never_goes_before_the_start() {
        let mut form = timed();
        // An earlier time on the same day means the next day.
        form.update(Event::Time(Field::End, 9 * 60));
        assert_eq!(form.date(Field::End), date(2026, 6, 20));
        assert_eq!(form.time(Field::End), 540);

        // An end date before the start comes back to just after it.
        form.update(Event::Date(Field::End, date(2026, 6, 1)));
        assert_eq!(form.date(Field::End), date(2026, 6, 19));
        assert_eq!(form.time(Field::End), 660 + TIME_STEP);

        // The same time as the start is the same time the next day, not a
        // time nobody chose.
        let mut form = timed();
        form.update(Event::Time(Field::End, 660));
        assert_eq!(
            (form.date(Field::End), form.time(Field::End)),
            (date(2026, 6, 20), 660)
        );

        // At the last step of the day, the shortest entry ends at midnight.
        let late = at(23, 45);
        let mut form = State::new(
            &Draft {
                when: When::timed(late, late),
            },
            &UTC,
        );
        form.update(Event::Time(Field::Start, 23 * 60 + 45));
        assert_eq!(
            (form.date(Field::End), form.time(Field::End)),
            (date(2026, 6, 20), 0)
        );
    }

    #[test]
    fn an_end_put_on_the_next_day_comes_back_when_a_later_time_is_typed() {
        let mut form = State::new(
            &Draft {
                when: When::timed(at(9, 0), at(10, 0)),
            },
            &UTC,
        );
        form.update(Event::Time(Field::End, 8 * 60));
        assert_eq!(
            form.date(Field::End),
            date(2026, 6, 20),
            "08:00 is overnight"
        );
        assert_eq!(form.times(Field::End)[0], TimeOfDay(9 * 60 + TIME_STEP));
        form.update(Event::Time(Field::End, 11 * 60));
        assert_eq!(
            form.when(),
            When::timed(at(9, 0), at(11, 0)),
            "not 26 hours"
        );

        // A next day picked on purpose stays.
        form.update(Event::Date(Field::End, date(2026, 6, 20)));
        form.update(Event::Time(Field::End, 12 * 60));
        assert_eq!(form.date(Field::End), date(2026, 6, 20));

        // Moving the start keeps a rolled end rolled.
        let mut form = timed();
        form.update(Event::Time(Field::End, 9 * 60));
        form.update(Event::Time(Field::Start, 10 * 60));
        form.update(Event::Time(Field::End, 12 * 60));
        assert_eq!(
            (form.date(Field::End), form.time(Field::End)),
            (date(2026, 6, 19), 720)
        );
    }

    #[test]
    fn whole_days_end_on_the_day_a_midnight_end_closes() {
        let mut form = State::new(
            &Draft {
                when: When::timed(at(23, 0), at(23, 0) + jiff::SignedDuration::from_hours(1)),
            },
            &UTC,
        );
        form.update(Event::AllDay(true));
        assert_eq!(
            form.when(),
            When::day(date(2026, 6, 19)),
            "one day, not two"
        );
        form.update(Event::AllDay(false));
        assert_eq!(
            form.when(),
            When::timed(at(23, 0), at(23, 0) + jiff::SignedDuration::from_hours(1)),
            "and back"
        );
        form.update(Event::AllDay(false));
        assert_eq!(form.date(Field::End), date(2026, 6, 20), "no change twice");
    }

    #[test]
    fn editing_the_title_alone_keeps_the_notes_and_the_seconds() {
        let start = at(9, 0) + jiff::SignedDuration::from_secs(30);
        let end = at(10, 0) + jiff::SignedDuration::from_secs(15);
        let notes = "  Agenda first\nthen figures\n";
        let entry = Entry::new(EntryId(2), "Review", start, end).notes(notes);
        let mut form = State::edit(&entry, &UTC);
        form.update(Event::Title(String::from("Review two")));
        assert_eq!(form.notes(), notes);
        let saved = form.entry(EntryId(9));
        assert_eq!(saved.when(), When::timed(start, end));
        assert_eq!(saved.notes_of(), Some(notes));

        // A changed end loses its seconds; the untouched start keeps them.
        form.update(Event::Time(Field::End, 11 * 60));
        assert_eq!(form.when(), When::timed(start, at(11, 0)));

        // Edited notes are trimmed.
        form.update(Event::Notes(Action::Edit(
            iced::widget::text_editor::Edit::Insert('x'),
        )));
        assert!(!form.notes().starts_with(' '));
        // Moving the cursor is not an edit.
        let mut form = State::edit(&entry, &UTC);
        form.update(Event::Notes(Action::Move(
            iced::widget::text_editor::Motion::Down,
        )));
        assert_eq!(form.notes(), notes);
    }

    #[test]
    fn times_out_of_range_are_kept_within_the_day() {
        let mut form = timed();
        form.update(Event::Time(Field::Start, 5000));
        assert_eq!(form.time(Field::Start), DAY_MINUTES - 1);
    }

    #[test]
    fn all_day_keeps_the_dates_and_brings_the_times_back() {
        let mut form = timed();
        form.update(Event::AllDay(true));
        assert_eq!(form.when(), When::day(date(2026, 6, 19)));
        form.update(Event::Date(Field::End, date(2026, 6, 21)));
        assert_eq!(
            form.when(),
            When::all_day(date(2026, 6, 19), date(2026, 6, 21))
        );
        form.update(Event::Date(Field::End, date(2026, 6, 1)));
        assert_eq!(form.date(Field::End), date(2026, 6, 19), "same day is fine");
        form.update(Event::AllDay(false));
        assert_eq!((form.time(Field::Start), form.time(Field::End)), (660, 750));

        // A draft for a whole day has a morning hour ready.
        let mut form = whole_day();
        form.update(Event::AllDay(false));
        assert_eq!((form.time(Field::Start), form.time(Field::End)), (540, 600));
    }

    #[test]
    fn repeat_and_colour_reach_the_entry() {
        let mut form = timed();
        form.update(Event::Repeats(Repeats::Weekly));
        form.update(Event::Tone(Tone::Warning));
        let entry = form.entry(EntryId(1));
        assert_eq!(entry.repeat_of(), Some(Repeat::weekly()));
        assert_eq!(entry.tone_of(), Tone::Warning);
        form.update(Event::Repeats(Repeats::Never));
        assert_eq!(form.entry(EntryId(1)).repeat_of(), None);
    }

    #[test]
    fn every_repeat_choice_has_a_name_and_round_trips() {
        for choice in Repeats::ALL {
            assert!(!choice.to_string().is_empty());
            assert_eq!(Repeats::of(choice.frequency()), choice);
        }
        assert_eq!(Repeats::default(), Repeats::Never);
        assert_eq!(Repeats::Never.frequency(), None);
        let units: Vec<_> = Repeats::ALL
            .iter()
            .map(|choice| (choice.unit(1), choice.unit(2)))
            .collect();
        assert_eq!(
            units,
            vec![
                ("", ""),
                ("day", "days"),
                ("week", "weeks"),
                ("month", "months"),
                ("year", "years")
            ]
        );
        let names: Vec<_> = Stops::ALL.iter().map(Stops::to_string).collect();
        assert_eq!(names, vec!["Never", "On a date", "After"]);
        assert_eq!(Stops::default(), Stops::Never);
    }

    #[test]
    fn a_repeat_takes_its_interval_and_its_end_from_the_form() {
        let mut form = timed();
        assert_eq!(form.repeat(), None);
        assert_eq!((form.interval(), form.count()), (1, DEFAULT_COUNT));
        assert_eq!(form.until(), date(2026, 7, 19), "a month after the start");

        form.update(Event::Repeats(Repeats::Weekly));
        assert_eq!(form.repeat(), Some(Repeat::weekly()));
        form.update(Event::Interval(String::from("2")));
        assert_eq!(form.repeat(), Some(Repeat::weekly().interval(2)));

        form.update(Event::Stops(Stops::After));
        form.update(Event::Count(String::from("6")));
        assert_eq!(form.repeat(), Some(Repeat::weekly().interval(2).count(6)));

        form.update(Event::Stops(Stops::On));
        form.update(Event::Until(date(2026, 9, 1)));
        let rule = Repeat::weekly().interval(2).until(date(2026, 9, 1));
        assert_eq!(form.repeat(), Some(rule));
        assert_eq!(form.entry(EntryId(1)).repeat_of(), Some(rule));

        // The interval and the end wait while the entry does not repeat.
        form.update(Event::Repeats(Repeats::Never));
        assert_eq!(form.repeat(), None);
        form.update(Event::Repeats(Repeats::Monthly));
        assert_eq!(
            form.repeat(),
            Some(Repeat::monthly().interval(2).until(date(2026, 9, 1)))
        );
    }

    #[test]
    fn number_fields_keep_digits_and_count_blank_as_one() {
        let mut form = timed();
        form.update(Event::Repeats(Repeats::Daily));
        form.update(Event::Interval(String::from("a1b2c3d4")));
        assert_eq!(form.interval(), 123, "three digits at most");
        form.update(Event::Interval(String::new()));
        assert_eq!(form.interval(), 1, "blank while typing");
        form.update(Event::Interval(String::from("0")));
        assert_eq!(form.interval_text(), "", "a 0 is never shown");
        assert_eq!(form.interval(), 1);
        assert_eq!(form.repeat(), Some(Repeat::daily()));
        form.update(Event::Interval(String::from("05")));
        assert_eq!(form.interval_text(), "5");
        form.update(Event::Interval(String::from("10")));
        assert_eq!(form.interval(), 10);

        form.update(Event::Stops(Stops::After));
        form.update(Event::Count(String::from("-5 times")));
        assert_eq!(form.count(), 5);
        form.update(Event::Count(String::from("0")));
        assert_eq!(form.count(), 1);
    }

    #[test]
    fn an_entry_ending_on_a_date_or_after_a_count_fills_the_form() {
        let base = Entry::new(EntryId(4), "Review", at(15, 0), at(16, 0));
        let on = base
            .clone()
            .repeat(Repeat::monthly().until(date(2026, 12, 19)));
        let form = State::edit(&on, &UTC);
        assert_eq!(form.repeats(), Repeats::Monthly);
        assert_eq!(form.stops(), Stops::On);
        assert_eq!(form.until(), date(2026, 12, 19));
        assert_eq!(form.entry(EntryId(9)), on);

        let after = base.repeat(Repeat::yearly().interval(4).count(3));
        let form = State::edit(&after, &UTC);
        assert_eq!(form.repeats(), Repeats::Yearly);
        assert_eq!(form.stops(), Stops::After);
        assert_eq!((form.interval(), form.count()), (4, 3));
        assert_eq!(form.entry(EntryId(9)), after);
    }

    #[test]
    fn the_last_date_of_a_repeat_stays_on_or_after_the_start() {
        let mut form = timed();
        form.update(Event::Repeats(Repeats::Daily));
        form.update(Event::Stops(Stops::On));
        form.update(Event::Until(date(2026, 6, 1)));
        assert_eq!(form.until(), date(2026, 6, 19), "not before the start");

        form.update(Event::Until(date(2026, 6, 25)));
        form.update(Event::Date(Field::Start, date(2026, 8, 3)));
        assert_eq!(form.until(), date(2026, 8, 3), "carried by the start");
    }

    #[test]
    fn the_picker_for_the_last_date_opens_turns_and_closes() {
        let mut form = timed();
        form.update(Event::Repeats(Repeats::Daily));
        form.update(Event::Stops(Stops::On));
        form.update(Event::UntilPicker(true));
        assert!(form.is_picking_until());
        assert_eq!(form.picker(), None, "not the start's or the end's");
        assert_eq!(form.picker.map(|p| p.month), Some(date(2026, 7, 1)));
        form.update(Event::PickerMonth(2));
        assert_eq!(form.picker.map(|p| p.month), Some(date(2026, 9, 1)));
        form.update(Event::Until(date(2026, 9, 14)));
        assert!(!form.is_picking_until());
        assert_eq!(form.until(), date(2026, 9, 14));

        // One picker at a time, and it closes with the field it is for.
        form.update(Event::UntilPicker(true));
        form.update(Event::Picker(Some(Field::Start)));
        assert!(!form.is_picking_until());
        assert_eq!(form.picker(), Some(Field::Start));
        form.update(Event::UntilPicker(true));
        form.update(Event::UntilPicker(false));
        assert!(!form.is_picking_until());
        form.update(Event::UntilPicker(true));
        form.update(Event::Stops(Stops::Never));
        assert!(!form.is_picking_until());
        form.update(Event::Stops(Stops::On));
        form.update(Event::UntilPicker(true));
        form.update(Event::Repeats(Repeats::Never));
        assert!(!form.is_picking_until());
        form.update(Event::UntilPicker(true));
        form.update(Event::Picker(None));
        assert!(!form.is_picking_until());
    }

    #[test]
    fn location_and_notes_reach_the_entry() {
        let mut form = timed();
        assert_eq!(form.location(), "");
        assert_eq!(form.notes(), "");
        let blank = form.entry(EntryId(1));
        assert_eq!((blank.location_of(), blank.notes_of()), (None, None));

        form.update(Event::Location(String::from(" Room 4 ")));
        for letter in "Agenda".chars() {
            form.update(Event::Notes(Action::Edit(
                iced::widget::text_editor::Edit::Insert(letter),
            )));
        }
        form.update(Event::Notes(Action::Edit(
            iced::widget::text_editor::Edit::Enter,
        )));
        assert_eq!(form.notes(), "Agenda", "space at the ends is dropped");
        let entry = form.entry(EntryId(1));
        assert_eq!(entry.location_of(), Some("Room 4"));
        assert_eq!(entry.notes_of(), Some("Agenda"));
    }

    #[test]
    fn typed_times_are_read_with_or_without_a_colon() {
        assert_eq!(TimeOfDay::parse("09:40"), Some(TimeOfDay(580)));
        assert_eq!(TimeOfDay::parse(" 9:40 "), Some(TimeOfDay(580)));
        assert_eq!(TimeOfDay::parse("940"), Some(TimeOfDay(580)));
        assert_eq!(TimeOfDay::parse("2359"), Some(TimeOfDay(1439)));
        assert_eq!(TimeOfDay::parse("0:00"), Some(TimeOfDay(0)));
        assert_eq!(TimeOfDay::parse("9"), Some(TimeOfDay(540)), "a bare hour");
        assert_eq!(TimeOfDay::parse("17"), Some(TimeOfDay(1020)));
        assert_eq!(TimeOfDay::parse("09.05"), Some(TimeOfDay(545)));
        assert_eq!(TimeOfDay::parse("9.05"), Some(TimeOfDay(545)));
        for bad in [
            "", "24", "24:00", "12:60", "9:4", "123:00", "ab:cd", "€12", "12:3x", ":30", "9:",
            "9.", "9:30.5", ".",
        ] {
            assert_eq!(TimeOfDay::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn times_and_colours_have_names() {
        assert_eq!(TimeOfDay(0).to_string(), "00:00");
        assert_eq!(TimeOfDay(570).to_string(), "09:30");
        let all = TimeOfDay::all();
        assert_eq!(all.len(), 96);
        assert_eq!(all.last(), Some(&TimeOfDay(1425)));

        // A menu can start anywhere and still lists the whole day.
        let from_ten = TimeOfDay::from(607);
        assert_eq!(from_ten.len(), 96);
        assert_eq!(from_ten.first(), Some(&TimeOfDay(600)));
        assert_eq!(from_ten.last(), Some(&TimeOfDay(585)));
        assert_eq!(TimeOfDay::from(0), all);
        assert_eq!(TimeOfDay::from(9999).first(), Some(&TimeOfDay(1425)));

        // The start's menu lists the day in order; the end's begins after
        // the start on the same day, and lists the day on another.
        let form = timed();
        assert_eq!(form.times(Field::Start), all);
        assert_eq!(form.times(Field::End).first(), Some(&TimeOfDay(675)));
        assert_eq!(form.times(Field::End).len(), 96);
        let mut overnight = timed();
        overnight.update(Event::Date(Field::End, date(2026, 6, 20)));
        assert_eq!(overnight.times(Field::End), all);
        let names: Vec<_> = Tone::ALL
            .iter()
            .map(|tone| Colour(*tone).to_string())
            .collect();
        assert_eq!(names, vec!["Default", "Neutral", "Green", "Amber", "Red"]);
    }

    #[test]
    fn the_date_picker_opens_on_the_field_month_and_closes_on_a_choice() {
        let mut form = timed().with_week_start(Weekday::Sunday);
        form.update(Event::Picker(Some(Field::End)));
        assert_eq!(form.picker(), Some(Field::End));
        assert_eq!(form.picker.map(|p| p.month), Some(date(2026, 6, 1)));

        form.update(Event::PickerMonth(1));
        form.update(Event::PickerMonth(1));
        assert_eq!(form.picker.map(|p| p.month), Some(date(2026, 8, 1)));
        form.update(Event::PickerMonth(-3));
        assert_eq!(form.picker.map(|p| p.month), Some(date(2026, 5, 1)));

        form.update(Event::Date(Field::End, date(2026, 6, 20)));
        assert_eq!(form.picker(), None);
        assert_eq!(form.date(Field::End), date(2026, 6, 20));

        form.update(Event::Picker(Some(Field::Start)));
        form.update(Event::Picker(None));
        assert_eq!(form.picker(), None);
        form.update(Event::PickerMonth(1));
        assert_eq!(form.picker(), None, "nothing to move");
    }

    #[test]
    fn builder_defaults_and_options() {
        let form = timed();
        let built: Editor<'_, Event> = editor(&form);
        assert!(built.on_event.is_none());
        assert!(built.on_submit.is_none());
        assert_eq!(built.shortcuts, Shortcuts::all());

        let built = built
            .on_event(|event| event)
            .on_submit(Event::Picker(None))
            .shortcuts(Shortcuts::none());
        assert!(built.on_event.is_some());
        assert!(built.on_submit.is_some());
        assert_eq!(built.shortcuts, Shortcuts::none());
        assert!(format!("{built:?}").contains("enabled: true"));
    }

    #[test]
    fn parts_can_be_left_out_and_laid_out_by_the_app() {
        let mut form = timed();
        {
            let built: Editor<'_, Event> = editor(&form);
            assert!(Part::ALL.iter().all(|part| built.shows(*part)));
            let fields = built.fields();
            assert!(fields.title.is_some() && fields.notes.is_some() && fields.colour.is_some());
            assert!(
                fields.every.is_none() && fields.stops.is_none(),
                "it does not repeat"
            );
        }
        {
            let built: Editor<'_, Event> = editor(&form)
                .without(Part::Colour)
                .without(Part::Location)
                .without(Part::Colour);
            assert!(!built.shows(Part::Colour));
            assert!(format!("{built:?}").contains("Location"));
            let fields = built.fields();
            assert!(fields.colour.is_none() && fields.location.is_none());
            assert!(fields.title.is_some() && fields.starts.is_some() && fields.ends.is_some());
        }

        form.update(Event::Repeats(Repeats::Weekly));
        let fields = editor::<Event>(&form).fields();
        assert!(fields.every.is_some() && fields.stops.is_some());
        let fields = editor::<Event>(&form).without(Part::Repeat).fields();
        assert!(fields.repeat.is_none() && fields.every.is_none() && fields.stops.is_none());
    }

    #[test]
    fn the_raw_field_text_the_picker_month_and_the_notes_can_be_read() {
        let mut form = timed().with_week_start(Weekday::Sunday);
        assert_eq!(form.week_start(), Weekday::Sunday);
        form.update(Event::Interval(String::new()));
        assert_eq!(form.interval_text(), "");
        assert_eq!(form.interval(), 1, "blank counts as one");
        form.update(Event::Count(String::from("2a")));
        assert_eq!(form.count_text(), "2");

        assert_eq!(form.picker_month(), None);
        form.update(Event::Picker(Some(Field::Start)));
        let month = form.date(Field::Start).first_of_month();
        assert_eq!(form.picker_month(), Some(month));
        form.update(Event::PickerMonth(2));
        assert_eq!(form.picker_month(), month.checked_add(2.months()).ok());

        form.update(Event::Notes(Action::Edit(
            iced::widget::text_editor::Edit::Paste(std::sync::Arc::new(String::from("Bring cake"))),
        )));
        assert_eq!(form.notes_content().text().trim(), "Bring cake");
    }
}
