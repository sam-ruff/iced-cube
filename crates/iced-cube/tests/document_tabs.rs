#![cfg(feature = "document-tabs")]

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::widget::container;
use iced::{
    Element, Event as Input, Length, Point, Rectangle, Renderer, Size, Theme, keyboard, mouse,
};
use iced_cube::navigation::document_tabs::{Event, MenuItem, State, document, document_tabs};
use iced_cube::overlay::{context_menu, dropdown_menu};
use iced_test::selector::Candidate;
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
    for label in ["Close", "Close saved", "Close all", "Pin"] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
    assert!(
        ui.find("Close to the right").is_err(),
        "it would close what Close others does"
    );
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

fn finger(ui: &mut Simulator<'_, Message>, from: Point, to: Point) {
    let id = iced::touch::Finger(1);
    ui.point_at(from);
    let _ = ui.simulate([Input::Touch(iced::touch::Event::FingerPressed {
        id,
        position: from,
    })]);
    ui.point_at(to);
    let _ = ui.simulate([
        Input::Touch(iced::touch::Event::FingerMoved { id, position: to }),
        Input::Touch(iced::touch::Event::FingerLifted { id, position: to }),
    ]);
}

#[test]
fn a_tap_activates_a_tab_but_a_swipe_from_one_does_not() {
    let state = state();
    let mut ui = simulator(view(&state, 600.0, false));
    let beta = centre(&mut ui, "beta.rs");
    finger(&mut ui, beta, Point::new(beta.x + 3.0, beta.y));
    assert_eq!(messages(ui), [Event::Activate(2)]);

    let mut ui = simulator(view(&state, 600.0, false));
    let gamma = ui
        .find("gamma.rs")
        .expect("tab is rendered")
        .bounds()
        .center();
    finger(&mut ui, gamma, Point::new(gamma.x - 80.0, gamma.y));
    assert!(
        messages(ui).is_empty(),
        "a swipe that starts on a tab activates nothing"
    );
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

/// Lays its content out at `from` pixels wide until `r` is pressed, then at
/// `to`, as a window resize would.
struct Resizer<'a> {
    content: Element<'a, Message>,
    from: f32,
    to: f32,
}

impl Widget<Message, Theme, Renderer> for Resizer<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Shrink, Length::Shrink)
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<bool>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(false)
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let width = if *tree.state.downcast_ref::<bool>() {
            self.to
        } else {
            self.from
        };
        let limits = layout::Limits::new(Size::ZERO, Size::new(width, limits.max().height));
        let node = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &limits);
        layout::Node::with_children(node.size(), vec![node])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(child) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                child,
                cursor,
                viewport,
            );
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(child) = layout.children().next() {
            self.content
                .as_widget_mut()
                .operate(&mut tree.children[0], child, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Input,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Input::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
            && *key == keyboard::Key::Character("r".into())
        {
            *tree.state.downcast_mut::<bool>() = true;
            shell.invalidate_layout();
            return;
        }
        if let Some(child) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                child,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }
}

/// Asserts the active `file-6.rs` tab is shown in full before the menu
/// button at the end of a strip `width` pixels wide.
fn assert_last_tab_in_view(ui: &mut Simulator<'_, Message>, width: f32) {
    let found = ui
        .find("file-6.rs")
        .map(|target| (target.bounds(), target.visible_bounds()));
    let Ok((bounds, Some(shown))) = found else {
        panic!("the active label is not visible: {found:?}");
    };
    assert_eq!(shown.width, bounds.width, "shown in full");
    assert!(shown.x > 0.0, "starts at {}", shown.x);
    assert!(
        shown.x + shown.width <= width - 32.0,
        "ends at {}",
        shown.x + shown.width
    );
}

#[test]
fn the_active_tab_stays_in_view_when_the_strip_narrows() {
    let state = many();
    let resizer = Resizer {
        content: document_tabs(&state).on_event(Message::Tabs).into(),
        from: 1000.0,
        to: 260.0,
    };
    let mut ui = simulator(Element::new(resizer));
    assert!(ui.find("file-1.rs").is_ok());
    let _ = ui.tap_key(keyboard::Key::Character("r".into()));
    assert_last_tab_in_view(&mut ui, 260.0);
}

