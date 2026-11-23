//! Drives the calendar examples from start to finish, as someone using
//! them would: every message a press, drag or key sends goes back through
//! the example's `update` before the next step is drawn.

use gallery::app::Mode;
use gallery::{FONT, FONT_FILES, Gallery, ThemeChoice};
use iced::keyboard::key::Named;
use iced::{Event as Input, Point, Size, mouse};
use iced_cube::data::calendar::editor::notes_id;
use iced_test::simulator::Simulator;

/// A docs preview on a desktop.
const DESKTOP: Size = Size::new(720.0, 480.0);

type Ui<'a> = Simulator<'a, gallery::Message>;

fn open(id: &str) -> Gallery {
    iced_cube::theme::set_font(FONT);
    let gallery = Gallery::new(Mode::Single, Some(id), ThemeChoice::Light);
    assert_eq!(gallery.selected(), id, "the story exists");
    gallery
}

fn render(gallery: &Gallery) -> Ui<'_> {
    let settings = iced::Settings {
        default_font: FONT,
        fonts: FONT_FILES.iter().map(|font| (*font).into()).collect(),
        ..iced::Settings::default()
    };
    Simulator::with_size(settings, DESKTOP, gallery.view())
}

/// Runs `actions` against what is on screen, then lets the example handle
/// everything they sent.
fn step(gallery: &mut Gallery, actions: impl FnOnce(&mut Ui<'_>)) {
    let messages: Vec<gallery::Message> = {
        let mut ui = render(gallery);
        actions(&mut ui);
        ui.into_messages().collect()
    };
    for message in messages {
        let _ = gallery.update(message);
    }
}

/// Whether `label` is on screen right now.
fn shows(gallery: &Gallery, label: &str) -> bool {
    render(gallery).find(label).is_ok()
}

#[track_caller]
fn assert_shows(gallery: &Gallery, labels: &[&str]) {
    for label in labels {
        assert!(shows(gallery, label), "{label} should be on screen");
    }
}

#[track_caller]
fn assert_hides(gallery: &Gallery, labels: &[&str]) {
    for label in labels {
        assert!(!shows(gallery, label), "{label} should not be on screen");
    }
}

#[track_caller]
fn centre(ui: &mut Ui<'_>, label: &str) -> Point {
    match ui.find(label) {
        Ok(target) => target.bounds().center(),
        Err(error) => panic!("{label} is not rendered: {error:?}"),
    }
}

#[track_caller]
fn click(ui: &mut Ui<'_>, label: &str) {
    if let Err(error) = ui.click(label) {
        panic!("{label} cannot be clicked: {error:?}");
    }
}

fn move_to(ui: &mut Ui<'_>, point: Point) {
    ui.point_at(point);
    let _ = ui.simulate([Input::Mouse(mouse::Event::CursorMoved { position: point })]);
}

fn drag(ui: &mut Ui<'_>, from: Point, to: Point) {
    move_to(ui, from);
    let _ = ui.simulate([Input::Mouse(mouse::Event::ButtonPressed(
        mouse::Button::Left,
    ))]);
    // Part of the way first, as a pointer travels.
    move_to(ui, Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0));
    move_to(ui, to);
    let _ = ui.simulate([Input::Mouse(mouse::Event::ButtonReleased(
        mouse::Button::Left,
    ))]);
}

/// A point in the hours of the day column under the header `day`, such as
/// "FRI", worked out from where the hour labels are drawn.
#[track_caller]
fn slot(ui: &mut Ui<'_>, day: &str, hour: f32) -> Point {
    let ten = centre(ui, "10:00").y;
    let eleven = centre(ui, "11:00").y;
    Point::new(centre(ui, day).x, ten + (hour - 10.0) * (eleven - ten))
}

/// Turns the wheel over `label`, so what is below it comes into view.
#[track_caller]
fn scroll_down_from(ui: &mut Ui<'_>, label: &str) {
    let point = centre(ui, label);
    move_to(ui, point);
    let _ = ui.simulate([Input::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -400.0 },
    })]);
}

