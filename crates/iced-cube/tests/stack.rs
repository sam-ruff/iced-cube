use iced::Element;
use iced::widget::text;
use iced_cube::layout::stack::Gap;
use iced_cube::{button, hstack, vstack};
use iced_test::simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Left,
    Right,
}

#[test]
fn every_gap_renders_all_children() {
    for gap in Gap::ALL {
        let element: Element<'_, Message> = vstack([text("Top").into(), text("Bottom").into()])
            .gap(gap)
            .push(
                hstack([text("Left").into(), text("Right").into()])
                    .gap(gap)
                    .wrap(),
            )
            .into();
        let mut ui = simulator(element);
        for label in ["Top", "Bottom", "Left", "Right"] {
            assert!(ui.find(label).is_ok(), "{gap:?} {label}");
        }
    }
}

#[test]
fn children_keep_their_messages() {
    let element: Element<'_, Message> = hstack([
        button("Left").on_press(Message::Left).into(),
        button("Right").on_press(Message::Right).into(),
    ])
    .into();
    let mut ui = simulator(element);
    ui.click("Right").expect("Right is rendered");
    ui.click("Left").expect("Left is rendered");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Right, Message::Left]);
}
