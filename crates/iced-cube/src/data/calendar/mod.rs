//! A calendar of entries with day, week, month, year, schedule and
//! four-day views, and triggers that tell the app when an entry is
//! created, is due or is about to be.
//!
//! The header has a Today button, arrows, the period in words and a menu
//! of the [`View`]s. Day, week and four days put days side by side
//! against the hours; the month is a grid of weeks with whole-day entries
//! as bars and timed ones as a dot, a time and a title; the year is
//! twelve small months; and the schedule lists the month's entries with
//! each day's date beside them.
//!
//! [`State`] holds the [`Entries`], the [`View`], the period on show and
//! the selection. Its [`update`](State::update) applies an [`Event`] and
//! returns an [`Output`] for the things an app acts on: an entry was
//! opened, a new one was asked for, an entry changed, another period came
//! on show. The calendar never reads the clock; today's date and the
//! current time are passed in.
//!
//! ```no_run
//! use iced::Element;
//! use iced_cube::data::calendar::{self, Entry, EntryId, Output, calendar};
//! use jiff::Timestamp;
//!
//! #[derive(Debug, Clone)]
//! enum Message {
//!     Calendar(calendar::Event),
//! }
//!
//! struct App {
//!     calendar: calendar::State,
//! }
//!
//! impl App {
//!     fn new(now: Timestamp) -> Self {
//!         Self {
//!             calendar: calendar::State::at(now, calendar::Entries::system()),
//!         }
//!     }
//!
//!     fn update(&mut self, message: Message) {
//!         let Message::Calendar(event) = message;
//!         match self.calendar.update(event) {
//!             // The user pressed a day twice or dragged out a time.
//!             Some(Output::Requested(draft)) => {
//!                 let id = self.calendar.entries().next_id();
//!                 let entry = Entry::at(id, "New entry", draft.when);
//!                 let _ = self.calendar.update(calendar::Event::Insert(entry));
//!             }
//!             Some(Output::Opened { entry, .. }) => println!("open {entry:?}"),
//!             _ => {}
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         calendar(&self.calendar).on_event(Message::Calendar).into()
//!     }
//! }
//! ```
//!
//! # Entries
//!
//! An [`Entry`] is timed, from one instant up to another, or covers whole
//! days. It can [repeat](Repeat) daily, weekly, monthly or yearly. Timed
//! entries are stored as instants and shown in the time zone of the
//! [`Entries`] collection, so changing the time zone moves what is drawn
//! and not what is stored.
//!
//! # Selecting and activating
//!
//! The first press on a day, a time slot or an entry selects it. A press
//! on what is already selected activates it: an entry is
//! [opened](Output::Opened), and a day or a slot [asks](Output::Requested)
//! for a new entry there. Two quick presses therefore open or create, and
//! so do two taps on a touch screen. In the week and day views a drag on
//! empty space asks for an entry of that length, a drag on an entry moves
//! it and a drag on its lower edge changes when it ends.
//!
//! The calendar only asks; the app decides what to show. [`editor`] is a
//! ready-made form for that: title, all day, start and end date and time,
//! how often it repeats and when that stops, colour, location and notes.
//!
//! # Triggers
//!
//! See [`schedule`] for telling the app when entries are created, due or
//! almost due, including the ones missed while the app was not running.
//!
//! # Keyboard
//!
//! Shortcuts resolve through a [`Keymap`](crate::keys::Keymap) of
//! [`Action`]s; see [`default_keymap`] and [`crate::keys`].

mod agenda;
pub mod editor;
pub mod entry;
mod gesture;
pub mod layout;
mod month;
mod parts;
pub mod repeat;
pub mod schedule;
mod state;
pub mod style;
mod time_grid;
mod year;

use std::ops::Range;
use std::rc::Rc;

use iced::widget::{self, column, container, responsive, row, text};
use iced::{Alignment, Element, Length, Theme};

pub use entry::{
    Change, Changed, Draft, Emphasis, Entries, Entry, EntryId, Occurrence, Tone, When, instant,
};
/// The date and time library the calendar's types come from, so an app
/// can use the same version without depending on it directly.
pub use jiff;
pub use parts::{
    bar_label, day_id, entry_id, is_line, span_label, time_label, weekday_label, when_label,
};
pub use repeat::{Ends, Frequency, Repeat};
pub use state::{
    Action, Allow, DOUBLE_CLICK, Event, FOUR_DAYS, Output, SLOT_MINUTES, Selection, State, View,
    default_keymap,
};
pub use time_grid::{
    GUTTER, SNAP_MINUTES, hour_offset, hours_id, point_at, scroll_id, scroll_to_hour,
};

use crate::forms::select::select;
use crate::lucide;
use crate::primitives::button::{self, Size, button};
use crate::primitives::icon_button::{self, icon_button};
use crate::theme::{self, Tokens, space, text_size};
use parts::{Handler, send};

/// The height of an hour in the week and day views unless set.
pub const DEFAULT_HOUR_HEIGHT: f32 = 48.0;

