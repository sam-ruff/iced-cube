#![cfg(feature = "selectable-text")]

use iced::time::{Duration, Instant};
use iced::widget::{button, column, container, scrollable, span, text_input};
use iced::{Element, Event, Length, Point, Rectangle, keyboard, mouse, window};
use iced_cube::Chord;
use iced_cube::data::selectable_text::{Action, default_keymap, selectable_text, selection_area};
use iced_test::simulator::{Simulator, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Copied(String),
    Selected(String),
    Link(String),
    Pressed,
    Typed(String),
    Scrolled(f32),
}

const FIRST: &str = "alpha beta gamma";
const SECOND: &str = "delta epsilon";

fn page<'a>() -> Element<'a, Message> {
    selection_area(
        column![
            selectable_text::<String, _>(vec![span(FIRST)]),
            selectable_text(vec![
                span("delta ").link("https://example.test".to_owned()),
                span("epsilon"),
            ])
            .on_link_click(Message::Link),
            button("Press").on_press(Message::Pressed),
        ]
        .spacing(20)
        .width(Length::Fill),
    )
    // Long enough that presses in a row are a double click on any machine.
    .double_click(Duration::from_secs(3600))
    .on_copy(Message::Copied)
    .on_select(Message::Selected)
    .into()
}

/// Forty numbered lines in a scrollable 200 pixels tall, with the area
/// around the scrollable so a drag past its edge scrolls it.
fn long_page<'a>() -> Element<'a, Message> {
    let lines = (0..40)
        .map(|number| selectable_text::<String, _>(vec![span(format!("line {number}"))]).into());
    container(
        selection_area(
            scrollable(column(lines).width(Length::Fill))
                .height(200)
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y)),
        )
        .on_select(Message::Selected),
    )
    .padding(100)
    .into()
}

fn bounds(ui: &mut Simulator<'_, Message>, text: &str) -> Rectangle {
    match ui.find(text).map(|found| found.visible_bounds()) {
        Ok(Some(bounds)) => bounds,
        Ok(None) => panic!("{text} is scrolled out of view"),
        Err(error) => panic!("{text} is not drawn: {error}"),
    }
}

fn mouse(ui: &mut Simulator<'_, Message>, at: Point, event: mouse::Event) {
    ui.point_at(at);
    let _ = ui.simulate([Event::Mouse(event)]);
}

fn press(ui: &mut Simulator<'_, Message>, at: Point) {
    mouse(ui, at, mouse::Event::ButtonPressed(mouse::Button::Left));
}

fn release(ui: &mut Simulator<'_, Message>, at: Point) {
    mouse(ui, at, mouse::Event::ButtonReleased(mouse::Button::Left));
}

fn move_to(ui: &mut Simulator<'_, Message>, at: Point) {
    mouse(ui, at, mouse::Event::CursorMoved { position: at });
}

fn drag(ui: &mut Simulator<'_, Message>, from: Point, to: Point) {
    press(ui, from);
    move_to(ui, to);
    release(ui, to);
}

fn frame(ui: &mut Simulator<'_, Message>, at: Instant) {
    let _ = ui.simulate([Event::Window(window::Event::RedrawRequested(at))]);
}

fn key(ui: &mut Simulator<'_, Message>, key: keyboard::Key, modifiers: keyboard::Modifiers) {
    let text = match (&key, modifiers.is_empty()) {
        (keyboard::Key::Character(letter), true) => Some(letter.clone()),
        _ => None,
    };
    // Text fields read letters from the physical key, as winit sends it.
    let physical_key = match &key {
        keyboard::Key::Character(letter) if letter == "a" => {
            keyboard::key::Physical::Code(keyboard::key::Code::KeyA)
        }
        keyboard::Key::Character(letter) if letter == "c" => {
            keyboard::key::Physical::Code(keyboard::key::Code::KeyC)
        }
        _ => keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified),
    };
    let _ = ui.simulate([
        Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)),
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key,
            location: keyboard::Location::Standard,
            modifiers,
            repeat: false,
            text,
        }),
        Event::Keyboard(keyboard::Event::ModifiersChanged(
            keyboard::Modifiers::empty(),
        )),
    ]);
}

