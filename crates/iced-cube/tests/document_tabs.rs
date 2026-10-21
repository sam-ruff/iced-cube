#![cfg(feature = "document-tabs")]

use iced::widget::container;
use iced::{Element, Event as Input, Point, Rectangle, mouse};
use iced_cube::navigation::document_tabs::{Event, MenuItem, State, document, document_tabs};
use iced_cube::overlay::{context_menu, dropdown_menu};
use iced_test::simulator;
use iced_test::simulator::Simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Tabs(Event<u8>),
}

fn state() -> State<u8> {
    State::new([
        document(1, "alpha.rs"),
        document(2, "beta.rs").modified(true),
        document(3, "gamma.rs"),
    ])
}

fn view(state: &State<u8>, width: f32, new_tab: bool) -> Element<'_, Message> {
    container(
        document_tabs(state)
            .new_tab(new_tab)
            .on_event(Message::Tabs),
    )
    .width(width)
    .into()
}

fn messages(ui: Simulator<'_, Message>) -> Vec<Event<u8>> {
    ui.into_messages()
        .map(|Message::Tabs(event)| event)
        .collect()
}

/// Where a label is drawn. A missing label lands at the origin, where the
/// test's own assertions then fail.
fn label_bounds(ui: &mut Simulator<'_, Message>, label: &str) -> Rectangle {
    ui.find(label)
        .map(|target| target.bounds())
        .unwrap_or_default()
}

/// The centre of the slot after a tab's label: spacing, then half the slot.
fn slot_after(ui: &mut Simulator<'_, Message>, label: &str) -> Point {
    let bounds = label_bounds(ui, label);
    Point::new(bounds.x + bounds.width + 8.0 + 8.0, bounds.center_y())
}

fn centre(ui: &mut Simulator<'_, Message>, label: &str) -> Point {
    label_bounds(ui, label).center()
}

fn press(button: mouse::Button) -> Input {
    Input::Mouse(mouse::Event::ButtonPressed(button))
}

fn release(button: mouse::Button) -> Input {
    Input::Mouse(mouse::Event::ButtonReleased(button))
}

fn move_to(position: Point) -> Input {
    Input::Mouse(mouse::Event::CursorMoved { position })
}

#[test]
fn pressing_a_tab_activates_it() {
    let state = state();
    let mut ui = simulator(view(&state, 600.0, false));
    ui.click("beta.rs").expect("tab is rendered");
    assert_eq!(messages(ui), [Event::Activate(2)]);
}

#[test]
fn the_active_tab_close_button_asks_to_close() {
    let state = state();
    let mut ui = simulator(view(&state, 600.0, false));
    let close = slot_after(&mut ui, "alpha.rs");
    ui.point_at(close);
    let _ = ui.simulate(simulator::click());
    assert_eq!(messages(ui), [Event::Close(1)]);
}

#[test]
fn a_modified_tab_shows_its_close_button_on_hover() {
    let state = state();
    let mut ui = simulator(view(&state, 600.0, false));
    let close = slot_after(&mut ui, "beta.rs");
    ui.point_at(close);
    let _ = ui.simulate([move_to(close)]);
    let _ = ui.simulate(simulator::click());
    assert_eq!(messages(ui), [Event::Close(2)]);
}

#[test]
fn a_middle_click_asks_to_close() {
    let state = state();
    let mut ui = simulator(view(&state, 600.0, false));
    let gamma = centre(&mut ui, "gamma.rs");
    ui.point_at(gamma);
    let _ = ui.simulate([press(mouse::Button::Middle), release(mouse::Button::Middle)]);
    assert_eq!(messages(ui), [Event::Close(3)]);
}

#[test]
fn dragging_a_tab_past_another_moves_it() {
    let state = state();
    let mut ui = simulator(view(&state, 600.0, false));
    let alpha = centre(&mut ui, "alpha.rs");
    let end = slot_after(&mut ui, "gamma.rs") + iced::Vector::new(20.0, 0.0);
    ui.point_at(alpha);
    let _ = ui.simulate([press(mouse::Button::Left)]);
    let _ = ui.simulate([move_to(alpha + iced::Vector::new(30.0, 0.0))]);
    ui.point_at(end);
    let _ = ui.simulate([move_to(end), release(mouse::Button::Left)]);
    assert_eq!(messages(ui), [Event::Activate(1), Event::Move(1, 2)]);
}

#[test]
fn a_small_wobble_is_not_a_drag() {
    let state = state();
    let mut ui = simulator(view(&state, 600.0, false));
    let beta = centre(&mut ui, "beta.rs");
    ui.point_at(beta);
    let _ = ui.simulate([press(mouse::Button::Left)]);
    let wobble = beta + iced::Vector::new(2.0, 1.0);
    ui.point_at(wobble);
    let _ = ui.simulate([move_to(wobble), release(mouse::Button::Left)]);
    assert_eq!(messages(ui), [Event::Activate(2)]);
}

#[test]
fn a_pinned_tab_cannot_be_dragged_among_the_others() {
    let state = State::new([
        document(1, "pinned.rs").pinned(true),
        document(2, "beta.rs"),
        document(3, "gamma.rs"),
    ]);
    let mut ui = simulator(view(&state, 600.0, false));
    let pinned = centre(&mut ui, "pinned.rs");
    let end = slot_after(&mut ui, "gamma.rs");
    ui.point_at(pinned);
    let _ = ui.simulate([press(mouse::Button::Left)]);
    ui.point_at(end);
    let _ = ui.simulate([move_to(end), release(mouse::Button::Left)]);
    assert_eq!(messages(ui), [Event::Activate(1)]);
}

