#![cfg(feature = "property-grid")]

use iced::keyboard::{Key, Modifiers, key::Named};
use iced::widget::{self, container};
use iced::{Color, Element, Point, Size, keyboard, mouse};
use iced_cube::data::property_grid::{
    self, DESCRIPTION_HEIGHT, EDITOR_ID, Entry, Event, HANDLE_WIDTH, MenuItem, NOTHING_SELECTED,
    Output, RESET_WIDTH, SEARCH, State, Value, choice, colour, name_width, number, property_grid,
    read_only, text, toggle,
};
use iced_cube::overlay::context_menu;
use iced_test::selector::Candidate;
use iced_test::simulator::{self, Simulator};

#[derive(Debug, Clone)]
enum Message {
    Grid(Event<u32>),
}

const LONG_NAME: &str = "A property name far too long to fit in its column";

fn sample() -> State<u32> {
    State::new([
        read_only(1, "Id", "rect-1"),
        number(2, "Width", 120.0)
            .category("Layout")
            .range(0.0, 400.0)
            .step(1.0)
            .unit("px")
            .default(100.0),
        toggle(3, "Visible", true).category("Layout"),
        colour(4, "Fill", Color::from_rgb8(0x3b, 0x82, 0xf6)).category("Appearance"),
        choice(5, "Blend", ["Normal", "Multiply", "Screen"], "Normal")
            .category("Appearance")
            .description("How the layer mixes with what is under it."),
        text(6, LONG_NAME, "Value").category("Appearance"),
    ])
}

fn view(state: &State<u32>) -> Element<'_, Message> {
    property_grid(state).on_event(Message::Grid).into()
}

fn events(ui: Simulator<'_, Message>) -> Vec<Event<u32>> {
    ui.into_messages()
        .map(|message| match message {
            Message::Grid(event) => event,
        })
        .collect()
}

fn apply(events: Vec<Event<u32>>, state: &mut State<u32>) -> Vec<Output<u32>> {
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

/// The id of the focused text field, if any.
fn focused(ui: &mut Simulator<'_, Message>) -> Option<widget::Id> {
    ui.find(|candidate: Candidate<'_>| match candidate {
        Candidate::Focusable { id, state, .. } if state.is_focused() => id.cloned(),
        _ => None,
    })
    .ok()
}

#[test]
fn renders_headings_names_and_values() {
    let state = sample();
    let mut ui = simulator::simulator(view(&state));
    for label in [
        "Layout",
        "Appearance",
        "Width",
        "120px",
        "#3B82F6",
        "rect-1",
        "Normal",
    ] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
    assert!(ui.find(NOTHING_SELECTED).is_ok());
}

#[test]
fn clicking_a_value_edits_it_in_a_focused_field() {
    let mut state = sample();
    let clicked = {
        let mut ui = simulator::simulator(view(&state));
        assert!(ui.click("120px").is_ok());
        events(ui)
    };
    assert!(clicked.contains(&Event::Edit(2)), "{clicked:?}");
    let _ = apply(clicked, &mut state);
    assert_eq!(state.editing(), Some(2));

    let typed = {
        let mut ui = simulator::simulator(view(&state));
        assert_eq!(focused(&mut ui), Some(EDITOR_ID));
        let _ = ui.typewrite("5");
        events(ui)
    };
    assert_eq!(
        typed,
        vec![Event::Input(2, "5".into())],
        "the text was selected"
    );
    let outputs = apply(typed, &mut state);
    assert_eq!(
        outputs,
        vec![Output::Changed {
            id: 2,
            value: Value::Number(5.0)
        }]
    );
}

#[test]
fn escape_in_the_field_cancels_and_enter_submits() {
    let mut state = sample();
    let _ = state.update(Event::Edit(2));
    let _ = state.update(Event::Input(2, "150".into()));

    let mut ui = simulator::simulator(view(&state));
    let _ = ui.simulate([press_key(Key::Named(Named::Enter), Modifiers::empty())]);
    assert_eq!(events(ui), vec![Event::Submit]);

    let cancelled = {
        let mut ui = simulator::simulator(view(&state));
        let _ = ui.simulate([press_key(Key::Named(Named::Escape), Modifiers::empty())]);
        events(ui)
    };
    assert_eq!(cancelled, vec![Event::Cancel]);
    let outputs = apply(cancelled, &mut state);
    assert_eq!(
        outputs,
        vec![Output::Changed {
            id: 2,
            value: Value::Number(120.0)
        }]
    );
}

