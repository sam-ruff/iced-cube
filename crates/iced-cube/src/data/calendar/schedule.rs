//! Triggers: telling the app when something happens to an entry.
//!
//! A [`Trigger`] is built from a few base parts: a change to an entry
//! ([`Change`]), or a moment measured from where an occurrence starts or
//! ends ([`Anchor`]). Each trigger carries a key of the app's own type,
//! and a [`Scheduler`] hands that key back in a [`Fired`] whenever the
//! trigger goes off, so the app decides what each one means.
//!
//! The scheduler never reads the clock. The app passes the time to
//! [`Scheduler::poll`], usually from [`timer`], which makes every rule
//! here testable with made-up times.
//!
//! ```
//! use std::time::Duration;
//!
//! use iced_cube::data::calendar::schedule::{Anchor, Delivery, Kind, Scheduler, Trigger};
//! use iced_cube::data::calendar::{Entries, Entry, EntryId};
//! use jiff::Timestamp;
//!
//! #[derive(Debug, Clone, PartialEq)]
//! enum Reminder {
//!     Soon,
//!     Now,
//! }
//!
//! let start: Timestamp = "2026-11-03T09:00:00Z".parse()?;
//! let end: Timestamp = "2026-11-03T09:30:00Z".parse()?;
//! let entries = Entries::utc().with([Entry::new(EntryId(1), "Stand-up", start, end)]);
//!
//! let mut scheduler = Scheduler::new()
//!     .with(Trigger::new(Reminder::Soon).before(Anchor::Start, Duration::from_secs(600)))
//!     .with(Trigger::new(Reminder::Now).at(Anchor::Start));
//!
//! // The first poll only sets where the scheduler starts from.
//! assert!(scheduler.poll(&entries, "2026-11-03T08:00:00Z".parse()?).is_empty());
//!
//! let fired = scheduler.poll(&entries, "2026-11-03T08:50:10Z".parse()?);
//! assert_eq!(fired.len(), 1);
//! assert_eq!(fired[0].trigger, Reminder::Soon);
//! assert!(!fired[0].is_missed());
//!
//! // Nothing polled for two hours, so the start is delivered late.
//! let fired = scheduler.poll(&entries, "2026-11-03T11:00:00Z".parse()?);
//! assert_eq!(fired[0].trigger, Reminder::Now);
//! assert!(matches!(
//!     fired[0].kind,
//!     Kind::Due { delivery: Delivery::Missed { .. }, .. }
//! ));
//! # Ok::<(), jiff::Error>(())
//! ```
//!
//! # In an app
//!
//! [`timer`] supplies the time while a trigger is pending, and the app
//! polls in `update`:
//!
//! ```no_run
//! use iced::{Element, Subscription};
//! use iced_cube::data::calendar::schedule::{self, Anchor, Scheduler, Trigger};
//! use iced_cube::data::calendar::{self, calendar};
//! use jiff::Timestamp;
//!
//! #[derive(Debug, Clone, PartialEq)]
//! enum Reminder {
//!     Starting,
//! }
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Calendar(calendar::Event),
//!     Clock(Timestamp),
//! }
//!
//! struct App {
//!     calendar: calendar::State,
//!     scheduler: Scheduler<Reminder>,
//! }
//!
//! impl App {
//!     fn new() -> Self {
//!         Self {
//!             calendar: calendar::State::at(Timestamp::now(), calendar::Entries::system()),
//!             scheduler: Scheduler::new()
//!                 .with(Trigger::new(Reminder::Starting).at(Anchor::Start)),
//!         }
//!     }
//!
//!     fn update(&mut self, message: Message) {
//!         match message {
//!             Message::Calendar(event) => {
//!                 let _ = self.calendar.update(event);
//!             }
//!             Message::Clock(now) => {
//!                 let _ = self.calendar.update(calendar::Event::Tick(now));
//!                 for fired in self.scheduler.poll(self.calendar.entries(), now) {
//!                     let Some(entry) = self.calendar.entries().get(fired.entry) else {
//!                         continue;
//!                     };
//!                     match fired.trigger {
//!                         Reminder::Starting => println!("{} is starting", entry.title()),
//!                     }
//!                 }
//!             }
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         calendar(&self.calendar).on_event(Message::Calendar).into()
//!     }
//!
//!     fn subscription(&self) -> Subscription<Message> {
//!         schedule::timer(&self.scheduler, self.calendar.entries()).map(Message::Clock)
//!     }
//! }
//! ```
//!
//! # Changes, scopes and repeats
//!
//! Triggers on changes come from [`Scheduler::observe`]. A [`Scope`]
//! limits a trigger to a tag or one entry, and [`Trigger::every`] repeats
//! one until its anchor:
//!
//! ```
//! use std::time::Duration;
//!
//! use iced_cube::data::calendar::schedule::{Anchor, Kind, Scheduler, Scope, Trigger};
//! use iced_cube::data::calendar::{Change, Entries, Entry, EntryId};
//! use jiff::Timestamp;
//!
//! #[derive(Debug, Clone, PartialEq)]
//! enum Rule {
//!     Created,
//!     WorkSoon,
//!     Nag,
//! }
//!
//! const MINUTE: Duration = Duration::from_secs(60);
//!
//! let mut scheduler = Scheduler::new()
//!     .with(Trigger::new(Rule::Created).on(Change::Created))
//!     .with(
//!         Trigger::new(Rule::WorkSoon)
//!             .before(Anchor::Start, 10 * MINUTE)
//!             .scope(Scope::tagged("work")),
//!     )
//!     .with(
//!         Trigger::new(Rule::Nag)
//!             .before(Anchor::Start, 15 * MINUTE)
//!             .every(5 * MINUTE)
//!             .scope(Scope::Entry(EntryId(2))),
//!     )
//!     .resume_from("2026-11-03T08:00:00Z".parse()?);
//!
//! let start: Timestamp = "2026-11-03T09:00:00Z".parse()?;
//! let end: Timestamp = "2026-11-03T10:00:00Z".parse()?;
//! let mut entries = Entries::utc();
//!
//! // A change is observed as it happens.
//! let stand_up = Entry::new(EntryId(1), "Stand-up", start, end).tag("work");
//! if let Some(changed) = entries.insert(stand_up) {
//!     let fired = scheduler.observe(&changed);
//!     assert_eq!(fired[0].trigger, Rule::Created);
//!     assert_eq!(fired[0].kind, Kind::Changed(Change::Created));
//! }
//! let _ = entries.insert(Entry::new(EntryId(2), "Release", start, end));
//!
//! // By 08:52 two things came due at 08:50: the reminder for the tagged
//! // entry, and the second nag for the release. The first nag, at 08:45,
//! // is not delivered as well.
//! let fired = scheduler.poll(&entries, "2026-11-03T08:52:00Z".parse()?);
//! let keys: Vec<_> = fired.iter().map(|fired| (&fired.trigger, fired.entry)).collect();
//! assert_eq!(keys, vec![(&Rule::WorkSoon, EntryId(1)), (&Rule::Nag, EntryId(2))]);
//! # Ok::<(), jiff::Error>(())
//! ```
//!
//! # Missed triggers and snoozing
//!
//! Whatever came due while nothing was polling arrives on the next poll
//! as [`Delivery::Missed`]. Save [`Scheduler::watermark`] when the app
//! closes and pass it to [`Scheduler::resume_from`] to catch up after a
//! restart:
//!
//! ```
//! use iced_cube::data::calendar::schedule::{Anchor, Scheduler, Trigger};
//! use iced_cube::data::calendar::{Entries, Entry, EntryId};
//! use jiff::{SignedDuration, Timestamp};
//!
//! let start: Timestamp = "2026-11-03T09:00:00Z".parse()?;
//! let end: Timestamp = "2026-11-03T10:00:00Z".parse()?;
//! let entries = Entries::utc().with([Entry::new(EntryId(1), "Stand-up", start, end)]);
//!
//! // Last open the evening before; opened again at 09:30.
//! let saved: Timestamp = "2026-11-02T18:00:00Z".parse()?;
//! let now: Timestamp = "2026-11-03T09:30:00Z".parse()?;
//! let mut scheduler = Scheduler::new()
//!     .with(Trigger::new("starting").at(Anchor::Start))
//!     .resume_from(saved);
//!
//! let fired = scheduler.poll(&entries, now);
//! assert_eq!(fired[0].late_by(), Some(SignedDuration::from_mins(30)));
//! assert_eq!(scheduler.watermark(), Some(now));
//!
//! // Put it off for ten minutes. It comes back once, on time.
//! let later: Timestamp = "2026-11-03T09:40:00Z".parse()?;
//! assert!(scheduler.snooze(&fired[0], later));
//! assert!(scheduler.poll(&entries, "2026-11-03T09:35:00Z".parse()?).is_empty());
//! let again = scheduler.poll(&entries, later);
//! assert_eq!(again.len(), 1);
//! assert!(!again[0].is_missed());
//! # Ok::<(), jiff::Error>(())
//! ```

