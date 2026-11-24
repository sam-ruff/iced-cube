#![cfg(feature = "calendar")]

use iced::keyboard::{self, Key, key::Named};
use iced::time::Duration;
use iced::widget::{self, button};
use iced::{Element, Event as Input, Point, Settings, Size, mouse};
use iced_cube::data::calendar::entry::midnight;
use iced_cube::data::calendar::{
    Change, Entries, Entry, EntryId, Event, Output, Selection, State, View, When, calendar, day_id,
    entry_id, hours_id, instant, next_button_id, point_at, previous_button_id, today_button_id,
    view_menu_id,
};
use iced_test::simulator::Simulator;
use jiff::Timestamp;
use jiff::civil::{Date, date, time};
use jiff::tz::TimeZone;

const DESKTOP: Size = Size::new(754.0, 640.0);
/// Long enough that two presses in a test always make a double click.
const PATIENT: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Calendar(Event),
    New,
}

fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
    instant(
        date(2026, 11, day),
        time(hour, minute, 0, 0),
        &TimeZone::UTC,
    )
}

fn day(day: i8) -> Date {
    date(2026, 11, day)
}

/// November 2026 with today on Thursday the 19th.
fn state(view: View) -> State {
    let entries = Entries::utc().with([
        Entry::new(EntryId(1), "Stand-up", at(17, 9, 0), at(17, 10, 0)),
        Entry::new(EntryId(2), "Design review", at(19, 14, 0), at(19, 15, 0)),
        Entry::all_day(EntryId(3), "Offsite", day(18), day(18)),
        Entry::new(EntryId(4), "Board", at(23, 10, 0), at(23, 11, 0)).locked(true),
    ]);
    State::new(day(19))
        .with_entries(entries)
        .with_view(view)
        .with_double_click(PATIENT)
}

fn view(state: &State) -> Element<'_, Message> {
    calendar(state)
        .hours(8..18)
        .on_event(Message::Calendar)
        .into()
}

fn drive(
    state: &mut State,
    view: impl for<'a> Fn(&'a State) -> Element<'a, Message>,
    actions: impl FnOnce(&mut Simulator<'_, Message>),
) -> (Vec<Message>, Vec<Output>) {
    let messages: Vec<Message> = {
        let mut ui = Simulator::with_size(Settings::default(), DESKTOP, view(state));
        actions(&mut ui);
        ui.into_messages().collect()
    };
    let outputs = messages
        .iter()
        .filter_map(|message| match message {
            Message::Calendar(event) => state.update(event.clone()),
            Message::New => None,
        })
        .collect();
    (messages, outputs)
}

fn events(messages: &[Message]) -> Vec<Event> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::Calendar(event) => Some(event.clone()),
            Message::New => None,
        })
        .collect()
}

fn centre_of(ui: &mut Simulator<'_, Message>, id: widget::Id) -> Point {
    match ui.find(id.clone()) {
        Ok(target) => target.bounds().center(),
        Err(error) => panic!("{id:?} is not rendered: {error:?}"),
    }
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
    move_to(ui, Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0));
    move_to(ui, to);
    release(ui);
}

fn when_of(state: &State, id: u64) -> Option<When> {
    state.entries().get(EntryId(id)).map(Entry::when)
}

#[test]
fn a_timed_entry_dragged_to_another_day_in_the_month_keeps_its_time() {
    let mut state = state(View::Month);
    let (messages, outputs) = drive(&mut state, view, |ui| {
        let from = centre_of(ui, entry_id(EntryId(1), at(17, 9, 0)));
        let to = centre_of(ui, day_id(day(20)));
        drag(ui, from, to);
    });
    assert_eq!(
        events(&messages),
        vec![Event::MoveDays {
            entry: EntryId(1),
            start: at(17, 9, 0),
            days: 3
        }]
    );
    assert!(matches!(outputs.as_slice(), [Output::Changed(_)]));
    assert_eq!(
        when_of(&state, 1),
        Some(When::timed(at(20, 9, 0), at(20, 10, 0)))
    );
}