#[test]
fn an_invalid_draft_shows_its_error_under_the_rows() {
    let mut state = sample();
    let _ = state.update(Event::Edit(2));
    assert!(state.update(Event::Input(2, "abc".into())).is_none());
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("Enter a number").is_ok());
    assert_eq!(state.value(2), Some(&Value::Number(120.0)));
}

#[test]
fn the_description_follows_the_highlight() {
    let mut state = sample();
    let _ = state.update(Event::Press(Entry::Property(5)));
    let mut ui = simulator::simulator(view(&state));
    assert!(
        ui.find("How the layer mixes with what is under it.")
            .is_ok()
    );
    assert!(ui.find(NOTHING_SELECTED).is_err());
}

#[test]
fn keys_only_move_the_highlight_once_the_grid_has_focus() {
    let state = sample();
    let mut ui = simulator::simulator(view(&state));
    let _ = ui.simulate([press_key(Key::Named(Named::ArrowDown), Modifiers::empty())]);
    assert!(events(ui).is_empty(), "unfocused");

    let mut ui = simulator::simulator(view(&state));
    assert!(ui.click("Id").is_ok());
    let _ = ui.simulate([press_key(Key::Named(Named::ArrowDown), Modifiers::empty())]);
    let events = events(ui);
    assert_eq!(
        events,
        vec![
            Event::Focus(true),
            Event::Press(Entry::Property(1)),
            Event::Next
        ]
    );
}

#[test]
fn ctrl_f_moves_the_keyboard_to_the_filter() {
    let mut state = sample();
    let pressed = {
        let mut ui = simulator::simulator(view(&state));
        assert!(ui.click("Id").is_ok());
        let _ = ui.simulate([press_key(Key::Character("f".into()), Modifiers::COMMAND)]);
        events(ui)
    };
    assert_eq!(pressed.last(), Some(&Event::FocusSearch));
    let _ = apply(pressed, &mut state);

    let mut ui = simulator::simulator(view(&state));
    assert_eq!(focused(&mut ui), Some(SEARCH));
    let _ = ui.typewrite("w");
    let _ = ui.simulate([press_key(Key::Character("f".into()), Modifiers::COMMAND)]);
    assert_eq!(
        events(ui),
        vec![Event::Search("w".into())],
        "Ctrl+F in the box selects its text rather than typing"
    );
}

#[test]
fn a_heading_toggles_and_a_checkbox_sets_its_value() {
    let state = sample();
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.click("Layout").is_ok());
    let value = ui.find("120px").expect("a value").bounds();
    let visible = ui.find("Visible").expect("a name").bounds();
    click_at(&mut ui, Point::new(value.x + 8.0, visible.center_y()));
    let events = events(ui);
    assert!(events.contains(&Event::Press(Entry::Category("Layout".into()))));
    assert!(
        events.contains(&Event::Set(3, Value::Bool(false))),
        "{events:?}"
    );
}

#[test]
fn choosing_an_option_sets_it() {
    let mut state = sample();
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.click("Normal").is_ok());
    assert!(ui.click("Screen").is_ok());
    let outputs = apply(events(ui), &mut state);
    assert!(outputs.contains(&Output::Changed {
        id: 5,
        value: Value::Text("Screen".into())
    }));
}

#[test]
fn the_reset_button_puts_back_the_default() {
    let mut state = sample();
    let mut ui = simulator::simulator(view(&state));
    let width = ui.find("Width").expect("a modified name").bounds();
    // The reset slot ends the row, inside the border and the row padding.
    let x = 1024.0 - 1.0 - 8.0 - RESET_WIDTH / 2.0;
    click_at(&mut ui, Point::new(x, width.center_y()));
    let outputs = apply(events(ui), &mut state);
    assert_eq!(
        outputs,
        vec![Output::Reset {
            id: 2,
            value: Value::Number(100.0)
        }]
    );
}

