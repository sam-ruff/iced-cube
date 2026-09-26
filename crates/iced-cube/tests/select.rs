use iced::{Element, Event, Point, mouse};
use iced_cube::forms::select;
use iced_cube::forms::select::HEIGHT;
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Fruit(&'static str),
}

const FRUITS: &[&str] = &["Apple", "Banana", "Cherry"];

fn view(selected: Option<&'static str>, enabled: bool) -> Element<'static, Message> {
    let select = select(FRUITS, selected).placeholder("Pick a fruit");
    if enabled {
        select.on_select(Message::Fruit).into()
    } else {
        select.into()
    }
}

fn click_at(ui: &mut Simulator<'_, Message>, y: f32) {
    let position = Point::new(100.0, y);
    ui.point_at(position);
    ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position })]);
    ui.simulate(click());
}

/// Opens the menu, which lists one option per field height, and clicks `index`.
fn choose(ui: &mut Simulator<'_, Message>, index: usize) {
    click_at(ui, HEIGHT / 2.0);
    click_at(ui, HEIGHT * (index as f32 + 1.5));
}

#[test]
fn choosing_an_option_emits_it() {
    let mut ui = simulator(view(None, true));
    choose(&mut ui, 1);

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Fruit("Banana")]);
}

#[test]
fn disabled_select_does_not_open() {
    let mut ui = simulator(view(Some("Apple"), false));
    choose(&mut ui, 1);

    assert_eq!(ui.into_messages().count(), 0);
}
