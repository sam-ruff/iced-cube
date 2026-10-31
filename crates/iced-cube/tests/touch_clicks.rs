#![cfg(feature = "touch-clicks")]

use iced::widget::text_editor::{self, Action, Content};
use iced::widget::{column, container, mouse_area, rich_text, scrollable, space, span, stack};
use iced::{Element, Event, Length, Point, mouse, touch};
use iced_cube::touch_clicks;
use iced_test::simulator::{Simulator, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Edit(Action),
    Link(&'static str),
    Scrolled(f32),
}

fn touch_at(ui: &mut Simulator<'_, Message>, event: touch::Event, at: Point) {
    ui.point_at(at);
    let _ = ui.simulate([Event::Touch(event)]);
}

fn tap(ui: &mut Simulator<'_, Message>, at: Point) {
    let id = touch::Finger(1);
    touch_at(ui, touch::Event::FingerPressed { id, position: at }, at);
    touch_at(ui, touch::Event::FingerLifted { id, position: at }, at);
}

fn drag(ui: &mut Simulator<'_, Message>, from: Point, dy: f32) {
    let id = touch::Finger(1);
    touch_at(ui, touch::Event::FingerPressed { id, position: from }, from);
    let mut at = from;
    for _ in 0..6 {
        at = Point::new(at.x, at.y + dy / 6.0);
        touch_at(ui, touch::Event::FingerMoved { id, position: at }, at);
    }
    touch_at(ui, touch::Event::FingerLifted { id, position: at }, at);
}

fn link() -> Element<'static, Message> {
    rich_text![span("Open the guide").link("guide")]
        .on_link_click(Message::Link)
        .into()
}

#[test]
fn a_tap_on_a_link_publishes_it() {
    let mut ui = simulator(column![touch_clicks(link()), space().height(400)]);
    tap(&mut ui, Point::new(20.0, 8.0));
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Link("guide")]);
}

#[test]
fn without_the_wrapper_a_tap_on_a_link_does_nothing() {
    let mut ui = simulator(column![link(), space().height(400)]);
    tap(&mut ui, Point::new(20.0, 8.0));
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn a_drag_over_a_link_scrolls_and_clicks_nothing() {
    let content = column![touch_clicks(link()), space().height(2000)];
    let view = scrollable(content)
        .height(400)
        .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y));
    let mut ui = simulator(view);
    drag(&mut ui, Point::new(20.0, 8.0), 120.0);
    drag(&mut ui, Point::new(20.0, 200.0), -120.0);
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .all(|message| matches!(message, Message::Scrolled(_))),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Scrolled(y) if *y > 50.0)),
        "{messages:?}"
    );
}

fn editor(content: &Content) -> Element<'_, Message> {
    iced::widget::text_editor(content)
        .id("notes")
        .height(120)
        .on_action(Message::Edit)
        .into()
}

#[test]
fn a_tap_focuses_an_editor_and_places_the_caret() {
    let content = Content::with_text("first line\nsecond line\nthird line");
    let view = column![touch_clicks(editor(&content)), space().height(300)];
    let mut ui = simulator(view);
    tap(&mut ui, Point::new(40.0, 30.0));
    let _ = ui.typewrite("x");
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        matches!(messages.first(), Some(Message::Edit(Action::Click(_)))),
        "{messages:?}"
    );
    assert!(
        messages.contains(&Message::Edit(Action::Edit(text_editor::Edit::Insert('x')))),
        "{messages:?}"
    );
}

#[test]
fn each_tap_is_exactly_one_click() {
    let content = Content::with_text("first line\nsecond line");
    let view = column![touch_clicks(editor(&content)), space().height(300)];
    let mut ui = simulator(view);
    tap(&mut ui, Point::new(40.0, 30.0));
    let clicks = ui
        .into_messages()
        .filter(|message| matches!(message, Message::Edit(Action::Click(_))))
        .count();
    assert_eq!(clicks, 1);
}

#[test]
fn a_tap_on_a_layer_above_does_not_click_the_content_below() {
    // An interactive layer that ignores touch, such as a panel over the
    // text, hides the content from the cursor as it would from a mouse.
    let cover = mouse_area(container(space()).width(Length::Fill).height(40))
        .interaction(mouse::Interaction::Pointer);
    let view = column![stack![touch_clicks(link()), cover], space().height(400)];
    let mut ui = simulator(view);
    tap(&mut ui, Point::new(20.0, 8.0));
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn a_tap_on_a_layer_above_unfocuses_the_editor_below() {
    let content = Content::with_text("text");
    let cover = mouse_area(container(space()).width(Length::Fill).height(40))
        .interaction(mouse::Interaction::Pointer);
    let view = column![
        stack![touch_clicks(editor(&content)), cover],
        space().height(300)
    ];
    let mut ui = simulator(view);
    tap(&mut ui, Point::new(40.0, 80.0));
    tap(&mut ui, Point::new(40.0, 20.0));
    let _ = ui.typewrite("x");
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        matches!(messages.first(), Some(Message::Edit(Action::Click(_)))),
        "{messages:?}"
    );
    assert!(
        !messages
            .iter()
            .any(|message| matches!(message, Message::Edit(Action::Edit(_)))),
        "{messages:?}"
    );
}

#[test]
fn a_tap_elsewhere_reaches_the_editor_as_a_click_outside() {
    let content = Content::with_text("text");
    let view = column![touch_clicks(editor(&content)), space().height(300)];
    let mut ui = simulator(view);
    tap(&mut ui, Point::new(40.0, 30.0));
    tap(&mut ui, Point::new(40.0, 300.0));
    let _ = ui.typewrite("x");
    let inserted = ui
        .into_messages()
        .any(|message| matches!(message, Message::Edit(Action::Edit(_))));
    assert!(!inserted, "the editor lost focus");
}

fn long_text() -> String {
    (1..=60)
        .map(|n| format!("line {n}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn scroll_mode_scrolls_the_editor_and_keeps_the_outer_list_still() {
    let content = Content::with_text(&long_text());
    let inner = touch_clicks(editor(&content)).scroll(true);
    let view = scrollable(column![inner, space().height(2000)].width(Length::Fill))
        .height(400)
        .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y));
    let mut ui = simulator(view);
    drag(&mut ui, Point::new(40.0, 100.0), -84.0);
    let messages: Vec<_> = ui.into_messages().collect();
    let lines: i32 = messages
        .iter()
        .filter_map(|message| match message {
            Message::Edit(Action::Scroll { lines }) => Some(*lines),
            _ => None,
        })
        .sum();
    assert!(lines >= 3, "scrolled {lines} lines: {messages:?}");
    assert!(
        !messages
            .iter()
            .any(|message| matches!(message, Message::Edit(Action::Click(_)))),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .all(|message| !matches!(message, Message::Scrolled(y) if *y > 0.0)),
        "{messages:?}"
    );
}

#[test]
fn without_scroll_mode_a_drag_on_an_editor_scrolls_the_outer_list() {
    let content = Content::with_text(&long_text());
    let view = scrollable(column![
        touch_clicks(editor(&content)),
        space().height(2000)
    ])
    .height(400)
    .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y));
    let mut ui = simulator(view);
    drag(&mut ui, Point::new(40.0, 100.0), -84.0);
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|message| matches!(message, Message::Scrolled(y) if *y > 50.0)),
        "{messages:?}"
    );
    assert!(
        !messages
            .iter()
            .any(|message| matches!(message, Message::Edit(_))),
        "{messages:?}"
    );
}
