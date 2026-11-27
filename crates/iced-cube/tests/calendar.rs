#![cfg(feature = "calendar")]

use iced::keyboard::{self, Key, key::Named};
use iced::{Element, Event as Input, Point, Settings, Size, mouse, touch};
use iced_cube::data::calendar::{
    Draft, Entries, Entry, EntryId, Event, Output, Repeat, Selection, State, Tone, View, When,
    calendar, instant,
};
use iced_test::simulator::Simulator;
use jiff::Timestamp;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

/// Wide enough for a whole week: the frame's border, the hour labels and
/// seven day columns.
const DESKTOP: Size = Size::new(754.0, 640.0);
/// The preview frame of a 360 pixel phone.
const PHONE: Size = Size::new(328.0, 600.0);

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Calendar(Event),
}

/// A time in November 2026, UTC.
fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
    instant(
        date(2026, 11, day),
        time(hour, minute, 0, 0),
        &TimeZone::UTC,
    )
}

/// The week of Monday 16 November 2026, with today on the Thursday.
fn state(view: View) -> State {
    let entries = Entries::utc().with([
        Entry::new(EntryId(1), "Stand-up", at(17, 9, 0), at(17, 10, 0)),
        Entry::new(EntryId(2), "Design review", at(19, 14, 0), at(19, 15, 0)).tone(Tone::Success),
        Entry::new(EntryId(3), "Lunch", at(19, 12, 0), at(19, 13, 0)),
        Entry::new(EntryId(4), "Retro", at(19, 16, 0), at(19, 17, 0)),
        Entry::all_day(
            EntryId(5),
            "Conference",
            date(2026, 11, 10),
            date(2026, 11, 11),
        ),
    ]);
    State::new(date(2026, 11, 19))
        .with_entries(entries)
        .with_view(view)
}

fn view(state: &State) -> Element<'_, Message> {
    calendar(state)
        .hours(8..18)
        .on_event(Message::Calendar)
        .into()
}

/// Runs `actions` against a view of `state`, then applies what it sent,
/// as an app would. Returns the events and the outputs.
fn drive(
    state: &mut State,
    size: Size,
    actions: impl FnOnce(&mut Simulator<'_, Message>),
) -> (Vec<Event>, Vec<Output>) {
    let events: Vec<Event> = {
        let mut ui = Simulator::with_size(Settings::default(), size, view(state));
        actions(&mut ui);
        ui.into_messages()
            .map(|Message::Calendar(event)| event)
            .collect()
    };
    let outputs = events
        .iter()
        .filter_map(|event| state.update(event.clone()))
        .collect();
    (events, outputs)
}

fn centre(ui: &mut Simulator<'_, Message>, label: &str) -> Point {
    match ui.find(label) {
        Ok(target) => target.bounds().center(),
        Err(error) => panic!("{label} is not rendered: {error:?}"),
    }
}

fn shows(ui: &mut Simulator<'_, Message>, label: &str) -> bool {
    ui.find(label).is_ok()
}

fn move_to(ui: &mut Simulator<'_, Message>, point: Point) {
    ui.point_at(point);
    let _ = ui.simulate([Input::Mouse(mouse::Event::CursorMoved { position: point })]);
}

fn press(ui: &mut Simulator<'_, Message>, point: Point) {
    move_to(ui, point);
    let _ = ui.simulate([Input::Mouse(mouse::Event::ButtonPressed(
        mouse::Button::Left,
    ))]);
}

fn release(ui: &mut Simulator<'_, Message>) {
    let _ = ui.simulate([Input::Mouse(mouse::Event::ButtonReleased(
        mouse::Button::Left,
    ))]);
}

fn click_at(ui: &mut Simulator<'_, Message>, point: Point) {
    press(ui, point);
    release(ui);
}

fn drag(ui: &mut Simulator<'_, Message>, from: Point, to: Point) {
    press(ui, from);
    // Part of the way first, as a pointer travels.
    move_to(ui, Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0));
    move_to(ui, to);
    release(ui);
}