use std::collections::BTreeMap;
use std::time::Duration;

use iced::Subscription;
use jiff::{SignedDuration, Timestamp};

use super::entry::{Change, Changed, Entries, Entry, EntryId, Occurrence};

/// How often [`timer`] and [`clock`] tick.
pub const TICK: Duration = Duration::from_secs(30);

/// How late a trigger can be delivered and still count as on time: two
/// ticks, so an ordinary timer never reports a miss.
pub const DEFAULT_GRACE: Duration = Duration::from_secs(60);

/// How far before the watermark a poll can be and still be taken for the
/// clock being corrected. Further back than this, the watermark itself is
/// taken to be wrong and the scheduler starts again from the time given.
pub const REWIND_LIMIT: Duration = Duration::from_secs(24 * 60 * 60);

/// How far ahead [`Scheduler::next_due`] looks, nearest first.
const HORIZONS: [i64; 4] = [1, 31, 366, 3660];

const DAY_SECONDS: i64 = 24 * 60 * 60;

/// The end of an occurrence a trigger is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Anchor {
    Start,
    End,
}

impl Anchor {
    pub const ALL: [Anchor; 2] = [Anchor::Start, Anchor::End];

    fn of(self, occurrence: &Occurrence) -> Timestamp {
        match self {
            Anchor::Start => occurrence.start,
            Anchor::End => occurrence.end,
        }
    }
}

/// Which entries a trigger applies to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Scope {
    #[default]
    All,
    /// Entries with this [tag](Entry::tag).
    Tagged(String),
    /// One entry.
    Entry(EntryId),
}

impl Scope {
    /// Entries with `tag`.
    pub fn tagged(tag: impl Into<String>) -> Self {
        Scope::Tagged(tag.into())
    }