/// Opens the menu showing `current` and picks `option` from it.
#[track_caller]
fn choose(gallery: &mut Gallery, current: &str, option: &str) {
    step(gallery, |ui| {
        click(ui, current);
        click(ui, option);
    });
}

/// Drags out an entry on Friday from 11:15 to 12:15 in the week of the
/// creating example, which opens the form. Quarter-past times are only
/// in the form: the week behind it labels the hours.
fn drag_out_friday(gallery: &mut Gallery) {
    step(gallery, |ui| {
        let from = slot(ui, "FRI", 11.25);
        let to = slot(ui, "FRI", 12.25);
        drag(ui, from, to);
    });
}

/// Opens the form for an entry already on the calendar: a press selects
/// it and a second press opens it.
fn open_entry(gallery: &mut Gallery, title: &str) {
    step(gallery, |ui| click(ui, title));
    step(gallery, |ui| click(ui, title));
}

#[test]
fn a_dragged_out_entry_is_named_and_saved() {
    let mut gallery = open("calendar/create");
    assert_hides(&gallery, &["New entry", "Retro"]);

    drag_out_friday(&mut gallery);
    assert_shows(
        &gallery,
        &[
            "New entry",
            "Add a title",
            "Fri 19 Jun 2026",
            "11:15",
            "12:15",
            "Does not repeat",
            "Cancel",
            "Save",
        ],
    );
    assert_hides(&gallery, &["Delete"]);

    step(&mut gallery, |ui| {
        click(ui, "Add a title");
        let _ = ui.typewrite("Retro");
    });
    step(&mut gallery, |ui| click(ui, "Save"));
    assert_hides(&gallery, &["New entry", "Add a title"]);
    assert_shows(&gallery, &["Retro", "Stand-up", "Design review"]);

    // It opens again with what was saved.
    open_entry(&mut gallery, "Retro");
    assert_shows(
        &gallery,
        &["Edit entry", "Fri 19 Jun 2026", "11:15", "12:15", "Delete"],
    );
}

#[test]
fn enter_in_the_title_saves_and_a_blank_title_gets_a_name() {
    let mut gallery = open("calendar/create");
    drag_out_friday(&mut gallery);
    step(&mut gallery, |ui| {
        click(ui, "Add a title");
        let _ = ui.typewrite("Retro");
        let _ = ui.tap_key(Named::Enter);
    });
    assert_hides(&gallery, &["New entry"]);
    assert_shows(&gallery, &["Retro"]);

    // A second one, saved without a title.
    step(&mut gallery, |ui| {
        let from = slot(ui, "MON", 11.0);
        let to = slot(ui, "MON", 12.0);
        drag(ui, from, to);
    });
    step(&mut gallery, |ui| click(ui, "Save"));
    assert_shows(&gallery, &["(No title)", "Retro"]);
}

#[test]
fn cancelling_or_escape_adds_nothing() {
    let mut gallery = open("calendar/create");
    drag_out_friday(&mut gallery);
    step(&mut gallery, |ui| {
        click(ui, "Add a title");
        let _ = ui.typewrite("Retro");
    });
    step(&mut gallery, |ui| click(ui, "Cancel"));
    assert_hides(&gallery, &["New entry", "Retro", "(No title)"]);

    drag_out_friday(&mut gallery);
    assert_shows(&gallery, &["New entry"]);
    // One Escape closes the dialog, even with the title focused.
    step(&mut gallery, |ui| {
        let _ = ui.tap_key(Named::Escape);
    });
    assert_hides(&gallery, &["New entry", "(No title)"]);
}