/// A point in the hours of the day column under the header `day`, such as
/// "TUE", worked out from where the hour labels are drawn.
fn slot(ui: &mut Simulator<'_, Message>, day: &str, hour: f32) -> Point {
    let nine = centre(ui, "09:00").y;
    let ten = centre(ui, "10:00").y;
    Point::new(centre(ui, day).x, nine + (hour - 9.0) * (ten - nine))
}

#[test]
fn the_header_moves_between_periods_and_views() {
    let mut state = state(View::Month);
    let (events, outputs) = drive(&mut state, DESKTOP, |ui| {
        assert!(shows(ui, "November 2026"));
        let today = match ui.find("Today") {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        // Previous and next sit after the Today button: its padding, a
        // gap, then two 36 pixel buttons.
        click_at(
            ui,
            Point::new(today.x + today.width + 38.0, today.center_y()),
        );
        click_at(
            ui,
            Point::new(today.x + today.width + 78.0, today.center_y()),
        );
        // The menu shows the view on show, and lists the rest when opened.
        let _ = ui.click("Month");
        for view in ["Day", "Year", "Schedule", "4 days"] {
            assert!(shows(ui, view), "{view}");
        }
        let _ = ui.click("Week");
        let _ = ui.click("Today");
    });
    assert_eq!(
        events,
        vec![
            Event::Previous,
            Event::Next,
            Event::Show(View::Week),
            Event::Today
        ]
    );
    assert_eq!(outputs.len(), 4);
    assert_eq!(state.view(), View::Week);
    assert_eq!(state.selection(), Some(Selection::Day(date(2026, 11, 19))));
    assert_eq!(state.title(), "16 - 22 Nov 2026");
}

#[test]
fn a_month_shows_its_entries_and_a_press_selects_one() {
    let mut state = state(View::Month);
    let (events, outputs) = drive(&mut state, DESKTOP, |ui| {
        for label in ["MON", "SUN", "09:00 Stand-up", "Conference", "30"] {
            assert!(shows(ui, label), "{label}");
        }
        let _ = ui.click("09:00 Stand-up");
    });
    let picked = Selection::Entry {
        entry: EntryId(1),
        start: at(17, 9, 0),
    };
    assert_eq!(events, vec![Event::Press(picked)]);
    assert_eq!(outputs, vec![Output::Selected(Some(picked))]);

    // The same press again opens it.
    let (_, outputs) = drive(&mut state, DESKTOP, |ui| {
        let _ = ui.click("09:00 Stand-up");
    });
    assert_eq!(
        outputs,
        vec![Output::Opened {
            entry: EntryId(1),
            start: at(17, 9, 0)
        }]
    );
}

#[test]
fn pressing_a_day_selects_it_and_pressing_it_again_asks_for_an_entry() {
    let mut state = state(View::Month);
    let day = date(2026, 11, 25);
    let (events, outputs) = drive(&mut state, DESKTOP, |ui| {
        let _ = ui.click("25");
    });
    assert_eq!(events, vec![Event::Press(Selection::Day(day))]);
    assert_eq!(outputs, vec![Output::Selected(Some(Selection::Day(day)))]);

    let (_, outputs) = drive(&mut state, DESKTOP, |ui| {
        let _ = ui.click("25");
    });
    assert_eq!(
        outputs,
        vec![Output::Requested(Draft {
            when: When::day(day)
        })]
    );
    assert_eq!(
        state.entries().len(),
        5,
        "nothing is added until the app inserts"
    );
}

#[test]
fn a_crowded_day_counts_what_it_hides_and_opens_the_day() {
    let mut state = state(View::Month);
    let (events, _) = drive(&mut state, Size::new(754.0, 480.0), |ui| {
        assert!(!shows(ui, "12:00 Lunch"), "three entries do not fit");
        let _ = ui.click("+3 more");
    });
    assert_eq!(events, vec![Event::OpenDay(date(2026, 11, 19))]);
    assert_eq!(state.view(), View::Day);
    assert_eq!(state.cursor(), date(2026, 11, 19));
}

#[test]
fn a_calendar_without_a_handler_is_inert() {
    let state = state(View::Week);
    let disabled: Element<'_, Message> = calendar(&state).hours(8..18).into();
    let mut ui = Simulator::with_size(Settings::default(), DESKTOP, disabled);
    let _ = ui.click("Month");
    let _ = ui.click("Today");
    let stand_up = centre(&mut ui, "Stand-up");
    press(&mut ui, stand_up);
    move_to(&mut ui, Point::new(stand_up.x, stand_up.y + 80.0));
    release(&mut ui);
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn a_week_draws_days_hours_and_entries() {
    let state = state(View::Week);
    let mut ui = Simulator::with_size(Settings::default(), DESKTOP, view(&state));
    for label in [
        "MON",
        "SUN",
        "16",
        "22",
        "09:00",
        "17:00",
        "Stand-up",
        "09:00-10:00",
        "Design review",
        "Lunch",
    ] {
        assert!(shows(&mut ui, label), "{label}");
    }
    assert!(!shows(&mut ui, "08:00"), "the first line has no label");
    assert!(!shows(&mut ui, "Conference"), "last week's entry");

    // An entry sits in its day's column at its time.
    let block = centre(&mut ui, "Stand-up");
    let column = centre(&mut ui, "TUE").x;
    assert!((block.x - column).abs() < 50.0, "{block:?} {column}");
    let nine = centre(&mut ui, "09:00").y;
    let ten = centre(&mut ui, "10:00").y;
    assert!(block.y > nine && block.y < ten, "{block:?}");
}

#[test]
fn pressing_an_entry_or_a_time_in_the_week_selects_it() {
    let mut state = state(View::Week);
    let (events, _) = drive(&mut state, DESKTOP, |ui| {
        let stand_up = centre(ui, "Stand-up");
        click_at(ui, stand_up);
        let empty = slot(ui, "FRI", 11.7);
        click_at(ui, empty);
    });
    assert_eq!(
        events,
        vec![
            Event::Press(Selection::Entry {
                entry: EntryId(1),
                start: at(17, 9, 0)
            }),
            Event::Press(Selection::Slot {
                date: date(2026, 11, 20),
                minute: 690
            }),
        ]
    );

    // Pressing the selected time again asks for an hour there.
    let (_, outputs) = drive(&mut state, DESKTOP, |ui| {
        let empty = slot(ui, "FRI", 11.7);
        click_at(ui, empty);
    });
    assert_eq!(
        outputs,
        vec![Output::Requested(Draft {
            when: When::timed(at(20, 11, 30), at(20, 12, 30))
        })]
    );
}

#[test]
fn dragging_empty_space_asks_for_an_entry_of_that_length() {
    let mut state = state(View::Week);
    let (events, outputs) = drive(&mut state, DESKTOP, |ui| {
        let from = slot(ui, "WED", 10.0);
        let to = slot(ui, "WED", 11.5);
        drag(ui, from, to);
    });
    let draft = Draft {
        when: When::timed(at(18, 10, 0), at(18, 11, 30)),
    };
    assert_eq!(events, vec![Event::Request(draft)]);
    assert_eq!(outputs, vec![Output::Requested(draft)]);
}

#[test]
fn dragging_an_entry_moves_it_to_another_day_and_time() {
    let mut state = state(View::Week);
    let (events, outputs) = drive(&mut state, DESKTOP, |ui| {
        // Grabbed at 09:15 and dropped two hours later on Friday.
        let from = slot(ui, "TUE", 9.25);
        let to = slot(ui, "FRI", 11.25);
        drag(ui, from, to);
    });
    assert_eq!(
        events,
        vec![Event::Reschedule {
            entry: EntryId(1),
            start: at(17, 9, 0),
            to: (at(20, 11, 0), at(20, 12, 0)),
        }]
    );
    assert!(matches!(outputs.as_slice(), [Output::Changed(_)]));
    let moved = state.entries().get(EntryId(1)).map(Entry::when);
    assert_eq!(moved, Some(When::timed(at(20, 11, 0), at(20, 12, 0))));
}

#[test]
fn dragging_the_lower_edge_changes_when_an_entry_ends() {
    let mut state = state(View::Week);
    let (events, _) = drive(&mut state, DESKTOP, |ui| {
        let ten = slot(ui, "TUE", 10.0);
        let edge = Point::new(ten.x, ten.y - 4.0);
        let to = slot(ui, "TUE", 11.5);
        drag(ui, edge, to);
    });
    assert_eq!(
        events,
        vec![Event::Reschedule {
            entry: EntryId(1),
            start: at(17, 9, 0),
            to: (at(17, 9, 0), at(17, 11, 30)),
        }]
    );
}

#[test]
fn dragging_the_top_edge_changes_when_an_entry_starts() {
    let mut state = state(View::Week);
    let (events, outputs) = drive(&mut state, DESKTOP, |ui| {
        let nine = slot(ui, "TUE", 9.0);
        let edge = Point::new(nine.x, nine.y + 3.0);
        let to = slot(ui, "TUE", 8.5);
        drag(ui, edge, to);
    });
    assert_eq!(
        events,
        vec![Event::Reschedule {
            entry: EntryId(1),
            start: at(17, 9, 0),
            to: (at(17, 8, 30), at(17, 10, 0)),
        }]
    );
    assert!(matches!(outputs.as_slice(), [Output::Changed(_)]));
    let longer = state.entries().get(EntryId(1)).map(Entry::when);
    assert_eq!(longer, Some(When::timed(at(17, 8, 30), at(17, 10, 0))));
}

#[test]
fn dragging_the_top_edge_of_a_repeat_changes_the_length_of_every_occurrence() {
    let mut state = State::new(date(2026, 11, 19))
        .with_entries(Entries::utc().with([
            Entry::new(EntryId(1), "Stand-up", at(16, 9, 0), at(16, 10, 0)).repeat(Repeat::daily()),
        ]))
        .with_view(View::Week);
    let (events, _) = drive(&mut state, DESKTOP, |ui| {
        let nine = slot(ui, "WED", 9.0);
        let to = slot(ui, "WED", 9.5);
        drag(ui, Point::new(nine.x, nine.y + 3.0), to);
    });
    assert_eq!(
        events,
        vec![Event::Reschedule {
            entry: EntryId(1),
            start: at(18, 9, 0),
            to: (at(18, 9, 30), at(18, 10, 0)),
        }]
    );
    let shorter = state.entries().get(EntryId(1)).map(Entry::when);
    assert_eq!(shorter, Some(When::timed(at(16, 9, 30), at(16, 10, 0))));
}

#[test]
fn escape_abandons_a_drag() {
    let mut state = state(View::Week);
    let (events, _) = drive(&mut state, DESKTOP, |ui| {
        let from = slot(ui, "TUE", 9.25);
        let to = slot(ui, "FRI", 11.25);
        press(ui, from);
        move_to(ui, to);
        let _ = ui.simulate([Input::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Named(Named::Escape),
            modified_key: Key::Named(Named::Escape),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        })]);
        release(ui);
    });
    assert_eq!(events, vec![]);
}