    pub fn includes(&self, entry: &Entry) -> bool {
        match self {
            Scope::All => true,
            Scope::Tagged(tag) => entry.tag_of() == Some(tag.as_str()),
            Scope::Entry(id) => entry.id() == *id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Moment {
    Change(Change),
    Time {
        anchor: Anchor,
        /// Negative before the anchor.
        offset: SignedDuration,
        every: Option<SignedDuration>,
    },
}

/// A rule for telling the app about an entry. `K` is the app's own key,
/// usually an enum with one variant per rule.
///
/// A new trigger goes off when an occurrence starts, for every entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Trigger<K> {
    key: K,
    moment: Moment,
    scope: Scope,
}

impl<K> Trigger<K> {
    pub fn new(key: K) -> Self {
        Self {
            key,
            moment: Moment::Time {
                anchor: Anchor::Start,
                offset: SignedDuration::ZERO,
                every: None,
            },
            scope: Scope::All,
        }
    }

    /// Goes off when an entry is created, updated or removed. Reported by
    /// [`Scheduler::observe`], not by polling.
    pub fn on(mut self, change: Change) -> Self {
        self.moment = Moment::Change(change);
        self
    }

    /// Goes off when an occurrence reaches `anchor`: its start for "due",
    /// its end for "over".
    pub fn at(self, anchor: Anchor) -> Self {
        self.offset(anchor, SignedDuration::ZERO)
    }

    /// Goes off `lead` before `anchor`, for "almost due".
    pub fn before(self, anchor: Anchor, lead: Duration) -> Self {
        let lead = signed(lead);
        self.offset(anchor, lead.checked_neg().unwrap_or(SignedDuration::ZERO))
    }

    /// Goes off `delay` after `anchor`, such as a follow-up an hour after
    /// a meeting ends.
    pub fn after(self, anchor: Anchor, delay: Duration) -> Self {
        self.offset(anchor, signed(delay))
    }

    /// Goes off again every `period` until the anchor is reached. Only a
    /// trigger set [`before`](Trigger::before) its anchor repeats, and when
    /// several repeats come due between two polls only the latest is
    /// delivered.
    pub fn every(mut self, period: Duration) -> Self {
        let period = signed(period);
        if let Moment::Time { every, .. } = &mut self.moment {
            *every = period.is_positive().then_some(period);
        }
        self
    }

    /// Limits the trigger to some entries.
    pub fn scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn scope_of(&self) -> &Scope {
        &self.scope
    }

    /// Whether the trigger is reported by polling, as opposed to by a
    /// change.
    pub fn is_timed(&self) -> bool {
        matches!(self.moment, Moment::Time { .. })
    }

    fn offset(mut self, anchor: Anchor, offset: SignedDuration) -> Self {
        let every = match self.moment {
            Moment::Time { every, .. } => every,
            Moment::Change(_) => None,
        };
        self.moment = Moment::Time {
            anchor,
            offset,
            every,
        };
        self
    }

    /// When the trigger goes off for `occurrence`.
    fn firing(&self, occurrence: &Occurrence) -> Option<Firing> {
        let Moment::Time {
            anchor,
            offset,
            every,
        } = self.moment
        else {
            return None;
        };
        let anchor = anchor.of(occurrence);
        Some(Firing {
            first: anchor.checked_add(offset).ok()?,
            anchor,
            every: every.filter(|_| offset.is_negative()),
        })
    }
}

fn signed(duration: Duration) -> SignedDuration {
    SignedDuration::try_from(duration).unwrap_or(SignedDuration::MAX)
}

/// The times a trigger goes off for one occurrence: once at `first`, or
/// every `every` from `first` while still before `anchor`.
#[derive(Debug, Clone, Copy)]
struct Firing {
    first: Timestamp,
    anchor: Timestamp,
    every: Option<SignedDuration>,
}

impl Firing {
    /// The latest time at or before `now`.
    fn latest(&self, now: Timestamp) -> Option<Timestamp> {
        if self.first > now {
            return None;
        }
        let Some(every) = self.every else {
            return Some(self.first);
        };
        let last = now.min(
            self.anchor
                .checked_sub(SignedDuration::from_nanos(1))
                .ok()?,
        );
        self.step(steps(last.duration_since(self.first), every))
    }

    /// The earliest time after `after`.
    fn earliest_after(&self, after: Timestamp) -> Option<Timestamp> {
        if self.first > after {
            return Some(self.first);
        }
        let every = self.every?;
        let next = steps(after.duration_since(self.first), every).checked_add(1)?;
        self.step(next).filter(|time| *time < self.anchor)
    }

    fn step(&self, count: i64) -> Option<Timestamp> {
        let every = self.every?;
        let nanos = every.as_nanos().checked_mul(i128::from(count))?;
        let nanos = i64::try_from(nanos).ok()?;
        self.first
            .checked_add(SignedDuration::from_nanos(nanos))
            .ok()
    }
}

/// How many whole `every` fit in `elapsed`.
fn steps(elapsed: SignedDuration, every: SignedDuration) -> i64 {
    let every = every.as_nanos().max(1);
    i64::try_from(elapsed.as_nanos().max(0) / every).unwrap_or(i64::MAX)
}

/// Whether a trigger arrived when it was due.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delivery {
    OnTime,
    /// The trigger came due while nothing was polling: the app was closed,
    /// the machine asleep or the clock changed. It is delivered once,
    /// late, and the app decides whether it still matters.
    Missed {
        late_by: SignedDuration,
    },
}

/// Why a trigger went off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The entry was created, updated or removed.
    Changed(Change),
    /// A moment measured from an occurrence was reached.
    Due {
        /// The start of the occurrence, which tells apart the repeats of a
        /// repeating entry.
        start: Timestamp,
        /// When the trigger should have gone off.
        due: Timestamp,
        delivery: Delivery,
    },
}

/// A trigger going off for an entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fired<K> {
    /// The key of the [`Trigger`].
    pub trigger: K,
    pub entry: EntryId,
    pub kind: Kind,
}

impl<K> Fired<K> {
    pub fn is_missed(&self) -> bool {
        self.late_by().is_some()
    }

    /// How late a missed trigger is.
    pub fn late_by(&self) -> Option<SignedDuration> {
        match self.kind {
            Kind::Due {
                delivery: Delivery::Missed { late_by },
                ..
            } => Some(late_by),
            _ => None,
        }
    }

    /// When a timed trigger should have gone off.
    pub fn due(&self) -> Option<Timestamp> {
        match self.kind {
            Kind::Due { due, .. } => Some(due),
            Kind::Changed(_) => None,
        }
    }
}

/// A fired trigger the user put off until later.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Snooze<K> {
    pub trigger: K,
    pub entry: EntryId,
    /// The start of the occurrence it was for.
    pub start: Timestamp,
    /// When to deliver it again.
    pub until: Timestamp,
}

/// Works out which triggers have gone off.
///
/// It remembers one instant, the [`watermark`](Scheduler::watermark): the
/// time of the last poll. Each poll delivers everything that came due
/// since then, once, and moves the watermark on. Save the watermark when
/// the app closes and pass it to [`resume_from`](Scheduler::resume_from)
/// on the next start to receive whatever was missed in between.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheduler<K> {
    triggers: Vec<Trigger<K>>,
    watermark: Option<Timestamp>,
    grace: SignedDuration,
    snoozed: Vec<Snooze<K>>,
}

impl<K> Default for Scheduler<K> {
    fn default() -> Self {
        Self {
            triggers: Vec::new(),
            watermark: None,
            grace: signed(DEFAULT_GRACE),
            snoozed: Vec::new(),
        }
    }
}

impl<K> Scheduler<K> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a trigger.
    pub fn with(mut self, trigger: Trigger<K>) -> Self {
        self.triggers.push(trigger);
        self
    }

    /// Carries on from a saved [`watermark`](Scheduler::watermark), so the
    /// first poll delivers what came due since then as
    /// [`Delivery::Missed`]. Without it the first poll starts from the
    /// time it is given and nothing counts as missed.
    pub fn resume_from(mut self, watermark: Timestamp) -> Self {
        self.watermark = Some(watermark);
        self
    }

    /// How late a trigger can be and still be [`Delivery::OnTime`].
    /// Defaults to [`DEFAULT_GRACE`]. Use at least twice the interval the
    /// app polls at.
    pub fn grace(mut self, grace: Duration) -> Self {
        self.grace = signed(grace);
        self
    }

    /// Restores a snooze saved from [`snoozed`](Scheduler::snoozed).
    pub fn with_snooze(mut self, snooze: Snooze<K>) -> Self {
        self.snoozed.push(snooze);
        self
    }

    pub fn triggers(&self) -> &[Trigger<K>] {
        &self.triggers
    }

    /// Adds a trigger to a scheduler already in use. It applies from the
    /// next poll, to times after the watermark.
    pub fn add(&mut self, trigger: Trigger<K>) {
        self.triggers.push(trigger);
    }

    /// The time of the last poll. Save it to resume later.
    pub fn watermark(&self) -> Option<Timestamp> {
        self.watermark
    }

    /// The snoozes still waiting, for saving alongside the watermark.
    pub fn snoozed(&self) -> &[Snooze<K>] {
        &self.snoozed
    }

    /// Whether [`timer`] runs: something is snoozed, or there is a timed
    /// trigger and an entry it could apply to.
    pub fn needs_timer(&self, entries: &Entries) -> bool {
        !self.snoozed.is_empty()
            || (!entries.is_empty() && self.triggers.iter().any(Trigger::is_timed))
    }
}