#[test]
fn the_start_and_end_are_changed_in_the_form() {
    let mut gallery = open("calendar/create");
    drag_out_friday(&mut gallery);
    step(&mut gallery, |ui| {
        click(ui, "Add a title");
        let _ = ui.typewrite("Retro");
    });

    // A later start carries the end with it.
    choose(&mut gallery, "11:15", "11:45");
    assert_shows(&gallery, &["11:45", "12:45"]);
    assert_hides(&gallery, &["11:15", "12:15"]);

    // A new end changes the length.
    choose(&mut gallery, "12:45", "13:30");
    assert_shows(&gallery, &["11:45", "13:30"]);
    assert_hides(&gallery, &["12:45"]);

    // Another day, from the month under the start date.
    step(&mut gallery, |ui| click(ui, "Fri 19 Jun 2026"));
    assert_shows(&gallery, &["June 2026"]);
    // A day of next week, which the week behind the dialog does not show.
    step(&mut gallery, |ui| click(ui, "24"));
    assert_hides(&gallery, &["June 2026", "Fri 19 Jun 2026"]);
    assert_shows(&gallery, &["Wed 24 Jun 2026"]);

    step(&mut gallery, |ui| click(ui, "Save"));
    assert_hides(&gallery, &["Retro"]);
    choose(&mut gallery, "Week", "Month");
    open_entry(&mut gallery, "11:45 Retro");
    assert_shows(
        &gallery,
        &["Edit entry", "Wed 24 Jun 2026", "11:45", "13:30"],
    );
}

#[test]
fn an_existing_entry_is_changed_and_then_deleted() {
    let mut gallery = open("calendar/create");
    open_entry(&mut gallery, "Design review");
    assert_shows(
        &gallery,
        &[
            "Edit entry",
            "Design review",
            "Thu 18 Jun 2026",
            "15:30",
            "Delete",
        ],
    );

    choose(&mut gallery, "Default", "Green");
    choose(&mut gallery, "15:30", "16:15");
    step(&mut gallery, |ui| click(ui, "Save"));
    assert_hides(&gallery, &["Edit entry"]);

    open_entry(&mut gallery, "Design review");
    assert_shows(&gallery, &["16:15", "Green"]);
    assert_hides(&gallery, &["15:30", "Default"]);

    step(&mut gallery, |ui| click(ui, "Delete"));
    assert_hides(&gallery, &["Edit entry", "Design review"]);
    assert_shows(&gallery, &["Stand-up"]);
}

#[test]
fn an_all_day_entry_has_dates_and_no_times() {
    let mut gallery = open("calendar/create");
    drag_out_friday(&mut gallery);
    step(&mut gallery, |ui| {
        click(ui, "Add a title");
        let _ = ui.typewrite("Offsite");
    });
    step(&mut gallery, |ui| click(ui, "All day"));
    assert_hides(&gallery, &["11:15", "12:15"]);
    assert_shows(&gallery, &["Fri 19 Jun 2026"]);

    step(&mut gallery, |ui| click(ui, "Save"));
    assert_shows(&gallery, &["Offsite"]);
    open_entry(&mut gallery, "Offsite");
    assert_shows(&gallery, &["Edit entry", "Fri 19 Jun 2026"]);
    assert_hides(&gallery, &["11:15"]);
}

#[test]
fn a_repeat_with_an_interval_and_an_end_is_saved_and_read_back() {
    let mut gallery = open("calendar/create");
    step(&mut gallery, |ui| {
        let from = slot(ui, "MON", 11.0);
        let to = slot(ui, "MON", 12.0);
        drag(ui, from, to);
    });
    step(&mut gallery, |ui| {
        click(ui, "Add a title");
        let _ = ui.typewrite("Sync");
    });
    assert_hides(&gallery, &["Every", "Stops"]);

    choose(&mut gallery, "Does not repeat", "Daily");
    // The fields for the repeat are below the fold of the dialog.
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        click(ui, "1");
        let _ = ui.tap_key(Named::End);
        let _ = ui.tap_key(Named::Backspace);
        let _ = ui.typewrite("3");
    });
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        assert!(ui.find("days").is_ok(), "every 3 days");
        click(ui, "Never");
        click(ui, "After");
    });
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        assert!(ui.find("times").is_ok());
        click(ui, "10");
        let _ = ui.tap_key(Named::End);
        let _ = ui.tap_key(Named::Backspace);
        let _ = ui.tap_key(Named::Backspace);
        let _ = ui.typewrite("2");
    });
    step(&mut gallery, |ui| click(ui, "Save"));

    // Monday and Thursday, and not a third time on Sunday.
    step(&mut gallery, |ui| {
        let sync = centre(ui, "Sync");
        let monday = centre(ui, "MON");
        assert!((sync.x - monday.x).abs() < 60.0, "the first is on Monday");
    });
    choose(&mut gallery, "Week", "Day");
    assert_shows(&gallery, &["Sync", "Design review"]);

    open_entry(&mut gallery, "Sync");
    assert_shows(&gallery, &["Edit entry", "Daily"]);
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        for label in ["Every", "3", "days", "After", "2", "times"] {
            assert!(ui.find(label).is_ok(), "{label}");
        }
    });
}

