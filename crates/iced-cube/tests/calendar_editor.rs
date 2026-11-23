#![cfg(feature = "calendar")]

use iced::{Element, Point, Settings, Size};
use iced_cube::data::calendar::editor::{Event, Field, Repeats, State, Stops, editor, notes_id};
use iced_cube::data::calendar::{Draft, Entry, EntryId, Repeat, Tone, When, instant};
use iced_test::simulator::{Simulator, click};
use jiff::Timestamp;
use jiff::civil::{date, time};
use jiff::tz::TimeZone;

/// About the inside of a dialog.
const DIALOG: Size = Size::new(420.0, 520.0);
/// The inside of a dialog on a 360 pixel phone.
const PHONE: Size = Size::new(296.0, 520.0);

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Form(Event),
    Save,
}

/// A time on Friday 19 June 2026, UTC.
fn at(hour: i8, minute: i8) -> Timestamp {
    instant(date(2026, 6, 19), time(hour, minute, 0, 0), &TimeZone::UTC)
}

fn form() -> State {
    let draft = Draft {
        when: When::timed(at(11, 0), at(12, 30)),
    };
    State::new(&draft, &TimeZone::UTC)
}

fn view(state: &State) -> Element<'_, Message> {
    editor(state)
        .on_event(Message::Form)
        .on_submit(Message::Save)
        .into()
}

/// Runs `actions` against a view of `state`, then applies what it sent.
fn drive(
    state: &mut State,
    size: Size,
    actions: impl FnOnce(&mut Simulator<'_, Message>),
) -> Vec<Message> {
    let messages: Vec<Message> = {
        let mut ui = Simulator::with_size(Settings::default(), size, view(state));
        actions(&mut ui);
        ui.into_messages().collect()
    };
    for message in &messages {
        if let Message::Form(event) = message {
            state.update(event.clone());
        }
    }
    messages
}

fn shows(ui: &mut Simulator<'_, Message>, label: &str) -> bool {
    ui.find(label).is_ok()
}

/// Presses the track of the "All day" switch, which sits at the left edge
/// in the row above "Starts". A switch's label is not text a selector can
/// find.
fn toggle_all_day(ui: &mut Simulator<'_, Message>) {
    if let Err(error) = ui.click("All day") {
        panic!("{error:?}");
    }
}

#[test]
fn the_form_shows_the_span_it_was_given() {
    let state = form();
    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    for label in [
        "Add a title",
        "Starts",
        "Ends",
        "Fri 19 Jun 2026",
        "11:00",
        "12:30",
        "Does not repeat",
        "Default",
    ] {
        assert!(shows(&mut ui, label), "{label}");
    }
    assert!(!shows(&mut ui, "June 2026"), "the date picker is closed");
}

#[test]
fn typing_a_title_and_pressing_enter_saves() {
    let mut state = form();
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Add a title");
        let _ = ui.typewrite("Retro");
        let _ = ui.tap_key(iced::keyboard::key::Named::Enter);
    });
    assert_eq!(
        messages.last(),
        Some(&Message::Save),
        "Enter in the title submits"
    );
    assert!(
        messages.contains(&Message::Form(Event::Title(String::from("R")))),
        "{messages:?}"
    );
}

#[test]
fn a_time_is_chosen_from_its_menu_and_the_end_follows_the_start() {
    let mut state = form();
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("11:00");
        assert!(shows(ui, "11:15"), "the menu lists the times");
        let _ = ui.click("11:45");
    });
    assert_eq!(
        messages,
        vec![Message::Form(Event::Time(Field::Start, 705))]
    );
    assert_eq!(state.time(Field::End), 795, "still an hour and a half long");

    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    assert!(shows(&mut ui, "11:45"));
    assert!(shows(&mut ui, "13:15"));
}

#[test]
fn a_date_is_picked_from_a_month_that_can_be_turned() {
    let mut state = form();
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Fri 19 Jun 2026");
    });
    assert_eq!(
        messages,
        vec![Message::Form(Event::Picker(Some(Field::Start)))]
    );
    assert_eq!(state.picker(), Some(Field::Start));

    let messages = drive(&mut state, DIALOG, |ui| {
        assert!(shows(ui, "June 2026"));
        let _ = ui.click("22");
    });
    assert_eq!(
        messages,
        vec![Message::Form(Event::Date(Field::Start, date(2026, 6, 22)))]
    );
    assert_eq!(state.picker(), None, "choosing a day closes the picker");
    assert_eq!(state.date(Field::End), date(2026, 6, 22), "the end follows");

    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    assert!(shows(&mut ui, "Mon 22 Jun 2026"));
    assert!(!shows(&mut ui, "June 2026"));
}