fn chord(ui: &mut Simulator<'_, Message>, letter: &str) {
    key(
        ui,
        keyboard::Key::Character(letter.into()),
        keyboard::Modifiers::COMMAND,
    );
}

fn left_middle(bounds: Rectangle) -> Point {
    Point::new(bounds.x + 0.5, bounds.center_y())
}

fn right_of(bounds: Rectangle) -> Point {
    Point::new(bounds.x + bounds.width + 40.0, bounds.center_y())
}

fn selections(messages: &[Message]) -> Vec<&str> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::Selected(text) => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn offsets(messages: &[Message]) -> Vec<f32> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::Scrolled(offset) => Some(*offset),
            _ => None,
        })
        .collect()
}

#[test]
fn a_drag_along_one_text_selects_it_and_the_copy_chord_copies_it() {
    let mut ui = simulator(page());
    let first = bounds(&mut ui, FIRST);
    drag(&mut ui, left_middle(first), right_of(first));
    chord(&mut ui, "c");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        [
            Message::Selected(FIRST.to_owned()),
            Message::Copied(FIRST.to_owned())
        ]
    );
}

#[test]
fn a_drag_across_texts_joins_them_with_a_line_break() {
    let mut ui = simulator(page());
    let first = bounds(&mut ui, FIRST);
    let second = bounds(&mut ui, SECOND);
    drag(&mut ui, left_middle(first), right_of(second));

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, [Message::Selected(format!("{FIRST}\n{SECOND}"))]);
}

#[test]
fn a_drag_upwards_selects_the_same_text() {
    let mut ui = simulator(page());
    let first = bounds(&mut ui, FIRST);
    let second = bounds(&mut ui, SECOND);
    drag(&mut ui, right_of(second), left_middle(first));

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, [Message::Selected(format!("{FIRST}\n{SECOND}"))]);
}

#[test]
fn a_double_click_selects_a_word_and_a_third_click_the_block() {
    let mut ui = simulator(page());
    let first = bounds(&mut ui, FIRST);
    let at = left_middle(first);
    for _ in 0..3 {
        press(&mut ui, at);
        release(&mut ui, at);
    }

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        [
            Message::Selected("alpha".to_owned()),
            Message::Selected(FIRST.to_owned())
        ]
    );
}

#[test]
fn select_all_needs_a_click_in_the_area_first() {
    let mut ui = simulator(page());
    chord(&mut ui, "a");
    let first = bounds(&mut ui, FIRST);
    press(&mut ui, left_middle(first));
    release(&mut ui, left_middle(first));
    chord(&mut ui, "a");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, [Message::Selected(format!("{FIRST}\n{SECOND}"))]);
}

#[test]
fn nothing_is_copied_without_a_selection() {
    let mut ui = simulator(page());
    let first = bounds(&mut ui, FIRST);
    press(&mut ui, left_middle(first));
    release(&mut ui, left_middle(first));
    chord(&mut ui, "c");

    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn escape_and_typing_drop_the_selection() {
    for dropper in [
        keyboard::Key::Named(keyboard::key::Named::Escape),
        keyboard::Key::Character("x".into()),
    ] {
        let mut ui = simulator(page());
        let first = bounds(&mut ui, FIRST);
        drag(&mut ui, left_middle(first), right_of(first));
        key(&mut ui, dropper, keyboard::Modifiers::empty());
        chord(&mut ui, "c");

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, [Message::Selected(FIRST.to_owned())]);
    }
}

#[test]
fn keys_that_type_nothing_keep_the_selection() {
    let mut ui = simulator(page());
    let first = bounds(&mut ui, FIRST);
    drag(&mut ui, left_middle(first), right_of(first));
    key(
        &mut ui,
        keyboard::Key::Named(keyboard::key::Named::PageDown),
        keyboard::Modifiers::empty(),
    );
    chord(&mut ui, "c");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        [
            Message::Selected(FIRST.to_owned()),
            Message::Copied(FIRST.to_owned())
        ]
    );
}

