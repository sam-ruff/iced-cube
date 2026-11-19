#![cfg(feature = "calendar")]

use iced::{Element, Point, Settings, Size};
use iced_cube::data::calendar::editor::{Event, Field, Repeats, State, editor};
use iced_cube::data::calendar::{Draft, Entry, EntryId, Tone, When, instant};
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
    let starts = match ui.find("Starts") {
        Ok(target) => target.bounds(),
        Err(error) => panic!("{error:?}"),
    };
    ui.point_at(Point::new(16.0, starts.center_y() - 41.0));
    let _ = ui.simulate(click());
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

#[test]
fn an_existing_entry_fills_the_form() {
    let entry = Entry::new(EntryId(4), "Review", at(15, 0), at(16, 0)).tone(Tone::Warning);
    let state = State::edit(&entry, &TimeZone::UTC);
    let mut ui = Simulator::with_size(Settings::default(), DIALOG, view(&state));
    for label in ["Review", "15:00", "16:00", "Amber"] {
        assert!(shows(&mut ui, label), "{label}");
    }
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
    let state = form();
    let mut ui = Simulator::with_size(Settings::default(), PHONE, view(&state));
    for label in [
        "Starts",
        "Ends",
        "11:00",
        "12:30",
        "Does not repeat",
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
}