#[test]
fn dragging_the_line_between_the_columns_resizes_the_names() {
    let mut state = sample();
    let names = name_width(None, 1024.0 - 2.0 - 16.0);
    let handle = 1.0 + 8.0 + names + HANDLE_WIDTH / 2.0;
    let started = {
        let mut ui = simulator::simulator(view(&state));
        let width = ui.find("Width").expect("a row").bounds();
        click_at(&mut ui, Point::new(handle, width.center_y()));
        events(ui)
    };
    assert!(started.contains(&Event::ResizeStart), "{started:?}");
    let _ = apply(started, &mut state);

    let mut ui = simulator::simulator(view(&state));
    let width = ui.find("Width").expect("a row").bounds();
    let position = Point::new(250.0, width.center_y());
    ui.point_at(position);
    let _ = ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved { position })]);
    let _ = ui.simulate([iced::Event::Mouse(mouse::Event::ButtonReleased(
        mouse::Button::Left,
    ))]);
    let _ = apply(events(ui), &mut state);
    assert!(!state.is_resizing());
    let expected = 250.0 - 1.0 - 8.0 - HANDLE_WIDTH / 2.0;
    assert_eq!(state.name_width(), Some(expected));
}

#[test]
fn a_custom_renderer_draws_the_value() {
    let state = sample();
    let grid: Element<'_, Message> = property_grid(&state)
        .custom(1, |_| iced::widget::text("Drawn by the app").into())
        .on_event(Message::Grid)
        .into();
    let mut ui = simulator::simulator(grid);
    assert!(ui.find("Drawn by the app").is_ok());
    assert!(ui.find("rect-1").is_err());
}

#[test]
fn a_grid_without_messages_is_inert() {
    let state = sample();
    let grid: Element<'_, Message> = property_grid(&state).into();
    let mut ui = simulator::simulator(grid);
    assert!(ui.click("120px").is_ok());
    assert!(ui.click("Layout").is_ok());
    assert!(events(ui).is_empty());
}

#[test]
fn a_narrow_inspector_keeps_every_row_on_one_line() {
    const WIDTH: f32 = 280.0;
    let mut state = sample();
    let _ = state.update(Event::Press(Entry::Property(2)));
    let grid = container(view(&state)).width(WIDTH);
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(WIDTH, 600.0), grid);
    for label in [LONG_NAME, "Width", "120px", "#3B82F6", "Visible", "Layout"] {
        let bounds = ui.find(label).expect(label).bounds();
        assert!(
            bounds.x >= 0.0 && bounds.x + bounds.width <= WIDTH,
            "{label} runs off the edge: {bounds:?}"
        );
        assert!(bounds.height <= 20.0, "{label} wraps: {bounds:?}");
    }
    let long = ui.find(LONG_NAME).expect("a long name").bounds();
    let value = ui.find("Value").expect("its value").bounds();
    assert!(
        long.x + long.width <= value.x,
        "the name stops at its column"
    );
    let filter = ui.find(SEARCH).expect("the filter").bounds();
    assert!(filter.x + filter.width <= WIDTH);
}

#[test]
fn escape_in_the_filter_clears_it_then_leaves_it() {
    let mut state = sample();
    let _ = state.update(Event::Search("wid".into()));
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.click("wid").is_ok());
    let _ = ui.simulate([press_key(Key::Named(Named::Escape), Modifiers::empty())]);
    assert_eq!(focused(&mut ui), Some(SEARCH), "the first Escape stays");
    let emitted = events(ui);
    assert!(
        emitted.contains(&Event::Search(String::new())),
        "{emitted:?}"
    );
    let _ = apply(emitted, &mut state);
    assert_eq!(state.query(), "");

    let mut ui = simulator::simulator(view(&state));
    assert!(ui.click("Filter").is_ok());
    assert_eq!(focused(&mut ui), Some(SEARCH));
    let _ = ui.simulate([press_key(Key::Named(Named::Escape), Modifiers::empty())]);
    assert_eq!(focused(&mut ui), None, "the second Escape leaves the box");
    assert!(
        !events(ui)
            .iter()
            .any(|event| matches!(event, Event::Search(_))),
    );
}

