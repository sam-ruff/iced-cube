#![cfg(feature = "bottom-nav")]

use iced::widget::{column, space};
use iced::{Element, Event, Length, Point, Size, touch};
use iced_cube::navigation::bottom_nav::{HEIGHT, MIN_SLOT_WIDTH};
use iced_cube::{bottom_nav, lucide};
use iced_test::simulator::{Simulator, simulator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Search,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Go(Screen),
}

fn view(active: Screen, enabled: bool, inset: f32) -> Element<'static, Message> {
    let mut nav = bottom_nav(active)
        .item(Screen::Home, lucide!(House), "Home")
        .item(Screen::Search, lucide!(Search), "Search")
        .item(Screen::Settings, lucide!(Settings), "Settings")
        .inset(inset);
    if enabled {
        nav = nav.on_select(Message::Go);
    }
    column![space().height(Length::Fill), nav].into()
}

fn tap(ui: &mut Simulator<'_, Message>, at: Point) {
    let id = touch::Finger(1);
    ui.point_at(at);
    let _ = ui.simulate([
        Event::Touch(touch::Event::FingerPressed { id, position: at }),
        Event::Touch(touch::Event::FingerLifted { id, position: at }),
    ]);
}

#[test]
fn taps_and_clicks_select_destinations_including_the_active_one() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(Screen::Home, true, 0.0));
    let search = ui.find("Search")?.bounds().center();
    tap(&mut ui, search);
    let home = ui.find("Home")?.bounds().center();
    tap(&mut ui, home);
    ui.click("Settings")?;
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        vec![
            Message::Go(Screen::Search),
            Message::Go(Screen::Home),
            Message::Go(Screen::Settings),
        ]
    );
    Ok(())
}

#[test]
fn a_tap_anywhere_in_a_slot_counts() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(Screen::Home, true, 0.0));
    let label = ui.find("Search")?.bounds();
    // Well above the label, on the pill row, still inside the slot.
    tap(&mut ui, Point::new(label.center_x(), label.y - 30.0));
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Go(Screen::Search)]);
    Ok(())
}

#[test]
fn without_on_select_nothing_is_sent() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(Screen::Home, false, 0.0));
    ui.click("Search")?;
    assert_eq!(ui.into_messages().count(), 0);
    Ok(())
}

#[test]
fn the_inset_lifts_the_destinations_above_the_window_edge() -> Result<(), iced_test::Error> {
    let size = Size::new(360.0, 640.0);
    let mut plain = Simulator::with_size(
        iced::Settings::default(),
        size,
        view(Screen::Home, true, 0.0),
    );
    let mut inset = Simulator::with_size(
        iced::Settings::default(),
        size,
        view(Screen::Home, true, 24.0),
    );
    let low = plain.find("Home")?.bounds();
    let high = inset.find("Home")?.bounds();
    assert_eq!(low.y - high.y, 24.0);
    assert!(low.y > size.height - HEIGHT);
    Ok(())
}

#[test]
fn slots_share_a_narrow_width_equally() -> Result<(), iced_test::Error> {
    let size = Size::new(240.0, 400.0);
    let mut ui = Simulator::with_size(
        iced::Settings::default(),
        size,
        view(Screen::Home, true, 0.0),
    );
    let home = ui.find("Home")?.bounds().center_x();
    let search = ui.find("Search")?.bounds().center_x();
    let settings = ui.find("Settings")?.bounds().center_x();
    assert!((search - home - size.width / 3.0).abs() < 1.0);
    assert!((settings - search - size.width / 3.0).abs() < 1.0);
    assert!(size.width / 3.0 >= MIN_SLOT_WIDTH);
    Ok(())
}