#[test]
fn a_tap_selects_and_a_finger_that_moves_does_not() {
    let mut state = state(View::Week);
    let (events, _) = drive(&mut state, DESKTOP, |ui| {
        let id = touch::Finger(1);
        let stand_up = centre(ui, "Stand-up");
        ui.point_at(stand_up);
        let _ = ui.simulate([
            Input::Touch(touch::Event::FingerPressed {
                id,
                position: stand_up,
            }),
            Input::Touch(touch::Event::FingerLifted {
                id,
                position: stand_up,
            }),
        ]);

        // A finger that travels is scrolling the hours.
        let lunch = centre(ui, "Lunch");
        let dragged = Point::new(lunch.x, lunch.y + 60.0);
        ui.point_at(lunch);
        let _ = ui.simulate([Input::Touch(touch::Event::FingerPressed {
            id,
            position: lunch,
        })]);
        ui.point_at(dragged);
        let _ = ui.simulate([
            Input::Touch(touch::Event::FingerMoved {
                id,
                position: dragged,
            }),
            Input::Touch(touch::Event::FingerLifted {
                id,
                position: dragged,
            }),
        ]);
    });
    assert_eq!(
        events,
        vec![Event::Press(Selection::Entry {
            entry: EntryId(1),
            start: at(17, 9, 0)
        })]
    );
}