#[test]
fn a_repeat_stops_on_a_date_picked_in_the_form() {
    let mut gallery = open("calendar/create");
    open_entry(&mut gallery, "Stand-up");
    choose(&mut gallery, "Does not repeat", "Weekly");
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        click(ui, "Never");
        click(ui, "On a date");
    });
    // The date starts a month after the entry's own.
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        click(ui, "Thu 16 Jul 2026");
    });
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        assert!(ui.find("July 2026").is_ok());
        click(ui, "30");
    });
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        assert!(ui.find("Thu 30 Jul 2026").is_ok());
        assert!(ui.find("July 2026").is_err(), "the picker closed");
    });
    step(&mut gallery, |ui| click(ui, "Save"));

    open_entry(&mut gallery, "Stand-up");
    assert_shows(&gallery, &["Weekly"]);
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        for label in ["On a date", "Thu 30 Jul 2026", "week"] {
            assert!(ui.find(label).is_ok(), "{label}");
        }
    });
}

#[test]
fn location_and_notes_are_saved_and_read_back() {
    let mut gallery = open("calendar/create");
    drag_out_friday(&mut gallery);
    step(&mut gallery, |ui| {
        click(ui, "Add a title");
        let _ = ui.typewrite("Retro");
    });
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        click(ui, "Add a location");
        let _ = ui.typewrite("Room 4");
    });
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        assert!(ui.find("Room 4").is_ok());
        if let Err(error) = ui.click(notes_id()) {
            panic!("the notes cannot be clicked: {error:?}");
        }
        let _ = ui.typewrite("Bring the plan");
    });
    step(&mut gallery, |ui| click(ui, "Save"));
    assert_shows(&gallery, &["Retro"]);

    // Text in the notes is not something a selector finds, so only the
    // location is read back here. The form's own tests read the notes.
    open_entry(&mut gallery, "Retro");
    step(&mut gallery, |ui| {
        scroll_down_from(ui, "Starts");
        assert!(ui.find("Room 4").is_ok());
        assert!(ui.find("Add a location").is_err());
    });
}

#[test]
fn a_read_only_calendar_changes_only_through_the_apps_own_controls() {
    let mut gallery = open("calendar/read-only");
    assert_shows(
        &gallery,
        &["14:00 Design review", "Conference", "Add", "Remove"],
    );

    // Pressing a day twice would ask for an entry on a calendar that
    // allowed it. Here it only selects the day.
    step(&mut gallery, |ui| click(ui, "24"));
    step(&mut gallery, |ui| click(ui, "24"));
    step(&mut gallery, |ui| {
        click(ui, "Add to the selected day");
        let _ = ui.typewrite("Dentist");
    });
    step(&mut gallery, |ui| click(ui, "Add"));
    assert_shows(&gallery, &["Dentist", "Add to the selected day"]);
    step(&mut gallery, |ui| {
        let dentist = centre(ui, "Dentist");
        let day = centre(ui, "24");
        assert!((dentist.x - day.x).abs() < 60.0, "on the selected day");
        assert!(dentist.y > day.y);
    });

    // Remove takes out the entry that is selected.
    step(&mut gallery, |ui| click(ui, "14:00 Design review"));
    step(&mut gallery, |ui| click(ui, "Remove"));
    assert_hides(&gallery, &["14:00 Design review"]);
    assert_shows(&gallery, &["Dentist", "Conference"]);
}

