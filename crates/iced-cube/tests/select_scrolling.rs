#![cfg(feature = "select")]

use iced::keyboard::key::Named;
use iced::time::Duration;
use iced::widget::{column, container, scrollable, text};
use iced::{Element, Event, Length, Point, Settings, mouse, touch};
use iced_cube::forms::select::{self, HEIGHT, MAX_VISIBLE_ROWS};
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Time(String),
    Day(&'static str),
}

/// Every quarter of an hour in a day: 96 options.
fn times() -> Vec<String> {
    (0..96)
        .map(|quarter| format!("{:02}:{:02}", quarter / 4, quarter % 4 * 15))
        .collect()
}

/// A long time select at the top left, 200 pixels wide.
fn time_select(selected: &str) -> select::Select<'static, String, Message> {
    select::select(times(), Some(selected.to_owned()))
        .on_select(Message::Time)
        .type_ahead_pause(Duration::from_secs(3600))
}

fn open(ui: &mut Simulator<'_, Message>) {
    click_at(ui, Point::new(100.0, HEIGHT / 2.0));
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    let _ = ui.simulate(click());
}

fn visible(ui: &mut Simulator<'_, Message>, label: &str) -> bool {
    ui.find(label)
        .ok()
        .and_then(|target| target.visible_bounds())
        .is_some()
}

/// The top of the open list's rows: below the field, the gap and the
/// surface's padding.
const ROWS_TOP: f32 = HEIGHT + 4.0 + 4.0;
const ROW: f32 = 32.0;
const VIEWPORT: f32 = MAX_VISIBLE_ROWS as f32 * ROW;

#[test]
fn a_long_list_opens_around_the_selected_option() {
    let mut ui = simulator(time_select("14:00"));
    open(&mut ui);
    for shown in ["13:15", "13:45", "14:15", "15:00"] {
        assert!(visible(&mut ui, shown), "{shown} is in view");
    }
    for hidden in ["00:00", "13:00", "15:15", "23:45"] {
        assert!(!visible(&mut ui, hidden), "{hidden} is scrolled away");
    }
    // Eight whole rows, the first right at the top of the list.
    let first = ui
        .find("13:15")
        .ok()
        .and_then(|target| target.visible_bounds())
        .expect("13:15 is in view");
    assert!(
        (first.y - ROWS_TOP - 6.0).abs() < 0.5,
        "starts on a whole row"
    );
    let last = ui
        .find("15:00")
        .ok()
        .and_then(|target| target.visible_bounds())
        .expect("15:00 is in view");
    assert!(last.y + last.height <= ROWS_TOP + VIEWPORT);
}

#[test]
fn every_option_is_reached_by_keys_and_the_highlight_stays_in_view() {
    let mut ui = simulator(time_select("14:00"));
    open(&mut ui);
    for _ in 0..3 {
        assert_eq!(ui.tap_key(Named::PageDown), iced::event::Status::Captured);
    }
    assert!(visible(&mut ui, "20:00"), "three pages of eight rows on");
    let _ = ui.tap_key(Named::PageUp);
    assert!(visible(&mut ui, "18:00"));
    let _ = ui.tap_key(Named::End);
    assert!(visible(&mut ui, "23:45"));
    let _ = ui.tap_key(Named::Home);
    assert!(visible(&mut ui, "00:00"));
    for _ in 0..10 {
        let _ = ui.tap_key(Named::ArrowDown);
    }
    assert!(visible(&mut ui, "02:30"), "the arrows scroll as they go");
    let _ = ui.tap_key(Named::Enter);
    assert!(ui.find("00:00").is_err(), "choosing closes the list");
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Time("02:30".to_owned())]
    );
}

/// The select at the top of a page that scrolls, with "Top" above it.
fn on_a_page() -> Element<'static, Message> {
    scrollable(
        column![
            text("Top"),
            time_select("14:00"),
            container(text("Bottom")).padding([1200, 0]),
        ]
        .width(Length::Fill),
    )
    .height(Length::Fill)
    .into()
}

fn wheel(ui: &mut Simulator<'_, Message>, at: Point, lines: f32) -> iced::event::Status {
    ui.point_at(at);
    let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: at })]);
    ui.simulate([Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 0.0, y: lines },
    })])
    .first()
    .copied()
    .unwrap_or(iced::event::Status::Ignored)
}

