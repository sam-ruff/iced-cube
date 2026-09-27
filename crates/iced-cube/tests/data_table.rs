#![cfg(feature = "data-table")]

use std::time::{Duration, Instant};

use iced::keyboard::{Key, Modifiers, key::Named};
use iced::widget::container;
use iced::{Element, Length, Point, Size, keyboard, mouse};
use iced_cube::data::data_table::{Align, Column, Event, Output, State, column as col, data_table};
use iced_cube::overlay::{context_menu, dropdown_menu};
use iced_test::simulator::{self, Simulator};

#[derive(Debug, Clone)]
enum Message {
    Table(Event<u32>),
    Menu(context_menu::Event<u8, u32>),
}

#[derive(Debug, Clone)]
struct Payment {
    id: u32,
    email: String,
    status: &'static str,
    amount: f64,
}

fn columns() -> Vec<Column<Payment>> {
    vec![
        col("email", "Email", |p: &Payment| p.email.clone().into()).width(Length::FillPortion(3)),
        col("status", "Status", |p: &Payment| p.status.into()).filterable(true),
        col("amount", "Amount", |p: &Payment| p.amount.into())
            .align(Align::End)
            .format(|p| format!("${:.2}", p.amount)),
    ]
}

fn payments(count: u32) -> Vec<Payment> {
    (1..=count)
        .map(|id| Payment {
            id,
            email: format!("user{id}@example.com"),
            status: ["Paid", "Pending", "Failed"][(id % 3) as usize],
            amount: f64::from(id) * 10.0,
        })
        .collect()
}

fn state(count: u32) -> State<Payment, u32> {
    State::new(columns(), payments(count), |p| p.id)
}

fn view(state: &State<Payment, u32>) -> Element<'_, Message> {
    data_table(state).on_event(Message::Table).into()
}

fn desktop<'a>(element: Element<'a, Message>) -> Simulator<'a, Message> {
    Simulator::with_size(iced::Settings::default(), Size::new(800.0, 700.0), element)
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event<u32>> {
    ui.into_messages()
        .filter_map(|message| match message {
            Message::Table(event) => Some(event),
            Message::Menu(_) => None,
        })
        .collect()
}

fn apply(events: Vec<Event<u32>>, state: &mut State<Payment, u32>) -> Vec<Output<u32>> {
    events
        .into_iter()
        .filter_map(|event| state.update(event))
        .collect()
}

fn press_key(key: Key, modifiers: Modifiers) -> iced::Event {
    iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers,
        text: None,
        repeat: false,
    })
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate(simulator::click());
}

#[test]
fn renders_the_page_the_footer_and_the_toolbar() {
    let state = state(23);
    let mut ui = desktop(view(&state));
    for text in [
        "Email",
        "Amount",
        "user1@example.com",
        "user10@example.com",
        "$100.00",
        "Showing 1-10 of 23",
        "Page 1 of 3",
        "Rows per page",
        "Columns",
        "Status: All",
    ] {
        assert!(ui.find(text).is_ok(), "{text}");
    }
    assert!(
        ui.find("user11@example.com").is_err(),
        "only the page is built"
    );
}

#[test]
fn amounts_line_up_on_the_right() {
    let state = state(3);
    let mut ui = desktop(view(&state));
    let header = ui.find("Amount").expect("header").bounds();
    let short = ui.find("$10.00").expect("cell").bounds();
    let long = ui.find("$30.00").expect("cell").bounds();
    assert_eq!(short.x + short.width, long.x + long.width);
    assert!(
        header.x > short.x - 40.0,
        "the header sits at the right too"
    );
}

#[test]
fn clicking_a_header_sorts_by_it() {
    let mut state = state(5);
    let mut ui = desktop(view(&state));
    ui.click("Amount").expect("header");
    let emitted = events(ui);
    assert!(emitted.contains(&Event::Sort("amount")), "{emitted:?}");
    let _ = apply(emitted, &mut state);
    let _ = state.update(Event::Sort("amount"));
    let mut ui = desktop(view(&state));
    let top = ui.find("$50.00").expect("largest").bounds();
    let bottom = ui.find("$10.00").expect("smallest").bounds();
    assert!(top.y < bottom.y, "descending puts the largest first");
}

#[test]
fn typing_in_the_search_box_searches() {
    let state = state(5);
    let mut ui = desktop(view(&state));
    ui.click("Search...").expect("search box");
    let _ = ui.typewrite("u3");
    let emitted = events(ui);
    assert_eq!(
        emitted.last(),
        Some(&Event::Search("u3".into())),
        "{emitted:?}"
    );
}

