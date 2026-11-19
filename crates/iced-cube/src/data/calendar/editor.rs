//! A form for creating or changing an entry: its title, whether it lasts
//! all day, when it starts and ends, how it repeats and its colour.
//!
//! The form has no buttons of its own. Put it in a dialog, a popover or a
//! side panel with whatever actions the app wants, start it from the
//! [`Draft`] the calendar asked for or from an existing [`Entry`], and
//! turn it back into an entry with [`State::entry`]:
//!
//! ```
//! use iced_cube::data::calendar::editor;
//! use iced_cube::data::calendar::{Draft, EntryId, When, instant};
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
//! let entry = form.entry(EntryId(7));
//! assert_eq!(entry.title(), "Retrospective");
//! assert_eq!(entry.when(), When::timed(at(12), at(13)));
//! ```

use iced::widget::button::Status;
use iced::widget::{self, Column, Row, Space, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};
use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};

use super::entry::{Draft, Entry, EntryId, Tone, When, add_days, date_of};
use super::layout::{DAY_MINUTES, instant_at, minute_on, month_weeks};
use super::parts::weekday_label;
use super::repeat::{Frequency, Repeat};
use super::style;
use crate::forms::select::select;
use crate::lucide;
use crate::overlay::anchored::Align;
use crate::overlay::popover::popover;
use crate::primitives::button::{self, Size, button};
use crate::primitives::icon_button::{self, icon_button};
use crate::primitives::input::{Shortcuts, input};
use crate::primitives::switch::switch;
use crate::theme::{self, Tokens, space, text_size};

/// The step between the times offered, in minutes.
pub const TIME_STEP: u16 = 15;
/// The title given to an entry saved without one.
pub const UNTITLED: &str = "(No title)";
/// How long a new timed entry is when the draft was for whole days.
pub const DEFAULT_MINUTES: u16 = 60;

/// The side of a day in the date picker.
const CELL: f32 = 32.0;
/// The width of a time's menu.
const TIME_WIDTH: f32 = 96.0;

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
}

impl std::fmt::Display for TimeOfDay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.0 / 60, self.0 % 60)
    }
}

/// How the entry repeats, as the form offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Repeats {
    #[default]
    Never,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    /// A rule the form does not offer, such as every two weeks, kept as it
    /// was when the entry is saved.
    Custom(Repeat),
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

    /// The choice that stands for `repeat`.
    pub fn of(repeat: Option<Repeat>) -> Self {
        let Some(repeat) = repeat else {
            return Repeats::Never;
        };
        if repeat != Repeat::new(repeat.frequency()) {
            return Repeats::Custom(repeat);
        }
        match repeat.frequency() {
            Frequency::Daily => Repeats::Daily,
            Frequency::Weekly => Repeats::Weekly,
            Frequency::Monthly => Repeats::Monthly,
            Frequency::Yearly => Repeats::Yearly,
        }
    }

    /// The rule to save.
    pub fn rule(self) -> Option<Repeat> {
        match self {
            Repeats::Never => None,
            Repeats::Daily => Some(Repeat::daily()),
            Repeats::Weekly => Some(Repeat::weekly()),
            Repeats::Monthly => Some(Repeat::monthly()),
            Repeats::Yearly => Some(Repeat::yearly()),
            Repeats::Custom(repeat) => Some(repeat),
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
            Repeats::Custom(_) => "Custom",
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Title(String),
    AllDay(bool),
    /// Sets the start or end date, and closes the date picker.
    Date(Field, Date),
    /// Sets the start or end time, in minutes from midnight.
    Time(Field, u16),
    Repeats(Repeats),
    Tone(Tone),
    /// Opens the date picker for a field, or closes it with `None`.
    Picker(Option<Field>),
    /// Moves the date picker by this many months.
    PickerMonth(i32),
}

/// A date picker that is open: whose it is, and the month it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Picker {
    field: Field,
    month: Date,
}

