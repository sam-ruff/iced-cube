//! What the calendar shows and what is selected in it.

use iced::keyboard::key::Named;
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp, ToSpan};

use super::entry::{Changed, Draft, Entries, Entry, EntryId, When, add_days};
use super::layout::{self, DAY_MINUTES, instant_at};
use crate::keys::{self, Chord, Keymap};

/// How long an entry asked for by pressing a time slot is.
pub const SLOT_MINUTES: u16 = 60;

/// How many days [`View::FourDays`] shows.
pub const FOUR_DAYS: i32 = 4;

/// How the entries are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum View {
    /// One day against the hours.
    Day,
    /// Seven days side by side against the hours.
    Week,
    /// A grid of weeks with entries across the days.
    #[default]
    Month,
    /// The twelve months of a year, small.
    Year,
    /// The month's entries as a list, day by day.
    Agenda,
    /// Four days against the hours, starting on the day on show.
    FourDays,
}

impl View {
    /// Every view, in the order the header's menu lists them.
    pub const ALL: [View; 6] = [
        View::Day,
        View::Week,
        View::Month,
        View::Year,
        View::Agenda,
        View::FourDays,
    ];

    pub fn label(self) -> &'static str {
        match self {
            View::Day => "Day",
            View::Week => "Week",
            View::Month => "Month",
            View::Year => "Year",
            View::Agenda => "Schedule",
            View::FourDays => "4 days",
        }
    }
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// What the user may change through the calendar's own gestures and
/// shortcuts. Everything is allowed by default.
///
/// This only limits what the calendar offers. [`Event::Insert`] and
/// [`Event::Remove`] always apply, so the app, or controls of its own, can
/// still add, change and remove entries on a calendar that is read-only
/// to the pointer and the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Allow {
    /// Asking for a new entry: a second press on a day or a time, a drag
    /// across empty space, or [`Action::Activate`] on a day.
    pub create: bool,
    /// Dragging an entry to move it or to change when it ends.
    pub reschedule: bool,
    /// Removing the selected entry with [`Action::Remove`].
    pub remove: bool,
}

impl Allow {
    /// Everything: the default.
    pub const ALL: Allow = Allow {
        create: true,
        reschedule: true,
        remove: true,
    };

    /// Nothing: a read-only calendar. Days and entries can still be
    /// selected, entries opened and the period and view changed.
    pub const NONE: Allow = Allow {
        create: false,
        reschedule: false,
        remove: false,
    };
}

impl Default for Allow {
    fn default() -> Self {
        Self::ALL
    }
}

/// What is selected: at most one thing at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Selection {
    Day(Date),
    /// A time on a day in the week and day views, in minutes from
    /// midnight.
    Slot {
        date: Date,
        minute: u16,
    },
    /// One occurrence of an entry, told apart by when it starts.
    Entry {
        entry: EntryId,
        start: Timestamp,
    },
}

/// Everything that changes the calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Show(View),
    /// Moves on by a month, a week or a day, whichever the view shows.
    Next,
    Previous,
    /// Goes to today and selects it.
    Today,
    /// Shows the period holding this date, without selecting anything.
    GoTo(Date),
    /// A press on a day, a time slot or an entry. The first press selects
    /// it; a press on what is already selected activates it, as
    /// [`Event::Activate`] does.
    Press(Selection),
    /// Activates the selection: an entry is opened, and a day or a time
    /// slot asks for a new entry there. With nothing selected it asks for
    /// one on the day on show.
    Activate,
    /// Clears the selection.
    Clear,
    /// Moves the selected day by this many days and follows it when it
    /// leaves the period on show.
    MoveDay(i32),
    /// Selects the next entry on the selected day, wrapping.
    NextEntry,
    PreviousEntry,
    /// Shows one day, as the "more" count of a crowded day does.
    OpenDay(Date),
    /// A span dragged out on empty space.
    Request(Draft),
    /// An occurrence dragged to a new time. `start` says which occurrence;
    /// a repeating entry moves as a whole by the same amount.
    Reschedule {
        entry: EntryId,
        start: Timestamp,
        to: (Timestamp, Timestamp),
    },
    /// Adds an entry, or replaces the one with the same id.
    Insert(Entry),
    Remove(EntryId),
    /// The current time, for "today" and the now line.
    Tick(Timestamp),
}

/// What the app may want to act on after an [`Event`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Selected(Option<Selection>),
    /// An entry was activated: show or edit it.
    Opened {
        entry: EntryId,
        start: Timestamp,
    },
    /// The user asked for a new entry here. Nothing is added until the app
    /// sends [`Event::Insert`].
    Requested(Draft),
    /// An entry was created, updated or removed. Pass it to
    /// [`Scheduler::observe`](super::schedule::Scheduler::observe) for
    /// triggers on changes.
    Changed(Box<Changed>),
    /// A different period or view is on show, such as after
    /// [`Event::Next`]. Useful for loading the entries of that period.
    Viewed {
        view: View,
        first: Date,
        last: Date,
    },
}

/// The entries, the view, the period on show and the selection.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    entries: Entries,
    view: View,
    /// A date in the period on show.
    cursor: Date,
    today: Date,
    now: Option<Timestamp>,
    selection: Option<Selection>,
    week_start: Weekday,
    allow: Allow,
}

impl State {
    /// An empty calendar in UTC showing the month of `today`.
    ///
    /// The calendar never reads the clock. Pass today's date here, and
    /// keep it current with [`Event::Tick`] if the app stays open.
    pub fn new(today: Date) -> Self {
        Self {
            entries: Entries::utc(),
            view: View::default(),
            cursor: today,
            today,
            now: None,
            selection: None,
            week_start: Weekday::Monday,
            allow: Allow::ALL,
        }
    }

    /// A calendar showing the month of `now` in the time zone of
    /// `entries`, with the now line drawn.
    pub fn at(now: Timestamp, entries: Entries) -> Self {
        let mut state = Self::new(entries.date_of(now)).with_entries(entries);
        state.now = Some(now);
        state
    }

    /// Replaces the entries and their time zone.
    pub fn with_entries(mut self, entries: Entries) -> Self {
        self.entries = entries;
        self
    }

    pub fn with_view(mut self, view: View) -> Self {
        self.set_view(view);
        self
    }

    /// The day weeks start on. Defaults to Monday.
    pub fn with_week_start(mut self, week_start: Weekday) -> Self {
        self.week_start = week_start;
        self
    }

    /// Shows the period holding `date`.
    pub fn with_cursor(mut self, date: Date) -> Self {
        self.cursor = date;
        self
    }

