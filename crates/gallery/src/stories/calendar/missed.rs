use iced::widget::{column, row, text};
use iced::{Alignment, Element, Length};
use iced_cube::data::calendar::schedule::{Anchor, Fired, Scheduler, Trigger};
use iced_cube::data::calendar::{Entries, Entry, EntryId, instant, when_label};
use iced_cube::feedback::badge::Variant as BadgeVariant;
use iced_cube::primitives::button::{Size, Variant};
use iced_cube::theme::{Tokens, semibold, text_size};
use iced_cube::{badge, button, card, scroll_area};
use jiff::civil::{date, time};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Reminder {
    Starting,
}

#[derive(Debug, Clone)]
pub enum Message {
    Snooze(usize),
    Dismiss(usize),
    Wait,
}

#[derive(Debug)]
pub struct Example {
    entries: Entries,
    scheduler: Scheduler<Reminder>,
    now: Timestamp,
    /// The reminders on screen, oldest first.
    reminders: Vec<Fired<Reminder>>,
}

/// A time in June 2026.
fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
    instant(date(2026, 6, day), time(hour, minute, 0, 0), &TimeZone::UTC)
}

impl Default for Example {
    fn default() -> Self {
        let entries = Entries::utc().with([
            Entry::new(EntryId(1), "Retro", at(17, 16, 0), at(17, 17, 0)),
            Entry::new(EntryId(2), "Dentist", at(18, 8, 30), at(18, 9, 0)),
            Entry::new(EntryId(3), "Stand-up", at(18, 9, 0), at(18, 9, 15)),
            Entry::new(EntryId(4), "Design review", at(18, 9, 40), at(18, 10, 40)),
        ]);
        // The app was last open on Wednesday afternoon. The watermark it
        // saved then is all the scheduler needs to catch up.
        let mut scheduler = Scheduler::new()
            .with(Trigger::new(Reminder::Starting).at(Anchor::Start))
            .resume_from(at(17, 15, 0));
        let now = at(18, 9, 30);
        let reminders = scheduler.poll(&entries, now);

        Self {
            entries,
            scheduler,
            now,
            reminders,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Snooze(index) => {
                if index >= self.reminders.len() {
                    return;
                }
                let reminder = self.reminders.remove(index);
                let later = SignedDuration::from_mins(10);
                let until = self.now.checked_add(later).unwrap_or(self.now);
                let _ = self.scheduler.snooze(&reminder, until);
            }
            Message::Dismiss(index) => {
                if index < self.reminders.len() {
                    let _ = self.reminders.remove(index);
                }
            }
            Message::Wait => {
                let wait = SignedDuration::from_mins(5);
                self.now = self.now.checked_add(wait).unwrap_or(self.now);
                let due = self.scheduler.poll(&self.entries, self.now);
                self.reminders.extend(due);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let time_zone = self.entries.time_zone();
        let now = self.now.to_zoned_datetime(time_zone.clone());
        let heading = row![
            text(format!("It is {}.", now.strftime("%A %H:%M"))).size(14),
            button("Wait 5 minutes")
                .variant(Variant::Outline)
                .size(Size::Sm)
                .on_press(Message::Wait),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .wrap();

        let mut list = column![].spacing(8);
        for (index, reminder) in self.reminders.iter().enumerate() {
            let Some(entry) = self.entries.get(reminder.entry) else {
                continue;
            };
            let status = match reminder.late_by() {
                Some(late) => badge(format!("Missed, {} late", lateness(late)))
                    .variant(BadgeVariant::Destructive),
                None => badge("On time").variant(BadgeVariant::Secondary),
            };
            let when = text(when_label(&entry.when(), time_zone))
                .size(text_size::SM)
                .style(|theme| text::Style {
                    color: Some(Tokens::of(theme).muted_foreground),
                });
            let actions = row![
                button("Snooze 10 minutes")
                    .variant(Variant::Outline)
                    .size(Size::Sm)
                    .on_press(Message::Snooze(index)),
                button("Dismiss")
                    .variant(Variant::Ghost)
                    .size(Size::Sm)
                    .on_press(Message::Dismiss(index)),
            ]
            .spacing(8)
            .wrap();
            let title = row![
                text(entry.title().to_owned())
                    .size(text_size::SM)
                    .font(semibold()),
                status,
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .wrap();
            list = list.push(
                card()
                    .body(column![title, when, actions].spacing(8))
                    .width(Length::Fill),
            );
        }
        if self.reminders.is_empty() {
            list = list.push(text("No reminders. Wait for a snoozed one to come back.").size(14));
        }

        column![heading, scroll_area(list).height(Length::Fill)]
            .spacing(12)
            .width(Length::Fixed(420.0))
            .into()
    }
}

/// How late a reminder is, in words.
fn lateness(late: SignedDuration) -> String {
    let (hours, minutes) = (late.as_hours(), late.as_mins() % 60);
    match (hours, minutes) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}