#[test]
fn the_active_tab_stays_in_view_when_the_strip_widens() {
    let state = many();
    let resizer = Resizer {
        content: document_tabs(&state).on_event(Message::Tabs).into(),
        from: 200.0,
        to: 300.0,
    };
    let mut ui = simulator(Element::new(resizer));
    let _ = ui.tap_key(keyboard::Key::Character("r".into()));
    assert_last_tab_in_view(&mut ui, 300.0);
}

/// A kept pinned tab, then eight files with the last one active.
fn pinned_and_many() -> State<u8> {
    let files = (1..=8).map(|id| document(id, format!("file-{id}.rs")));
    let jobs = document(0, "Jobs")
        .pinned(true)
        .closable(false)
        .locked(true);
    State::new(std::iter::once(jobs).chain(files)).with_active(8)
}

#[test]
fn pinned_tabs_stay_at_the_start_of_a_narrow_strip() {
    let state = pinned_and_many();
    let mut ui = simulator(view(&state, 300.0, false));
    let found = ui
        .find("Jobs")
        .map(|target| (target.bounds(), target.visible_bounds()));
    let Ok((pinned, Some(shown))) = found else {
        panic!("the pinned label is not visible: {found:?}");
    };
    assert!((shown.width - pinned.width).abs() < 0.01, "shown in full");
    assert_eq!(shown.x, pinned.x);
    assert!(pinned.x < 20.0, "at the left edge, at {}", pinned.x);

    let last = ui.find("file-8.rs").expect("label is rendered");
    let last = last.visible_bounds().expect("the active label is visible");
    assert!(last.x > pinned.x + pinned.width, "after the pinned tab");

    ui.click("Jobs").expect("pinned tab is rendered");
    assert_eq!(messages(ui), [Event::Activate(0)]);
}

#[test]
fn the_start_of_the_scrolling_tabs_hits_an_unpinned_tab() {
    let state = pinned_and_many();
    let mut ui = simulator(view(&state, 300.0, false));
    let pinned = label_bounds(&mut ui, "Jobs");
    // The pinned tab ends one padding after its label.
    let start = Point::new(pinned.x + pinned.width + 12.0 + 4.0, pinned.center_y());
    ui.point_at(start);
    let _ = ui.simulate(simulator::click());
    // A partly scrolled tab sits there, so the press may land on its label
    // or its close button.
    let events = messages(ui);
    assert!(
        matches!(events.as_slice(), [Event::Activate(id) | Event::Close(id)] if *id != 0),
        "{events:?}"
    );
}

#[test]
fn pinned_tabs_reorder_in_place_while_the_others_are_scrolled() {
    let files = (1..=8).map(|id| document(id, format!("file-{id}.rs")));
    let pinned = [document(10, "Jobs"), document(11, "Notes")].map(|d| d.pinned(true));
    let state = State::new(pinned.into_iter().chain(files)).with_active(8);
    let mut ui = simulator(view(&state, 300.0, false));
    let jobs = centre(&mut ui, "Jobs");
    let notes = label_bounds(&mut ui, "Notes");
    let end = Point::new(notes.x + notes.width + 8.0, notes.center_y());
    ui.point_at(jobs);
    let _ = ui.simulate([press(mouse::Button::Left)]);
    ui.point_at(end);
    let _ = ui.simulate([move_to(end), release(mouse::Button::Left)]);
    assert_eq!(messages(ui), [Event::Activate(10), Event::Move(10, 1)]);
}

#[test]
fn hidden_tabs_never_show_under_the_pinned_tab() {
    let state = pinned_and_many();
    let mut ui = simulator(view(&state, 300.0, false));
    let pinned = label_bounds(&mut ui, "Jobs");
    for id in 1..=8 {
        let label = format!("file-{id}.rs");
        let Some(shown) = ui
            .find(label.as_str())
            .ok()
            .and_then(|target| target.visible_bounds())
        else {
            continue;
        };
        assert!(
            shown.x > pinned.x + pinned.width,
            "{label} shows at {shown:?}"
        );
    }
}

