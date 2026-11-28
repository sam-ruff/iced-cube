#![cfg(feature = "select")]

use iced::keyboard::key::Named;
use iced::widget::{button, column, container, text};
use iced::{Element, Event, Length, Point, mouse};
use iced_cube::forms::select;
use iced_cube::forms::select::HEIGHT;
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Fruit(&'static str),
    Other,
}

const FRUITS: &[&str] = &["Apple", "Banana", "Cherry"];

fn view(selected: Option<&'static str>, enabled: bool) -> Element<'static, Message> {
    let select = select(FRUITS, selected).placeholder("Pick a fruit");
    let select: Element<'static, Message> = if enabled {
        select.on_select(Message::Fruit).into()
    } else {
        select.into()
    };
    column![
        select,
        container(button(text("Other")).on_press(Message::Other))
            .padding([240, 0])
            .width(Length::Fill),
    ]
    .into()
}

fn click_at(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    ui.simulate(click());
}

fn open(ui: &mut Simulator<'_, Message>) {
    click_at(ui, Point::new(100.0, HEIGHT / 2.0));
}

#[test]
fn clicking_the_field_opens_the_list() {
    let mut ui = simulator(view(None, true));
    assert!(ui.find("Pick a fruit").is_ok());
    assert!(ui.find("Banana").is_err());
    open(&mut ui);
    for fruit in FRUITS {
        assert!(ui.find(*fruit).is_ok(), "{fruit}");
    }
    assert!(ui.into_messages().next().is_none());
}

#[test]
fn choosing_an_option_emits_it_and_closes_the_list() {
    let mut ui = simulator(view(None, true));
    open(&mut ui);
    let banana = ui.find("Banana").expect("list is open");
    assert!(banana.bounds().y > HEIGHT, "the list opens below the field");
    ui.click("Banana").expect("option is rendered");
    assert!(ui.find("Cherry").is_err(), "the list closes after a choice");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Fruit("Banana")]);
}

#[test]
fn escape_and_outside_clicks_close_the_list_without_choosing() {
    let mut ui = simulator(view(Some("Apple"), true));
    open(&mut ui);
    let _ = ui.tap_key(Named::Escape);
    assert!(ui.find("Banana").is_err());

    open(&mut ui);
    click_at(&mut ui, Point::new(600.0, 150.0));
    assert!(ui.find("Banana").is_err());

    open(&mut ui);
    ui.click("Other").expect("button below is rendered");
    assert!(ui.find("Banana").is_err());
    assert_eq!(
        ui.into_messages().count(),
        0,
        "the click that closes the list goes no further"
    );
}

#[test]
fn an_open_list_keeps_keys_from_what_is_behind_it() {
    let mut ui = simulator(view(Some("Apple"), true));
    for key in [Named::ArrowDown, Named::Enter, Named::Backspace] {
        let status = ui.tap_key(key);
        assert_eq!(status, iced::event::Status::Ignored, "closed: {key:?}");
    }

    open(&mut ui);
    for key in [
        Named::ArrowDown,
        Named::ArrowUp,
        Named::PageDown,
        Named::PageUp,
        Named::Backspace,
    ] {
        let status = ui.tap_key(key);
        assert_eq!(status, iced::event::Status::Captured, "open: {key:?}");
    }
    assert!(ui.find("Banana").is_ok(), "the list stays open");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn tab_closes_the_list_and_goes_on_to_move_focus() {
    let mut ui = simulator(view(Some("Apple"), true));
    open(&mut ui);
    let status = ui.tap_key(Named::Tab);
    assert_eq!(status, iced::event::Status::Ignored, "left for focus");
    assert!(ui.find("Banana").is_err(), "the list closes");
    assert_eq!(ui.into_messages().count(), 0, "nothing is chosen");
}

#[cfg(feature = "dialog")]
mod keyboard {
    use super::*;

    /// Tabs to the select, the only focusable widget in the dialog.
    fn focus_select(ui: &mut Simulator<'_, Message>) {
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(600.0, 150.0),
        })]);
        let _ = ui.tap_key(Named::Tab);
    }

    /// Puts `content` in a dialog, whose Tab moves focus as an app's
    /// `focus_next` task would.
    fn tab_focuses<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        iced_cube::overlay::dialog::dialog(text("Base"))
            .open(true)
            .body(content)
            .close_button(false)
            .into()
    }

    #[test]
    fn a_focused_select_opens_on_space_enter_or_arrow_down() {
        for key in [Named::Space, Named::Enter, Named::ArrowDown] {
            let mut ui = simulator(tab_focuses(view(Some("Apple"), true)));
            focus_select(&mut ui);
            assert!(ui.find("Banana").is_err(), "{key:?}: closed until the key");
            assert_eq!(ui.tap_key(key), iced::event::Status::Captured);
            assert!(ui.find("Banana").is_ok(), "{key:?} opens the list");
            assert_eq!(ui.into_messages().count(), 0);
        }
    }

    #[test]
    fn arrows_move_the_highlight_and_enter_chooses_it() {
        let mut ui = simulator(tab_focuses(view(Some("Apple"), true)));
        focus_select(&mut ui);
        let _ = ui.tap_key(Named::Space);
        let _ = ui.tap_key(Named::ArrowDown);
        let _ = ui.tap_key(Named::ArrowDown);
        let _ = ui.tap_key(Named::ArrowDown);
        let _ = ui.tap_key(Named::ArrowUp);
        let _ = ui.tap_key(Named::Enter);
        assert!(ui.find("Cherry").is_err(), "choosing closes the list");

        let _ = ui.tap_key(Named::Space);
        let _ = ui.tap_key(Named::End);
        let _ = ui.tap_key(Named::Space);
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![Message::Fruit("Banana"), Message::Fruit("Cherry")]
        );
    }

    #[test]
    fn a_pointer_press_takes_focus_off_the_select() {
        let mut ui = simulator(tab_focuses(view(Some("Apple"), true)));
        focus_select(&mut ui);
        click_at(&mut ui, Point::new(600.0, 150.0));
        let _ = ui.tap_key(Named::Space);
        assert!(ui.find("Banana").is_err());
    }

    #[test]
    fn a_disabled_select_is_never_focused() {
        let mut ui = simulator(tab_focuses(view(Some("Apple"), false)));
        focus_select(&mut ui);
        let _ = ui.tap_key(Named::Space);
        assert!(ui.find("Banana").is_err());
    }
}

