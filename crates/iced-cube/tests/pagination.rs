#![cfg(feature = "pagination")]

use iced::widget::column;
use iced::{Element, Point};
use iced_cube::navigation::pagination::{Event, State, Variant, pagination};
use iced_test::simulator::{self, Simulator, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Page(Event),
}

/// Clicks at a point, where `click` with a point would click the centre of
/// the outermost widget containing it.
fn click_at(ui: &mut Simulator<'_, Message>, point: Point) {
    ui.point_at(point);
    let _ = ui.simulate(simulator::click());
}

fn view(state: &State, variant: Variant) -> Element<'_, Message> {
    pagination(state)
        .variant(variant)
        .on_event(Message::Page)
        .into()
}

#[test]
fn clicking_a_page_number_selects_it() {
    let state = State::new(10).with_page(5);
    let mut ui = simulator(view(&state, Variant::Numbers));
    ui.click("6").expect("page 6 is shown");
    ui.click("10").expect("the last page is shown");
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        vec![
            Message::Page(Event::Select(6)),
            Message::Page(Event::Select(10))
        ]
    );
}

#[test]
fn pages_far_from_the_current_one_are_left_out() {
    let state = State::new(20).with_page(10);
    let mut ui = simulator(view(&state, Variant::Numbers));
    for shown in ["1", "9", "10", "11", "20"] {
        assert!(ui.find(shown).is_ok(), "{shown}");
    }
    for hidden in ["2", "8", "12", "19"] {
        assert!(ui.find(hidden).is_err(), "{hidden}");
    }
}

#[test]
fn the_arrows_step_one_page_and_stop_at_the_ends() {
    let middle = State::new(10).with_page(5);
    let mut ui = simulator(view(&middle, Variant::Numbers));
    let first = ui.find("1").expect("page 1").bounds();
    let last = ui.find("10").expect("page 10").bounds();
    let centre = first.center_y();
    click_at(&mut ui, Point::new(first.x - 24.0, centre));
    click_at(&mut ui, Point::new(last.x + last.width + 24.0, centre));
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        vec![Message::Page(Event::Previous), Message::Page(Event::Next)]
    );

    let start = State::new(10);
    let mut ui = simulator(view(&start, Variant::Numbers));
    let first = ui.find("1").expect("page 1").bounds();
    click_at(&mut ui, Point::new(first.x - 24.0, first.center_y()));
    assert_eq!(
        ui.into_messages().count(),
        0,
        "previous is disabled on page 1"
    );
}

#[test]
fn compact_shows_the_position_and_steps_with_its_arrows() {
    let mut state = State::new(12).with_page(3);
    let messages: Vec<_> = {
        let mut ui = simulator(view(&state, Variant::Compact));
        let label = ui.find("Page 3 of 12").expect("position label").bounds();
        let centre = label.center_y();
        // First, previous, label, next, last: each arrow is 36 pixels with
        // a 4 pixel gap, and the label has 8 pixels of padding each side.
        click_at(&mut ui, Point::new(label.x - 8.0 - 4.0 - 18.0, centre));
        click_at(
            &mut ui,
            Point::new(label.x - 8.0 - 4.0 - 36.0 - 4.0 - 18.0, centre),
        );
        let end = label.x + label.width;
        click_at(
            &mut ui,
            Point::new(end + 8.0 + 4.0 + 36.0 + 4.0 + 18.0, centre),
        );
        ui.into_messages().collect()
    };
    assert_eq!(
        messages,
        vec![
            Message::Page(Event::Previous),
            Message::Page(Event::First),
            Message::Page(Event::Last),
        ]
    );
    for Message::Page(event) in messages {
        let _ = state.update(event);
    }
    assert_eq!(state.page(), 12);
}

#[test]
fn pagination_without_a_handler_is_disabled() {
    let state = State::new(5).with_page(2);
    let element: Element<'_, Message> = pagination(&state).into();
    let mut ui = simulator(element);
    ui.click("3").expect("page 3 is shown");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn numbers_drop_siblings_to_fit_a_phone() {
    let state = State::new(10).with_page(5);
    // Seven slots and two arrows at 36 pixels with 4 pixel gaps take 356
    // pixels; without siblings, five slots take 276.
    let element: Element<'_, Message> = column![view(&state, Variant::Numbers)].width(320).into();
    let mut ui = simulator(element);
    assert!(ui.find("5").is_ok());
    assert!(ui.find("4").is_err(), "siblings are dropped");
    let last = ui.find("10").expect("the last page is shown").bounds();
    assert!(last.x + last.width < 320.0);
}

#[test]
fn numbers_fall_back_to_compact_when_nothing_else_fits() {
    let state = State::new(2000).with_page(1000);
    let element: Element<'_, Message> = column![view(&state, Variant::Numbers)].width(200).into();
    let mut ui = simulator(element);
    assert!(ui.find("1000").is_err());
    let label = ui
        .find("Page 1000 of 2000")
        .expect("compact label")
        .bounds();
    assert!(label.height < 24.0, "one line, got {}", label.height);
}