#[test]
fn an_unclosable_tab_has_no_close_item_or_button() {
    let mut state = State::new([
        document(0, "Jobs")
            .pinned(true)
            .closable(false)
            .locked(true),
        document(1, "log.txt").closable(false),
        document(2, "a.rs"),
    ])
    .with_active(1);
    let _ = state.update(Event::Menu(context_menu::Event::OpenFromKeyboard(1)));
    let mut ui = simulator(view(&state, 600.0, false));
    // Close others would close only a.rs, as Close all does.
    for label in ["Close", "Unpin", "Close saved", "Close others"] {
        assert!(ui.find(label).is_err(), "{label} is offered");
    }
    for label in ["Close all", "Pin"] {
        assert!(ui.find(label).is_ok(), "{label} is missing");
    }

    let unclosable = State::new([document(1, "log.txt").closable(false), document(2, "a.rs")]);
    let mut ui = simulator(view(&unclosable, 600.0, false));
    let label = label_bounds(&mut ui, "log.txt");
    // Where the close button would be, just after the label.
    let slot = Point::new(label.x + label.width + 6.0, label.center_y());
    ui.point_at(slot);
    let _ = ui.simulate([move_to(slot)]);
    let _ = ui.simulate(simulator::click());
    assert_eq!(messages(ui), [Event::Activate(1)], "a press, not a close");
}

const CHAPTERS: [&str; 12] = [
    "Introduction to the project.md",
    "Getting started.md",
    "Installation.md",
    "Configuration.md",
    "Command line.md",
    "Plugins.md",
    "Theming.md",
    "Keyboard shortcuts.md",
    "Accessibility.md",
    "Deployment.md",
    "Troubleshooting.md",
    "Release notes.md",
];

fn chapters() -> State<u8> {
    let documents = (0u8..)
        .zip(CHAPTERS)
        .map(|(id, title)| document(id, title).modified(id == 4));
    State::new(documents).with_active(10)
}

/// Where the row for `label` sits in the open list of every tab, below the
/// strip, and the part of it that is shown.
fn row_bounds(ui: &mut Simulator<'_, Message>, label: &str) -> (Rectangle, Option<Rectangle>) {
    let selector = move |candidate: Candidate<'_>| match candidate {
        Candidate::Text {
            content,
            bounds,
            visible_bounds,
            ..
        } if content == label && bounds.y > 36.0 => Some((bounds, visible_bounds)),
        _ => None,
    };
    ui.find(selector).unwrap_or_default()
}

fn small_window<'a>(state: &'a State<u8>) -> Simulator<'a, Message> {
    let element: Element<'_, Message> = container(document_tabs(state).on_event(Message::Tabs))
        .width(340)
        .into();
    Simulator::with_size(iced::Settings::default(), Size::new(360.0, 200.0), element)
}

#[test]
fn the_list_of_every_tab_opens_below_its_button_on_one_line_per_tab() {
    let mut state = chapters();
    let _ = state.update(Event::Overflow(dropdown_menu::Event::Toggle));
    let mut ui = small_window(&state);
    let (first, _) = row_bounds(&mut ui, CHAPTERS[0]);
    assert!(first.y >= 36.0 + 4.0, "below the strip, at {}", first.y);
    for title in CHAPTERS {
        let (bounds, _) = row_bounds(&mut ui, title);
        assert!(bounds.height < 24.0, "{title} wraps: {bounds:?}");
        assert!(
            bounds.x + bounds.width <= 360.0,
            "{title} runs off: {bounds:?}"
        );
    }
    let (modified, _) = row_bounds(&mut ui, "Modified");
    assert!(modified.height < 24.0);
}

#[test]
fn the_list_of_every_tab_scrolls_to_the_active_one() {
    let mut state = chapters();
    let _ = state.update(Event::Overflow(dropdown_menu::Event::Toggle));
    let mut ui = small_window(&state);
    let (active, shown) = row_bounds(&mut ui, CHAPTERS[10]);
    let shown = shown.expect("the active row is visible");
    assert_eq!(shown.size(), active.size(), "shown in full");
    assert!(shown.y + shown.height <= 200.0, "inside the window");
    let (_, first) = row_bounds(&mut ui, CHAPTERS[0]);
    assert_eq!(first, None, "the start of the list scrolled away");
}

#[test]
fn the_keyboard_tab_menu_opens_below_the_tab_in_a_short_window() {
    let mut state = chapters();
    let _ = state.update(Event::Menu(context_menu::Event::OpenFromKeyboard(10)));
    let mut ui = small_window(&state);
    let (close, _) = row_bounds(&mut ui, "Close");
    assert!(close.y >= 36.0, "below the strip, at {}", close.y);
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