#[test]
fn all_day_hides_the_times() {
    let mut state = form();
    let messages = drive(&mut state, DIALOG, |ui| {
        toggle_all_day(ui);
    });
    assert_eq!(messages, vec![Message::Form(Event::AllDay(true))]);

    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    assert!(!shows(&mut ui, "11:00"));
    assert!(!shows(&mut ui, "12:30"));
    assert!(shows(&mut ui, "Fri 19 Jun 2026"));
    assert_eq!(state.when(), When::day(date(2026, 6, 19)));
}

#[test]
fn repeat_and_colour_come_from_their_menus() {
    let mut state = form();
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Does not repeat");
        let _ = ui.click("Weekly");
    });
    assert_eq!(
        messages,
        vec![Message::Form(Event::Repeats(Repeats::Weekly))]
    );
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Default");
        let _ = ui.click("Green");
    });
    assert_eq!(messages, vec![Message::Form(Event::Tone(Tone::Success))]);

    let entry = state.entry(EntryId(3));
    assert_eq!(entry.tone_of(), Tone::Success);
    assert!(entry.repeat_of().is_some());
}

/// A form that repeats weekly.
fn weekly() -> State {
    let mut state = form();
    state.update(Event::Repeats(Repeats::Weekly));
    state
}

#[test]
fn the_repeat_fields_show_only_while_the_entry_repeats() {
    let mut state = form();
    {
        let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
        for label in ["Every", "Stops", "week"] {
            assert!(!shows(&mut ui, label), "{label}");
        }
    }
    let _ = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Does not repeat");
        let _ = ui.click("Monthly");
    });
    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    for label in ["Every", "month", "Stops", "Never"] {
        assert!(shows(&mut ui, label), "{label}");
    }
    assert!(
        !shows(&mut ui, "times"),
        "no count until it stops after one"
    );
}

#[test]
fn the_interval_is_typed_and_names_its_unit() {
    let mut state = weekly();
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("1");
        let _ = ui.tap_key(iced::keyboard::key::Named::End);
        let _ = ui.typewrite("2");
    });
    assert_eq!(
        messages,
        vec![Message::Form(Event::Interval(String::from("12")))]
    );
    assert_eq!(state.interval(), 12);
    assert_eq!(
        state.entry(EntryId(1)).repeat_of(),
        Some(Repeat::weekly().interval(12))
    );

    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    assert!(shows(&mut ui, "weeks"), "more than one week");
    assert!(!shows(&mut ui, "week"));
}

#[test]
fn letters_typed_into_the_interval_are_dropped() {
    let mut state = weekly();
    let _ = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("1");
        let _ = ui.tap_key(iced::keyboard::key::Named::End);
        let _ = ui.typewrite("x");
    });
    assert_eq!(state.interval(), 1);
    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    assert!(!shows(&mut ui, "1x"));
}

#[test]
fn a_repeat_can_stop_after_a_number_of_times() {
    let mut state = weekly();
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Never");
        let _ = ui.click("After");
    });
    assert_eq!(messages, vec![Message::Form(Event::Stops(Stops::After))]);

    let messages = drive(&mut state, DIALOG, |ui| {
        assert!(shows(ui, "times"));
        let _ = ui.click("10");
        let _ = ui.tap_key(iced::keyboard::key::Named::End);
        let _ = ui.tap_key(iced::keyboard::key::Named::Backspace);
        let _ = ui.tap_key(iced::keyboard::key::Named::Backspace);
        let _ = ui.typewrite("6");
    });
    assert_eq!(
        messages.last(),
        Some(&Message::Form(Event::Count(String::from("6"))))
    );
    assert_eq!(
        state.entry(EntryId(1)).repeat_of(),
        Some(Repeat::weekly().count(6))
    );
}

#[test]
fn a_repeat_can_stop_on_a_date_picked_from_a_month() {
    let mut state = weekly();
    let _ = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Never");
        let _ = ui.click("On a date");
    });
    assert_eq!(state.stops(), Stops::On);

    // The date starts a month after the entry.
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Sun 19 Jul 2026");
    });
    assert_eq!(messages, vec![Message::Form(Event::UntilPicker(true))]);

    let messages = drive(&mut state, DIALOG, |ui| {
        // The arrow to the next month sits at the right of the month's name,
        // and has no text to find it by.
        let name = match ui.find("July 2026") {
            Ok(target) => target.bounds(),
            Err(error) => panic!("{error:?}"),
        };
        ui.point_at(Point::new(name.center_x() + 96.0, name.center_y()));
        let _ = ui.simulate(click());
    });
    assert_eq!(messages, vec![Message::Form(Event::PickerMonth(1))]);

    let messages = drive(&mut state, DIALOG, |ui| {
        assert!(shows(ui, "August 2026"));
        let _ = ui.click("14");
    });
    assert_eq!(
        messages,
        vec![Message::Form(Event::Until(date(2026, 8, 14)))]
    );
    assert!(
        !state.is_picking_until(),
        "choosing a day closes the picker"
    );
    assert_eq!(
        state.entry(EntryId(1)).repeat_of(),
        Some(Repeat::weekly().until(date(2026, 8, 14)))
    );

    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    assert!(shows(&mut ui, "Fri 14 Aug 2026"));
    assert!(!shows(&mut ui, "August 2026"));
}