#[test]
fn a_double_click_on_the_empty_strip_asks_for_a_new_tab() {
    for new_tab in [true, false] {
        let state = state();
        let mut ui = simulator(view(&state, 600.0, new_tab));
        let empty = Point::new(580.0, centre(&mut ui, "alpha.rs").y);
        ui.point_at(empty);
        let _ = ui.simulate(simulator::click());
        let _ = ui.simulate(simulator::click());
        let expected: &[Event<u8>] = if new_tab { &[Event::New] } else { &[] };
        assert_eq!(messages(ui), expected, "new_tab {new_tab}");
    }
}

#[test]
fn a_right_click_opens_the_tab_menu_there() {
    let state = state();
    let mut ui = simulator(view(&state, 600.0, false));
    let beta = centre(&mut ui, "beta.rs");
    ui.point_at(beta);
    let _ = ui.simulate([press(mouse::Button::Right)]);
    let events = messages(ui);
    assert!(
        matches!(
            events.as_slice(),
            [Event::Menu(context_menu::Event::Open(2, _))]
        ),
        "{events:?}"
    );
}

#[test]
fn the_open_menu_offers_the_close_actions() {
    let mut state = state();
    let _ = state.update(Event::Menu(context_menu::Event::OpenFromKeyboard(1)));
    let mut ui = simulator(view(&state, 600.0, false));
    for label in ["Close", "Close to the right", "Close saved", "Pin"] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
    ui.click("Close others").expect("menu item is rendered");
    assert_eq!(
        messages(ui).last(),
        Some(&Event::Menu(context_menu::Event::Menu(
            dropdown_menu::Event::Activate(MenuItem::CloseOthers)
        )))
    );
}

#[test]
fn tabs_without_a_handler_are_disabled() {
    let state = state();
    let element: Element<'_, Message> = document_tabs(&state).into();
    let mut ui = simulator(element);
    ui.click("beta.rs").expect("tab is rendered");
    let close = slot_after(&mut ui, "alpha.rs");
    ui.point_at(close);
    let _ = ui.simulate(simulator::click());
    assert!(messages(ui).is_empty());
}

fn many() -> State<u8> {
    State::new((1..=6).map(|id| document(id, format!("file-{id}.rs")))).with_active(6)
}

#[test]
fn overflowing_tabs_keep_the_active_one_in_view() {
    let state = many();
    let mut ui = simulator(view(&state, 260.0, false));
    let y = centre(&mut ui, "file-1.rs").y;
    // The menu button takes the last 32 pixels. The active tab ends just
    // before it, its close button last.
    let close = Point::new(260.0 - 32.0 - 20.0, y);
    let label = Point::new(260.0 - 32.0 - 60.0, y);
    ui.point_at(close);
    let _ = ui.simulate(simulator::click());
    ui.point_at(label);
    let _ = ui.simulate(simulator::click());
    assert_eq!(messages(ui), [Event::Close(6), Event::Activate(6)]);
}

#[test]
fn found_labels_follow_the_scroll() {
    let state = many();
    let mut ui = simulator(view(&state, 260.0, false));
    let found = ui.find("file-6.rs").expect("label is rendered");
    let last = found.visible_bounds().expect("the active label is visible");
    assert_eq!(last.width, found.bounds().width, "shown in full");
    assert!(last.x > 0.0, "starts at {}", last.x);
    assert!(
        last.x + last.width <= 260.0 - 32.0,
        "ends at {}",
        last.x + last.width
    );
}

#[test]
fn overflowing_tabs_show_a_menu_button_at_the_end() {
    let state = many();
    let mut ui = simulator(view(&state, 260.0, false));
    let y = centre(&mut ui, "file-1.rs").y;
    ui.point_at(Point::new(260.0 - 16.0, y));
    let _ = ui.simulate(simulator::click());
    assert_eq!(
        messages(ui),
        [Event::Overflow(dropdown_menu::Event::Toggle)]
    );
}

#[test]
fn tabs_that_fit_have_no_menu_button() {
    let state = state();
    let mut ui = simulator(view(&state, 600.0, false));
    let y = centre(&mut ui, "alpha.rs").y;
    ui.point_at(Point::new(600.0 - 16.0, y));
    let _ = ui.simulate(simulator::click());
    assert!(messages(ui).is_empty());
}

#[test]
fn the_wheel_scrolls_overflowing_tabs() {
    let state = State::new((1..=6).map(|id| document(id, format!("file-{id}.rs"))));
    let mut ui = simulator(view(&state, 260.0, false));
    let y = centre(&mut ui, "file-1.rs").y;
    let near_start = Point::new(20.0, y);
    ui.point_at(near_start);
    let _ = ui.simulate([Input::Mouse(mouse::Event::WheelScrolled {
        delta: mouse::ScrollDelta::Lines { x: 0.0, y: -100.0 },
    })]);
    let _ = ui.simulate(simulator::click());
    let events = messages(ui);
    assert_eq!(events.len(), 1);
    assert_ne!(events, [Event::Activate(1)], "scrolled past the first tab");
}

#[test]
fn long_labels_stay_on_one_line_and_end_in_an_ellipsis() {
    let label = "a very long file name that would never fit.rs";
    let state = State::new([document(1, label), document(2, "short.rs")]);
    let element: Element<'_, Message> = container(
        document_tabs(&state)
            .max_label_width(120.0)
            .on_event(Message::Tabs),
    )
    .width(200)
    .into();
    let mut ui = simulator(element);
    let bounds = ui.find(label).expect("label is rendered").bounds();
    assert!(bounds.width <= 120.0, "width {}", bounds.width);
    assert!(bounds.height < 24.0, "height {}", bounds.height);
    let short = ui.find("short.rs").expect("label is rendered").bounds();
    assert!(short.height < 24.0);
}