#[test]
fn a_day_shows_one_column_with_the_whole_time_range() {
    let mut state = state(View::Day);
    let (events, _) = drive(&mut state, DESKTOP, |ui| {
        assert!(shows(ui, "Thursday 19 November 2026"));
        assert!(shows(ui, "Design review"));
        assert!(shows(ui, "14:00-15:00"));
        assert!(!shows(ui, "Stand-up"), "Tuesday's entry");
        let _ = ui.click("Lunch");
    });
    assert_eq!(
        events,
        vec![Event::Press(Selection::Entry {
            entry: EntryId(3),
            start: at(19, 12, 0)
        })]
    );
}

#[test]
fn the_agenda_lists_the_month_and_a_press_selects() {
    let mut state = state(View::Agenda);
    let (events, _) = drive(&mut state, DESKTOP, |ui| {
        for label in [
            "NOV, TUE",
            "All day",
            "Conference",
            "NOV, THU",
            "12:00 - 13:00",
            "Lunch",
        ] {
            assert!(shows(ui, label), "{label}");
        }
        // The date sits beside its day's first entry.
        let date = centre(ui, "NOV, THU");
        let first = centre(ui, "12:00 - 13:00");
        assert!(date.x < first.x, "{date:?} {first:?}");
        assert!((date.y - first.y).abs() < 4.0, "{date:?} {first:?}");
        let _ = ui.click("Lunch");
    });
    assert_eq!(
        events,
        vec![Event::Press(Selection::Entry {
            entry: EntryId(3),
            start: at(19, 12, 0)
        })]
    );

    let empty = State::new(date(2026, 11, 19)).with_view(View::Agenda);
    let mut ui = Simulator::with_size(Settings::default(), DESKTOP, view(&empty));
    assert!(shows(&mut ui, "No entries in this period"));
}

