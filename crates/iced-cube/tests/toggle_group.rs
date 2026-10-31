#![cfg(feature = "toggle-group")]

use iced::Element;
use iced::widget::column;
use iced_cube::primitives::toggle_group::{Event, State, Variant, item, toggle_group};
use iced_test::simulator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Bold,
    Italic,
    Underline,
}

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Group(Event<Format>),
}

fn items() -> [iced_cube::primitives::toggle_group::Item<Format>; 3] {
    [
        item(Format::Bold, "Bold"),
        item(Format::Italic, "Italic"),
        item(Format::Underline, "Underline").disabled(true),
    ]
}

fn view(state: &State<Format>, variant: Variant) -> Element<'_, Message> {
    toggle_group(state)
        .variant(variant)
        .on_event(Message::Group)
        .into()
}

#[test]
fn clicking_an_item_emits_toggle() {
    for variant in Variant::ALL {
        let state = State::multiple(items());
        let mut ui = simulator(view(&state, variant));
        ui.click("Italic").expect("Italic is rendered");
        ui.click("Bold").expect("Bold is rendered");
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![
                Message::Group(Event::Toggle(Format::Italic)),
                Message::Group(Event::Toggle(Format::Bold)),
            ],
            "{variant:?}"
        );
    }
}

#[test]
fn disabled_items_emit_nothing() {
    let state = State::single(items());
    let mut ui = simulator(view(&state, Variant::Outline));
    ui.click("Underline").expect("Underline is still rendered");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn emitted_messages_drive_single_and_multiple_state() {
    for (mut state, expected) in [
        (State::single(items()), vec![Format::Italic]),
        (State::multiple(items()), vec![Format::Bold, Format::Italic]),
    ] {
        let messages: Vec<_> = {
            let mut ui = simulator(view(&state, Variant::Default));
            ui.click("Bold").expect("Bold is rendered");
            ui.click("Italic").expect("Italic is rendered");
            ui.into_messages().collect()
        };
        for Message::Group(event) in messages {
            let _ = state.update(event);
        }
        assert_eq!(state.selected(), expected);
    }
}

#[test]
fn a_group_without_a_handler_is_disabled() {
    let state = State::single(items());
    let element: Element<'_, Message> = toggle_group(&state).into();
    let mut ui = simulator(element);
    ui.click("Bold").expect("Bold is rendered");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn labels_stay_on_one_line_in_a_narrow_container() {
    let state = State::single([item(0, "Left aligned"), item(1, "Right aligned")]);
    for variant in Variant::ALL {
        let element: Element<'_, u8> = column![toggle_group(&state).variant(variant)]
            .width(60)
            .into();
        let mut ui = simulator(element);
        for label in ["Left aligned", "Right aligned"] {
            let bounds = ui.find(label).expect("item is rendered").bounds();
            assert!(
                bounds.height < 24.0,
                "{variant:?} {label}: {}",
                bounds.height
            );
        }
        let left = ui.find("Left aligned").expect("first item").bounds();
        let right = ui.find("Right aligned").expect("second item").bounds();
        match variant {
            Variant::Default => assert!(right.y > left.y, "separate toggles wrap"),
            Variant::Outline => assert!(right.x > left.x + left.width, "segments share a row"),
        }
    }
}

#[test]
fn a_height_overrides_the_size_and_the_whole_height_takes_a_press() {
    let state = State::single(items());
    for variant in Variant::ALL {
        let element: Element<'_, Message> = column![
            toggle_group(&state)
                .variant(variant)
                .height(48.0)
                .on_event(Message::Group)
        ]
        .into();
        let mut ui = simulator(element);
        let label = ui.find("Bold").expect("Bold is rendered").bounds().center();
        // 20 above the label's centre is outside a 36 pixel item and inside a 48 pixel one.
        let at = iced::Point::new(label.x, label.y - 20.0);
        ui.point_at(at);
        let _ = ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
        ]);
        assert_eq!(ui.into_messages().count(), 1, "{variant:?}");
    }
}

fn segmented(state: &State<Format>, variant: Variant) -> Element<'_, Message> {
    toggle_group(state)
        .variant(variant)
        .width(iced::Length::Fill)
        .on_event(Message::Group)
        .into()
}

#[test]
fn a_fill_width_shares_the_row_equally_with_centred_labels() {
    let state = State::single(items()).required(true);
    for variant in Variant::ALL {
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(360.0, 200.0),
            segmented(&state, variant),
        );
        let centres: Vec<f32> = ["Bold", "Italic", "Underline"]
            .into_iter()
            .map(|label| ui.find(label).expect("rendered").bounds().center_x())
            .collect();
        let step = centres[1] - centres[0];
        assert!(
            (centres[2] - centres[1] - step).abs() < 1.5,
            "{variant:?} {centres:?}"
        );
        // Three equal items across about 360 pixels.
        assert!((step - 120.0).abs() < 4.0, "{variant:?} {centres:?}");
    }
}

#[test]
fn taps_toggle_items() {
    let state = State::single(items()).required(true);
    for variant in Variant::ALL {
        let mut ui = simulator(segmented(&state, variant));
        let at = ui.find("Italic").expect("rendered").bounds().center();
        let id = iced::touch::Finger(1);
        ui.point_at(at);
        let _ = ui.simulate([
            iced::Event::Touch(iced::touch::Event::FingerPressed { id, position: at }),
            iced::Event::Touch(iced::touch::Event::FingerLifted { id, position: at }),
        ]);
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![Message::Group(Event::Toggle(Format::Italic))],
            "{variant:?}"
        );
    }
}

#[cfg(feature = "button")]
#[test]
fn an_outlined_group_is_as_tall_as_a_button() {
    let state = State::single([item(0, "A")]);
    let element: Element<'_, u8> = column![
        toggle_group(&state).variant(Variant::Outline),
        iced_cube::button("B"),
    ]
    .into();
    let mut ui = simulator(element);
    let a = ui.find("A").expect("item").bounds();
    let b = ui.find("B").expect("button").bounds();
    assert_eq!(b.y - a.y, 36.0);
}