/// The least an hour can be set to in the week and day views.
pub const MIN_HOUR_HEIGHT: f32 = 36.0;

/// The width below which the header puts the navigation buttons on a
/// second line.
pub const HEADER_BREAK: f32 = 520.0;

/// The width of the menu of views in the header.
pub const VIEW_MENU_WIDTH: f32 = 124.0;

/// The id of the header's Today button.
pub fn today_button_id() -> widget::Id {
    widget::Id::new("iced-cube-calendar-today")
}

/// The id of the header's button to the previous period.
pub fn previous_button_id() -> widget::Id {
    widget::Id::new("iced-cube-calendar-previous")
}

/// The id of the header's button to the next period.
pub fn next_button_id() -> widget::Id {
    widget::Id::new("iced-cube-calendar-next")
}

/// The id of the header's menu of views.
pub fn view_menu_id() -> widget::Id {
    widget::Id::new("iced-cube-calendar-views")
}

/// A calendar builder. Convert it into an [`Element`] to render.
pub struct Calendar<'a, Message> {
    state: &'a State,
    on_event: Option<Box<dyn Fn(Event) -> Message + 'a>>,
    width: Length,
    height: Length,
    header: bool,
    leading: Option<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
    hours: (u8, u8),
    hour_height: f32,
    start_hour: Option<u8>,
}

/// Renders `state`. Without [`on_event`](Calendar::on_event) it renders
/// disabled: nothing can be pressed or dragged.
pub fn calendar<Message>(state: &State) -> Calendar<'_, Message> {
    Calendar {
        state,
        on_event: None,
        width: Length::Fill,
        height: Length::Fill,
        header: true,
        leading: None,
        trailing: None,
        hours: (0, 24),
        hour_height: DEFAULT_HOUR_HEIGHT,
        start_hour: None,
    }
}

impl<Message> std::fmt::Debug for Calendar<'_, Message> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Calendar")
            .field("view", &self.state.view())
            .field("cursor", &self.state.cursor())
            .field("width", &self.width)
            .field("height", &self.height)
            .field("header", &self.header)
            .field("leading", &self.leading.is_some())
            .field("trailing", &self.trailing.is_some())
            .field("hours", &self.hours)
            .field("hour_height", &self.hour_height)
            .field("start_hour", &self.start_hour)
            .field("enabled", &self.on_event.is_some())
            .finish()
    }
}