#[test]
fn a_column_filter_offers_its_values() {
    let state = state(6);
    let mut ui = desktop(view(&state));
    ui.click("Status: All").expect("filter");
    ui.click("Status: Paid").expect("option in the open list");
    let emitted = events(ui);
    assert!(
        emitted.contains(&Event::Filter("status", Some("Paid".into()))),
        "{emitted:?}"
    );
}

#[test]
fn the_columns_menu_hides_a_column() {
    let mut state = state(3);
    let mut ui = desktop(view(&state));
    ui.click("Columns").expect("trigger");
    let emitted = events(ui);
    let _ = apply(emitted, &mut state);
    assert!(state.columns_menu().is_open());

    let mut ui = desktop(view(&state));
    let items: Vec<_> = ["Email", "Status", "Amount"]
        .iter()
        .filter(|label| ui.find(**label).is_ok())
        .collect();
    assert_eq!(items.len(), 3);
    let _ = ui.tap_key(Named::ArrowDown);
    let _ = ui.tap_key(Named::Enter);
    let emitted = events(ui);
    let _ = apply(emitted, &mut state);
    assert!(state.is_hidden("email"));
    let mut ui = desktop(view(&state));
    assert!(ui.find("user1@example.com").is_err());
}

#[test]
fn row_checkboxes_and_the_header_checkbox_select() {
    let mut state = state(12).with_selection(true);
    let mut ui = desktop(view(&state));
    let email = ui.find("user2@example.com").expect("row").bounds();
    // The checkbox column is 40 pixels wide, before the first cell's padding.
    click_at(&mut ui, Point::new(email.x - 8.0 - 20.0, email.center_y()));
    let header = ui.find("Email").expect("header").bounds();
    click_at(
        &mut ui,
        Point::new(header.x - 8.0 - 20.0, header.center_y()),
    );
    let emitted = events(ui);
    assert!(emitted.contains(&Event::Select(2, true)), "{emitted:?}");
    assert!(emitted.contains(&Event::SelectPage(true)), "{emitted:?}");
    let outputs = apply(emitted, &mut state);
    assert!(matches!(outputs.last(), Some(Output::Selected(keys)) if keys.len() == 10));
    let mut ui = desktop(view(&state));
    assert!(ui.find("Showing 1-10 of 12, 10 selected").is_ok());
}

#[test]
fn the_pager_turns_pages() {
    let mut state = state(23);
    let mut ui = desktop(view(&state));
    let page = ui.find("Page 1 of 3").expect("pager").bounds();
    // The next button is the last thing on the footer row.
    click_at(
        &mut ui,
        Point::new(
            page.x + page.width + 8.0 + 32.0 + 8.0 + 16.0,
            page.center_y(),
        ),
    );
    let emitted = events(ui);
    assert!(emitted.contains(&Event::NextPage), "{emitted:?}");
    let _ = apply(emitted, &mut state);
    let mut ui = desktop(view(&state));
    assert!(ui.find("Showing 11-20 of 23").is_ok());
    assert!(ui.find("user11@example.com").is_ok());
}

#[test]
fn keys_are_ignored_until_the_table_has_focus() {
    let state = state(5).with_selection(true);
    let mut ui = desktop(view(&state));
    let _ = ui.tap_key(Named::ArrowDown);
    assert!(events(ui).is_empty());

    let mut ui = desktop(view(&state));
    ui.click("user2@example.com").expect("row");
    let _ = ui.tap_key(Named::ArrowDown);
    let emitted = events(ui);
    assert!(
        matches!(emitted.first(), Some(Event::Focus(true))),
        "{emitted:?}"
    );
    assert!(emitted.contains(&Event::Press(2)));
    assert_eq!(emitted.last(), Some(&Event::Next));
}

#[test]
fn keyboard_selection_runs_end_to_end() {
    let mut state = state(12).with_selection(true);
    let steps: [(Key, Modifiers); 4] = [
        (Key::Named(Named::ArrowDown), Modifiers::empty()),
        (Key::Named(Named::Space), Modifiers::empty()),
        (Key::Character("a".into()), Modifiers::COMMAND),
        (Key::Named(Named::PageDown), Modifiers::empty()),
    ];
    let mut ui = desktop(view(&state));
    ui.click("user1@example.com").expect("row");
    let _ = apply(events(ui), &mut state);
    for (key, modifiers) in steps {
        // A fresh simulator starts unfocused, so click the highlighted row.
        let highlighted = state.highlighted().copied().expect("a highlighted row");
        let mut ui = desktop(view(&state));
        ui.click(format!("user{highlighted}@example.com").as_str())
            .expect("highlighted row");
        let _ = ui.simulate([press_key(key, modifiers)]);
        let _ = apply(events(ui), &mut state);
    }
    assert_eq!(state.selected().len(), 10, "Ctrl+A selected the page");
    assert_eq!(state.page(), 1);
    assert_eq!(state.highlighted(), Some(&11));
}

