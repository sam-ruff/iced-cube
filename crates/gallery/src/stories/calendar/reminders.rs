use std::time::Duration;

use futures_timer::Delay;
use iced::futures::{Stream, stream};
use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length, Subscription};
use iced_cube::button;
use iced_cube::data::calendar::schedule::{Anchor, Scheduler, Trigger};
use iced_cube::data::calendar::{
    self, Entries, Entry, EntryId, Tone, View, calendar, instant, time_label,
};
use iced_cube::overlay::toast::{self, Variant, toast, toasts};
use iced_cube::primitives::button::Variant as ButtonVariant;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

/// The app's own names for its triggers.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Reminder {
    Soon,
    Starting,
}

#[derive(Debug, Clone)]
pub enum Message {
    Calendar(calendar::Event),
    Toast(toast::Event),
    /// A minute passes on the story's clock.
    Minute,
    Pause(bool),
    Restart,
}

#[derive(Debug)]
pub struct Example {
    calendar: calendar::State,
    scheduler: Scheduler<Reminder>,
    toasts: toast::State,
    now: Timestamp,
    paused: bool,
}

/// A time on Thursday 18 June 2026.
fn at(hour: i8, minute: i8) -> Timestamp {
    instant(date(2026, 6, 18), time(hour, minute, 0, 0), &TimeZone::UTC)
}

impl Default for Example {
    fn default() -> Self {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Stand-up", at(10, 0), at(10, 15)),
            Entry::new(EntryId(2), "Design review", at(10, 30), at(11, 30)).tone(Tone::Success),
            Entry::new(EntryId(3), "Lunch with Ada", at(12, 0), at(13, 0)).tone(Tone::Neutral),
        ]);
        let now = at(9, 40);

        Self {
            calendar: calendar::State::at(now, entries).with_view(View::Day),
            scheduler: Scheduler::new()
                .with(Trigger::new(Reminder::Soon).before(Anchor::Start, Duration::from_secs(600)))
                .with(Trigger::new(Reminder::Starting).at(Anchor::Start)),
            toasts: toast::State::new(),
            now,
            paused: false,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Calendar(event) => {
                let _ = self.calendar.update(event);
            }
            Message::Toast(event) => {
                let _ = self.toasts.update(event);
            }
            Message::Minute => {
                let minute = SignedDuration::from_mins(1);
                self.now = self.now.checked_add(minute).unwrap_or(self.now);
                let _ = self.calendar.update(calendar::Event::Tick(self.now));

                // Everything that came due since the last poll, once each.
                for fired in self.scheduler.poll(self.calendar.entries(), self.now) {
                    let Some(entry) = self.calendar.entries().get(fired.entry) else {
                        continue;
                    };
                    let title = entry.title();
                    self.toasts.push(match fired.trigger {
                        Reminder::Soon => toast(format!("{title} in 10 minutes")),
                        Reminder::Starting => {
                            toast(format!("{title} is starting")).variant(Variant::Success)
                        }
                    });
                }
            }
            Message::Pause(paused) => self.paused = paused,
            Message::Restart => *self = Self::default(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let clock = time_label(self.now, self.calendar.time_zone());
        let pause = if self.paused { "Resume" } else { "Pause" };
        let controls = row![
            text(format!(
                "It is {clock}. A minute passes four times a second."
            ))
            .size(14),
            button(pause)
                .variant(ButtonVariant::Outline)
                .on_press(Message::Pause(!self.paused)),
            button("Restart")
                .variant(ButtonVariant::Outline)
                .on_press(Message::Restart),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .wrap();

        let content = column![
            controls,
            calendar(&self.calendar)
                .header(false)
                .hours(9..14)
                .height(Length::Fill)
                .on_event(Message::Calendar),
        ]
        .spacing(12);

        toasts(&self.toasts, content)
            .on_event(Message::Toast)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // A real app uses `schedule::timer`, which ticks every 30 seconds
        // with the real time while a trigger is pending.
        let clock = if self.paused || self.now >= at(13, 0) {
            Subscription::none()
        } else {
            Subscription::run(minutes)
        };
        Subscription::batch([clock, toast::timer(&self.toasts).map(Message::Toast)])
    }
}

fn minutes() -> impl Stream<Item = Message> {
    stream::unfold((), |()| async {
        Delay::new(Duration::from_millis(250)).await;
        Some((Message::Minute, ()))
    })
}