/// What the form holds. The end is kept at or after the start: moving the
/// start moves the end with it, so the entry keeps its length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    id: Option<EntryId>,
    title: String,
    all_day: bool,
    start: (Date, u16),
    end: (Date, u16),
    repeats: Repeats,
    tone: Tone,
    tag: Option<String>,
    time_zone: TimeZone,
    week_start: Weekday,
    picker: Option<Picker>,
}

impl State {
    /// A form for a new entry over the span the calendar asked for.
    pub fn new(draft: &Draft, time_zone: &TimeZone) -> Self {
        Self::over(draft.when, time_zone)
    }

    /// A form for changing `entry`.
    pub fn edit(entry: &Entry, time_zone: &TimeZone) -> Self {
        Self {
            id: Some(entry.id()),
            title: entry.title().to_owned(),
            repeats: Repeats::of(entry.repeat_of()),
            tone: entry.tone_of(),
            tag: entry.tag_of().map(str::to_owned),
            ..Self::over(entry.when(), time_zone)
        }
    }

    fn over(when: When, time_zone: &TimeZone) -> Self {
        let nine = 9 * 60;
        let (all_day, start, end) = match when {
            When::AllDay { first, last } => (true, (first, nine), (last, nine + DEFAULT_MINUTES)),
            When::Timed { start, end } => (
                false,
                local(start, time_zone),
                local(end.max(start), time_zone),
            ),
        };
        Self {
            id: None,
            title: String::new(),
            all_day,
            start,
            end,
            repeats: Repeats::Never,
            tone: Tone::default(),
            tag: None,
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

    pub fn tone(&self) -> Tone {
        self.tone
    }

    /// The date of a field.
    pub fn date(&self, field: Field) -> Date {
        self.at(field).0
    }

    /// The time of a field, in minutes from midnight.
    pub fn time(&self, field: Field) -> u16 {
        self.at(field).1
    }

    /// The times a field's menu lists. The start's begins an hour before
    /// the time chosen; the end's begins just after the start when both
    /// are on the same day, since nothing earlier would be on that day.
    pub fn times(&self, field: Field) -> Vec<TimeOfDay> {
        let first = match field {
            Field::End if self.end.0 == self.start.0 => self.start.1 + TIME_STEP,
            _ => self.time(field).saturating_sub(60),
        };
        TimeOfDay::from(first % DAY_MINUTES)
    }

    /// The field whose date picker is open.
    pub fn picker(&self) -> Option<Field> {
        self.picker.map(|picker| picker.field)
    }

    /// The span the form describes.
    pub fn when(&self) -> When {
        if self.all_day {
            return When::all_day(self.start.0, self.end.0);
        }
        When::timed(
            instant_at(self.start.0, self.start.1, &self.time_zone),
            instant_at(self.end.0, self.end.1, &self.time_zone),
        )
    }

    /// The entry the form describes. A new entry takes `new_id`; a changed
    /// one keeps its own. A blank title becomes [`UNTITLED`].
    pub fn entry(&self, new_id: EntryId) -> Entry {
        let title = match self.title.trim() {
            "" => UNTITLED,
            title => title,
        };
        let mut entry = Entry::at(self.id.unwrap_or(new_id), title, self.when()).tone(self.tone);
        if let Some(tag) = &self.tag {
            entry = entry.tag(tag.clone());
        }
        if let Some(rule) = self.repeats.rule() {
            entry = entry.repeat(rule);
        }
        entry
    }

    pub fn update(&mut self, event: Event) {
        match event {
            Event::Title(title) => self.title = title,
            Event::AllDay(all_day) => {
                self.all_day = all_day;
                self.settle(Field::End);
            }
            Event::Date(field, date) => {
                self.picker = None;
                self.set(field, (date, self.time(field)));
            }
            Event::Time(field, minute) => {
                self.set(field, (self.date(field), minute.min(DAY_MINUTES - 1)));
            }
            Event::Repeats(repeats) => self.repeats = repeats,
            Event::Tone(tone) => self.tone = tone,
            Event::Picker(field) => {
                self.picker = field.map(|field| Picker {
                    field,
                    month: self.date(field).first_of_month(),
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
        self.settle(field);
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
        // An end time picked earlier than the start on the same day means
        // the next day, as in "22:00 to 01:00".
        if changed == Field::End && self.end.0 == self.start.0 && self.end.1 < self.start.1 {
            self.end.0 = add_days(self.start.0, 1);
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

/// An entry form builder. Convert it into an [`Element`] to render.
pub struct Editor<'a, Message> {
    state: &'a State,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
    on_submit: Option<Message>,
    shortcuts: Shortcuts,
}

/// Renders the form for `state`. Without [`on_event`](Editor::on_event)
/// every control renders disabled.
pub fn editor<Message>(state: &State) -> Editor<'_, Message> {
    Editor {
        state,
        on_event: None,
        on_submit: None,
        shortcuts: Shortcuts::all(),
    }
}

impl<Message> std::fmt::Debug for Editor<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Editor")
            .field("state", self.state)
            .field("enabled", &self.on_event.is_some())
            .field("submits", &self.on_submit.is_some())
            .field("shortcuts", &self.shortcuts)
            .finish()
    }
}

impl<'a, Message> Editor<'a, Message> {
    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    /// Sent when Enter is pressed in the title, usually the same message
    /// as the Save button.
    pub fn on_submit(mut self, message: Message) -> Self {
        self.on_submit = Some(message);
        self
    }

    /// The editing shortcuts of the title field. Defaults to all of them.
    pub fn shortcuts(mut self, shortcuts: Shortcuts) -> Self {
        self.shortcuts = shortcuts;
        self
    }
}

type Handler<'a, Message> = Option<std::rc::Rc<dyn Fn(Event) -> Message + 'a>>;

fn send<Message>(handler: &Handler<'_, Message>, event: Event) -> Option<Message> {
    handler.as_ref().map(|handler| handler(event))
}

impl<'a, Message: Clone + 'a> From<Editor<'a, Message>> for Element<'a, Message> {
    fn from(editor: Editor<'a, Message>) -> Self {
        let Editor {
            state,
            on_event,
            on_submit,
            shortcuts,
        } = editor;
        let on_event: Handler<'a, Message> = on_event.map(std::rc::Rc::from);

        let title = input("Add a title", state.title())
            .shortcuts(shortcuts)
            .on_input_maybe(
                on_event
                    .clone()
                    .map(|handler| move |title| handler(Event::Title(title))),
            )
            .on_submit_maybe(on_submit);
        let all_day = switch(state.is_all_day()).label("All day").on_toggle_maybe(
            on_event
                .clone()
                .map(|handler| move |on| handler(Event::AllDay(on))),
        );

        let repeats = select(&Repeats::ALL[..], Some(state.repeats()))
            .width(Length::FillPortion(3))
            .on_select_maybe(
                on_event
                    .clone()
                    .map(|handler| move |repeats| handler(Event::Repeats(repeats))),
            );
        let colours: Vec<Colour> = Tone::ALL.into_iter().map(Colour).collect();
        let colour = select(colours, Some(Colour(state.tone())))
            .width(Length::FillPortion(2))
            .on_select_maybe(
                on_event
                    .clone()
                    .map(|handler| move |colour: Colour| handler(Event::Tone(colour.0))),
            );

        column![
            title,
            all_day,
            moment(state, Field::Start, "Starts", &on_event),
            moment(state, Field::End, "Ends", &on_event),
            row![repeats, colour].spacing(space::SM),
        ]
        .spacing(space::MD)
        .width(Length::Fill)
        .into()
    }
}

/// One end of the entry: its name, its date and, unless it lasts all day,
/// its time.
fn moment<'a, Message: Clone + 'a>(
    state: &'a State,
    field: Field,
    name: &'a str,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let label = text(name)
        .size(text_size::SM)
        .wrapping(text::Wrapping::None)
        .style(|theme: &Theme| text::Style {
            color: Some(Tokens::of(theme).muted_foreground),
        });
    let mut controls = Row::new()
        .spacing(space::SM)
        .align_y(Alignment::Center)
        .push(container(label).width(Length::Fixed(48.0)))
        .push(date_field(state, field, on_event));
    if !state.is_all_day() {
        let handler = on_event.clone();
        controls = controls.push(
            select(state.times(field), Some(TimeOfDay(state.time(field))))
                .width(TIME_WIDTH)
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
    field: Field,
    on_event: &Handler<'a, Message>,
) -> Element<'a, Message> {
    let open = state.picker.filter(|picker| picker.field == field);
    let toggle = if open.is_some() { None } else { Some(field) };
    let trigger = button(state.date(field).strftime("%a %-d %b %Y").to_string())
        .variant(button::Variant::Outline)
        .width(Length::Fill)
        .on_press_maybe(send(on_event, Event::Picker(toggle)));

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
    container(date).width(Length::Fill).into()
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

    let weeks = month_weeks(picker.month, state.week_start);
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
    let chosen = state.date(picker.field);
    for week in &weeks {
        grid = grid.push(Row::with_children(week.iter().map(|date| {
            if date.month() != picker.month.month() {
                return Space::new()
                    .width(Length::Fixed(CELL))
                    .height(Length::Fixed(CELL))
                    .into();
            }
            let day = style::Day {
                today: false,
                selected: *date == chosen,
                in_month: true,
            };
            let number = text(date.day().to_string())
                .size(text_size::XS)
                .wrapping(text::Wrapping::None);
            widget::button(container(number).center(Length::Fill))
                .width(Length::Fixed(CELL))
                .height(Length::Fixed(CELL))
                .padding(0)
                .on_press_maybe(send(on_event, Event::Date(picker.field, *date)))
                .style(move |theme, status: Status| {
                    style::year_day(&Tokens::of(theme), day, status)
                })
                .into()
        })));
    }
    column![header, grid]
        .spacing(space::SM)
        .width(Length::Fixed(7.0 * CELL))
        .into()
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
            .repeat(rule);
        let mut form = State::edit(&entry, &UTC);
        assert_eq!(form.id(), Some(EntryId(4)));
        assert!(!form.is_new());
        assert_eq!(form.title(), "Review");
        assert_eq!(form.tone(), Tone::Success);
        assert_eq!(form.repeats(), Repeats::Custom(rule));
        assert_eq!(form.entry(EntryId(99)), entry, "unchanged, with its own id");

        form.update(Event::Title(String::from("Retro")));
        let saved = form.entry(EntryId(99));
        assert_eq!(saved.id(), EntryId(4));
        assert_eq!(saved.title(), "Retro");
        assert_eq!(saved.tag_of(), Some("work"));
        assert_eq!(saved.repeat_of(), Some(rule));
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

        // The same time as the start is one step long.
        let mut form = timed();
        form.update(Event::Time(Field::End, 660));
        assert_eq!(form.time(Field::End), 660 + TIME_STEP);

        // At the last step of the day, the shortest entry ends at midnight.
        let late = at(23, 45);
        let mut form = State::new(
            &Draft {
                when: When::timed(late, late),
            },
            &UTC,
        );
        form.update(Event::Time(Field::End, 23 * 60 + 45));
        assert_eq!(
            (form.date(Field::End), form.time(Field::End)),
            (date(2026, 6, 20), 0)
        );
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
            assert_eq!(Repeats::of(choice.rule()), choice);
        }
        assert_eq!(Repeats::default(), Repeats::Never);
        let custom = Repeat::daily().count(3);
        assert_eq!(Repeats::of(Some(custom)), Repeats::Custom(custom));
        assert_eq!(Repeats::Custom(custom).rule(), Some(custom));
        assert_eq!(Repeats::Custom(custom).to_string(), "Custom");
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

        // The start's menu opens an hour early, the end's after the start.
        let form = timed();
        assert_eq!(form.times(Field::Start).first(), Some(&TimeOfDay(600)));
        assert_eq!(form.times(Field::End).first(), Some(&TimeOfDay(675)));
        let mut overnight = timed();
        overnight.update(Event::Date(Field::End, date(2026, 6, 20)));
        assert_eq!(
            overnight.times(Field::End).first(),
            Some(&TimeOfDay(750 - 60))
        );
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
}