impl<K: PartialEq> Scheduler<K> {
    /// Removes every trigger with `key`, and anything snoozed for it.
    pub fn remove(&mut self, key: &K) {
        self.triggers.retain(|trigger| trigger.key != *key);
        self.snoozed.retain(|snooze| snooze.trigger != *key);
    }
}

impl<K: Clone> Scheduler<K> {
    /// The change triggers that apply to `changed`. Pass what
    /// [`Entries::insert`], [`Entries::remove`] or the calendar's
    /// `Output::Changed` hand back.
    pub fn observe(&self, changed: &Changed) -> Vec<Fired<K>> {
        self.triggers
            .iter()
            .filter(|trigger| trigger.moment == Moment::Change(changed.change))
            .filter(|trigger| trigger.scope.includes(&changed.entry))
            .map(|trigger| Fired {
                trigger: trigger.key.clone(),
                entry: changed.entry.id(),
                kind: Kind::Changed(changed.change),
            })
            .collect()
    }

    /// Everything that came due after the watermark and no later than
    /// `now`, earliest first, then moves the watermark to `now`.
    ///
    /// A time at or before the watermark, as when the clock is set back,
    /// delivers nothing new and leaves the watermark alone, so nothing
    /// goes off twice. A time more than [`REWIND_LIMIT`] before it means
    /// the watermark came from a clock that was wrong, so the scheduler
    /// starts again from `now`.
    ///
    /// When a repeating entry has missed the same trigger for several
    /// occurrences, only the latest is delivered: a year away gives one
    /// overdue stand-up, not two hundred.
    ///
    /// Poll regularly, as [`timer`] does. An entry added with a time that
    /// is already past is reported if that time is after the last poll.
    pub fn poll(&mut self, entries: &Entries, now: Timestamp) -> Vec<Fired<K>> {
        let mut fired = self.wake(entries, now);
        let from = match self.watermark {
            Some(watermark) if watermark.duration_since(now) > signed(REWIND_LIMIT) => {
                self.watermark = Some(now);
                return fired;
            }
            Some(watermark) if now <= watermark => return fired,
            Some(watermark) => watermark,
            None => now.checked_sub(self.grace).unwrap_or(now),
        };
        self.watermark = Some(now);

        for trigger in &self.triggers {
            // The latest missed occurrence of each entry for this trigger.
            let mut missed: BTreeMap<EntryId, (Timestamp, Timestamp)> = BTreeMap::new();
            for (occurrence, firing) in candidates(trigger, entries, from, now) {
                let Some(due) = firing.latest(now).filter(|due| *due > from) else {
                    continue;
                };
                if self.delivery(due, now) != Delivery::OnTime {
                    let latest = missed
                        .entry(occurrence.entry)
                        .or_insert((due, occurrence.start));
                    if due > latest.0 {
                        *latest = (due, occurrence.start);
                    }
                    continue;
                }
                fired.push(Fired {
                    trigger: trigger.key.clone(),
                    entry: occurrence.entry,
                    kind: Kind::Due {
                        start: occurrence.start,
                        due,
                        delivery: Delivery::OnTime,
                    },
                });
            }
            fired.extend(missed.into_iter().map(|(entry, (due, start))| Fired {
                trigger: trigger.key.clone(),
                entry,
                kind: Kind::Due {
                    start,
                    due,
                    delivery: self.delivery(due, now),
                },
            }));
        }
        fired.sort_by_key(|fired| (fired.due(), fired.entry));
        fired
    }

    /// Delivers `fired` again at `until`. Only a timed trigger can be
    /// snoozed; returns whether it was. Snoozing the same thing again
    /// replaces the earlier snooze.
    pub fn snooze(&mut self, fired: &Fired<K>, until: Timestamp) -> bool
    where
        K: PartialEq,
    {
        let Kind::Due { start, .. } = fired.kind else {
            return false;
        };
        self.snoozed.retain(|snooze| {
            (&snooze.trigger, snooze.entry, snooze.start) != (&fired.trigger, fired.entry, start)
        });
        self.snoozed.push(Snooze {
            trigger: fired.trigger.clone(),
            entry: fired.entry,
            start,
            until,
        });
        true
    }

