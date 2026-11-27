#![cfg(feature = "select")]

//! A select used to type values: the not-valid line, typed text winning
//! over the highlight, committing on leaving, presses passed through and
//! the editable field.

use iced::keyboard::key::Named;
use iced::widget::{button, row};
use iced::{Element, Event, Point, mouse};
use iced_cube::forms::select::{self, DEFAULT_EMPTY, HEIGHT};
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Time(String),
    Save,
}

/// Every half hour from `from` to the end of the day.
fn half_hours(from: u8) -> Vec<String> {
    (from..48)
        .map(|half| format!("{:02}:{:02}", half / 2, half % 2 * 30))
        .collect()
}

/// Reads "9", "0940" or "9:40" as a time of day.
fn parse(typed: &str) -> Option<String> {
    let (hours, minutes) = match typed.split_once(':') {
        Some((hours, minutes)) => (hours, minutes),
        None if typed.len() <= 2 => (typed, "00"),
        None if typed.len() == 4 => typed.split_at(2),
        None => return None,
    };
    if minutes.len() != 2 {
        return None;
    }
    let (hours, minutes): (u8, u8) = (hours.parse().ok()?, minutes.parse().ok()?);
    (hours < 24 && minutes < 60).then(|| format!("{hours:02}:{minutes:02}"))
}

fn time(typed: String) -> Option<Message> {
    parse(&typed).map(Message::Time)
}

fn times(from: u8, selected: &str) -> select::Select<'static, String, Message> {
    select::select(half_hours(from), Some(selected.to_owned())).on_select(Message::Time)
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    let _ = ui.simulate(click());
}

fn open(ui: &mut Simulator<'_, Message>) {
    click_at(ui, Point::new(100.0, HEIGHT / 2.0));
}

fn messages(ui: Simulator<'_, Message>) -> Vec<Message> {
    ui.into_messages().collect()
}

fn is_open(ui: &mut Simulator<'_, Message>) -> bool {
    ui.find(select::list_id()).is_ok()
}

#[test]
fn a_value_turned_down_shows_why_while_the_rows_still_show() {
    let mut ui = simulator(times(18, "09:00").on_custom(time).not_valid("Use hh:mm"));
    open(&mut ui);
    let _ = ui.typewrite("25:00");
    assert!(ui.find("Use hh:mm").is_err(), "nothing said before Enter");
    assert_eq!(ui.tap_key(Named::Enter), iced::event::Status::Captured);
    assert!(
        ui.find("Use hh:mm").is_ok(),
        "the line shows above the rows"
    );
    assert!(ui.find("09:00").is_ok(), "the rows still show");
    let _ = ui.tap_key(Named::Backspace);
    assert!(ui.find("Use hh:mm").is_err(), "editing clears it");
    assert!(messages(ui).is_empty());
}

#[test]
fn the_lines_the_list_draws_itself_are_found_by_text() {
    let searchable = || times(18, "09:00").searchable(true).on_custom(time);
    let mut ui = simulator(searchable());
    open(&mut ui);
    let _ = ui.typewrite("9:40");
    assert!(ui.find("Use \u{201c}9:40\u{201d}").is_ok());

    let mut ui = simulator(times(18, "09:00").searchable(true));
    open(&mut ui);
    let _ = ui.typewrite("x");
    assert!(ui.find(DEFAULT_EMPTY).is_ok());
    assert!(ui.find(select::search_id()).is_ok(), "the search field");

    let mut ui = simulator(searchable().not_valid("Use hh:mm"));
    open(&mut ui);
    let _ = ui.typewrite("99");
    let _ = ui.tap_key(Named::Enter);
    assert!(ui.find("Use hh:mm").is_ok(), "in place of rows");
}

#[test]
fn by_default_a_match_wins_over_the_typed_text() {
    let mut ui = simulator(times(19, "09:30").searchable(true).on_custom(time));
    open(&mut ui);
    let _ = ui.typewrite("9");
    let _ = ui.tap_key(Named::Enter);
    assert_eq!(messages(ui), vec![Message::Time("09:30".to_owned())]);
}