#[test]
fn clicking_the_field_again_closes_the_list() {
    let mut ui = simulator(view(None, true));
    open(&mut ui);
    assert!(ui.find("Cherry").is_ok());
    open(&mut ui);
    assert!(ui.find("Cherry").is_err());
}

#[test]
fn a_long_value_stays_clear_of_the_chevron_in_a_narrow_field() {
    const LONG: &str = "postgres://ledger/reconciliation";
    let narrow: Element<'_, Message> =
        container(select(&[LONG], Some(LONG)).on_select(|_| Message::Other))
            .width(160)
            .into();
    let mut ui = simulator(narrow);
    let label = ui.find(LONG).expect("the value is shown").bounds();
    // The chevron and the gap before it sit to the right of the label.
    assert!(
        label.x + label.width <= 160.0 - 16.0 - 8.0,
        "label ends at {}",
        label.x + label.width
    );
}

/// A select whose options lead with a marker, except Cherry.
fn with_leading(selected: Option<&'static str>) -> Element<'static, Message> {
    select(FRUITS, selected)
        .placeholder("Pick a fruit")
        .leading(|fruit| (*fruit != "Cherry").then(|| text(format!("dot-{fruit}")).into()))
        .on_select(Message::Fruit)
        .into()
}

#[test]
fn the_field_draws_the_selected_options_leading_element_before_its_label() {
    let mut ui = simulator(with_leading(Some("Apple")));
    let dot = ui.find("dot-Apple").expect("leading is drawn").bounds();
    let label = ui.find("Apple").expect("label is drawn").bounds();
    assert!(dot.x + dot.width <= label.x, "leading comes first");
    assert!(dot.y < HEIGHT && label.y < HEIGHT, "both sit in the field");

    let mut ui = simulator(with_leading(None));
    assert!(ui.find("Pick a fruit").is_ok());
    assert!(ui.find("dot-Apple").is_err(), "the placeholder has none");

    let mut ui = simulator(with_leading(Some("Cherry")));
    let label = ui.find("Cherry").expect("label is drawn").bounds();
    assert!(
        label.x < 16.0,
        "an option without one starts at the padding"
    );
}

#[test]
fn each_open_option_draws_its_leading_element_and_choosing_still_works() {
    let mut ui = simulator(with_leading(Some("Apple")));
    open(&mut ui);
    let banana_dot = ui.find("dot-Banana").expect("leading in list").bounds();
    let banana = ui.find("Banana").expect("label in list").bounds();
    assert!(banana_dot.y > HEIGHT, "drawn in the list");
    assert!(banana_dot.x + banana_dot.width <= banana.x);
    assert!((banana_dot.y - banana.y).abs() < 4.0, "on the same row");
    assert!(ui.find("dot-Cherry").is_err());

    let status = ui.tap_key(Named::ArrowDown);
    assert_eq!(status, iced::event::Status::Captured);
    ui.click("Banana").expect("option is rendered");
    assert!(ui.find("dot-Banana").is_err(), "the list closes");
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Fruit("Banana")]);
}

#[test]
fn disabled_select_does_not_open() {
    let mut ui = simulator(view(Some("Apple"), false));
    open(&mut ui);
    assert!(ui.find("Banana").is_err());
    assert_eq!(ui.into_messages().count(), 0);
}

#[cfg(feature = "icedwright")]
#[path = "support/semantic.rs"]
mod semantic;

#[cfg(feature = "icedwright")]
#[test]
fn a_select_reports_its_value_and_its_open_list() {
    let mut recorder = semantic::Recorder::default();
    assert_eq!(
        recorder.yaml(view(Some("Banana"), true)),
        r#"- combobox "Pick a fruit" [expanded=false] [focused=false] [focusable] [placeholder="Pick a fruit"]:
  - /value: Banana
  - text: Banana
- text: Other
"#
    );

    let field = Point::new(20.0, HEIGHT / 2.0);
    let cursor = mouse::Cursor::Available(field);
    let _ = recorder.events(
        view(Some("Banana"), true),
        cursor,
        &[
            Event::Mouse(mouse::Event::CursorMoved { position: field }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
    );
    assert_eq!(
        recorder.yaml(view(Some("Banana"), true)),
        r#"- combobox "Pick a fruit" [expanded] [focused=false] [focusable] [placeholder="Pick a fruit"]:
  - /value: Banana
  - text: Banana
  - listbox:
    - generic:
      - scrollview:
        - option "Apple" [selected=false]:
          - text: Apple
        - option "Banana" [selected]:
          - text: Banana
        - option "Cherry" [selected=false]:
          - text: Cherry
- text: Other
"#
    );
}