#[test]
fn the_header_changes_the_view_and_the_period() {
    let mut gallery = open("calendar/month");
    assert_shows(&gallery, &["June 2026", "Month", "Today"]);

    for (view, title) in [
        ("Week", "15 - 21 Jun 2026"),
        ("Year", "2026"),
        ("Schedule", "June 2026"),
        ("Month", "June 2026"),
    ] {
        // The menu shows the view it is on, which opens it.
        step(&mut gallery, |ui| {
            let current = ["Month", "Week", "Year", "Schedule", "Day", "4 days"]
                .into_iter()
                .find(|name| ui.find(*name).is_ok());
            let Some(current) = current else {
                panic!("the menu of views is not rendered");
            };
            click(ui, current);
            click(ui, view);
        });
        assert_shows(&gallery, &[view, title]);
    }
}

#[test]
fn missed_reminders_are_flagged_dismissed_and_snoozed() {
    let mut gallery = open("calendar/missed");
    assert_shows(
        &gallery,
        &[
            "It is Thursday 09:30.",
            "Retro",
            "Dentist",
            "Stand-up",
            "Missed, 17 h 30 min late",
            "Missed, 1 h late",
            "Missed, 30 min late",
        ],
    );
    assert_hides(&gallery, &["Design review"]);

    // The oldest goes for good.
    step(&mut gallery, |ui| click(ui, "Dismiss"));
    assert_hides(&gallery, &["Retro", "Missed, 17 h 30 min late"]);

    // The next is put off for ten minutes.
    step(&mut gallery, |ui| click(ui, "Snooze 10 minutes"));
    assert_hides(&gallery, &["Dentist"]);
    step(&mut gallery, |ui| click(ui, "Wait 5 minutes"));
    assert_shows(&gallery, &["It is Thursday 09:35."]);
    assert_hides(&gallery, &["Dentist", "Design review"]);

    // Ten minutes on, it is back, with the one that came due meanwhile.
    step(&mut gallery, |ui| click(ui, "Wait 5 minutes"));
    assert_shows(
        &gallery,
        &[
            "It is Thursday 09:40.",
            "Dentist",
            "Design review",
            "On time",
        ],
    );
}

#[test]
fn custom_triggers_report_changes_and_coming_entries() {
    let mut gallery = open("calendar/triggers");
    assert_shows(
        &gallery,
        &["Press a time twice to add an entry, or an entry twice to remove it."],
    );

    // An entry pressed twice is removed, and the rule for removals fires.
    step(&mut gallery, |ui| click(ui, "Stand-up"));
    step(&mut gallery, |ui| click(ui, "Stand-up"));
    assert_shows(&gallery, &["09:40  Stand-up was removed"]);
    assert_hides(&gallery, &["Stand-up"]);

    // A time pressed twice adds an entry, and the rule for new ones fires.
    for _ in 0..2 {
        step(&mut gallery, |ui| {
            let column = centre(ui, "Lunch with Ada").x;
            let half_past_eleven = slot(ui, "Lunch with Ada", 11.5).y;
            move_to(ui, Point::new(column, half_past_eleven));
            let _ = ui.simulate(iced_test::simulator::click());
        });
    }
    assert_shows(&gallery, &["New entry", "09:40  New entry was created"]);

    // Time passing fires the rules measured from where an entry starts:
    // the release nags from a quarter of an hour before it.
    for _ in 0..10 {
        step(&mut gallery, |ui| click(ui, "Wait 5 minutes"));
    }
    assert_shows(&gallery, &["It is 10:30.", "10:30  Release is coming up"]);
    step(&mut gallery, |ui| click(ui, "Wait 5 minutes"));
    assert_shows(&gallery, &["10:35  Release is coming up"]);
}