    /// The next time anything is due after `after`, or `None` when nothing
    /// is within about ten years.
    pub fn next_due(&self, entries: &Entries, after: Timestamp) -> Option<Timestamp> {
        let snoozed = self
            .snoozed
            .iter()
            .map(|snooze| snooze.until.max(after))
            .min();
        let scheduled = HORIZONS.iter().find_map(|days| {
            let to = after
                .checked_add(SignedDuration::from_secs(days * DAY_SECONDS))
                .ok()?;
            self.triggers
                .iter()
                .flat_map(|trigger| candidates(trigger, entries, after, to))
                .filter_map(|(_, firing)| firing.earliest_after(after))
                .filter(|due| *due <= to)
                .min()
        });
        match (snoozed, scheduled) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Whether anything is still to come: something is snoozed, or a
    /// timed trigger has an occurrence ahead of the watermark.
    pub fn is_pending(&self, entries: &Entries) -> bool {
        if !self.snoozed.is_empty() {
            return true;
        }
        if entries.is_empty() || !self.triggers.iter().any(Trigger::is_timed) {
            return false;
        }
        match self.watermark {
            Some(watermark) => self.next_due(entries, watermark).is_some(),
            None => true,
        }
    }

    /// The snoozes that have run out, for entries that still exist.
    fn wake(&mut self, entries: &Entries, now: Timestamp) -> Vec<Fired<K>> {
        let (awake, asleep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.snoozed)
            .into_iter()
            .partition(|snooze| snooze.until <= now);
        self.snoozed = asleep;
        awake
            .into_iter()
            .filter(|snooze| entries.get(snooze.entry).is_some())
            .map(|snooze| Fired {
                trigger: snooze.trigger,
                entry: snooze.entry,
                kind: Kind::Due {
                    start: snooze.start,
                    due: snooze.until,
                    delivery: self.delivery(snooze.until, now),
                },
            })
            .collect()
    }

    fn delivery(&self, due: Timestamp, now: Timestamp) -> Delivery {
        let late_by = now.duration_since(due);
        if late_by > self.grace {
            Delivery::Missed { late_by }
        } else {
            Delivery::OnTime
        }
    }
}

/// The occurrences `trigger` could go off for after `from` and no later
/// than `to`.
fn candidates<K>(
    trigger: &Trigger<K>,
    entries: &Entries,
    from: Timestamp,
    to: Timestamp,
) -> Vec<(Occurrence, Firing)> {
    let Moment::Time { anchor, offset, .. } = trigger.moment else {
        return Vec::new();
    };
    // A trigger due in the window has its anchor `offset` away from it. A
    // repeating one keeps going until its anchor, so its window starts at
    // `from`.
    let lead = offset.min(SignedDuration::ZERO);
    let (Ok(earliest), Ok(latest)) = (
        from.checked_sub(offset.max(SignedDuration::ZERO)),
        to.checked_sub(lead),
    ) else {
        return Vec::new();
    };
    let margin = SignedDuration::from_secs(1);
    let (Ok(low), Ok(high)) = (earliest.checked_sub(margin), latest.checked_add(margin)) else {
        return Vec::new();
    };

    entries
        .iter()
        .filter(|entry| trigger.scope.includes(entry))
        .flat_map(|entry| super::repeat::occurrences(entry, entries.time_zone(), low, high))
        .filter(|occurrence| {
            let anchor = anchor.of(occurrence);
            anchor >= earliest && anchor <= latest
        })
        .filter_map(|occurrence| Some((occurrence, trigger.firing(&occurrence)?)))
        .collect()
}

/// Ticks every [`TICK`] with the current time while `scheduler` has a
/// timed trigger and there are entries, or something is snoozed. Feed
/// each tick to [`Scheduler::poll`].
///
/// It keeps ticking with nothing due, so the scheduler always knows
/// roughly what time it is and an entry added later is judged against
/// now, not against the last time something happened to be pending.
pub fn timer<K>(scheduler: &Scheduler<K>, entries: &Entries) -> Subscription<Timestamp> {
    if !scheduler.needs_timer(entries) {
        return Subscription::none();
    }
    clock()
}

/// Ticks every [`TICK`] with the current time, for keeping a calendar's
/// "today" and now line up to date.
pub fn clock() -> Subscription<Timestamp> {
    crate::ticker::every(TICK).map(|_| crate::time::timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::calendar::repeat::Repeat;
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Key {
        Soon,
        Due,
        Over,
        Nag,
        Created,
        Updated,
        Removed,
    }

    const MINUTE: Duration = Duration::from_secs(60);

    /// A time on 3 November 2026, UTC.
    fn at(hour: i8, minute: i8) -> Timestamp {
        on(3, hour, minute)
    }

    fn on(day: i8, hour: i8, minute: i8) -> Timestamp {
        date(2026, 11, day)
            .at(hour, minute, 0, 0)
            .to_zoned_datetime(TimeZone::UTC)
            .map(|zoned| zoned.timestamp())
            .unwrap_or_default()
    }

    /// 09:00 to 10:00.
    fn meeting(id: u64) -> Entry {
        Entry::new(EntryId(id), "Meeting", at(9, 0), at(10, 0))
    }

    fn soon() -> Trigger<Key> {
        Trigger::new(Key::Soon).before(Anchor::Start, 15 * MINUTE)
    }

    fn keys(fired: &[Fired<Key>]) -> Vec<Key> {
        fired.iter().map(|fired| fired.trigger).collect()
    }

    #[test]
    fn a_new_trigger_goes_off_at_the_start_for_every_entry() {
        let trigger = Trigger::new(Key::Due);
        assert!(trigger.is_timed());
        assert_eq!(trigger.key(), &Key::Due);
        assert_eq!(trigger.scope_of(), &Scope::All);
        assert_eq!(trigger, Trigger::new(Key::Due).at(Anchor::Start));
        assert!(!Trigger::new(Key::Created).on(Change::Created).is_timed());
    }

    #[test]
    fn triggers_go_off_once_when_their_time_passes() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new()
            .with(soon())
            .with(Trigger::new(Key::Due).at(Anchor::Start))
            .with(Trigger::new(Key::Over).after(Anchor::End, 5 * MINUTE));

        assert!(scheduler.poll(&entries, at(8, 0)).is_empty());
        assert!(scheduler.poll(&entries, at(8, 44)).is_empty());

        let fired = scheduler.poll(&entries, at(8, 45));
        assert_eq!(keys(&fired), vec![Key::Soon]);
        assert_eq!(
            fired[0].kind,
            Kind::Due {
                start: at(9, 0),
                due: at(8, 45),
                delivery: Delivery::OnTime
            }
        );
        assert_eq!(fired[0].entry, EntryId(1));
        assert!(scheduler.poll(&entries, at(8, 46)).is_empty());

        assert_eq!(keys(&scheduler.poll(&entries, at(9, 0))), vec![Key::Due]);
        assert!(scheduler.poll(&entries, at(10, 4)).is_empty());
        assert_eq!(keys(&scheduler.poll(&entries, at(10, 5))), vec![Key::Over]);
        assert!(scheduler.poll(&entries, at(23, 0)).is_empty());
    }

    #[test]
    fn the_first_poll_reaches_back_by_the_grace_only() {
        let entries = Entries::utc().with([meeting(1)]);
        let trigger = || Trigger::new(Key::Due).at(Anchor::Start);

        let mut just_after = Scheduler::new().with(trigger());
        let fired = just_after.poll(&entries, at(9, 0));
        assert_eq!(keys(&fired), vec![Key::Due]);
        assert!(!fired[0].is_missed());

        let mut long_after = Scheduler::new().with(trigger());
        assert!(long_after.poll(&entries, at(9, 2)).is_empty());
        assert_eq!(long_after.watermark(), Some(at(9, 2)));
    }

    #[test]
    fn triggers_missed_between_polls_arrive_late_and_flagged() {
        let entries = Entries::utc().with([meeting(1), meeting(2)]);
        let mut scheduler = Scheduler::new()
            .with(soon())
            .with(Trigger::new(Key::Due).at(Anchor::Start))
            .resume_from(at(7, 0));

        let fired = scheduler.poll(&entries, at(12, 0));
        assert_eq!(
            keys(&fired),
            vec![Key::Soon, Key::Soon, Key::Due, Key::Due],
            "earliest first"
        );
        assert!(fired.iter().all(Fired::is_missed));
        assert_eq!(fired[0].late_by(), Some(SignedDuration::from_mins(195)));
        assert_eq!(fired[2].late_by(), Some(SignedDuration::from_hours(3)));
        assert_eq!(fired[0].due(), Some(at(8, 45)));
        assert!(scheduler.poll(&entries, at(12, 1)).is_empty());
    }

    #[test]
    fn grace_decides_what_counts_as_missed() {
        let entries = Entries::utc().with([meeting(1)]);
        let build = |grace: Duration| {
            Scheduler::new()
                .with(Trigger::new(Key::Due).at(Anchor::Start))
                .grace(grace)
                .resume_from(at(8, 0))
        };
        let fired = build(MINUTE).poll(&entries, at(9, 1));
        assert!(!fired[0].is_missed(), "exactly the grace is on time");
        let fired = build(MINUTE).poll(&entries, at(9, 2));
        assert_eq!(fired[0].late_by(), Some(SignedDuration::from_mins(2)));
        let fired = build(10 * MINUTE).poll(&entries, at(9, 2));
        assert!(!fired[0].is_missed());
    }

    #[test]
    fn a_clock_set_back_delivers_nothing_twice() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new().with(Trigger::new(Key::Due).at(Anchor::Start));
        assert!(scheduler.poll(&entries, at(8, 58)).is_empty());
        assert_eq!(scheduler.poll(&entries, at(9, 0)).len(), 1);

        assert!(scheduler.poll(&entries, at(8, 30)).is_empty());
        assert_eq!(scheduler.watermark(), Some(at(9, 0)));
        assert!(scheduler.poll(&entries, at(9, 0)).is_empty());
        assert!(scheduler.poll(&entries, at(9, 30)).is_empty());
    }