    pub fn with_selection(mut self, selection: Selection) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Limits what the user may change through the calendar itself.
    pub fn with_allow(mut self, allow: Allow) -> Self {
        self.allow = allow;
        self
    }

    /// A calendar the user can look through but not change: nothing can
    /// be created, dragged or removed from it. The same as
    /// [`with_allow`](State::with_allow) with [`Allow::NONE`].
    pub fn read_only(self) -> Self {
        self.with_allow(Allow::NONE)
    }

    /// Changes what the user may do, such as when they sign in or a
    /// calendar they cannot edit comes on show.
    pub fn set_allow(&mut self, allow: Allow) {
        self.allow = allow;
    }

    pub fn allow(&self) -> Allow {
        self.allow
    }

    pub fn entries(&self) -> &Entries {
        &self.entries
    }

    pub fn time_zone(&self) -> &TimeZone {
        self.entries.time_zone()
    }

    pub fn view(&self) -> View {
        self.view
    }

    /// A date in the period on show.
    pub fn cursor(&self) -> Date {
        self.cursor
    }

    pub fn today(&self) -> Date {
        self.today
    }

    /// The time of the last [`Event::Tick`], which is where the now line
    /// is drawn.
    pub fn now(&self) -> Option<Timestamp> {
        self.now
    }

    pub fn week_start(&self) -> Weekday {
        self.week_start
    }

    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    /// The selected entry, if an entry is selected.
    pub fn selected_entry(&self) -> Option<&Entry> {
        match self.selection? {
            Selection::Entry { entry, .. } => self.entries.get(entry),
            _ => None,
        }
    }

    /// The day the selection is on, if anything is selected.
    pub fn selected_day(&self) -> Option<Date> {
        Some(match self.selection? {
            Selection::Day(date) | Selection::Slot { date, .. } => date,
            Selection::Entry { start, .. } => self.entries.date_of(start),
        })
    }

    pub fn is_selected(&self, selection: Selection) -> bool {
        self.selection == Some(selection)
    }

    /// The first and last date on show. The month view includes the days
    /// of neighbouring months that fill its first and last weeks.
    pub fn range(&self) -> (Date, Date) {
        match self.view {
            View::Month => {
                let weeks = layout::month_weeks(self.cursor, self.week_start);
                let first = weeks.first().map_or(self.cursor, |week| week[0]);
                let last = weeks.last().map_or(self.cursor, |week| week[6]);
                (first, last)
            }
            View::Week => {
                let week = layout::week_of(self.cursor, self.week_start);
                (week[0], week[6])
            }
            View::Day => (self.cursor, self.cursor),
            View::FourDays => (self.cursor, add_days(self.cursor, FOUR_DAYS - 1)),
            View::Year => (self.cursor.first_of_year(), self.cursor.last_of_year()),
            View::Agenda => (self.cursor.first_of_month(), self.cursor.last_of_month()),
        }
    }

    /// The period on show in words, such as "November 2026".
    pub fn title(&self) -> String {
        let format = |date: Date, format: &str| date.strftime(format).to_string();
        match self.view {
            View::Month | View::Agenda => format(self.cursor, "%B %Y"),
            View::Year => format(self.cursor, "%Y"),
            View::Day => format(self.cursor, "%A %-d %B %Y"),
            View::Week | View::FourDays => {
                let (first, last) = self.range();
                let start = if first.year() != last.year() {
                    "%-d %b %Y"
                } else if first.month() != last.month() {
                    "%-d %b"
                } else {
                    "%-d"
                };
                format!("{} - {}", format(first, start), format(last, "%-d %b %Y"))
            }
        }
    }

    /// A shorter [`title`](State::title) for narrow screens, such as
    /// "Thu 19 Nov 2026" for a day and "30 Nov - 6 Dec" for a week that
    /// crosses months.
    pub fn short_title(&self) -> String {
        let format = |date: Date, format: &str| date.strftime(format).to_string();
        match self.view {
            View::Month | View::Agenda | View::Year => self.title(),
            View::Day => format(self.cursor, "%a %-d %b %Y"),
            View::Week | View::FourDays => {
                let (first, last) = self.range();
                if (first.year(), first.month()) == (last.year(), last.month()) {
                    return self.title();
                }
                format!("{} - {}", format(first, "%-d %b"), format(last, "%-d %b"))
            }
        }
    }

    /// Applies an event and returns what the app may want to act on.
    pub fn update(&mut self, event: Event) -> Option<Output> {
        match event {
            Event::Show(view) => {
                if view == self.view {
                    return None;
                }
                if let Some(day) = self.selected_day() {
                    self.cursor = day;
                }
                self.set_view(view);
                Some(self.viewed())
            }
            Event::Next => self.step(1),
            Event::Previous => self.step(-1),
            Event::Today => {
                let before = (self.range(), self.selection);
                self.cursor = self.today;
                self.selection = Some(Selection::Day(self.today));
                if before.0 != self.range() {
                    return Some(self.viewed());
                }
                (before.1 != self.selection).then_some(Output::Selected(self.selection))
            }
            Event::GoTo(date) => self.show(date),
            Event::Press(selection) => {
                if self.selection == Some(selection) {
                    return self.activate();
                }
                self.selection = Some(selection);
                Some(Output::Selected(self.selection))
            }
            Event::Activate => self.activate(),
            Event::Clear => {
                self.selection.take()?;
                Some(Output::Selected(None))
            }
            Event::MoveDay(days) => {
                let from = self.selected_day().unwrap_or(self.cursor);
                let date = add_days(from, days);
                self.selection = Some(Selection::Day(date));
                let (first, last) = self.range();
                // The month view also shows days of its neighbours, and
                // moving onto one of those turns the page.
                let leaves_the_month = self.view == View::Month
                    && (date.year(), date.month()) != (self.cursor.year(), self.cursor.month());
                if date < first || date > last || leaves_the_month {
                    self.cursor = date;
                    return Some(self.viewed());
                }
                Some(Output::Selected(self.selection))
            }
            Event::NextEntry => self.cycle(1),
            Event::PreviousEntry => self.cycle(-1),
            Event::OpenDay(date) => {
                if self.view == View::Day && self.cursor == date {
                    return None;
                }
                self.cursor = date;
                self.set_view(View::Day);
                Some(self.viewed())
            }
            Event::Request(draft) => self.allow.create.then_some(Output::Requested(draft)),
            Event::Reschedule { entry, start, to } => {
                if !self.allow.reschedule {
                    return None;
                }
                let moved = self.rescheduled(entry, start, to)?;
                let changed = self.entries.insert(moved)?;
                if let Some(Selection::Entry {
                    entry: selected, ..
                }) = self.selection
                    && selected == entry
                {
                    self.selection = Some(Selection::Entry {
                        entry,
                        start: to.0.min(to.1),
                    });
                }
                Some(report(changed))
            }
            Event::Insert(entry) => self.entries.insert(entry).map(report),
            Event::Remove(id) => {
                let changed = self.entries.remove(id)?;
                if matches!(self.selection, Some(Selection::Entry { entry, .. }) if entry == id) {
                    self.selection = None;
                }
                Some(report(changed))
            }
            Event::Tick(now) => {
                self.now = Some(now);
                self.today = self.entries.date_of(now);
                None
            }
        }
    }