#[test]
fn a_read_only_calendar_selects_and_opens_but_takes_no_changes() {
    let mut state = state(View::Week).read_only();
    let before = state.entries().clone();
    let (events, outputs) = drive(&mut state, DESKTOP, |ui| {
        // Drags across empty space, on an entry and on its edges.
        let from = slot(ui, "WED", 10.0);
        let to = slot(ui, "WED", 11.5);
        drag(ui, from, to);
        let from = slot(ui, "TUE", 9.25);
        let to = slot(ui, "FRI", 11.25);
        drag(ui, from, to);
        let ten = slot(ui, "TUE", 10.0);
        let to = slot(ui, "TUE", 11.5);
        drag(ui, Point::new(ten.x, ten.y - 4.0), to);
        let nine = slot(ui, "TUE", 9.0);
        let to = slot(ui, "TUE", 8.5);
        drag(ui, Point::new(nine.x, nine.y + 3.0), to);
    });
    assert_eq!(events, vec![], "no drag sends anything");
    assert_eq!(outputs, vec![]);

    // Presses still select, and a second press opens an entry but asks
    // for nothing on empty time.
    let (events, outputs) = drive(&mut state, DESKTOP, |ui| {
        let empty = slot(ui, "FRI", 11.7);
        click_at(ui, empty);
        click_at(ui, empty);
        let stand_up = centre(ui, "Stand-up");
        click_at(ui, stand_up);
    });
    assert_eq!(events.len(), 3);
    assert_eq!(outputs.len(), 2, "two selections, and no request");
    let (_, outputs) = drive(&mut state, DESKTOP, |ui| {
        let stand_up = centre(ui, "Stand-up");
        click_at(ui, stand_up);
    });
    assert_eq!(
        outputs,
        vec![Output::Opened {
            entry: EntryId(1),
            start: at(17, 9, 0)
        }]
    );
    assert_eq!(state.entries(), &before);

    // The app can still add to it.
    let added = Entry::new(EntryId(9), "Added by the app", at(18, 9, 0), at(18, 10, 0));
    assert!(state.update(Event::Insert(added)).is_some());
    let mut ui = Simulator::with_size(Settings::default(), DESKTOP, view(&state));
    assert!(shows(&mut ui, "Added by the app"));
}