#[test]
fn enter_and_double_clicks_activate() {
    let mut state = state(3);
    let mut ui = desktop(view(&state));
    ui.click("user3@example.com").expect("row");
    ui.click("user3@example.com").expect("row");
    let _ = ui.tap_key(Named::Enter);
    let outputs = apply(events(ui), &mut state);
    assert!(outputs.contains(&Output::Activated(3)), "{outputs:?}");
}

#[test]
fn row_menus_open_by_right_click_button_and_keyboard() {
    let mut table = state(3);
    let _ = table.update(Event::Press(2));
    let menu = context_menu::State::<u8, u32>::new([dropdown_menu::item(1, "Refund")]);
    let element: Element<'_, Message> = data_table(&table)
        .context_menu(&menu, Message::Menu)
        .row_actions(true)
        .on_event(Message::Table)
        .into();
    let mut ui = desktop(element);
    let row = ui.find("user3@example.com").expect("row").bounds();
    ui.point_at(row.center());
    let _ = ui.simulate([
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
    ]);
    // The row actions button sits at the right end of the row.
    let amount = ui.find("$10.00").expect("first row").bounds();
    click_at(
        &mut ui,
        Point::new(amount.x + amount.width + 8.0 + 22.0, amount.center_y()),
    );
    ui.click("user2@example.com").expect("row");
    let _ = ui.simulate([press_key(Key::Named(Named::F10), Modifiers::SHIFT)]);

    let opened: Vec<_> = ui
        .into_messages()
        .filter_map(|message| match message {
            Message::Menu(context_menu::Event::Open(key, _)) => Some(("pointer", key)),
            Message::Menu(context_menu::Event::OpenFromKeyboard(key)) => Some(("keyboard", key)),
            _ => None,
        })
        .collect();
    assert_eq!(opened, [("pointer", 3), ("keyboard", 1), ("keyboard", 2)]);
}

#[test]
fn empty_and_loading_states_replace_the_rows() {
    let mut empty = state(3);
    let _ = empty.update(Event::Search("nothing".into()));
    let mut ui = desktop(
        data_table(&empty)
            .empty("No payments.")
            .on_event(Message::Table)
            .into(),
    );
    assert!(ui.find("No payments.").is_ok());
    ui.click("Clear filters").expect("button");
    assert!(events(ui).contains(&Event::ClearFilters));

    let loading = state(3);
    let mut ui = desktop(
        data_table(&loading)
            .loading(true)
            .on_event(Message::Table)
            .into(),
    );
    assert!(ui.find("Email").is_ok(), "the header stays");
    assert!(ui.find("user1@example.com").is_err(), "rows are replaced");
}

#[test]
fn a_narrow_table_becomes_cards_that_fit() {
    let state = state(4).with_selection(true);
    let element: Element<'_, Message> = container(view(&state)).width(328).into();
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(360.0, 1400.0), element);
    assert!(
        ui.find("Select all").is_ok(),
        "cards have their own select-all"
    );
    for text in [
        "user1@example.com",
        "$40.00",
        "Showing 1-4 of 4, 0 selected",
        "Columns",
    ] {
        let bounds = ui.find(text).expect(text).bounds();
        assert!(
            bounds.x >= 0.0 && bounds.x + bounds.width <= 328.0,
            "{text}: {bounds:?}"
        );
        assert!(bounds.height <= 20.0, "{text} is on one line: {bounds:?}");
    }
    let first = ui.find("user1@example.com").expect("card").bounds();
    let label = ui.find("Amount").expect("labelled line").bounds();
    assert!(
        label.y > first.y,
        "each column is a line below the card's title"
    );
}

#[test]
fn a_thousand_rows_only_build_the_page() {
    let started = Instant::now();
    let mut state = state(1000).with_page_sizes([100]);
    let _ = state.update(Event::Sort("amount"));
    let _ = state.update(Event::Sort("amount"));
    let _ = state.update(Event::Search("user".into()));
    let element: Element<'_, Message> = data_table(&state)
        .height(400)
        .on_event(Message::Table)
        .into();
    let mut ui = desktop(element);
    assert!(ui.find("user1000@example.com").is_ok(), "sorted descending");
    assert!(
        ui.find("user960@example.com").is_err(),
        "rows below the view are not built"
    );
    assert!(ui.find("Showing 1-100 of 1000").is_ok());
    let _ = ui.simulate([iced::Event::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -2000.0 },
    })]);
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
}