#[test]
fn the_wheel_scrolls_the_list_and_never_the_page() {
    let mut ui = simulator(on_a_page());
    let top = ui.find("Top").expect("page is drawn").bounds();
    let field_y = top.y + top.height;
    click_at(&mut ui, Point::new(100.0, field_y + HEIGHT / 2.0));
    assert!(visible(&mut ui, "14:00") || visible(&mut ui, "13:45"));

    let over_list = Point::new(100.0, field_y + HEIGHT + 100.0);
    assert_eq!(
        wheel(&mut ui, over_list, -5.0),
        iced::event::Status::Captured
    );
    assert!(!visible(&mut ui, "13:45"), "the list moved down");
    assert!(
        visible(&mut ui, "17:00"),
        "five lines of 60 pixels further on"
    );

    for _ in 0..20 {
        assert_eq!(
            wheel(&mut ui, over_list, -5.0),
            iced::event::Status::Captured
        );
    }
    assert!(visible(&mut ui, "23:45"), "stopped at the end");
    let after = ui.find("Top").expect("page is drawn").bounds();
    assert_eq!(after, top, "the page behind did not scroll");
    assert!(visible(&mut ui, "22:00"), "the list is still open");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn pressing_the_scrollbar_track_jumps_and_dragging_follows() {
    let mut ui = simulator(time_select("00:00"));
    open(&mut ui);
    assert!(visible(&mut ui, "00:15"));
    // The track runs down the list's right edge, inside a 2 pixel margin.
    // Row text starts 12 pixels in from the list's left edge.
    let list_x = ui.find("00:15").expect("drawn").bounds().x - 12.0;
    let track_x = list_x + 200.0 - 2.0 - 3.0;
    let bottom = Point::new(track_x, ROWS_TOP + VIEWPORT - 4.0);
    click_at(&mut ui, bottom);
    assert!(
        visible(&mut ui, "23:45"),
        "a press near the bottom goes there"
    );
    assert!(ui.find("12:00").is_ok(), "the list stays open");

    let near_bottom = Point::new(track_x, ROWS_TOP + VIEWPORT - 20.0);
    ui.point_at(near_bottom);
    let _ = ui.simulate([
        Event::Mouse(mouse::Event::CursorMoved {
            position: near_bottom,
        }),
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
    ]);
    let top = Point::new(track_x, ROWS_TOP - 100.0);
    ui.point_at(top);
    let _ = ui.simulate([
        Event::Mouse(mouse::Event::CursorMoved { position: top }),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]);
    assert!(
        visible(&mut ui, "00:15"),
        "dragging the thumb up scrolls back"
    );
    assert_eq!(
        ui.into_messages().count(),
        0,
        "the scrollbar chooses nothing"
    );
}

fn finger(event: touch::Event) -> Event {
    Event::Touch(event)
}

#[test]
fn a_finger_drag_scrolls_without_choosing_and_a_tap_chooses() {
    let mut ui = simulator(time_select("00:00"));
    open(&mut ui);
    let id = touch::Finger(1);
    let start = Point::new(80.0, ROWS_TOP + 200.0);
    let end = Point::new(80.0, ROWS_TOP + 8.0);
    let _ = ui.simulate([
        finger(touch::Event::FingerPressed {
            id,
            position: start,
        }),
        finger(touch::Event::FingerMoved { id, position: end }),
        finger(touch::Event::FingerLifted { id, position: end }),
    ]);
    assert!(
        !visible(&mut ui, "00:15"),
        "the list scrolled up under the finger"
    );
    assert!(visible(&mut ui, "02:00"));
    assert_eq!(ui.into_messages().count(), 0, "a drag chooses nothing");

    let mut ui = simulator(time_select("00:00"));
    open(&mut ui);
    let tap = ui
        .find("00:30")
        .ok()
        .and_then(|target| target.visible_bounds())
        .expect("00:30 is in view")
        .center();
    let _ = ui.simulate([
        finger(touch::Event::FingerPressed { id, position: tap }),
        finger(touch::Event::FingerLifted { id, position: tap }),
    ]);
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Time("00:30".to_owned())]
    );
}