#[test]
fn a_whole_day_entry_dragged_across_weeks_in_the_month_moves_by_days() {
    let mut state = state(View::Month);
    let (messages, _) = drive(&mut state, view, |ui| {
        let from = centre_of(ui, entry_id(EntryId(3), midnight(day(18), &TimeZone::UTC)));
        let to = centre_of(ui, day_id(day(26)));
        drag(ui, from, to);
    });
    assert_eq!(
        events(&messages),
        vec![Event::MoveDays {
            entry: EntryId(3),
            start: midnight(day(18), &TimeZone::UTC),
            days: 8
        }]
    );
    assert_eq!(when_of(&state, 3), Some(When::day(day(26))));
}

#[test]
fn a_whole_day_entry_dragged_along_the_strip_above_the_week_moves_by_days() {
    let mut state = state(View::Week);
    let (messages, _) = drive(&mut state, view, |ui| {
        let from = centre_of(ui, entry_id(EntryId(3), midnight(day(18), &TimeZone::UTC)));
        let column = centre_of(ui, day_id(day(16)));
        drag(ui, from, Point::new(column.x, from.y));
    });
    assert_eq!(
        events(&messages),
        vec![Event::MoveDays {
            entry: EntryId(3),
            start: midnight(day(18), &TimeZone::UTC),
            days: -2
        }]
    );
    assert_eq!(when_of(&state, 3), Some(When::day(day(16))));
}

#[test]
fn a_bar_let_go_where_it_was_picked_up_is_only_a_press() {
    let mut state = state(View::Month);
    let (messages, _) = drive(&mut state, view, |ui| {
        let at_bar = centre_of(ui, entry_id(EntryId(1), at(17, 9, 0)));
        click_at(ui, at_bar);
    });
    assert_eq!(
        events(&messages),
        vec![Event::Press(Selection::Entry {
            entry: EntryId(1),
            start: at(17, 9, 0)
        })]
    );
}