#[test]
fn preferring_custom_values_asks_the_app_first() {
    let preferring = || {
        times(19, "09:30")
            .searchable(true)
            .on_custom(time)
            .prefer_custom(true)
    };
    let mut ui = simulator(preferring());
    open(&mut ui);
    let _ = ui.typewrite("9");
    let _ = ui.tap_key(Named::Enter);
    assert!(!is_open(&mut ui));
    assert_eq!(messages(ui), vec![Message::Time("09:00".to_owned())]);

    // The app turns "1:3" down, so the highlighted match is chosen.
    let mut ui = simulator(preferring());
    open(&mut ui);
    let _ = ui.typewrite("1:3");
    let _ = ui.tap_key(Named::Enter);
    assert_eq!(messages(ui), vec![Message::Time("11:30".to_owned())]);

    // A highlight moved with the keys wins over the typed text.
    let mut ui = simulator(preferring());
    open(&mut ui);
    let _ = ui.typewrite("1");
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);
    assert_eq!(messages(ui), vec![Message::Time("10:30".to_owned())]);
}

/// A select that commits on leaving, with a Save button beside it, clear
/// of the open list.
fn form(pass_through: bool) -> Element<'static, Message> {
    row![
        times(18, "09:00")
            .on_custom(time)
            .commit_on_leave(true)
            .pass_through(pass_through),
        button("Save").on_press(Message::Save),
    ]
    .spacing(40)
    .into()
}

/// The middle of the Save button.
fn save(ui: &mut Simulator<'_, Message>) -> Point {
    let bounds = ui
        .find("Save")
        .ok()
        .and_then(|target| target.visible_bounds());
    let Some(bounds) = bounds else {
        panic!("Save is on screen");
    };
    bounds.center()
}

#[test]
fn tab_enters_the_typed_value() {
    let mut ui = simulator(form(false));
    open(&mut ui);
    let _ = ui.typewrite("9:40");
    assert_eq!(ui.tap_key(Named::Tab), iced::event::Status::Ignored);
    assert!(!is_open(&mut ui), "the list closes and focus moves on");
    assert_eq!(messages(ui), vec![Message::Time("09:40".to_owned())]);
}

#[test]
fn a_press_outside_enters_the_typed_value() {
    let mut ui = simulator(form(false));
    let at = Point::new(400.0, 400.0);
    open(&mut ui);
    let _ = ui.typewrite("9:40");
    click_at(&mut ui, at);
    assert!(!is_open(&mut ui));
    assert_eq!(messages(ui), vec![Message::Time("09:40".to_owned())]);
}

#[test]
fn a_value_turned_down_on_leaving_keeps_the_list_open() {
    let mut ui = simulator(form(false));
    open(&mut ui);
    let _ = ui.typewrite("25:00");
    assert_eq!(ui.tap_key(Named::Tab), iced::event::Status::Captured);
    assert!(is_open(&mut ui));
    assert!(ui.find("Not valid").is_ok());
    click_at(&mut ui, Point::new(400.0, 400.0));
    assert!(is_open(&mut ui), "a press outside does not close it either");
    let _ = ui.tap_key(Named::Escape);
    assert!(!is_open(&mut ui), "Escape still closes it");
    assert!(messages(ui).is_empty());
}

#[test]
fn escape_drops_the_typed_value() {
    let mut ui = simulator(form(false));
    open(&mut ui);
    let _ = ui.typewrite("9:40");
    let _ = ui.tap_key(Named::Escape);
    assert!(!is_open(&mut ui));
    assert!(messages(ui).is_empty());
}

#[test]
fn without_committing_tab_drops_the_typed_value() {
    let mut ui = simulator(times(18, "09:00").on_custom(time));
    open(&mut ui);
    let _ = ui.typewrite("9:40");
    let _ = ui.tap_key(Named::Tab);
    assert!(!is_open(&mut ui));
    assert!(messages(ui).is_empty());
}

#[test]
fn a_press_passed_through_reaches_the_button_under_it() {
    let mut ui = simulator(form(true));
    let at = save(&mut ui);
    open(&mut ui);
    let _ = ui.typewrite("9:40");
    click_at(&mut ui, at);
    assert!(!is_open(&mut ui));
    assert_eq!(
        messages(ui),
        vec![Message::Time("09:40".to_owned()), Message::Save],
        "one click enters the time, then saves"
    );
}