impl<'a, Message> Calendar<'a, Message> {
    pub fn on_event(mut self, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        self.on_event = Some(Box::new(on_event));
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The height of the whole calendar. It fills the space it is given by
    /// default, so give it a fixed height inside anything that scrolls.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Whether to show the Today button, the arrows, the title and the
    /// menu of views. Turn it off to build your own from [`State::title`]
    /// and the navigation [`Event`]s.
    pub fn header(mut self, header: bool) -> Self {
        self.header = header;
        self
    }

    /// Something of the app's at the start of the header, before Today,
    /// such as a button for a new entry.
    pub fn header_leading(mut self, element: impl Into<Element<'a, Message>>) -> Self {
        self.leading = Some(element.into());
        self
    }

    /// Something of the app's at the end of the header, after the menu of
    /// views.
    pub fn header_trailing(mut self, element: impl Into<Element<'a, Message>>) -> Self {
        self.trailing = Some(element.into());
        self
    }

    /// Scrolls a whole-day week or day view to `hour` whenever a timed
    /// view comes on show and when Today is pressed, so the working day is
    /// in view instead of midnight. Off by default.
    pub fn start_at_hour(mut self, hour: u8) -> Self {
        self.start_hour = Some(hour.min(23));
        self
    }

    /// The hours the week and day views show, such as `8..18` for a
    /// working day. Defaults to the whole day, which scrolls.
    pub fn hours(mut self, hours: Range<u8>) -> Self {
        let first = hours.start.min(23);
        self.hours = (first, hours.end.clamp(first + 1, 24));
        self
    }

    /// The least height of an hour in the week and day views, which is at
    /// least [`MIN_HOUR_HEIGHT`] so a half-hour entry still has room for
    /// its title. Hours grow past it to fill the space when the ones on
    /// show would leave room below, and scroll when they do not fit.
    pub fn hour_height(mut self, height: f32) -> Self {
        self.hour_height = height.max(MIN_HOUR_HEIGHT);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Calendar<'a, Message>> for Element<'a, Message> {
    fn from(calendar: Calendar<'a, Message>) -> Self {
        let Calendar {
            state,
            on_event,
            width,
            height,
            header,
            leading,
            trailing,
            hours,
            hour_height,
            start_hour,
        } = calendar;
        let on_event: Handler<'a, Message> = on_event.map(Rc::from);
        let options = time_grid::Options {
            hours,
            hour_height,
            start_hour,
        };

        let body = match state.view() {
            View::Month => month::view(state, &on_event),
            View::Year => year::view(state, &on_event),
            View::Agenda => agenda::view(state, &on_event),
            View::Day | View::Week | View::FourDays => time_grid::view(state, options, &on_event),
        };
        let framed = container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(1)
            .clip(true)
            .style(|theme: &Theme| style::frame(&Tokens::of(theme)));

        let mut content = column![].spacing(space::MD).width(width).height(height);
        if header {
            content = content.push(self::header(state, on_event.clone(), leading, trailing));
        }
        content.push(framed).into()
    }
}

/// The app's leading element, the Today button and arrows, the title, the
/// menu of views and the app's trailing element.
fn header<'a, Message: Clone + 'a>(
    state: &'a State,
    on_event: Handler<'a, Message>,
    leading: Option<Element<'a, Message>>,
    trailing: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    // The app's slots sit outside the part that rearranges itself on a
    // narrow screen, since an element can only be placed once.
    let adaptive = responsive(move |size| {
        let step = |glyph, label: &'static str, event: Event, id: widget::Id| {
            icon_button(glyph)
                .label(label)
                .id(id)
                .variant(icon_button::Variant::Ghost)
                .size(Size::Md)
                .on_press_maybe(send(&on_event, event))
        };
        let navigation = row![
            container(
                button("Today")
                    .variant(button::Variant::Outline)
                    .size(Size::Md)
                    .on_press_maybe(send(&on_event, Event::Today))
            )
            .id(today_button_id()),
            step(
                lucide!(ChevronLeft),
                "Previous",
                Event::Previous,
                previous_button_id()
            ),
            step(lucide!(ChevronRight), "Next", Event::Next, next_button_id()),
        ]
        .spacing(space::XS)
        .align_y(Alignment::Center);

        let narrow = size.width < HEADER_BREAK;
        let title = if narrow {
            state.short_title()
        } else {
            state.title()
        };
        let title = text(title)
            .size(if narrow { text_size::SM } else { text_size::MD })
            .font(theme::semibold())
            .wrapping(text::Wrapping::None)
            .style(|theme: &Theme| text::Style {
                color: Some(Tokens::of(theme).foreground),
            });

        let views = container(
            select(&View::ALL[..], Some(state.view()))
                .width(VIEW_MENU_WIDTH)
                .on_select_maybe(
                    on_event
                        .clone()
                        .map(|handler| move |view| handler(Event::Show(view))),
                ),
        )
        .id(view_menu_id());
        let title = container(title).width(Length::Fill).clip(true);

        // A phone has the title beside the menu and the buttons below.
        if narrow {
            let lead = row![title, views]
                .spacing(space::SM)
                .align_y(Alignment::Center);
            return column![lead, navigation].spacing(space::SM).into();
        }
        row![navigation, title, views]
            .spacing(space::MD)
            .align_y(Alignment::Center)
            .into()
    })
    .width(Length::Fill)
    .height(Length::Shrink);

    if leading.is_none() && trailing.is_none() {
        return adaptive.into();
    }
    let mut header = row![].spacing(space::XS).align_y(Alignment::Center);
    if let Some(leading) = leading {
        header = header.push(leading);
    }
    header = header.push(adaptive);
    if let Some(trailing) = trailing {
        header = header.push(trailing);
    }
    header.width(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn builder_defaults_and_options() {
        let state = State::new(date(2026, 11, 19));
        let calendar: Calendar<'_, Event> = calendar(&state);
        assert_eq!(calendar.width, Length::Fill);
        assert_eq!(calendar.height, Length::Fill);
        assert!(calendar.header);
        assert_eq!(calendar.hours, (0, 24));
        assert_eq!(calendar.hour_height, DEFAULT_HOUR_HEIGHT);
        assert!(calendar.on_event.is_none());

        let calendar = calendar
            .width(400)
            .height(300)
            .header(false)
            .hours(8..18)
            .hour_height(60.0)
            .on_event(|event| event);
        assert_eq!(calendar.width, Length::Fixed(400.0));
        assert_eq!(calendar.height, Length::Fixed(300.0));
        assert!(!calendar.header);
        assert_eq!(calendar.hours, (8, 18));
        assert_eq!(calendar.hour_height, 60.0);
        assert!(calendar.on_event.is_some());
        assert!(format!("{calendar:?}").contains("enabled: true"));
    }

    #[test]
    fn hours_and_hour_height_stay_in_range() {
        let state = State::new(date(2026, 11, 19));
        let hours = |range: Range<u8>| calendar::<Event>(&state).hours(range).hours;
        assert_eq!(hours(0..24), (0, 24));
        assert_eq!(hours(9..9), (9, 10));
        assert_eq!(hours(20..99), (20, 24));
        assert_eq!(hours(40..50), (23, 24));
        #[allow(clippy::reversed_empty_ranges)]
        let backwards = hours(18..8);
        assert_eq!(backwards, (18, 19));
        assert_eq!(
            calendar::<Event>(&state).hour_height(2.0).hour_height,
            MIN_HOUR_HEIGHT
        );
    }
}
