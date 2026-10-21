#![cfg(feature = "sidebar")]

use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::text;
use iced::{Element, Event as Input, Point, Settings, Size, mouse};
use iced_cube::navigation::sidebar::{
    self, Event, Output, Row, State, TRIGGER_ID, group, item, sidebar,
};
use iced_test::simulator::{Simulator, click};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Inbox,
    Projects,
    Alpha,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Message {
    Sidebar(Event<Page>),
}

const WIDE: Size = Size::new(900.0, 500.0);
const PHONE: Size = Size::new(360.0, 640.0);

fn state() -> State<Page> {
    State::new([
        group(
            "Platform",
            [
                item(Page::Home, "Home").icon(iced_cube::lucide!(House)),
                item(Page::Inbox, "Inbox")
                    .icon(iced_cube::lucide!(Inbox))
                    .badge("4"),
                item(Page::Projects, "Projects")
                    .icon(iced_cube::lucide!(FolderOpen))
                    .children([item(Page::Alpha, "Alpha")]),
            ],
        ),
        group(
            "Account",
            [item(Page::Settings, "Settings").icon(iced_cube::lucide!(Settings))],
        )
        .collapsible(true),
    ])
    .with_active(Page::Home)
}

fn view(state: &State<Page>) -> Element<'_, Message> {
    sidebar(state)
        .content(text("Page content"))
        .on_event(Message::Sidebar)
        .into()
}