#[test]
fn near_the_bottom_of_a_short_window_the_list_stays_inside_it() {
    let page = column![
        container(text("Above")).height(200),
        time_select("14:00").max_visible_rows(20),
    ];
    let mut ui = Simulator::with_size(Settings::default(), (400.0, 300.0), page);
    click_at(&mut ui, Point::new(100.0, 200.0 + HEIGHT / 2.0));
    let mut shown = 0;
    for time in times() {
        let Some(bounds) = ui.find(time.as_str()).ok().and_then(|t| t.visible_bounds()) else {
            continue;
        };
        shown += 1;
        assert!(
            bounds.y >= 0.0 && bounds.y + bounds.height <= 300.0,
            "{time} at {bounds:?}"
        );
    }
    assert!(shown >= 3, "the list shows rows, {shown} of them");
    assert!(visible(&mut ui, "14:15"), "around the selected time");
}

#[test]
fn a_list_opened_above_the_field_stays_above_while_filtering() {
    let page = column![
        container(text("Above")).height(280),
        time_select("14:00").searchable(true),
    ];
    let mut ui = Simulator::with_size(Settings::default(), (400.0, 420.0), page);
    click_at(&mut ui, Point::new(100.0, 280.0 + HEIGHT / 2.0));
    let shown = |ui: &mut Simulator<'_, Message>, label: &str| {
        ui.find(label)
            .ok()
            .and_then(|target| target.visible_bounds())
            .unwrap_or_else(|| panic!("{label} is in view"))
    };
    let row = shown(&mut ui, "14:15");
    assert!(row.y < 280.0, "opens above the field");
    let _ = ui.typewrite("14:3");
    let filtered = shown(&mut ui, "14:30");
    assert!(filtered.y < 280.0, "stays above at {filtered:?}");
}

#[test]
fn turned_off_the_list_is_as_tall_as_its_options() {
    let mut ui = simulator(time_select("14:00").scrollable(false));
    open(&mut ui);
    assert!(visible(&mut ui, "00:00"), "starts at the top");
    let last = ui.find("23:45").expect("every row is laid out").bounds();
    assert!(last.y > 3000.0, "no limit on the height");
}

#[test]
fn a_short_list_shows_every_row_without_scrolling() {
    let mut ui = simulator(
        select::select(vec!["Apple", "Banana", "Cherry"], Some("Banana"))
            .on_select(|_| Message::Day("fruit")),
    );
    open(&mut ui);
    let first = ui.find("Apple").expect("drawn").bounds();
    let last = ui.find("Cherry").expect("drawn").bounds();
    assert!(
        (last.y - first.y - 2.0 * ROW).abs() < 0.5,
        "rows 32 pixels apart"
    );
    assert!(
        (first.y - ROWS_TOP - 6.0).abs() < 0.5,
        "first row at the top"
    );
}

fn searchable(on_custom: bool) -> Element<'static, Message> {
    let select = time_select("09:00").searchable(true);
    if !on_custom {
        return select.into();
    }
    select
        .on_custom(|typed| {
            let (hours, minutes) = typed.split_once(':')?;
            let (hours, minutes): (u8, u8) = (hours.parse().ok()?, minutes.parse().ok()?);
            (hours < 24 && minutes < 60).then(|| Message::Time(format!("{hours:02}:{minutes:02}")))
        })
        .into()
}

#[test]
fn typing_filters_a_searchable_list() {
    let mut ui = simulator(searchable(false));
    open(&mut ui);
    assert_eq!(ui.typewrite("14:3"), iced::event::Status::Captured);
    assert!(visible(&mut ui, "14:30"));
    assert!(ui.find("13:45").is_err(), "filtered out");
    assert!(ui.find("09:15").is_err());
    let _ = ui.tap_key(Named::Backspace);
    assert!(visible(&mut ui, "14:00") && visible(&mut ui, "14:45"));
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Time("14:15".to_owned())]
    );
}

#[test]
fn the_query_is_cleared_when_the_list_closes() {
    let mut ui = simulator(searchable(false));
    open(&mut ui);
    let _ = ui.typewrite("23");
    assert!(ui.find("08:45").is_err());
    let _ = ui.tap_key(Named::Escape);
    open(&mut ui);
    assert!(visible(&mut ui, "08:45"), "every option shows again");
}