    fn set_view(&mut self, view: View) {
        self.view = view;
    }

    fn viewed(&self) -> Output {
        let (first, last) = self.range();
        Output::Viewed {
            view: self.view,
            first,
            last,
        }
    }

    fn show(&mut self, date: Date) -> Option<Output> {
        let before = self.range();
        self.cursor = date;
        (before != self.range()).then(|| self.viewed())
    }

    fn step(&mut self, direction: i32) -> Option<Output> {
        let months = |count: i32| {
            self.cursor
                .checked_add(i64::from(count).months())
                .unwrap_or(self.cursor)
        };
        let target = match self.view {
            View::Month | View::Agenda => months(direction),
            View::Year => months(12 * direction),
            View::Week => add_days(self.cursor, 7 * direction),
            View::FourDays => add_days(self.cursor, FOUR_DAYS * direction),
            View::Day => add_days(self.cursor, direction),
        };
        self.show(target)
    }

    fn activate(&mut self) -> Option<Output> {
        let time_zone = self.entries.time_zone();
        if let Some(Selection::Entry { entry, start }) = self.selection {
            self.entries.get(entry)?;
            return Some(Output::Opened { entry, start });
        }
        if !self.allow.create {
            return None;
        }
        Some(match self.selection {
            Some(Selection::Entry { entry, start }) => Output::Opened { entry, start },
            Some(Selection::Slot { date, minute }) => {
                let end = minute.saturating_add(SLOT_MINUTES).min(DAY_MINUTES);
                Output::Requested(Draft {
                    when: When::timed(
                        instant_at(date, minute, time_zone),
                        instant_at(date, end, time_zone),
                    ),
                })
            }
            Some(Selection::Day(date)) => Output::Requested(Draft {
                when: When::day(date),
            }),
            None => Output::Requested(Draft {
                when: When::day(self.cursor),
            }),
        })
    }

    /// Moves the selection through the entries of the selected day, or in
    /// the schedule through every entry it lists.
    fn cycle(&mut self, direction: i32) -> Option<Output> {
        let occurrences = if self.view == View::Agenda {
            let (first, last) = self.range();
            self.entries.between(first, last)
        } else {
            let day = self.selected_day().unwrap_or(self.cursor);
            self.entries.on(day)
        };
        let count = i32::try_from(occurrences.len()).ok().filter(|n| *n > 0)?;
        let current = occurrences.iter().position(|occurrence| {
            self.is_selected(Selection::Entry {
                entry: occurrence.entry,
                start: occurrence.start,
            })
        });
        let next = match current {
            Some(index) => (i32::try_from(index).ok()? + direction).rem_euclid(count),
            None if direction < 0 => count - 1,
            None => 0,
        };
        let occurrence = occurrences.get(usize::try_from(next).ok()?)?;
        let selection = Selection::Entry {
            entry: occurrence.entry,
            start: occurrence.start,
        };
        if self.selection == Some(selection) {
            return None;
        }
        self.selection = Some(selection);
        Some(Output::Selected(self.selection))
    }

    /// `entry` with the occurrence starting at `start` moved to `to`.
    fn rescheduled(
        &self,
        entry: EntryId,
        start: Timestamp,
        to: (Timestamp, Timestamp),
    ) -> Option<Entry> {
        let entry = self.entries.get(entry)?;
        let When::Timed {
            start: first_start, ..
        } = entry.when()
        else {
            return None;
        };
        let (new_start, new_end) = (to.0.min(to.1), to.0.max(to.1));
        let shift: SignedDuration = new_start.duration_since(start);
        let first = first_start.checked_add(shift).ok()?;
        let end = first.checked_add(new_end.duration_since(new_start)).ok()?;
        Some(entry.clone().rescheduled(When::timed(first, end)))
    }
}

fn report(changed: Changed) -> Output {
    Output::Changed(Box::new(changed))
}

/// What a calendar keyboard shortcut does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Next,
    Previous,
    Today,
    PreviousDay,
    NextDay,
    PreviousWeek,
    NextWeek,
    PreviousEntry,
    NextEntry,
    Month,
    Week,
    Day,
    Agenda,
    Year,
    FourDays,
    Activate,
    Remove,
    Close,
}

impl Action {
    /// The [`Event`] this action sends to `state`, or `None` when it would
    /// do nothing: removing with no entry selected, closing with nothing
    /// selected, or switching to the view already on show.
    pub fn event(self, state: &State) -> Option<Event> {
        let show = |view: View| (state.view() != view).then_some(Event::Show(view));
        match self {
            Action::Next => Some(Event::Next),
            Action::Previous => Some(Event::Previous),
            Action::Today => Some(Event::Today),
            // The schedule lists entries, not days, so there the arrows
            // step through the entries.
            Action::PreviousDay | Action::PreviousWeek if state.view() == View::Agenda => {
                Some(Event::PreviousEntry)
            }
            Action::NextDay | Action::NextWeek if state.view() == View::Agenda => {
                Some(Event::NextEntry)
            }
            Action::PreviousDay => Some(Event::MoveDay(-1)),
            Action::NextDay => Some(Event::MoveDay(1)),
            Action::PreviousWeek => Some(Event::MoveDay(-7)),
            Action::NextWeek => Some(Event::MoveDay(7)),
            Action::PreviousEntry => Some(Event::PreviousEntry),
            Action::NextEntry => Some(Event::NextEntry),
            Action::Month => show(View::Month),
            Action::Week => show(View::Week),
            Action::Day => show(View::Day),
            Action::Agenda => show(View::Agenda),
            Action::Year => show(View::Year),
            Action::FourDays => show(View::FourDays),
            Action::Activate => (state.selected_entry().is_some() || state.allow().create)
                .then_some(Event::Activate),
            Action::Remove => state
                .selected_entry()
                .filter(|_| state.allow().remove)
                .map(|entry| Event::Remove(entry.id())),
            Action::Close => state.selection().map(|_| Event::Clear),
        }
    }
}