/// Sends `inputs` to a fresh view, applies the messages and returns the
/// events and outputs, as an app would.
fn run(
    state: &mut State<Page>,
    size: Size,
    act: impl FnOnce(&mut Simulator<'_, Message>),
) -> (Vec<Event<Page>>, Vec<Output<Page>>) {
    let messages: Vec<Message> = {
        let mut ui = Simulator::with_size(Settings::default(), size, view(state));
        act(&mut ui);
        ui.into_messages().collect()
    };
    let events: Vec<Event<Page>> = messages
        .into_iter()
        .map(|Message::Sidebar(event)| event)
        .collect();
    let outputs = events
        .iter()
        .filter_map(|event| state.update(*event))
        .collect();
    (events, outputs)
}

fn key(named: Named) -> Input {
    Input::Keyboard(keyboard::Event::KeyPressed {
        key: Key::Named(named),
        modified_key: Key::Named(named),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: Modifiers::empty(),
        text: None,
        repeat: false,
    })
}

fn ctrl_b() -> Input {
    Input::Keyboard(keyboard::Event::KeyPressed {
        key: Key::Character("b".into()),
        modified_key: Key::Character("b".into()),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: Modifiers::COMMAND,
        text: None,
        repeat: false,
    })
}

fn tap(ui: &mut Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate([Input::Mouse(mouse::Event::CursorMoved { position })]);
    let _ = ui.simulate(click());
}

#[test]
fn clicking_an_item_activates_it_and_focuses_the_sidebar() {
    let mut state = state();
    let (events, outputs) = run(&mut state, WIDE, |ui| {
        ui.click("Inbox").expect("Inbox row");
    });
    assert!(events.contains(&Event::Focus(true)), "{events:?}");
    assert_eq!(outputs, [Output::Activated(Page::Inbox)]);
    assert_eq!(state.active(), Some(Page::Inbox));
    assert!(state.is_focused());
}

#[test]
fn a_parent_opens_its_children() {
    let mut state = state();
    let _ = run(&mut state, WIDE, |ui| {
        ui.click("Projects").expect("Projects row");
    });
    assert!(state.is_expanded(Page::Projects));
    let (_, outputs) = run(&mut state, WIDE, |ui| {
        ui.click("Alpha").expect("child row");
    });
    assert_eq!(outputs, [Output::Activated(Page::Alpha)]);
}

#[test]
fn the_focused_sidebar_moves_and_activates_with_the_keyboard() {
    let mut state = state();
    let _ = run(&mut state, WIDE, |ui| {
        ui.click("Home").expect("Home row");
    });
    assert_eq!(state.highlighted(), Some(Row::Item(Page::Home)));

    let (events, outputs) = run(&mut state, WIDE, |ui| {
        let _ = ui.simulate([key(Named::ArrowDown)]);
    });
    assert_eq!(events, [Event::Next]);
    assert!(outputs.is_empty());
    assert_eq!(state.highlighted(), Some(Row::Item(Page::Inbox)));

    let (_, outputs) = run(&mut state, WIDE, |ui| {
        let _ = ui.simulate([key(Named::Enter)]);
    });
    assert_eq!(outputs, [Output::Activated(Page::Inbox)]);

    let _ = run(&mut state, WIDE, |ui| {
        let _ = ui.simulate([key(Named::End)]);
    });
    assert_eq!(state.highlighted(), Some(Row::Item(Page::Settings)));
    let _ = run(&mut state, WIDE, |ui| {
        let _ = ui.simulate([key(Named::ArrowLeft)]);
    });
    let _ = run(&mut state, WIDE, |ui| {
        let _ = ui.simulate([key(Named::ArrowLeft)]);
    });
    assert!(!state.is_group_open(1), "Left collapsed the group");
    let (_, _) = run(&mut state, WIDE, |ui| {
        ui.find("Settings")
            .expect_err("hidden while the group is closed");
        let _ = ui.simulate([key(Named::ArrowRight)]);
    });
    assert!(state.is_group_open(1));
}

#[test]
fn keys_pass_by_an_unfocused_sidebar_and_escape_releases_it() {
    let mut state = state();
    let (events, _) = run(&mut state, WIDE, |ui| {
        let _ = ui.simulate([key(Named::ArrowDown)]);
    });
    assert!(events.is_empty(), "{events:?}");

    let _ = run(&mut state, WIDE, |ui| {
        ui.click("Home").expect("Home row");
    });
    let _ = run(&mut state, WIDE, |ui| {
        let _ = ui.simulate([key(Named::Escape)]);
    });
    assert!(!state.is_focused());

    let _ = run(&mut state, WIDE, |ui| {
        ui.click("Home").expect("Home row");
    });
    let _ = run(&mut state, WIDE, |ui| {
        tap(ui, Point::new(600.0, 300.0));
    });
    assert!(!state.is_focused(), "a press on the page takes the focus");
}

#[test]
fn ctrl_b_toggles_the_rail_while_focused() {
    let mut state = state();
    let _ = run(&mut state, WIDE, |ui| {
        ui.click("Home").expect("Home row");
    });
    let _ = run(&mut state, WIDE, |ui| {
        let _ = ui.simulate([ctrl_b()]);
    });
    assert!(state.is_compact());

    let mut ui = Simulator::with_size(Settings::default(), WIDE, view(&state));
    assert!(ui.find("Inbox").is_err(), "the rail shows icons only");
    assert!(ui.find("Page content").is_ok());
}

#[test]
fn rail_icons_still_activate() {
    let mut state = state().with_rail(true);
    // The rail is 48 wide with 8 of padding, and rows are 32 tall with 4
    // between them.
    let (_, outputs) = run(&mut state, WIDE, |ui| {
        tap(ui, Point::new(24.0, 8.0 + 32.0 + 4.0 + 16.0));
    });
    assert_eq!(outputs, [Output::Activated(Page::Inbox)]);
}

#[test]
fn narrow_windows_open_and_dismiss_the_drawer() {
    let mut state = state();
    let (events, _) = run(&mut state, PHONE, |ui| {
        let _ = ui.simulate([Input::Mouse(mouse::Event::CursorMoved {
            position: Point::new(200.0, 300.0),
        })]);
    });
    assert!(events.contains(&Event::Narrow(true)));
    assert!(state.is_narrow());

    let _ = run(&mut state, PHONE, |ui| {
        assert!(ui.find("Inbox").is_err(), "drawer starts closed");
        ui.click(TRIGGER_ID).expect("trigger");
    });
    assert!(state.is_drawer_open());

    let _ = run(&mut state, PHONE, |ui| {
        assert!(ui.find("Inbox").is_ok(), "drawer shows labels");
        tap(ui, Point::new(340.0, 400.0));
    });
    assert!(!state.is_drawer_open(), "a tap on the scrim closes it");

    let _ = run(&mut state, PHONE, |ui| {
        ui.click(TRIGGER_ID).expect("trigger");
    });
    let _ = run(&mut state, PHONE, |ui| {
        let _ = ui.simulate([key(Named::Escape)]);
    });
    assert!(!state.is_drawer_open(), "Escape closes it");

    let _ = run(&mut state, PHONE, |ui| {
        ui.click(TRIGGER_ID).expect("trigger");
    });
    let (_, outputs) = run(&mut state, PHONE, |ui| {
        ui.click("Inbox").expect("Inbox row");
    });
    assert_eq!(outputs, [Output::Activated(Page::Inbox)]);
    assert!(!state.is_drawer_open(), "choosing an item closes it");
}

#[test]
fn the_drawer_resolves_keys_while_open() {
    let mut state = state();
    let _ = state.update(Event::Narrow(true));
    let _ = state.update(Event::OpenDrawer);
    assert_eq!(state.highlighted(), Some(Row::Item(Page::Home)));
    let (_, outputs) = run(&mut state, PHONE, |ui| {
        let _ = ui.simulate([key(Named::ArrowDown), key(Named::Enter)]);
    });
    assert_eq!(outputs, [Output::Activated(Page::Inbox)]);
    assert!(!state.is_drawer_open());
}

#[test]
fn widening_the_window_docks_the_sidebar() {
    let mut state = state();
    let _ = state.update(Event::Narrow(true));
    let _ = state.update(Event::OpenDrawer);
    let (events, _) = run(&mut state, WIDE, |ui| {
        let _ = ui.simulate([Input::Mouse(mouse::Event::CursorMoved {
            position: Point::new(400.0, 300.0),
        })]);
    });
    assert!(events.contains(&Event::Narrow(false)));
    assert!(!state.is_drawer_open());
}

#[test]
fn a_sidebar_without_a_message_is_inert() {
    let state = state();
    let element: Element<'_, Message> = sidebar(&state).into();
    let mut ui = Simulator::with_size(Settings::default(), WIDE, element);
    let _ = ui.click("Inbox");
    let _ = ui.simulate([key(Named::ArrowDown)]);
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn shortcut_routes_ctrl_b_from_the_app() {
    let state = state();
    let event = iced_cube::keys::Event {
        key: Key::Character("b".into()),
        modifiers: Modifiers::COMMAND,
    };
    assert_eq!(
        state.shortcut(&sidebar::default_keymap(), &event),
        Some(Event::Toggle)
    );
}
