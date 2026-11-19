use std::time::Duration;

use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length};
use iced_cube::button;
use iced_cube::data::calendar::schedule::{Anchor, Fired, Scheduler, Scope, Trigger};
use iced_cube::data::calendar::{
    self, Change, Entries, Entry, EntryId, Output, Tone, View, calendar, instant, time_label,
};
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::theme::{Tokens, text_size};
use jiff::civil::{date, time};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

/// One variant per rule. The scheduler hands these back when a rule goes
/// off, and the app decides what each one means.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Rule {
    Created,
    Moved,
    Removed,
    /// Ten minutes before anything tagged "work".
    WorkSoon,
    /// Every five minutes in the quarter of an hour before the release.
    ReleaseNag,
    Over,
}

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
    Wait,
}

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
    scheduler: Scheduler<Rule>,
    now: Timestamp,
    log: Vec<String>,
}

const RELEASE: EntryId = EntryId(2);
const MINUTE: Duration = Duration::from_secs(60);

/// A time on Thursday 18 June 2026.
fn at(hour: i8, minute: i8) -> Timestamp {
    instant(date(2026, 6, 18), time(hour, minute, 0, 0), &TimeZone::UTC)
}

impl Default for Example {
    fn default() -> Self {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Stand-up", at(10, 0), at(10, 15)).tag("work"),
            Entry::new(RELEASE, "Release", at(10, 45), at(11, 15)).tone(Tone::Destructive),
            Entry::new(EntryId(3), "Lunch with Ada", at(12, 0), at(13, 0)).tone(Tone::Neutral),
        ]);
        let now = at(9, 40);

        let scheduler = Scheduler::new()
            // Changes to entries, reported as they happen.
            .with(Trigger::new(Rule::Created).on(Change::Created))
            .with(Trigger::new(Rule::Moved).on(Change::Updated))
            .with(Trigger::new(Rule::Removed).on(Change::Removed))
            // Moments measured from where an entry starts or ends.
            .with(
                Trigger::new(Rule::WorkSoon)
                    .before(Anchor::Start, 10 * MINUTE)
                    .scope(Scope::tagged("work")),
            )
            .with(
                Trigger::new(Rule::ReleaseNag)
                    .before(Anchor::Start, 15 * MINUTE)
                    .every(5 * MINUTE)
                    .scope(Scope::Entry(RELEASE)),
            )
            .with(Trigger::new(Rule::Over).at(Anchor::End))
            .resume_from(now);

        Self {
            calendar: calendar::State::at(now, entries).with_view(View::Day),
            scheduler,
            now,
            log: vec![String::from(
                "Press a time twice to add an entry, or an entry twice to remove it.",
            )],
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Calendar(event) => self.apply(event),
            Message::Wait => {
                let wait = SignedDuration::from_mins(5);
                self.now = self.now.checked_add(wait).unwrap_or(self.now);
                let _ = self.calendar.update(calendar::Event::Tick(self.now));
                for fired in self.scheduler.poll(self.calendar.entries(), self.now) {
                    let title = self.calendar.entries().get(fired.entry).map(Entry::title);
                    self.record(&fired, title.unwrap_or_default().to_owned());
                }
            }
        }
    }

    fn apply(&mut self, event: calendar::Event) {
        match self.calendar.update(event) {
            Some(Output::Requested(draft)) => {
                let id = self.calendar.entries().next_id();
                let entry = Entry::at(id, "New entry", draft.when).tag("work");
                self.apply(calendar::Event::Insert(entry));
            }
            Some(Output::Opened { entry, .. }) => self.apply(calendar::Event::Remove(entry)),
            // Created, moved by a drag, or removed.
            Some(Output::Changed(changed)) => {
                for fired in self.scheduler.observe(&changed) {
                    self.record(&fired, changed.entry.title().to_owned());
                }
            }
            _ => {}
        }
    }

    fn record(&mut self, fired: &Fired<Rule>, title: String) {
        let what = match fired.trigger {
            Rule::Created => "was created",
            Rule::Moved => "was moved",
            Rule::Removed => "was removed",
            Rule::WorkSoon => "starts in 10 minutes",
            Rule::ReleaseNag => "is coming up",
            Rule::Over => "is over",
        };
        let clock = time_label(self.now, self.calendar.time_zone());
        self.log.push(format!("{clock}  {title} {what}"));
    }

    pub fn view(&self) -> Element<'_, Message> {
        let clock = time_label(self.now, self.calendar.time_zone());
        let controls = row![
            text(format!("It is {clock}.")).size(14),
            button("Wait 5 minutes")
                .variant(Variant::Outline)
                .size(Size::Sm)
                .on_press(Message::Wait),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        // The newest lines, with the latest at the bottom.
        let shown = self.log.len().saturating_sub(3);
        let log = column(self.log[shown..].iter().map(|line| {
            text(line.as_str())
                .size(text_size::SM)
                .style(|theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                })
                .into()
        }))
        .spacing(4)
        .height(Length::Fixed(68.0));

        column![
            controls,
            calendar(&self.calendar)
                .header(false)
                .hours(9..14)
                .hour_height(40.0)
                .height(Length::Fill)
                .on_event(Message::Calendar),
            log,
        ]
        .spacing(12)
        .into()
    }
}