#[test]
fn enter_sends_a_typed_value_the_app_accepts() {
    let mut ui = simulator(searchable(true));
    open(&mut ui);
    let _ = ui.typewrite("9:40");
    let _ = ui.tap_key(Named::Enter);
    assert!(ui.find("00:00").is_err(), "the list closes");
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Time("09:40".to_owned())]
    );
}

#[test]
fn a_typed_value_the_app_turns_down_keeps_the_list_open() {
    let mut ui = simulator(searchable(true));
    open(&mut ui);
    let _ = ui.typewrite("25:00");
    assert_eq!(ui.tap_key(Named::Enter), iced::event::Status::Captured);
    for _ in 0..5 {
        let _ = ui.tap_key(Named::Backspace);
    }
    assert!(
        visible(&mut ui, "00:15"),
        "still open, every option back from the top"
    );
    assert_eq!(ui.into_messages().count(), 0);
}

/// Below the search field: its 32 pixel line, the rule and a 4 pixel gap.
const SEARCH: f32 = 32.0 + 1.0 + 4.0;

#[test]
fn clicking_the_offered_value_the_app_turns_down_keeps_the_list_open() {
    let mut ui = simulator(searchable(true));
    open(&mut ui);
    let _ = ui.typewrite("25:00");
    // The offered value sits where the first row would.
    click_at(&mut ui, Point::new(100.0, ROWS_TOP + SEARCH + ROW / 2.0));
    for _ in 0..5 {
        let _ = ui.tap_key(Named::Backspace);
    }
    assert!(visible(&mut ui, "00:15"), "the list stayed open");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn clicking_the_offered_value_the_app_accepts_chooses_it() {
    let mut ui = simulator(searchable(true));
    open(&mut ui);
    let _ = ui.typewrite("9:40");
    click_at(&mut ui, Point::new(100.0, ROWS_TOP + SEARCH + ROW / 2.0));
    assert!(ui.find("08:45").is_err(), "the list closes");
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Time("09:40".to_owned())]
    );
}

#[test]
fn the_search_field_has_the_editing_shortcuts() {
    use iced::keyboard::{self, Key, Modifiers};
    let mut ui = simulator(searchable(false));
    open(&mut ui);
    let _ = ui.typewrite("12 13");
    assert!(ui.find("13:00").is_err(), "nothing matches yet");
    let chord = |key: Named| {
        let key = Key::Named(key);
        [
            Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::CTRL)),
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: key.clone(),
                modified_key: key,
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: Modifiers::CTRL,
                text: None,
                repeat: false,
            }),
            Event::Keyboard(keyboard::Event::ModifiersChanged(Modifiers::empty())),
        ]
    };
    let _ = ui.simulate(chord(Named::Backspace));
    assert!(visible(&mut ui, "12:00"), "the last word went");
    assert!(ui.find("13:00").is_err());
    assert_eq!(ui.into_messages().count(), 0);
}

const DAYS: &[&str] = &[
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

#[test]
fn typing_in_a_plain_list_jumps_to_a_match() {
    let day_select = || {
        select::select(DAYS, Some("Monday"))
            .on_select(Message::Day)
            .type_ahead_pause(Duration::from_secs(3600))
    };
    let mut ui = simulator(day_select());
    open(&mut ui);
    assert_eq!(ui.typewrite("th"), iced::event::Status::Captured);
    let _ = ui.tap_key(Named::Enter);

    let mut ui2 = simulator(day_select());
    open(&mut ui2);
    let _ = ui2.typewrite("ss");
    let _ = ui2.tap_key(Named::Enter);

    let mut messages: Vec<_> = ui.into_messages().collect();
    messages.extend(ui2.into_messages());
    assert_eq!(
        messages,
        vec![Message::Day("Thursday"), Message::Day("Sunday")],
        "Th finds Thursday; S twice cycles to the second S"
    );
}

#[test]
fn a_select_without_options_opens_and_chooses_nothing() {
    let mut ui = simulator(select::select(Vec::<String>::new(), None).on_select(Message::Time));
    open(&mut ui);
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);
    assert_eq!(ui.into_messages().count(), 0);
}