#[test]
fn a_press_outside_the_area_drops_the_selection() {
    let page: Element<'_, Message> = column![
        selection_area(selectable_text::<String, _>(vec![span(FIRST)]))
            .on_copy(Message::Copied)
            .on_select(Message::Selected),
        selectable_text::<String, _>(vec![span("outside")]),
    ]
    .spacing(40)
    .into();
    let mut ui = simulator(page);
    let first = bounds(&mut ui, FIRST);
    let outside = bounds(&mut ui, "outside");
    drag(
        &mut ui,
        left_middle(first),
        Point::new(first.x + first.width - 0.5, first.center_y()),
    );
    press(&mut ui, left_middle(outside));
    release(&mut ui, left_middle(outside));
    chord(&mut ui, "c");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(selections(&messages).len(), 1);
    assert!(!messages.iter().any(|m| matches!(m, Message::Copied(_))));
}

#[test]
fn a_click_on_a_link_follows_it_and_a_drag_from_it_selects() {
    let mut ui = simulator(page());
    let second = bounds(&mut ui, SECOND);
    let link = left_middle(second);
    // Far enough along the link that the drag's press is not a double click.
    let further = Point::new(link.x + 20.0, link.y);
    press(&mut ui, further);
    release(&mut ui, further);
    drag(&mut ui, link, right_of(second));

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        [
            Message::Link("https://example.test".to_owned()),
            Message::Selected(SECOND.to_owned())
        ]
    );
}

#[test]
fn a_button_inside_the_area_still_presses_and_keeps_the_selection() {
    let mut ui = simulator(page());
    let first = bounds(&mut ui, FIRST);
    drag(&mut ui, left_middle(first), right_of(first));
    let _ = ui.click("Press");
    chord(&mut ui, "c");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        [
            Message::Selected(FIRST.to_owned()),
            Message::Pressed,
            Message::Copied(FIRST.to_owned())
        ]
    );
}

#[test]
fn a_right_click_keeps_the_selection() {
    let mut ui = simulator(page());
    let first = bounds(&mut ui, FIRST);
    drag(&mut ui, left_middle(first), right_of(first));
    let at = left_middle(first);
    mouse(
        &mut ui,
        at,
        mouse::Event::ButtonPressed(mouse::Button::Right),
    );
    mouse(
        &mut ui,
        at,
        mouse::Event::ButtonReleased(mouse::Button::Right),
    );
    chord(&mut ui, "c");

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(messages.contains(&Message::Copied(FIRST.to_owned())));
}

#[test]
fn a_focused_field_inside_the_area_keeps_its_own_chords() {
    // The press on the field is the field's, so the selection survives it
    // and both would answer the chords if the area did not stand back.
    let page: Element<'_, Message> = selection_area(column![
        selectable_text::<String, _>(vec![span(FIRST)]),
        text_input("Search", "query").on_input(Message::Typed),
    ])
    .on_copy(Message::Copied)
    .on_select(Message::Selected)
    .into();
    let mut ui = simulator(page);
    let first = bounds(&mut ui, FIRST);
    drag(&mut ui, left_middle(first), right_of(first));
    let _ = ui.click("query");
    chord(&mut ui, "a");
    chord(&mut ui, "c");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, [Message::Selected(FIRST.to_owned())]);
}

#[test]
fn a_keymap_replaces_the_default_chords() {
    let keymap = default_keymap()
        .unbind_action(&Action::Copy)
        .bind(Chord::character('y').command(), Action::Copy);
    let page: Element<'_, Message> =
        selection_area(selectable_text::<String, _>(vec![span(FIRST)]))
            .keymap(keymap)
            .on_copy(Message::Copied)
            .into();
    let mut ui = simulator(page);
    let first = bounds(&mut ui, FIRST);
    drag(&mut ui, left_middle(first), right_of(first));
    chord(&mut ui, "y");
    // The old chord is now someone else's, so it drops the selection.
    chord(&mut ui, "c");
    chord(&mut ui, "y");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, [Message::Copied(FIRST.to_owned())]);
}