#[test]
fn a_press_outside_goes_no_further_by_default() {
    let mut ui = simulator(form(false));
    let at = save(&mut ui);
    open(&mut ui);
    click_at(&mut ui, at);
    assert!(!is_open(&mut ui));
    assert!(messages(ui).is_empty(), "the click only closed the list");
}

#[test]
fn a_selected_value_off_the_list_shows_on_the_closed_control() {
    let mut ui = simulator(times(18, "16:41"));
    assert!(ui.find("16:41").is_ok());
    let mut ui = simulator(times(18, "16:41").editable(true));
    assert!(ui.find("16:41").is_ok(), "in the editable field too");
}

fn editable() -> select::Select<'static, String, Message> {
    times(18, "09:00")
        .editable(true)
        .searchable(true)
        .on_custom(time)
        .prefer_custom(true)
}

/// Focuses the editable field by clicking it, which also opens the list.
fn focus_field(ui: &mut Simulator<'_, Message>) {
    open(ui);
    assert!(is_open(ui), "a click on the field opens the list");
}

#[test]
fn typing_into_an_editable_select_filters_and_enter_chooses() {
    let mut ui = simulator(editable());
    focus_field(&mut ui);
    let _ = ui.tap_key(Named::Escape);
    assert!(!is_open(&mut ui));
    // The click put the caret after the label; clear it and type.
    for _ in 0..5 {
        let _ = ui.tap_key(Named::Backspace);
    }
    assert!(is_open(&mut ui), "editing opens the list");
    let _ = ui.typewrite("14:");
    assert!(ui.find("14:30").is_ok());
    assert!(ui.find("13:30").is_err(), "filtered by the field");
    let _ = ui.tap_key(Named::Enter);
    assert!(!is_open(&mut ui));
    assert_eq!(messages(ui), vec![Message::Time("14:00".to_owned())]);
}

#[test]
fn an_editable_select_takes_a_time_off_the_list() {
    let mut ui = simulator(editable());
    focus_field(&mut ui);
    for _ in 0..5 {
        let _ = ui.tap_key(Named::Backspace);
    }
    let _ = ui.typewrite("16:41");
    assert!(ui.find("Use \u{201c}16:41\u{201d}").is_ok());
    let _ = ui.tap_key(Named::Enter);
    assert_eq!(messages(ui), vec![Message::Time("16:41".to_owned())]);
}

#[test]
fn escape_puts_the_selected_label_back_in_an_editable_select() {
    let mut ui = simulator(editable());
    focus_field(&mut ui);
    let _ = ui.typewrite("3");
    assert!(ui.find("09:003").is_ok(), "typed after the label");
    let _ = ui.tap_key(Named::Escape);
    assert!(!is_open(&mut ui));
    assert!(ui.find("09:00").is_ok());
    assert!(ui.find("09:003").is_err());
    assert!(messages(ui).is_empty());
}

#[test]
fn down_opens_a_focused_editable_select_and_the_arrows_choose() {
    let mut ui = simulator(editable());
    focus_field(&mut ui);
    let _ = ui.tap_key(Named::Escape);
    assert_eq!(ui.tap_key(Named::ArrowDown), iced::event::Status::Captured);
    assert!(is_open(&mut ui));
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);
    assert_eq!(messages(ui), vec![Message::Time("09:30".to_owned())]);
}

#[test]
fn a_press_in_the_editable_field_keeps_editing_and_the_chevron_closes() {
    let mut ui = simulator(editable().commit_on_leave(true));
    focus_field(&mut ui);
    let _ = ui.typewrite("3");
    click_at(&mut ui, Point::new(40.0, HEIGHT / 2.0));
    assert!(is_open(&mut ui), "moving the caret does not leave the list");
    assert!(ui.find("09:003").is_ok(), "nor enter the text");
    click_at(&mut ui, Point::new(190.0, HEIGHT / 2.0));
    assert!(!is_open(&mut ui), "the chevron closes the list");
    assert!(messages(ui).is_empty());
}

#[test]
fn a_disabled_editable_select_takes_no_typing() {
    let mut ui = simulator(
        select::select(half_hours(18), Some("09:00".to_owned()))
            .editable(true)
            .on_select_maybe(None::<fn(String) -> Message>),
    );
    open(&mut ui);
    assert!(!is_open(&mut ui));
    let _ = ui.typewrite("1");
    assert!(ui.find("09:00").is_ok());
}