#[test]
fn a_location_is_typed_and_enter_saves() {
    let mut state = form();
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Add a location");
        let _ = ui.typewrite("Room 4");
        let _ = ui.tap_key(iced::keyboard::key::Named::Enter);
    });
    assert_eq!(messages.last(), Some(&Message::Save));
    assert_eq!(state.location(), "Room 4");
    assert_eq!(state.entry(EntryId(1)).location_of(), Some("Room 4"));

    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    assert!(shows(&mut ui, "Room 4"));
}

#[test]
fn notes_are_typed_over_several_lines() {
    let mut state = form();
    let messages = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click(notes_id());
        let _ = ui.typewrite("Plan");
        let _ = ui.tap_key(iced::keyboard::key::Named::Enter);
        let _ = ui.typewrite("Figures");
    });
    assert!(
        !messages.contains(&Message::Save),
        "Enter in the notes starts a line"
    );
    assert_eq!(state.notes(), "Plan\nFigures");
    assert_eq!(state.entry(EntryId(1)).notes_of(), Some("Plan\nFigures"));
}

#[test]
fn an_existing_entry_fills_the_form() {
    let entry = Entry::new(EntryId(4), "Review", at(15, 0), at(16, 0))
        .tone(Tone::Warning)
        .repeat(Repeat::daily().interval(3).count(8))
        .location("Room 4")
        .notes("Bring the plan");
    let state = State::edit(&entry, &TimeZone::UTC);
    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    for label in [
        "Review", "15:00", "16:00", "Amber", "Daily", "3", "days", "After", "8", "times", "Room 4",
    ] {
        assert!(shows(&mut ui, label), "{label}");
    }
    assert_eq!(state.notes(), "Bring the plan");
    assert_eq!(state.entry(EntryId(9)), entry, "saving it changes nothing");
}

#[test]
fn a_whole_entry_is_filled_in_and_saved() {
    let mut state = form();
    let _ = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Add a title");
        let _ = ui.typewrite("Retro");
    });
    // Each menu is opened and chosen from in turn.
    for (shown, option) in [
        ("11:00", "11:30"),
        ("13:00", "12:15"),
        ("Does not repeat", "Weekly"),
        ("Never", "After"),
        ("Default", "Green"),
    ] {
        let _ = drive(&mut state, DIALOG, |ui| {
            let _ = ui.click(shown);
            let _ = ui.click(option);
        });
    }
    let _ = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click("Add a location");
        let _ = ui.typewrite("Room 4");
    });
    let _ = drive(&mut state, DIALOG, |ui| {
        let _ = ui.click(notes_id());
        let _ = ui.typewrite("Agenda");
    });

    let at = |hour, minute| instant(date(2026, 6, 19), time(hour, minute, 0, 0), &TimeZone::UTC);
    let expected = Entry::new(EntryId(5), "Retro", at(11, 30), at(12, 15))
        .tone(Tone::Success)
        .repeat(Repeat::weekly().count(10))
        .location("Room 4")
        .notes("Agenda");
    assert_eq!(state.entry(EntryId(5)), expected);
}

#[test]
fn a_form_without_a_handler_is_inert() {
    let state = form();
    let disabled: Element<'_, Message> = editor(&state).into();
    let mut ui = Simulator::with_size(Settings::default(), DIALOG, disabled);
    toggle_all_day(&mut ui);
    let _ = ui.click("Fri 19 Jun 2026");
    let _ = ui.click("11:00");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn the_form_fits_a_phone() {
    // The widest the form gets: a repeat that stops on a date.
    let mut state = weekly();
    state.update(Event::Interval(String::from("2")));
    state.update(Event::Stops(Stops::On));
    let mut ui = Simulator::with_size(Settings::default(), PHONE, view(&state));
    for label in [
        "Starts",
        "Ends",
        "11:00",
        "12:30",
        "Every",
        "weeks",
        "Stops",
        "On a date",
        "Sun 19 Jul 2026",
        "Default",
    ] {
        let Ok(target) = ui.find(label) else {
            panic!("{label} is not rendered");
        };
        let bounds = target.bounds();
        assert!(
            bounds.x >= 0.0 && bounds.x + bounds.width <= PHONE.width,
            "{label}: {bounds:?}"
        );
        assert!(bounds.height < 24.0, "{label} wraps: {bounds:?}");
    }
    let location = ui.find("Add a location").map(|target| target.bounds());
    let notes = ui.find(notes_id()).map(|target| target.bounds());
    for field in [location, notes] {
        let Ok(bounds) = field else {
            panic!("a text field is not rendered");
        };
        assert!(bounds.x >= 0.0 && bounds.x + bounds.width <= PHONE.width);
    }
}