#[test]
fn the_filter_keeps_its_width_when_the_count_appears() {
    let mut state = sample();
    let before = simulator::simulator(view(&state))
        .find(SEARCH)
        .expect("filter")
        .bounds();
    let _ = state.update(Event::Search("i".into()));
    {
        let mut ui = simulator::simulator(view(&state));
        let after = ui.find(SEARCH).expect("filter").bounds();
        assert!(ui.find("5 shown").is_ok(), "every name but Blend");
        assert_eq!(before.width, after.width);
    }

    let _ = state.update(Event::Search("zebra".into()));
    let mut ui = simulator::simulator(view(&state));
    assert!(ui.find("None").is_ok());
}

#[test]
fn a_right_click_opens_the_menu_on_a_property() {
    #[derive(Debug, Clone)]
    enum Menu {
        Grid,
        Menu(context_menu::Event<MenuItem, u32>),
    }
    let state = sample();
    let menu = property_grid::menu();
    let grid: Element<'_, Menu> = property_grid(&state)
        .context_menu(&menu, Menu::Menu)
        .on_event(|_| Menu::Grid)
        .into();
    let mut ui = simulator::simulator(grid);
    let row = ui.find("120px").expect("a value").bounds();
    ui.point_at(row.center());
    let _ = ui.simulate([
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
    ]);
    let opened: Vec<u32> = ui
        .into_messages()
        .filter_map(|message| match message {
            Menu::Menu(context_menu::Event::Open(id, _)) => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(opened, [2]);

    let mut state = state;
    assert_eq!(
        state.update(MenuItem::Reset.event(2)),
        Some(Output::Reset {
            id: 2,
            value: Value::Number(100.0)
        })
    );
    assert_eq!(
        state.update(MenuItem::CopyValue.event(2)),
        Some(Output::CopyValue("100px".into()))
    );
}

#[test]
fn the_open_menu_lists_its_entries() {
    let state = sample();
    let mut menu = property_grid::menu();
    let _ = menu.update(context_menu::Event::OpenFromKeyboard(2));
    let grid: Element<'_, Message> = property_grid(&state)
        .context_menu(&menu, |_| Message::Grid(Event::Next))
        .on_event(Message::Grid)
        .into();
    let mut ui = simulator::simulator(grid);
    for label in ["Copy value", "Reset to default"] {
        assert!(ui.find(label).is_ok(), "{label}");
    }
}

#[test]
fn a_narrow_grid_gives_names_the_room_values_can_spare() {
    const WIDTH: f32 = 300.0;
    let state = sample();
    let grid = container(view(&state)).width(WIDTH);
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(WIDTH, 600.0), grid);
    let name = ui.find("Width").expect("a name").bounds();
    let value = ui.find("120px").expect("its value").bounds();
    let names = value.x - name.x;
    let values = WIDTH - value.x;
    assert!(names > values, "names {names}, values {values}");
}

#[test]
fn the_description_wraps_and_grows() {
    const WIDTH: f32 = 240.0;
    let long = "A description long enough that it needs several lines in a narrow grid, \
                and more lines still, so the area under the rows has to grow to show it.";
    let mut state = State::new([text(1, "Name", "Box").description(long)]);
    {
        let grid = container(view(&state)).width(WIDTH);
        let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(WIDTH, 600.0), grid);
        let empty = ui.find(NOTHING_SELECTED).expect("placeholder").bounds();
        assert!(empty.x + empty.width <= WIDTH, "{empty:?}");
        assert!(empty.height > 20.0, "the placeholder wraps: {empty:?}");
    }
    let _ = state.update(Event::Press(Entry::Property(1)));
    let grid = iced::widget::column![
        container(view(&state)).width(WIDTH),
        iced::widget::text("Below the grid"),
    ];
    let mut ui = Simulator::with_size(iced::Settings::default(), Size::new(WIDTH, 600.0), grid);
    let title = ui.find("Name").expect("title").bounds();
    let body = ui.find(long).expect("description").bounds();
    let below = ui.find("Below the grid").expect("text below").bounds();
    assert!(body.height > 40.0, "wraps: {body:?}");
    assert!(
        body.y + body.height - title.y > DESCRIPTION_HEIGHT,
        "the area grows past its least height"
    );
    assert!(body.y + body.height < below.y, "nothing is cut off");
    assert!(body.x + body.width <= WIDTH);
}
