#![cfg(feature = "switch")]

use iced::widget::column;
use iced::{Element, Point};
use iced_cube::primitives::switch;
use iced_cube::primitives::switch::TRACK_HEIGHT;
use iced_test::simulator::{Simulator, click, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Wifi(bool),
    Bluetooth(bool),
}

const GAP: f32 = 16.0;

fn view(enabled: bool) -> Element<'static, Message> {
    column![
        switch(false).label("Wi-Fi").on_toggle(Message::Wifi),
        switch(true)
            .label("Bluetooth")
            .on_toggle_maybe(enabled.then_some(Message::Bluetooth)),
    ]
    .spacing(GAP)
    .into()
}

fn click_track(ui: &mut Simulator<'_, Message>, row: usize) {
    let top = row as f32 * (TRACK_HEIGHT + GAP);
    ui.point_at(Point::new(TRACK_HEIGHT, top + TRACK_HEIGHT / 2.0));
    ui.simulate(click());
}

#[test]
fn clicking_a_switch_emits_the_new_value() {
    let mut ui = simulator(view(true));
    click_track(&mut ui, 0);
    click_track(&mut ui, 1);

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        vec![Message::Wifi(true), Message::Bluetooth(false)]
    );
}

#[test]
fn disabled_switch_emits_nothing() {
    let mut ui = simulator(view(false));
    click_track(&mut ui, 1);

    assert_eq!(ui.into_messages().count(), 0);
}