#[test]
fn four_days_start_on_the_day_on_show() {
    let mut state = state(View::FourDays);
    let (events, _) = drive(&mut state, DESKTOP, |ui| {
        assert!(shows(ui, "19 - 22 Nov 2026"));
        for day in ["THU", "FRI", "SAT", "SUN"] {
            assert!(shows(ui, day), "{day}");
        }
        assert!(!shows(ui, "WED"));
        assert!(shows(ui, "Design review"));
        assert!(!shows(ui, "Stand-up"), "Tuesday is before the four days");
        let empty = slot(ui, "SAT", 10.0);
        click_at(ui, empty);
    });
    assert_eq!(
        events,
        vec![Event::Press(Selection::Slot {
            date: date(2026, 11, 21),
            minute: 600
        })]
    );
}

#[test]
fn a_year_shows_twelve_months_and_a_press_selects_a_day() {
    let mut state = state(View::Year);
    let (events, _) = drive(&mut state, DESKTOP, |ui| {
        assert!(shows(ui, "2026"));
        for month in ["January", "June", "December"] {
            assert!(shows(ui, month), "{month}");
        }
        // Thursday 15 January is in the third row of its month, under the
        // name and the weekday letters. Days are 32 pixels square.
        let name = match ui.find("January") {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        let cell = 32.0;
        let left = name.x - 6.0;
        let top = name.y + name.height + 2.0 + cell + 2.0;
        click_at(
            ui,
            Point::new(left + 3.5 * cell, top + 2.0 * (cell + 2.0) + cell / 2.0),
        );
    });
    assert_eq!(
        events,
        vec![Event::Press(Selection::Day(date(2026, 1, 15)))]
    );
    assert_eq!(state.selected_day(), Some(date(2026, 1, 15)));
}

/// Whether the text `label` is drawn wholly inside a window `width` wide.
fn fits(ui: &mut Simulator<'_, Message>, label: &str, width: f32) -> bool {
    match ui.find(label) {
        Ok(target) => {
            let bounds = target.bounds();
            bounds.x >= 0.0 && bounds.x + bounds.width <= width
        }
        Err(_) => false,
    }
}

#[test]
fn a_narrow_month_shows_dots_and_lists_the_selected_day() {
    let mut state = state(View::Month);
    let (events, _) = drive(&mut state, PHONE, |ui| {
        assert!(!shows(ui, "09:00 Stand-up"), "no room for bars");
        assert!(!shows(ui, "MON"), "weekdays are single letters");
        // With nothing selected the list is for the day on show.
        for label in ["Thursday 19 November (today)", "Design review", "Lunch"] {
            assert!(fits(ui, label, PHONE.width), "{label}");
        }
        for label in ["Today", "Month", "November 2026"] {
            assert!(fits(ui, label, PHONE.width), "{label}");
        }
        let _ = ui.click("17");
    });
    assert_eq!(
        events,
        vec![Event::Press(Selection::Day(date(2026, 11, 17)))]
    );

    let (events, _) = drive(&mut state, PHONE, |ui| {
        assert!(shows(ui, "Tuesday 17 November"));
        assert!(!shows(ui, "Lunch"));
        let _ = ui.click("Stand-up");
    });
    assert_eq!(
        events,
        vec![Event::Press(Selection::Entry {
            entry: EntryId(1),
            start: at(17, 9, 0)
        })]
    );
}

#[test]
fn a_narrow_week_shows_one_day_under_a_strip_of_the_week() {
    let mut state = state(View::Week);
    let (events, _) = drive(&mut state, PHONE, |ui| {
        assert!(fits(ui, "Design review", PHONE.width));
        assert!(!shows(ui, "Stand-up"), "only the day on show");
        assert!(!shows(ui, "THU"), "the strip uses single letters");
        for day in ["16", "19", "22"] {
            assert!(fits(ui, day, PHONE.width), "{day}");
        }
        let _ = ui.click("17");
    });
    assert_eq!(
        events,
        vec![Event::Press(Selection::Day(date(2026, 11, 17)))]
    );

    let mut ui = Simulator::with_size(Settings::default(), PHONE, view(&state));
    assert!(fits(&mut ui, "Stand-up", PHONE.width), "the pressed day");
    assert!(!shows(&mut ui, "Design review"));
}