impl keys::Action for Action {
    const ALL: &'static [Self] = &[
        Action::Previous,
        Action::Next,
        Action::Today,
        Action::PreviousDay,
        Action::NextDay,
        Action::PreviousWeek,
        Action::NextWeek,
        Action::PreviousEntry,
        Action::NextEntry,
        Action::Month,
        Action::Week,
        Action::Day,
        Action::Agenda,
        Action::Year,
        Action::FourDays,
        Action::Activate,
        Action::Remove,
        Action::Close,
    ];

    fn defaults() -> Keymap<Self> {
        default_keymap()
    }

    fn name(self) -> &'static str {
        match self {
            Action::Next => "Next",
            Action::Previous => "Previous",
            Action::Today => "Today",
            Action::PreviousDay => "Previous day",
            Action::NextDay => "Next day",
            Action::PreviousWeek => "Previous week",
            Action::NextWeek => "Next week",
            Action::PreviousEntry => "Previous entry",
            Action::NextEntry => "Next entry",
            Action::Month => "Month",
            Action::Week => "Week",
            Action::Day => "Day",
            Action::Agenda => "Schedule",
            Action::Year => "Year",
            Action::FourDays => "4 days",
            Action::Activate => "Activate",
            Action::Remove => "Remove",
            Action::Close => "Close",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Action::Next => "Shows the next period: a day, a week, four days, a month or a year.",
            Action::Previous => "Shows the previous period.",
            Action::Today => "Goes to today and selects it.",
            Action::PreviousDay => "Selects the day before.",
            Action::NextDay => "Selects the day after.",
            Action::PreviousWeek => "Selects the same day a week earlier.",
            Action::NextWeek => "Selects the same day a week later.",
            Action::PreviousEntry => "Selects the previous entry on the selected day.",
            Action::NextEntry => "Selects the next entry on the selected day.",
            Action::Month => "Switches to the month view.",
            Action::Week => "Switches to the week view.",
            Action::Day => "Switches to the day view.",
            Action::Agenda => "Switches to the schedule, a list of the month's entries.",
            Action::Year => "Switches to the year view.",
            Action::FourDays => "Switches to the four day view.",
            Action::Activate => {
                "Opens the selected entry, or asks for a new one on the selected day."
            }
            Action::Remove => "Removes the selected entry.",
            Action::Close => "Clears the selection.",
        }
    }
}