#[test]
fn escape_is_captured_only_when_it_drops_a_selection() {
    let escape = || {
        [Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            repeat: false,
            text: None,
        })]
    };
    let mut ui = simulator(page());
    let first = bounds(&mut ui, FIRST);
    drag(&mut ui, left_middle(first), right_of(first));

    assert_eq!(ui.simulate(escape()), [iced::event::Status::Captured]);
    // With nothing selected, Escape is left for a dialog around the area.
    assert_eq!(ui.simulate(escape()), [iced::event::Status::Ignored]);
    press(&mut ui, right_of(first));
    release(&mut ui, right_of(first));
    assert_eq!(ui.simulate(escape()), [iced::event::Status::Ignored]);
}

#[test]
fn tab_and_other_command_chords_drop_the_selection() {
    for (dropper, modifiers) in [
        (
            keyboard::Key::Named(keyboard::key::Named::Tab),
            keyboard::Modifiers::empty(),
        ),
        (
            keyboard::Key::Character("k".into()),
            keyboard::Modifiers::COMMAND,
        ),
    ] {
        let mut ui = simulator(page());
        let first = bounds(&mut ui, FIRST);
        drag(&mut ui, left_middle(first), right_of(first));
        key(&mut ui, dropper, modifiers);
        chord(&mut ui, "c");

        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(messages, [Message::Selected(FIRST.to_owned())]);
    }
}

#[test]
fn text_scrolled_out_of_view_is_never_under_the_pointer() {
    let lines = (0..40)
        .map(|number| selectable_text::<String, _>(vec![span(format!("line {number}"))]).into());
    let page: Element<'_, Message> = selection_area(
        column![
            scrollable(column(lines).width(Length::Fill)).height(100),
            selectable_text::<String, _>(vec![span("footer text")]),
        ]
        .spacing(20)
        .width(Length::Fill),
    )
    .on_select(Message::Selected)
    .into();
    let mut ui = simulator(page);
    let footer = bounds(&mut ui, "footer text");
    drag(&mut ui, left_middle(footer), right_of(footer));

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(selections(&messages), ["footer text"]);
}

#[test]
fn a_drag_inside_a_nested_scrollable_scrolls_neither() {
    let inner = scrollable(
        column((0..40).map(|number| {
            selectable_text::<String, _>(vec![span(format!("inner {number}"))]).into()
        }))
        .width(Length::Fill),
    )
    .height(150)
    .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y));
    let before = (0..30)
        .map(|number| selectable_text::<String, _>(vec![span(format!("before {number}"))]).into());
    let page: Element<'_, Message> =
        selection_area(scrollable(column![column(before), inner].width(Length::Fill)).height(400))
            .on_select(Message::Selected)
            .into();
    let mut ui = simulator(page);
    // Scroll the outer page to its end, so the inner list is in view and
    // far from where it is laid out.
    mouse(
        &mut ui,
        Point::new(50.0, 50.0),
        mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels {
                x: 0.0,
                y: -10_000.0,
            },
        },
    );
    // Drag below the inner list, at the bottom of the page, to scroll it:
    // being pulled back to its top afterwards would then show.
    let top = bounds(&mut ui, "inner 0");
    let start = Instant::now();
    press(&mut ui, left_middle(top));
    move_to(&mut ui, Point::new(top.x + 20.0, 440.0));
    for tick in 0..=10 {
        frame(&mut ui, start + Duration::from_millis(16 * tick));
    }
    release(&mut ui, Point::new(top.x + 20.0, 440.0));

    // Now a drag between two lines well inside the inner list.
    let from = Point::new(top.x + 0.5, 300.0);
    let to = Point::new(top.x + 0.5, 340.0);
    press(&mut ui, from);
    move_to(&mut ui, to);
    for tick in 11..=30 {
        frame(&mut ui, start + Duration::from_millis(16 * tick));
    }
    release(&mut ui, to);

    let messages: Vec<_> = ui.into_messages().collect();
    let scrolled = offsets(&messages);
    assert!(
        scrolled.last().is_some_and(|offset| *offset > 0.0),
        "the first drag did not scroll the inner list: {scrolled:?}"
    );
    assert!(
        scrolled.windows(2).all(|pair| pair[0] <= pair[1]),
        "the inner list was pulled back: {scrolled:?}"
    );
}