    #[test]
    fn a_nag_repeats_until_the_anchor() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new().with(
            Trigger::new(Key::Nag)
                .before(Anchor::Start, 15 * MINUTE)
                .every(5 * MINUTE),
        );
        assert!(scheduler.poll(&entries, at(8, 40)).is_empty());

        let mut due = Vec::new();
        for minute in 41..=59 {
            due.extend(
                scheduler
                    .poll(&entries, at(8, minute))
                    .iter()
                    .filter_map(Fired::due),
            );
        }
        assert_eq!(due, vec![at(8, 45), at(8, 50), at(8, 55)]);
        assert!(
            scheduler.poll(&entries, at(9, 0)).is_empty(),
            "stops at the start"
        );
        assert!(scheduler.poll(&entries, at(9, 5)).is_empty());
    }

    #[test]
    fn repeats_that_pile_up_arrive_as_the_latest_one() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new()
            .with(
                Trigger::new(Key::Nag)
                    .before(Anchor::Start, 15 * MINUTE)
                    .every(5 * MINUTE),
            )
            .resume_from(at(8, 0));

        let fired = scheduler.poll(&entries, at(8, 52));
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].due(), Some(at(8, 50)));

        let mut asleep = Scheduler::new()
            .with(
                Trigger::new(Key::Nag)
                    .before(Anchor::Start, 15 * MINUTE)
                    .every(5 * MINUTE),
            )
            .resume_from(at(8, 0));
        let fired = asleep.poll(&entries, at(11, 0));
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].due(), Some(at(8, 55)), "the last before the start");
        assert!(fired[0].is_missed());
    }

    #[test]
    fn every_only_repeats_a_trigger_set_before_its_anchor() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new()
            .with(Trigger::new(Key::Due).at(Anchor::Start).every(MINUTE))
            .with(
                Trigger::new(Key::Nag)
                    .before(Anchor::Start, MINUTE)
                    .every(Duration::ZERO),
            );
        assert!(scheduler.poll(&entries, at(8, 58)).is_empty());
        let mut fired = Vec::new();
        for minute in 0..10 {
            fired.extend(scheduler.poll(&entries, at(9, minute)));
        }
        assert_eq!(keys(&fired), vec![Key::Nag, Key::Due]);
    }

    #[test]
    fn scopes_limit_a_trigger_to_some_entries() {
        let entries =
            Entries::utc().with([meeting(1).tag("work"), meeting(2).tag("home"), meeting(3)]);
        let fired_for = |scope: Scope| {
            let mut scheduler = Scheduler::new()
                .with(Trigger::new(Key::Due).scope(scope))
                .resume_from(at(8, 0));
            let fired = scheduler.poll(&entries, at(9, 0));
            fired.iter().map(|fired| fired.entry.0).collect::<Vec<_>>()
        };
        assert_eq!(fired_for(Scope::All), vec![1, 2, 3]);
        assert_eq!(fired_for(Scope::tagged("work")), vec![1]);
        assert_eq!(fired_for(Scope::Entry(EntryId(3))), vec![3]);
        assert!(fired_for(Scope::tagged("none")).is_empty());
    }

    #[test]
    fn change_triggers_are_observed_not_polled() {
        let mut entries = Entries::utc();
        let mut scheduler = Scheduler::new()
            .with(Trigger::new(Key::Created).on(Change::Created))
            .with(Trigger::new(Key::Updated).on(Change::Updated))
            .with(
                Trigger::new(Key::Removed)
                    .on(Change::Removed)
                    .scope(Scope::tagged("work")),
            );

        let Some(created) = entries.insert(meeting(1)) else {
            panic!("created");
        };
        let fired = scheduler.observe(&created);
        assert_eq!(keys(&fired), vec![Key::Created]);
        assert_eq!(fired[0].kind, Kind::Changed(Change::Created));
        assert_eq!(fired[0].due(), None);
        assert!(!fired[0].is_missed());

        let Some(updated) = entries.insert(meeting(1).tag("work")) else {
            panic!("updated");
        };
        assert_eq!(keys(&scheduler.observe(&updated)), vec![Key::Updated]);

        let Some(removed) = entries.remove(EntryId(1)) else {
            panic!("removed");
        };
        assert_eq!(keys(&scheduler.observe(&removed)), vec![Key::Removed]);

        let _ = entries.insert(meeting(2));
        let Some(untagged) = entries.remove(EntryId(2)) else {
            panic!("removed");
        };
        assert!(scheduler.observe(&untagged).is_empty(), "out of scope");

        assert!(scheduler.poll(&entries, at(8, 0)).is_empty());
        assert!(scheduler.poll(&entries, at(12, 0)).is_empty());
        assert!(!scheduler.is_pending(&entries));
    }

    #[test]
    fn entries_changed_between_polls_use_their_new_times() {
        let mut entries = Entries::utc().with([meeting(1), meeting(2)]);
        let mut scheduler = Scheduler::new().with(Trigger::new(Key::Due).at(Anchor::Start));
        assert!(scheduler.poll(&entries, at(8, 0)).is_empty());

        // One moves to 11:00, the other is removed.
        let _ = entries.insert(Entry::new(EntryId(1), "Meeting", at(11, 0), at(12, 0)));
        let _ = entries.remove(EntryId(2));
        assert!(scheduler.poll(&entries, at(9, 0)).is_empty());
        assert_eq!(scheduler.poll(&entries, at(11, 0)).len(), 1);

        // Moved again after it went off, so it goes off again.
        let _ = entries.insert(Entry::new(EntryId(1), "Meeting", at(13, 0), at(14, 0)));
        assert_eq!(scheduler.poll(&entries, at(13, 0)).len(), 1);
    }

    #[test]
    fn repeating_entries_go_off_for_each_occurrence() {
        let entries = Entries::utc().with([meeting(1).repeat(Repeat::daily().count(3))]);
        let mut scheduler = Scheduler::new()
            .with(Trigger::new(Key::Due).at(Anchor::Start))
            .resume_from(at(8, 0));
        let mut starts = Vec::new();
        for day in 3..=7 {
            for fired in scheduler.poll(&entries, on(day, 9, 0)) {
                assert!(!fired.is_missed());
                if let Kind::Due { start, .. } = fired.kind {
                    starts.push(start);
                }
            }
        }
        assert_eq!(starts, vec![on(3, 9, 0), on(4, 9, 0), on(5, 9, 0)]);
    }

    #[test]
    fn missed_occurrences_of_one_entry_arrive_as_the_latest() {
        let entries = Entries::utc().with([
            meeting(1).repeat(Repeat::daily()),
            // A second entry keeps its own reminder.
            meeting(2),
        ]);
        let mut scheduler = Scheduler::new()
            .with(Trigger::new(Key::Soon).before(Anchor::Start, 15 * MINUTE))
            .with(Trigger::new(Key::Due).at(Anchor::Start))
            .resume_from(at(8, 0));

        // A week away: seven stand-ups were missed, for two triggers.
        let fired = scheduler.poll(&entries, on(9, 12, 0));
        let got: Vec<_> = fired
            .iter()
            .map(|fired| (fired.trigger, fired.entry.0, fired.due()))
            .collect();
        assert_eq!(
            got,
            vec![
                (Key::Soon, 2, Some(on(3, 8, 45))),
                (Key::Due, 2, Some(on(3, 9, 0))),
                (Key::Soon, 1, Some(on(9, 8, 45))),
                (Key::Due, 1, Some(on(9, 9, 0))),
            ]
        );
        assert!(fired.iter().all(Fired::is_missed));

        // Back on time, each occurrence is delivered as usual.
        assert_eq!(
            keys(&scheduler.poll(&entries, on(10, 8, 45))),
            vec![Key::Soon]
        );
        assert_eq!(
            keys(&scheduler.poll(&entries, on(10, 9, 0))),
            vec![Key::Due]
        );
    }

    #[test]
    fn an_occurrence_due_now_is_not_swallowed_by_older_misses() {
        let entries = Entries::utc().with([meeting(1).repeat(Repeat::daily())]);
        let mut scheduler = Scheduler::new()
            .with(Trigger::new(Key::Due).at(Anchor::Start))
            .resume_from(at(8, 0));
        let fired = scheduler.poll(&entries, on(6, 9, 0));
        let got: Vec<_> = fired
            .iter()
            .map(|fired| (fired.due(), fired.is_missed()))
            .collect();
        assert_eq!(
            got,
            vec![(Some(on(5, 9, 0)), true), (Some(on(6, 9, 0)), false)],
            "the latest miss, and today's on time"
        );
    }

    #[test]
    fn a_watermark_from_a_wrong_clock_is_dropped() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new().with(Trigger::new(Key::Due).at(Anchor::Start));
        assert!(scheduler.poll(&entries, at(8, 0)).is_empty());

        // The clock jumps a month ahead for one poll, then comes back.
        let glitch = on(30, 8, 0);
        let fired = scheduler.poll(&entries, glitch);
        assert_eq!(fired.len(), 1, "what the wrong clock said was missed");
        assert!(scheduler.poll(&entries, at(8, 30)).is_empty());
        assert_eq!(scheduler.watermark(), Some(at(8, 30)), "starts again");

        // The scheduler works again from the real time.
        let later = Entries::utc().with([Entry::new(EntryId(2), "Later", at(10, 0), at(11, 0))]);
        assert_eq!(scheduler.poll(&later, at(10, 0)).len(), 1);
    }

    #[test]
    fn the_timer_runs_while_a_timed_trigger_has_entries() {
        let entries = Entries::utc().with([meeting(1)]);
        let none: Scheduler<Key> = Scheduler::new();
        assert!(!none.needs_timer(&entries));
        let changes = Scheduler::new().with(Trigger::new(Key::Created).on(Change::Created));
        assert!(!changes.needs_timer(&entries));

        let mut timed = Scheduler::new().with(Trigger::new(Key::Due).at(Anchor::Start));
        assert!(!timed.needs_timer(&Entries::utc()));
        assert!(timed.needs_timer(&entries));
        // It keeps running after the last entry has passed, so an entry
        // added later is judged against a current watermark.
        let fired = timed.poll(&entries, at(9, 0));
        assert!(!timed.is_pending(&entries));
        assert!(timed.needs_timer(&entries));

        let mut snoozing = Scheduler::new().with(Trigger::new(Key::Created).on(Change::Created));
        assert!(snoozing.snooze(&fired[0], at(9, 10)));
        assert!(snoozing.needs_timer(&Entries::utc()));
    }

    #[test]
    fn an_entry_added_after_the_last_poll_is_not_reported_as_missed() {
        let mut entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new().with(Trigger::new(Key::Due).at(Anchor::Start));
        // The timer keeps polling through a quiet afternoon.
        for hour in 8..=14 {
            let _ = scheduler.poll(&entries, at(hour, 30));
        }
        // At 14:40 an entry is added for 14:00, already past.
        let _ = entries.insert(Entry::new(EntryId(2), "Late", at(14, 0), at(15, 0)));
        assert!(scheduler.poll(&entries, at(14, 40)).is_empty());
    }

    #[test]
    fn snoozing_the_same_thing_again_replaces_the_snooze() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new().with(soon()).resume_from(at(8, 0));
        let fired = scheduler.poll(&entries, at(8, 45));
        assert!(scheduler.snooze(&fired[0], at(8, 50)));
        assert!(scheduler.snooze(&fired[0], at(8, 55)));
        assert_eq!(scheduler.snoozed().len(), 1);
        assert!(scheduler.poll(&entries, at(8, 50)).is_empty());
        assert_eq!(scheduler.poll(&entries, at(8, 55)).len(), 1);
    }

    #[test]
    fn whole_day_entries_are_due_at_midnight() {
        let day = date(2026, 11, 4);
        let entries = Entries::utc().with([Entry::all_day(EntryId(1), "Holiday", day, day)]);
        let mut scheduler = Scheduler::new()
            .with(Trigger::new(Key::Soon).before(Anchor::Start, 60 * MINUTE))
            .with(Trigger::new(Key::Over).at(Anchor::End))
            .resume_from(at(12, 0));
        assert_eq!(keys(&scheduler.poll(&entries, at(23, 0))), vec![Key::Soon]);
        assert!(scheduler.poll(&entries, on(4, 23, 59)).is_empty());
        assert_eq!(
            keys(&scheduler.poll(&entries, on(5, 0, 0))),
            vec![Key::Over]
        );
    }

    #[test]
    fn a_snoozed_trigger_comes_back_once() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new().with(soon()).resume_from(at(8, 0));
        let fired = scheduler.poll(&entries, at(8, 45));
        assert!(scheduler.snooze(&fired[0], at(8, 50)));
        assert_eq!(scheduler.snoozed().len(), 1);

        assert!(scheduler.poll(&entries, at(8, 49)).is_empty());
        let again = scheduler.poll(&entries, at(8, 50));
        assert_eq!(keys(&again), vec![Key::Soon]);
        assert_eq!(
            again[0].kind,
            Kind::Due {
                start: at(9, 0),
                due: at(8, 50),
                delivery: Delivery::OnTime
            }
        );
        assert!(scheduler.snoozed().is_empty());
        assert!(scheduler.poll(&entries, at(8, 51)).is_empty());
    }

    #[test]
    fn a_snooze_can_be_restored_and_arrives_late_if_it_ran_out() {
        let entries = Entries::utc().with([meeting(1)]);
        let snooze = Snooze {
            trigger: Key::Soon,
            entry: EntryId(1),
            start: at(9, 0),
            until: at(8, 50),
        };
        let mut scheduler = Scheduler::new()
            .with(soon())
            .with_snooze(snooze)
            .resume_from(at(8, 46));
        let fired = scheduler.poll(&entries, at(9, 30));
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].late_by(), Some(SignedDuration::from_mins(40)));
    }

    #[test]
    fn a_snooze_for_a_removed_entry_is_dropped() {
        let mut entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new().with(soon()).resume_from(at(8, 0));
        let fired = scheduler.poll(&entries, at(8, 45));
        assert!(scheduler.snooze(&fired[0], at(8, 50)));
        let _ = entries.remove(EntryId(1));
        assert!(scheduler.poll(&entries, at(8, 50)).is_empty());
        assert!(scheduler.snoozed().is_empty());
    }

    #[test]
    fn a_change_cannot_be_snoozed() {
        let mut entries = Entries::utc();
        let mut scheduler = Scheduler::new().with(Trigger::new(Key::Created).on(Change::Created));
        let Some(created) = entries.insert(meeting(1)) else {
            panic!("created");
        };
        let fired = scheduler.observe(&created);
        assert!(!scheduler.snooze(&fired[0], at(9, 0)));
        assert!(scheduler.snoozed().is_empty());
    }

    #[test]
    fn next_due_finds_the_nearest_time() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new()
            .with(soon())
            .with(Trigger::new(Key::Due).at(Anchor::Start));
        assert_eq!(scheduler.next_due(&entries, at(8, 0)), Some(at(8, 45)));
        assert_eq!(scheduler.next_due(&entries, at(8, 45)), Some(at(9, 0)));
        assert_eq!(scheduler.next_due(&entries, at(9, 0)), None);
        assert_eq!(
            scheduler.next_due(&entries, on(1, 0, 0)),
            Some(at(8, 45)),
            "beyond the first horizon"
        );

        let fired = Fired {
            trigger: Key::Soon,
            entry: EntryId(1),
            kind: Kind::Due {
                start: at(9, 0),
                due: at(8, 45),
                delivery: Delivery::OnTime,
            },
        };
        assert!(scheduler.snooze(&fired, at(8, 50)));
        assert_eq!(scheduler.next_due(&entries, at(8, 46)), Some(at(8, 50)));
    }

    #[test]
    fn next_due_follows_a_nag_and_a_repeating_entry() {
        let entries = Entries::utc().with([meeting(1)]);
        let nag = Scheduler::new().with(
            Trigger::new(Key::Nag)
                .before(Anchor::Start, 15 * MINUTE)
                .every(5 * MINUTE),
        );
        assert_eq!(nag.next_due(&entries, at(8, 45)), Some(at(8, 50)));
        assert_eq!(nag.next_due(&entries, at(8, 52)), Some(at(8, 55)));
        assert_eq!(nag.next_due(&entries, at(8, 55)), None);

        let daily = Entries::utc().with([meeting(1).repeat(Repeat::daily())]);
        let due = Scheduler::new().with(Trigger::new(Key::Due).at(Anchor::Start));
        assert_eq!(due.next_due(&daily, at(9, 0)), Some(on(4, 9, 0)));
    }

    #[test]
    fn pending_only_while_there_is_something_to_wait_for() {
        let entries = Entries::utc().with([meeting(1)]);
        let empty: Scheduler<Key> = Scheduler::new();
        assert!(!empty.is_pending(&entries));

        let mut scheduler = Scheduler::new().with(Trigger::new(Key::Due).at(Anchor::Start));
        assert!(!scheduler.is_pending(&Entries::utc()));
        assert!(scheduler.is_pending(&entries), "not polled yet");
        let _ = scheduler.poll(&entries, at(8, 0));
        assert!(scheduler.is_pending(&entries));
        let fired = scheduler.poll(&entries, at(9, 0));
        assert!(!scheduler.is_pending(&entries), "nothing left");
        assert!(scheduler.snooze(&fired[0], at(9, 10)));
        assert!(scheduler.is_pending(&entries));
    }

    #[test]
    fn triggers_can_be_added_and_removed_later() {
        let entries = Entries::utc().with([meeting(1)]);
        let mut scheduler = Scheduler::new().with(soon()).resume_from(at(8, 0));
        scheduler.add(Trigger::new(Key::Due).at(Anchor::Start));
        assert_eq!(scheduler.triggers().len(), 2);
        let fired = scheduler.poll(&entries, at(8, 45));
        assert!(scheduler.snooze(&fired[0], at(8, 50)));

        scheduler.remove(&Key::Soon);
        assert_eq!(scheduler.triggers().len(), 1);
        assert!(scheduler.snoozed().is_empty());
        assert_eq!(keys(&scheduler.poll(&entries, at(9, 0))), vec![Key::Due]);
    }
}