/// The default calendar shortcuts:
///
/// | Keys | Action |
/// | --- | --- |
/// | `PageUp` | [`Action::Previous`] |
/// | `PageDown` | [`Action::Next`] |
/// | `T` | [`Action::Today`] |
/// | `ArrowLeft` | [`Action::PreviousDay`] |
/// | `ArrowRight` | [`Action::NextDay`] |
/// | `ArrowUp` | [`Action::PreviousWeek`] |
/// | `ArrowDown` | [`Action::NextWeek`] |
/// | `Alt+ArrowUp` | [`Action::PreviousEntry`] |
/// | `Alt+ArrowDown` | [`Action::NextEntry`] |
/// | `M` | [`Action::Month`] |
/// | `W` | [`Action::Week`] |
/// | `D` | [`Action::Day`] |
/// | `A` | [`Action::Agenda`] |
/// | `Y` | [`Action::Year`] |
/// | `X` | [`Action::FourDays`] |
/// | `Enter` | [`Action::Activate`] |
/// | `Delete`, `Backspace` | [`Action::Remove`] |
/// | `Escape` | [`Action::Close`] |
///
/// The arrow and page keys are shared with tabs, sliders and pagination,
/// so on a screen with several of those resolve the calendar's keymap only
/// while the calendar is what the user is working in.
pub fn default_keymap() -> Keymap<Action> {
    Keymap::new()
        .bind(Chord::named(Named::PageUp), Action::Previous)
        .bind(Chord::named(Named::PageDown), Action::Next)
        .bind(Chord::character('t'), Action::Today)
        .bind(Chord::named(Named::ArrowLeft), Action::PreviousDay)
        .bind(Chord::named(Named::ArrowRight), Action::NextDay)
        .bind(Chord::named(Named::ArrowUp), Action::PreviousWeek)
        .bind(Chord::named(Named::ArrowDown), Action::NextWeek)
        .bind(Chord::named(Named::ArrowUp).alt(), Action::PreviousEntry)
        .bind(Chord::named(Named::ArrowDown).alt(), Action::NextEntry)
        .bind(Chord::character('m'), Action::Month)
        .bind(Chord::character('w'), Action::Week)
        .bind(Chord::character('d'), Action::Day)
        .bind(Chord::character('a'), Action::Agenda)
        .bind(Chord::character('y'), Action::Year)
        .bind(Chord::character('x'), Action::FourDays)
        .bind(Chord::named(Named::Enter), Action::Activate)
        .bind(Chord::named(Named::Delete), Action::Remove)
        .bind(Chord::named(Named::Backspace), Action::Remove)
        .bind(Chord::named(Named::Escape), Action::Close)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::calendar::entry::Change;
    use crate::data::calendar::repeat::Repeat;
    use jiff::civil::date;

    /// Thursday 19 November 2026.
    fn today() -> Date {
        date(2026, 11, 19)
    }

    fn at(day: i8, hour: i8) -> Timestamp {
        instant_at(
            date(2026, 11, day),
            u16::try_from(hour).unwrap_or(0) * 60,
            &TimeZone::UTC,
        )
    }

    fn meeting(id: u64, day: i8, hour: i8) -> Entry {
        Entry::new(EntryId(id), "Meeting", at(day, hour), at(day, hour + 1))
    }

    fn state() -> State {
        State::new(today()).with_entries(Entries::utc().with([
            meeting(1, 19, 9),
            meeting(2, 19, 14),
            meeting(3, 20, 9),
        ]))
    }

    fn entry(id: u64, day: i8, hour: i8) -> Selection {
        Selection::Entry {
            entry: EntryId(id),
            start: at(day, hour),
        }
    }

    #[test]
    fn a_new_calendar_shows_the_month_of_today_with_nothing_selected() {
        let state = State::new(today());
        assert_eq!(state.view(), View::Month);
        assert_eq!(state.cursor(), today());
        assert_eq!(state.today(), today());
        assert_eq!(state.now(), None);
        assert_eq!(state.selection(), None);
        assert_eq!(state.selected_day(), None);
        assert_eq!(state.week_start(), Weekday::Monday);
        assert!(state.entries().is_empty());
        assert_eq!(state.time_zone(), &TimeZone::UTC);
    }

    #[test]
    fn at_takes_today_from_the_time_and_the_time_zone() {
        let Ok(tokyo) = TimeZone::get("Asia/Tokyo") else {
            panic!("the time zone database is bundled for tests");
        };
        let state = State::at(at(19, 20), Entries::new(tokyo));
        assert_eq!(state.today(), date(2026, 11, 20));
        assert_eq!(state.now(), Some(at(19, 20)));
    }

    #[test]
    fn builder_options() {
        let state = State::new(today())
            .with_view(View::Week)
            .with_week_start(Weekday::Sunday)
            .with_cursor(date(2026, 12, 1))
            .with_selection(Selection::Day(date(2026, 12, 2)));
        assert_eq!(state.view(), View::Week);
        assert_eq!(state.week_start(), Weekday::Sunday);
        assert_eq!(state.cursor(), date(2026, 12, 1));
        assert_eq!(state.selected_day(), Some(date(2026, 12, 2)));
    }

    #[test]
    fn every_view_has_a_label_a_range_and_a_title() {
        let mut state = state();
        let expected = [
            (
                View::Month,
                (date(2026, 10, 26), date(2026, 12, 6)),
                "November 2026",
            ),
            (
                View::Week,
                (date(2026, 11, 16), date(2026, 11, 22)),
                "16 - 22 Nov 2026",
            ),
            (View::Day, (today(), today()), "Thursday 19 November 2026"),
            (
                View::Agenda,
                (date(2026, 11, 1), date(2026, 11, 30)),
                "November 2026",
            ),
            (View::Year, (date(2026, 1, 1), date(2026, 12, 31)), "2026"),
            (
                View::FourDays,
                (today(), date(2026, 11, 22)),
                "19 - 22 Nov 2026",
            ),
        ];
        assert_eq!(View::ALL.len(), expected.len());
        for (view, range, title) in expected {
            let _ = state.update(Event::Show(view));
            assert_eq!(state.view(), view);
            assert_eq!(state.range(), range, "{view:?}");
            assert_eq!(state.title(), title, "{view:?}");
            assert_eq!(view.to_string(), view.label());
        }
        let labels: Vec<_> = View::ALL.iter().map(|view| view.label()).collect();
        assert_eq!(
            labels,
            vec!["Day", "Week", "Month", "Year", "Schedule", "4 days"]
        );
    }

    #[test]
    fn a_year_steps_by_years_and_four_days_by_four() {
        let mut year = State::new(today()).with_view(View::Year);
        let _ = year.update(Event::Next);
        assert_eq!(year.cursor(), date(2027, 11, 19));
        assert_eq!(year.short_title(), "2027");
        let _ = year.update(Event::Previous);
        let _ = year.update(Event::Previous);
        assert_eq!(year.cursor(), date(2025, 11, 19));

        let mut days = State::new(date(2026, 11, 29)).with_view(View::FourDays);
        assert_eq!(days.short_title(), "29 Nov - 2 Dec");
        let _ = days.update(Event::Next);
        assert_eq!(days.range(), (date(2026, 12, 3), date(2026, 12, 6)));
        let _ = days.update(Event::Previous);
        let _ = days.update(Event::Previous);
        assert_eq!(days.cursor(), date(2026, 11, 25));

        // A day moved out of the four starts the next four.
        let mut days = State::new(today()).with_view(View::FourDays);
        let _ = days.update(Event::MoveDay(3));
        assert_eq!(days.cursor(), today());
        let _ = days.update(Event::MoveDay(1));
        assert_eq!(days.cursor(), date(2026, 11, 23));
    }

    #[test]
    fn week_titles_name_both_months_or_years_when_they_differ() {
        let state = State::new(date(2026, 12, 1)).with_view(View::Week);
        assert_eq!(state.title(), "30 Nov - 6 Dec 2026");
        let state = State::new(date(2026, 12, 31)).with_view(View::Week);
        assert_eq!(state.title(), "28 Dec 2026 - 3 Jan 2027");
    }

    #[test]
    fn short_titles_fit_a_phone() {
        let mut state = State::new(today());
        assert_eq!(state.short_title(), "November 2026");
        let _ = state.update(Event::Show(View::Agenda));
        assert_eq!(state.short_title(), "November 2026");
        let _ = state.update(Event::Show(View::Day));
        assert_eq!(state.short_title(), "Thu 19 Nov 2026");
        let _ = state.update(Event::Show(View::Week));
        assert_eq!(state.short_title(), "16 - 22 Nov 2026");

        let across_months = State::new(date(2026, 12, 1)).with_view(View::Week);
        assert_eq!(across_months.short_title(), "30 Nov - 6 Dec");
        let across_years = State::new(date(2026, 12, 31)).with_view(View::Week);
        assert_eq!(across_years.short_title(), "28 Dec - 3 Jan");
    }

    #[test]
    fn the_week_follows_the_day_weeks_start_on() {
        let state = State::new(today())
            .with_view(View::Week)
            .with_week_start(Weekday::Sunday);
        assert_eq!(state.range(), (date(2026, 11, 15), date(2026, 11, 21)));
    }

    #[test]
    fn next_and_previous_step_by_what_the_view_shows() {
        let mut state = state();
        assert_eq!(
            state.update(Event::Next),
            Some(Output::Viewed {
                view: View::Month,
                first: date(2026, 11, 30),
                last: date(2027, 1, 3)
            })
        );
        assert_eq!(state.cursor(), date(2026, 12, 19));
        let _ = state.update(Event::Previous);
        assert_eq!(state.cursor(), today());

        let _ = state.update(Event::Show(View::Week));
        let _ = state.update(Event::Next);
        assert_eq!(state.cursor(), date(2026, 11, 26));
        let _ = state.update(Event::Show(View::Day));
        let _ = state.update(Event::Previous);
        assert_eq!(state.cursor(), date(2026, 11, 25));
        let _ = state.update(Event::Show(View::Agenda));
        let _ = state.update(Event::Next);
        assert_eq!(state.cursor(), date(2026, 12, 25));
    }

    #[test]
    fn stepping_a_month_from_the_31st_lands_on_the_last_day() {
        let mut state = State::new(date(2027, 1, 31));
        let _ = state.update(Event::Next);
        assert_eq!(state.cursor(), date(2027, 2, 28));
    }

    #[test]
    fn showing_the_view_already_on_show_does_nothing() {
        let mut state = state();
        assert_eq!(state.update(Event::Show(View::Month)), None);
        assert!(state.update(Event::Show(View::Week)).is_some());
    }

    #[test]
    fn switching_view_goes_to_the_selected_day() {
        let mut state = state();
        let _ = state.update(Event::Press(Selection::Day(date(2026, 11, 3))));
        let _ = state.update(Event::Show(View::Day));
        assert_eq!(state.cursor(), date(2026, 11, 3));
    }

    #[test]
    fn go_to_reports_only_when_the_period_changes() {
        let mut state = state();
        assert_eq!(state.update(Event::GoTo(date(2026, 11, 2))), None);
        assert_eq!(state.cursor(), date(2026, 11, 2));
        assert!(matches!(
            state.update(Event::GoTo(date(2027, 3, 1))),
            Some(Output::Viewed { .. })
        ));
        assert_eq!(state.selection(), None);
    }

    #[test]
    fn today_comes_back_and_selects_today() {
        let mut state = state();
        assert_eq!(
            state.update(Event::Today),
            Some(Output::Selected(Some(Selection::Day(today()))))
        );
        assert_eq!(state.update(Event::Today), None);
        let _ = state.update(Event::Next);
        assert!(matches!(
            state.update(Event::Today),
            Some(Output::Viewed { .. })
        ));
        assert_eq!(state.cursor(), today());
    }

    #[test]
    fn the_first_press_selects_and_the_second_activates() {
        let mut state = state();
        let day = Selection::Day(date(2026, 11, 3));
        assert_eq!(
            state.update(Event::Press(day)),
            Some(Output::Selected(Some(day)))
        );
        assert!(state.is_selected(day));
        assert_eq!(
            state.update(Event::Press(day)),
            Some(Output::Requested(Draft {
                when: When::day(date(2026, 11, 3))
            }))
        );

        let picked = entry(1, 19, 9);
        assert_eq!(
            state.update(Event::Press(picked)),
            Some(Output::Selected(Some(picked)))
        );
        assert_eq!(state.selected_entry().map(Entry::id), Some(EntryId(1)));
        assert_eq!(state.selected_day(), Some(today()));
        assert_eq!(
            state.update(Event::Press(picked)),
            Some(Output::Opened {
                entry: EntryId(1),
                start: at(19, 9)
            })
        );
    }

    #[test]
    fn a_slot_asks_for_an_hour_and_stops_at_midnight() {
        let mut state = state();
        let slot = Selection::Slot {
            date: today(),
            minute: 600,
        };
        let _ = state.update(Event::Press(slot));
        assert_eq!(
            state.update(Event::Activate),
            Some(Output::Requested(Draft {
                when: When::timed(at(19, 10), at(19, 11))
            }))
        );
        let late = Selection::Slot {
            date: today(),
            minute: 1410,
        };
        let _ = state.update(Event::Press(late));
        assert_eq!(
            state.update(Event::Activate),
            Some(Output::Requested(Draft {
                when: When::timed(instant_at(today(), 1410, &TimeZone::UTC), at(20, 0))
            }))
        );
    }

    #[test]
    fn activating_with_nothing_selected_asks_for_the_day_on_show() {
        let mut state = state();
        assert_eq!(
            state.update(Event::Activate),
            Some(Output::Requested(Draft {
                when: When::day(today())
            }))
        );
    }

    #[test]
    fn activating_an_entry_that_is_gone_does_nothing() {
        let mut state = state().with_selection(entry(9, 19, 9));
        assert_eq!(state.update(Event::Activate), None);
        assert_eq!(state.selected_entry(), None);
    }

    #[test]
    fn everything_is_allowed_unless_set() {
        assert_eq!(Allow::default(), Allow::ALL);
        assert_eq!(state().allow(), Allow::ALL);
        assert_eq!(state().read_only().allow(), Allow::NONE);
        let mut state = state();
        state.set_allow(Allow {
            remove: false,
            ..Allow::ALL
        });
        assert!(state.allow().create && !state.allow().remove);
    }

    #[test]
    fn a_read_only_calendar_can_be_looked_through_but_not_changed() {
        let mut state = state().read_only();
        let day = Selection::Day(date(2026, 11, 3));
        let slot = Selection::Slot {
            date: today(),
            minute: 600,
        };

        // Selecting and moving about still work.
        assert_eq!(
            state.update(Event::Press(day)),
            Some(Output::Selected(Some(day)))
        );
        assert!(state.update(Event::Next).is_some());
        assert!(state.update(Event::Show(View::Week)).is_some());

        // A second press, Enter or a drag asks for nothing.
        let _ = state.update(Event::Press(day));
        assert_eq!(state.update(Event::Press(day)), None);
        assert_eq!(state.update(Event::Activate), None);
        let _ = state.update(Event::Press(slot));
        assert_eq!(state.update(Event::Press(slot)), None);
        let draft = Draft {
            when: When::day(today()),
        };
        assert_eq!(state.update(Event::Request(draft)), None);
        let _ = state.update(Event::Clear);
        assert_eq!(state.update(Event::Activate), None);
        assert_eq!(Action::Activate.event(&state), None);

        // An entry can still be opened, but not dragged or removed by key.
        let picked = entry(1, 19, 9);
        let _ = state.update(Event::Press(picked));
        assert_eq!(
            state.update(Event::Press(picked)),
            Some(Output::Opened {
                entry: EntryId(1),
                start: at(19, 9)
            })
        );
        assert_eq!(Action::Activate.event(&state), Some(Event::Activate));
        assert_eq!(Action::Remove.event(&state), None);
        let drag = Event::Reschedule {
            entry: EntryId(1),
            start: at(19, 9),
            to: (at(20, 9), at(20, 10)),
        };
        assert_eq!(state.update(drag), None);
        assert_eq!(
            state.entries().get(EntryId(1)).map(Entry::when),
            Some(meeting(1, 19, 9).when())
        );
    }

    #[test]
    fn the_app_can_still_change_a_read_only_calendar() {
        let mut state = state().read_only();
        assert!(state.update(Event::Insert(meeting(7, 21, 9))).is_some());
        assert!(state.update(Event::Insert(meeting(7, 22, 9))).is_some());
        assert!(state.update(Event::Remove(EntryId(7))).is_some());
        assert_eq!(state.entries().len(), 3);
    }

    #[test]
    fn each_permission_stands_alone() {
        let mut state = state().with_allow(Allow {
            create: false,
            ..Allow::ALL
        });
        let _ = state.update(Event::Press(entry(1, 19, 9)));
        assert_eq!(
            Action::Remove.event(&state),
            Some(Event::Remove(EntryId(1)))
        );
        let drag = Event::Reschedule {
            entry: EntryId(1),
            start: at(19, 9),
            to: (at(20, 9), at(20, 10)),
        };
        assert!(state.update(drag).is_some());
        let _ = state.update(Event::Clear);
        assert_eq!(state.update(Event::Activate), None);
    }

    #[test]
    fn clear_reports_only_when_something_was_selected() {
        let mut state = state();
        assert_eq!(state.update(Event::Clear), None);
        let _ = state.update(Event::Press(Selection::Day(today())));
        assert_eq!(state.update(Event::Clear), Some(Output::Selected(None)));
        assert_eq!(state.selection(), None);
    }

    #[test]
    fn moving_the_day_starts_from_the_period_on_show_and_turns_the_page() {
        let mut state = state();
        assert_eq!(
            state.update(Event::MoveDay(1)),
            Some(Output::Selected(Some(Selection::Day(date(2026, 11, 20)))))
        );
        let _ = state.update(Event::MoveDay(7));
        assert_eq!(state.selected_day(), Some(date(2026, 11, 27)));
        assert_eq!(state.cursor(), today(), "still in November");

        // 4 December is in the month grid, but belongs to the next month.
        assert!(matches!(
            state.update(Event::MoveDay(7)),
            Some(Output::Viewed { .. })
        ));
        assert_eq!(state.cursor(), date(2026, 12, 4));

        let mut week = State::new(today()).with_view(View::Week);
        let _ = week.update(Event::MoveDay(3));
        assert_eq!(week.cursor(), today());
        assert!(matches!(
            week.update(Event::MoveDay(1)),
            Some(Output::Viewed { .. })
        ));
        assert_eq!(week.cursor(), date(2026, 11, 23));
    }

    #[test]
    fn entries_of_the_selected_day_cycle_and_wrap() {
        let mut state = state();
        assert_eq!(
            state.update(Event::NextEntry),
            Some(Output::Selected(Some(entry(1, 19, 9))))
        );
        let _ = state.update(Event::NextEntry);
        assert!(state.is_selected(entry(2, 19, 14)));
        let _ = state.update(Event::NextEntry);
        assert!(state.is_selected(entry(1, 19, 9)), "wraps");
        let _ = state.update(Event::PreviousEntry);
        assert!(state.is_selected(entry(2, 19, 14)));

        let mut from_the_end = self::state();
        let _ = from_the_end.update(Event::PreviousEntry);
        assert!(from_the_end.is_selected(entry(2, 19, 14)));

        let mut one = self::state().with_selection(Selection::Day(date(2026, 11, 20)));
        let _ = one.update(Event::NextEntry);
        assert_eq!(one.update(Event::NextEntry), None, "the only entry");

        let mut none = self::state().with_selection(Selection::Day(date(2026, 11, 1)));
        assert_eq!(none.update(Event::NextEntry), None);
    }

    #[test]
    fn the_schedule_steps_through_all_its_entries_with_the_arrows() {
        let mut state = state().with_view(View::Agenda);
        for action in [Action::NextDay, Action::NextWeek] {
            assert_eq!(action.event(&state), Some(Event::NextEntry), "{action:?}");
        }
        for action in [Action::PreviousDay, Action::PreviousWeek] {
            assert_eq!(
                action.event(&state),
                Some(Event::PreviousEntry),
                "{action:?}"
            );
        }
        let _ = state.update(Event::NextEntry);
        let _ = state.update(Event::NextEntry);
        let _ = state.update(Event::NextEntry);
        assert!(state.is_selected(entry(3, 20, 9)), "on to the next day");
        let _ = state.update(Event::NextEntry);
        assert!(state.is_selected(entry(1, 19, 9)), "and round again");
    }

    #[test]
    fn open_day_switches_to_the_day_view() {
        let mut state = state();
        assert_eq!(
            state.update(Event::OpenDay(date(2026, 11, 3))),
            Some(Output::Viewed {
                view: View::Day,
                first: date(2026, 11, 3),
                last: date(2026, 11, 3)
            })
        );
        assert_eq!(state.view(), View::Day);
        assert_eq!(state.update(Event::OpenDay(date(2026, 11, 3))), None);
    }

    #[test]
    fn a_request_is_passed_on_and_adds_nothing() {
        let mut state = state();
        let draft = Draft {
            when: When::timed(at(19, 11), at(19, 12)),
        };
        assert_eq!(
            state.update(Event::Request(draft)),
            Some(Output::Requested(draft))
        );
        assert_eq!(state.entries().len(), 3);
    }

    #[test]
    fn insert_and_remove_report_the_change() {
        let mut state = state();
        let change = |output: Option<Output>| match output {
            Some(Output::Changed(changed)) => Some(changed.change),
            _ => None,
        };
        let created = state.update(Event::Insert(meeting(4, 21, 9)));
        assert_eq!(change(created), Some(Change::Created));
        let updated = state.update(Event::Insert(meeting(4, 22, 9)));
        assert_eq!(change(updated), Some(Change::Updated));
        assert_eq!(state.update(Event::Insert(meeting(4, 22, 9))), None);

        let _ = state.update(Event::Press(entry(4, 22, 9)));
        let removed = state.update(Event::Remove(EntryId(4)));
        assert_eq!(change(removed), Some(Change::Removed));
        assert_eq!(state.selection(), None, "the selection goes with it");
        assert_eq!(state.update(Event::Remove(EntryId(4))), None);
    }

    #[test]
    fn removing_another_entry_keeps_the_selection() {
        let mut state = state().with_selection(entry(1, 19, 9));
        let _ = state.update(Event::Remove(EntryId(2)));
        assert!(state.is_selected(entry(1, 19, 9)));
    }

    #[test]
    fn rescheduling_moves_the_entry_and_the_selection() {
        let mut state = state().with_selection(entry(1, 19, 9));
        let output = state.update(Event::Reschedule {
            entry: EntryId(1),
            start: at(19, 9),
            to: (at(20, 15), at(20, 17)),
        });
        let Some(Output::Changed(changed)) = output else {
            panic!("an update");
        };
        assert_eq!(changed.change, Change::Updated);
        assert_eq!(changed.entry.when(), When::timed(at(20, 15), at(20, 17)));
        assert_eq!(changed.before, Some(meeting(1, 19, 9)));
        assert!(state.is_selected(entry(1, 20, 15)));
    }

    #[test]
    fn rescheduling_one_occurrence_moves_the_whole_series() {
        let mut state = State::new(today())
            .with_entries(Entries::utc().with([meeting(1, 16, 9).repeat(Repeat::daily())]));
        // The occurrence on the 19th is dragged two hours later.
        let _ = state.update(Event::Reschedule {
            entry: EntryId(1),
            start: at(19, 9),
            to: (at(19, 11), at(19, 12)),
        });
        let moved = state.entries().get(EntryId(1)).map(Entry::when);
        assert_eq!(moved, Some(When::timed(at(16, 11), at(16, 12))));
    }

    #[test]
    fn rescheduling_ignores_unknown_whole_day_and_unmoved_entries() {
        let mut state = state();
        let day = date(2026, 11, 19);
        let _ = state.update(Event::Insert(Entry::all_day(EntryId(8), "Trip", day, day)));
        let to = (at(20, 9), at(20, 10));
        for id in [8, 99] {
            let event = Event::Reschedule {
                entry: EntryId(id),
                start: at(19, 0),
                to,
            };
            assert_eq!(state.update(event), None, "{id}");
        }
        let unmoved = Event::Reschedule {
            entry: EntryId(1),
            start: at(19, 9),
            to: (at(19, 9), at(19, 10)),
        };
        assert_eq!(state.update(unmoved), None);
    }

    #[test]
    fn a_tick_moves_today_and_the_now_line() {
        let mut state = state();
        assert_eq!(state.update(Event::Tick(at(20, 0))), None);
        assert_eq!(state.now(), Some(at(20, 0)));
        assert_eq!(state.today(), date(2026, 11, 20));
        assert_eq!(state.cursor(), today(), "the period on show stays");
    }

    fn press(keymap: &Keymap<Action>, chord: &str) -> Option<Action> {
        let Ok(chord) = chord.parse::<Chord>() else {
            panic!("{chord} is a chord");
        };
        keymap.resolve(chord.key(), chord.modifiers())
    }

    #[test]
    fn default_keymap_covers_every_action() {
        let keymap = default_keymap();
        let expected = [
            ("PageUp", Action::Previous),
            ("PageDown", Action::Next),
            ("T", Action::Today),
            ("ArrowLeft", Action::PreviousDay),
            ("ArrowRight", Action::NextDay),
            ("ArrowUp", Action::PreviousWeek),
            ("ArrowDown", Action::NextWeek),
            ("Alt+ArrowUp", Action::PreviousEntry),
            ("Alt+ArrowDown", Action::NextEntry),
            ("M", Action::Month),
            ("W", Action::Week),
            ("D", Action::Day),
            ("A", Action::Agenda),
            ("Y", Action::Year),
            ("X", Action::FourDays),
            ("Enter", Action::Activate),
            ("Delete", Action::Remove),
            ("Backspace", Action::Remove),
            ("Escape", Action::Close),
        ];
        for (chord, action) in expected {
            assert_eq!(press(&keymap, chord), Some(action), "{chord}");
        }
        assert_eq!(press(&keymap, "Ctrl+T"), None);
        for action in <Action as keys::Action>::ALL {
            assert!(!keymap.chords(action).is_empty(), "{action:?}");
            assert!(!keys::Action::name(*action).is_empty());
            assert!(!keys::Action::description(*action).is_empty());
        }
        assert_eq!(Keymap::<Action>::defaults(), keymap);
    }

    #[test]
    fn keymap_overrides_and_unbinding() {
        let Ok(chord) = "Ctrl+G".parse::<Chord>() else {
            panic!("a chord");
        };
        let keymap = default_keymap()
            .unbind(&Chord::character('t'))
            .bind(chord, Action::Today);
        assert_eq!(press(&keymap, "T"), None);
        assert_eq!(press(&keymap, "Ctrl+G"), Some(Action::Today));
        assert_eq!(press(&keymap, "M"), Some(Action::Month));
        assert!(
            default_keymap()
                .unbind_action(&Action::Remove)
                .chords(&Action::Remove)
                .is_empty()
        );
    }

    #[test]
    fn actions_map_to_events() {
        let state = state();
        assert_eq!(Action::Next.event(&state), Some(Event::Next));
        assert_eq!(Action::Previous.event(&state), Some(Event::Previous));
        assert_eq!(Action::Today.event(&state), Some(Event::Today));
        assert_eq!(Action::PreviousDay.event(&state), Some(Event::MoveDay(-1)));
        assert_eq!(Action::NextDay.event(&state), Some(Event::MoveDay(1)));
        assert_eq!(Action::PreviousWeek.event(&state), Some(Event::MoveDay(-7)));
        assert_eq!(Action::NextWeek.event(&state), Some(Event::MoveDay(7)));
        assert_eq!(Action::NextEntry.event(&state), Some(Event::NextEntry));
        assert_eq!(
            Action::PreviousEntry.event(&state),
            Some(Event::PreviousEntry)
        );
        assert_eq!(Action::Activate.event(&state), Some(Event::Activate));
        assert_eq!(Action::Month.event(&state), None, "already on show");
        assert_eq!(Action::Week.event(&state), Some(Event::Show(View::Week)));
        assert_eq!(Action::Day.event(&state), Some(Event::Show(View::Day)));
        assert_eq!(
            Action::Agenda.event(&state),
            Some(Event::Show(View::Agenda))
        );
        assert_eq!(Action::Year.event(&state), Some(Event::Show(View::Year)));
        assert_eq!(
            Action::FourDays.event(&state),
            Some(Event::Show(View::FourDays))
        );
    }

    #[test]
    fn remove_and_close_need_a_selection() {
        let mut state = state();
        assert_eq!(Action::Remove.event(&state), None);
        assert_eq!(Action::Close.event(&state), None);

        let _ = state.update(Event::Press(Selection::Day(today())));
        assert_eq!(Action::Remove.event(&state), None, "a day is not an entry");
        assert_eq!(Action::Close.event(&state), Some(Event::Clear));

        let _ = state.update(Event::Press(entry(2, 19, 14)));
        let Some(event) = Action::Remove.event(&state) else {
            panic!("an entry is selected");
        };
        assert_eq!(event, Event::Remove(EntryId(2)));
        assert!(state.update(event).is_some());
        assert_eq!(state.entries().len(), 2);
    }
}