#[test]
fn a_scrolled_page_selects_what_is_under_the_pointer() {
    let mut ui = simulator(long_page());
    mouse(
        &mut ui,
        Point::new(150.0, 150.0),
        mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: 0.0, y: -300.0 },
        },
    );
    assert!(
        ui.find("line 0")
            .is_ok_and(|top| top.visible_bounds().is_none()),
        "the page did not scroll"
    );

    let line = bounds(&mut ui, "line 20");
    drag(&mut ui, left_middle(line), right_of(line));

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(selections(&messages), ["line 20"]);
}

#[test]
fn a_drag_below_a_scrollable_scrolls_it_and_selects_what_comes_into_view() {
    let mut ui = simulator(long_page());
    let top = bounds(&mut ui, "line 0");
    press(&mut ui, left_middle(top));
    // 100 of padding and 200 of scrollable: 340 is 40 pixels below it.
    let below = Point::new(top.x + 20.0, 340.0);
    move_to(&mut ui, below);

    let start = Instant::now();
    for tick in 0..=20 {
        frame(&mut ui, start + Duration::from_millis(16 * tick));
    }
    release(&mut ui, below);

    let messages: Vec<_> = ui.into_messages().collect();
    let scrolled = offsets(&messages);
    assert!(
        scrolled.last().is_some_and(|offset| *offset > 100.0),
        "the drag did not scroll: {scrolled:?}"
    );
    assert!(
        scrolled.windows(2).all(|pair| pair[0] <= pair[1]),
        "the page scrolled back: {scrolled:?}"
    );
    let selected = selections(&messages);
    let [selected] = selected.as_slice() else {
        panic!("one selection was expected: {selected:?}");
    };
    assert!(selected.starts_with("line 0\nline 1\n"));
    assert!(
        selected.lines().count() > 12,
        "lines scrolled into view were not selected: {selected}"
    );
}

#[test]
fn a_drag_inside_a_scrollable_does_not_scroll_it() {
    let mut ui = simulator(long_page());
    let top = bounds(&mut ui, "line 0");
    let lower = bounds(&mut ui, "line 3");
    press(&mut ui, left_middle(top));
    move_to(&mut ui, left_middle(lower));
    let start = Instant::now();
    for tick in 0..=10 {
        frame(&mut ui, start + Duration::from_millis(16 * tick));
    }
    release(&mut ui, left_middle(lower));

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(offsets(&messages).iter().all(|offset| *offset == 0.0));
    assert_eq!(selections(&messages), ["line 0\nline 1\nline 2"]);
}

#[test]
fn scrolling_stops_once_the_button_is_released() {
    let mut ui = simulator(long_page());
    let top = bounds(&mut ui, "line 0");
    press(&mut ui, left_middle(top));
    let below = Point::new(top.x + 20.0, 340.0);
    move_to(&mut ui, below);
    release(&mut ui, below);
    let start = Instant::now();
    for tick in 0..=10 {
        frame(&mut ui, start + Duration::from_millis(16 * tick));
    }

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(offsets(&messages).iter().all(|offset| *offset == 0.0));
}

/// The operations behind the tasks a context menu runs.
mod operations {
    use futures::executor::block_on;
    use iced::advanced::widget::operation::{Operation, Outcome, black_box};
    use iced::widget::column;
    use iced::{Font, Pixels, Size};
    use iced_cube::data::selectable_text::operation;
    use iced_test::core::clipboard::Null;
    use iced_test::core::renderer::Headless;
    use iced_test::renderer::Renderer;
    use iced_test::runtime::user_interface::{Cache, UserInterface};

    use super::*;

    const AREA: &str = "article";

    struct Page {
        ui: UserInterface<'static, Message, iced::Theme, Renderer>,
        renderer: Renderer,
    }