#[test]
fn escape_puts_a_bar_back_and_the_release_does_nothing() {
    let mut state = state(View::Month);
    let (messages, _) = drive(&mut state, view, |ui| {
        let from = centre_of(ui, entry_id(EntryId(1), at(17, 9, 0)));
        let to = centre_of(ui, day_id(day(20)));
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
    assert_eq!(events(&messages), Vec::new());
    assert_eq!(
        when_of(&state, 1),
        Some(When::timed(at(17, 9, 0), at(17, 10, 0)))
    );
}

#[test]
fn a_locked_entry_cannot_be_dragged_in_the_month_or_the_week() {
    let mut month = state(View::Month);
    let (messages, _) = drive(&mut month, view, |ui| {
        let from = centre_of(ui, entry_id(EntryId(4), at(23, 10, 0)));
        let to = centre_of(ui, day_id(day(25)));
        drag(ui, from, to);
    });
    assert!(
        !events(&messages)
            .iter()
            .any(|event| matches!(event, Event::MoveDays { .. })),
        "{messages:?}"
    );

    let mut week = state(View::Week).with_cursor(day(23));
    let (messages, _) = drive(&mut week, view, |ui| {
        let bounds = match ui.find(hours_id()) {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        let hour = bounds.height / 10.0;
        let from = point_at(bounds, 7, 0, 10 * 60 + 15, 8, hour);
        let to = point_at(bounds, 7, 2, 12 * 60 + 15, 8, hour);
        drag(ui, from, to);
    });
    assert!(
        !events(&messages)
            .iter()
            .any(|event| matches!(event, Event::Reschedule { .. })),
        "{messages:?}"
    );
    assert_eq!(
        when_of(&week, 4),
        Some(When::timed(at(23, 10, 0), at(23, 11, 0)))
    );
}

#[test]
fn the_point_helper_drives_a_drag_in_the_hours() {
    let mut state = state(View::Week);
    let (messages, _) = drive(&mut state, view, |ui| {
        let bounds = match ui.find(hours_id()) {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        let hour = bounds.height / 10.0;
        // Tuesday's stand-up, grabbed a quarter of the way down, to Friday.
        let from = point_at(bounds, 7, 1, 9 * 60 + 15, 8, hour);
        let to = point_at(bounds, 7, 4, 11 * 60 + 15, 8, hour);
        drag(ui, from, to);
    });
    assert_eq!(
        events(&messages),
        vec![Event::Reschedule {
            entry: EntryId(1),
            start: at(17, 9, 0),
            to: (at(20, 11, 0), at(20, 12, 0)),
        }]
    );
}

#[test]
fn with_proposals_a_drag_is_drawn_where_it_would_go_until_confirmed() {
    let mut state = state(View::Month).with_proposals(true);
    let (_, outputs) = drive(&mut state, view, |ui| {
        let from = centre_of(ui, entry_id(EntryId(1), at(17, 9, 0)));
        let to = centre_of(ui, day_id(day(20)));
        drag(ui, from, to);
    });
    let [Output::Proposed(proposed)] = outputs.as_slice() else {
        panic!("one proposal, not {outputs:?}");
    };
    assert_eq!(proposed.change, Change::Updated);
    assert_eq!(
        when_of(&state, 1),
        Some(When::timed(at(17, 9, 0), at(17, 10, 0)))
    );

    // The preview is drawn on the new day, and nothing is left on the old.
    let mut ui = Simulator::with_size(Settings::default(), DESKTOP, view(&state));
    assert!(ui.find(entry_id(EntryId(1), at(20, 9, 0))).is_ok());
    assert!(ui.find(entry_id(EntryId(1), at(17, 9, 0))).is_err());
    drop(ui);

    assert!(matches!(
        state.update(Event::Confirm),
        Some(Output::Changed(_))
    ));
    assert_eq!(
        when_of(&state, 1),
        Some(When::timed(at(20, 9, 0), at(20, 10, 0)))
    );
}

#[test]
fn a_discarded_proposal_is_drawn_back_where_it_was() {
    let mut state = state(View::Month).with_proposals(true);
    let _ = drive(&mut state, view, |ui| {
        let from = centre_of(ui, entry_id(EntryId(1), at(17, 9, 0)));
        let to = centre_of(ui, day_id(day(20)));
        drag(ui, from, to);
    });
    assert_eq!(state.update(Event::Discard), None);
    let mut ui = Simulator::with_size(Settings::default(), DESKTOP, view(&state));
    assert!(ui.find(entry_id(EntryId(1), at(17, 9, 0))).is_ok());
    assert!(ui.find(entry_id(EntryId(1), at(20, 9, 0))).is_err());
}

#[test]
fn a_double_click_opens_an_entry_and_a_single_click_only_selects() {
    let mut state = state(View::Month).with_activate_on_second_press(false);
    let (messages, outputs) = drive(&mut state, view, |ui| {
        let chip = centre_of(ui, entry_id(EntryId(2), at(19, 14, 0)));
        click_at(ui, chip);
        click_at(ui, chip);
    });
    let selection = Selection::Entry {
        entry: EntryId(2),
        start: at(19, 14, 0),
    };
    assert_eq!(
        events(&messages),
        vec![Event::Press(selection), Event::DoublePress(selection)]
    );
    assert_eq!(
        outputs,
        vec![
            Output::Selected(Some(selection)),
            Output::Opened {
                entry: EntryId(2),
                start: at(19, 14, 0)
            }
        ]
    );

    // A later single press on the selected entry, in a fresh view as after
    // closing a dialog, does not open it again.
    let (messages, outputs) = drive(&mut state, view, |ui| {
        let chip = centre_of(ui, entry_id(EntryId(2), at(19, 14, 0)));
        click_at(ui, chip);
    });
    assert_eq!(events(&messages), vec![Event::Press(selection)]);
    assert_eq!(outputs, Vec::new());
}

#[test]
fn a_double_click_on_a_day_or_a_time_asks_for_a_new_entry() {
    let mut month = state(View::Month).with_activate_on_second_press(false);
    let (messages, outputs) = drive(&mut month, view, |ui| {
        let cell = centre_of(ui, day_id(day(25)));
        click_at(ui, cell);
        click_at(ui, cell);
    });
    assert_eq!(
        events(&messages),
        vec![
            Event::Press(Selection::Day(day(25))),
            Event::DoublePress(Selection::Day(day(25)))
        ]
    );
    assert!(matches!(outputs.last(), Some(Output::Requested(_))));

    let mut week = state(View::Week).with_activate_on_second_press(false);
    let (messages, outputs) = drive(&mut week, view, |ui| {
        let bounds = match ui.find(hours_id()) {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        let hour = bounds.height / 10.0;
        let slot = point_at(bounds, 7, 3, 11 * 60 + 10, 8, hour);
        click_at(ui, slot);
        click_at(ui, slot);
    });
    let slot = Selection::Slot {
        date: day(19),
        minute: 11 * 60,
    };
    assert_eq!(
        events(&messages),
        vec![Event::Press(slot), Event::DoublePress(slot)]
    );
    assert!(matches!(outputs.last(), Some(Output::Requested(_))));
}

#[test]
fn the_header_takes_the_apps_own_controls_and_its_buttons_have_ids() {
    fn with_new(state: &State) -> Element<'_, Message> {
        calendar(state)
            .header_leading(button("New entry").on_press(Message::New))
            .header_trailing(widget::text("Team"))
            .on_event(Message::Calendar)
            .into()
    }
    let mut state = state(View::Month);
    let (messages, _) = drive(&mut state, with_new, |ui| {
        for id in [today_button_id(), previous_button_id(), view_menu_id()] {
            assert!(ui.find(id.clone()).is_ok(), "{id:?}");
        }
        assert!(ui.find("Team").is_ok());
        let new = match ui.find("New entry") {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        let today = match ui.find(today_button_id()) {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        assert!(new.x < today.x, "the app's control comes first");
        let _ = ui.click("New entry");
        let next = centre_of(ui, next_button_id());
        click_at(ui, next);
    });
    assert_eq!(messages, vec![Message::New, Message::Calendar(Event::Next)]);
}

#[test]
fn an_entry_icon_says_what_it_means_on_hover() {
    let mut state = state(View::Month);
    let _ = state.update(Event::Insert(
        Entry::new(EntryId(9), "Planning", at(25, 10, 0), at(25, 11, 0))
            .icon(iced_cube::lucide!(Users))
            .icon_label("Team entry"),
    ));
    let mut ui = Simulator::with_size(Settings::default(), DESKTOP, view(&state));
    let chip = match ui.find(entry_id(EntryId(9), at(25, 10, 0))) {
        Ok(target) => target.bounds(),
        Err(error) => panic!("{error:?}"),
    };
    assert!(ui.find("Team entry").is_err(), "hidden until hovered");
    // The icon follows the dot at the start of the line.
    move_to(&mut ui, Point::new(chip.x + 17.0, chip.center_y()));
    assert!(ui.find("Team entry").is_ok());
}

#[test]
fn a_whole_day_week_opens_at_the_start_hour() {
    fn at_seven(state: &State) -> Element<'_, Message> {
        calendar(state)
            .start_at_hour(7)
            .on_event(Message::Calendar)
            .into()
    }
    let mut state = state(View::Week);
    let (messages, _) = drive(&mut state, at_seven, |ui| {
        // Any event lets the hours scroll; then the top of Thursday's
        // column is pressed.
        let heading = match ui.find(day_id(day(19))) {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        move_to(
            ui,
            Point::new(heading.center_x(), heading.y + heading.height + 40.0),
        );
        let strip = match ui.find("All day") {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        click_at(
            ui,
            Point::new(heading.center_x(), strip.y + strip.height + 16.0),
        );
    });
    assert_eq!(
        events(&messages),
        vec![Event::Press(Selection::Slot {
            date: day(19),
            minute: 7 * 60
        })]
    );
}