    impl Page {
        fn new() -> Self {
            let Some(mut renderer) = block_on(<Renderer as Headless>::new(
                Font::with_name("Fira Sans"),
                Pixels(16.0),
                Some("tiny-skia"),
            )) else {
                panic!("a headless renderer");
            };
            let page: Element<'static, Message> = column![
                selection_area(
                    column![
                        selectable_text::<String, _>(vec![span(FIRST)]),
                        selectable_text::<String, _>(vec![span(SECOND)]),
                    ]
                    .width(Length::Fill),
                )
                .id(AREA),
                selection_area(selectable_text::<String, _>(vec![span("elsewhere")])).id("other"),
            ]
            .into();
            let ui = UserInterface::build(
                page,
                Size::new(800.0, 600.0),
                Cache::default(),
                &mut renderer,
            );
            Self { ui, renderer }
        }

        fn mouse(&mut self, at: Point, event: mouse::Event) {
            let mut messages = Vec::new();
            let _ = self.ui.update(
                &[Event::Mouse(event)],
                mouse::Cursor::Available(at),
                &mut self.renderer,
                &mut Null,
                &mut messages,
            );
        }

        /// Drags along the first line, which is at the top left corner.
        fn select_first_line(&mut self) {
            let from = Point::new(0.5, 8.0);
            let to = Point::new(700.0, 8.0);
            self.mouse(from, mouse::Event::ButtonPressed(mouse::Button::Left));
            self.mouse(to, mouse::Event::CursorMoved { position: to });
            self.mouse(to, mouse::Event::ButtonReleased(mouse::Button::Left));
        }

        fn run(&mut self, mut operation: impl Operation<String>) -> Option<String> {
            self.ui
                .operate(&self.renderer, &mut black_box(&mut operation));
            match operation.finish() {
                Outcome::Some(text) => Some(text),
                _ => None,
            }
        }
    }

    #[test]
    fn selected_reads_the_selection_of_its_area() {
        let mut page = Page::new();
        assert_eq!(page.run(operation::selected(AREA)), Some(String::new()));

        page.select_first_line();
        assert_eq!(page.run(operation::selected(AREA)), Some(FIRST.to_owned()));
        assert_eq!(page.run(operation::selected("other")), Some(String::new()));
    }

    #[test]
    fn select_all_selects_its_area_and_leaves_the_others() {
        let mut page = Page::new();
        let everything = format!("{FIRST}\n{SECOND}");
        assert_eq!(
            page.run(operation::select_all(AREA)),
            Some(everything.clone())
        );
        assert_eq!(page.run(operation::selected(AREA)), Some(everything));
        assert_eq!(page.run(operation::selected("other")), Some(String::new()));
    }

    #[test]
    fn an_id_no_area_has_produces_nothing() {
        let mut page = Page::new();
        assert_eq!(page.run(operation::selected("missing")), None);
        assert_eq!(page.run(operation::select_all("missing")), None);
    }
}

#[cfg(feature = "icedwright")]
#[path = "support/semantic.rs"]
mod semantic;

#[cfg(feature = "icedwright")]
#[test]
fn texts_report_their_size_and_caret_positions_inside_a_document() {
    let element: Element<'_, Message> = selection_area(column![
        selectable_text::<String, _>([span("Hello world")]).size(14.0),
        selectable_text::<String, _>([span("Second")]),
    ])
    .into();
    let tree = semantic::Recorder::default().tree(element);
    assert_eq!(
        semantic::write(&tree),
        "- document:\n  - text: Hello world\n  - text: Second\n"
    );
    let Some(hello) = tree
        .iter()
        .find(|node| node.name.as_deref() == Some("Hello world"))
    else {
        panic!("no node for the first text");
    };
    assert_eq!(hello.text_size, Some(14.0));
    let Some(geometry) = &hello.text_geometry else {
        panic!("no text geometry");
    };
    let start = geometry.caret(0).map(|point| point.x);
    let space = geometry.caret(5).map(|point| point.x);
    let end = geometry.caret(11).map(|point| point.x);
    assert_eq!(start, Some(0.0));
    assert!(space > start && end > space, "{start:?} {space:?} {end:?}");
}
