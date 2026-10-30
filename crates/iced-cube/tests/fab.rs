#![cfg(feature = "fab")]

use iced::widget::{Id, column, text};
use iced::{Element, Event, touch};
use iced_cube::{fab, lucide};
use iced_test::simulator;

#[derive(Debug, Clone, PartialEq)]
enum Message {
    New,
    Share,
}

fn view(enabled: bool) -> Element<'static, Message> {
    column![
        fab(lucide!(Plus))
            .label("New note")
            .on_press_maybe(enabled.then_some(Message::New)),
        fab(lucide!(Share2))
            .tooltip("Share")
            .id("share")
            .on_press_maybe(enabled.then_some(Message::Share)),
        text("Below"),
    ]
    .spacing(16)
    .into()
}

#[test]
fn clicks_and_taps_press_the_fab() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(true));
    ui.click("New note")?;
    let share = ui.find(Id::new("share"))?.bounds();
    let at = share.center();
    let id = touch::Finger(1);
    ui.point_at(at);
    let _ = ui.simulate([
        Event::Touch(touch::Event::FingerPressed { id, position: at }),
        Event::Touch(touch::Event::FingerLifted { id, position: at }),
    ]);
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::New, Message::Share]);
    Ok(())
}

#[test]
fn an_icon_only_fab_is_a_56_pixel_square_and_an_extended_one_is_as_tall()
-> Result<(), iced_test::Error> {
    let mut ui = simulator(view(true));
    let share = ui.find(Id::new("share"))?.bounds();
    assert_eq!((share.width, share.height), (56.0, 56.0));
    let label = ui.find("New note")?.bounds();
    let below = ui.find("Below")?.bounds();
    // The extended fab sits above the square one, with 16 pixels between.
    assert!(label.y < share.y);
    assert_eq!(share.y, 56.0 + 16.0);
    assert!(below.y >= share.y + share.height);
    Ok(())
}

#[test]
fn a_disabled_fab_emits_nothing() -> Result<(), iced_test::Error> {
    let mut ui = simulator(view(false));
    ui.click("New note")?;
    ui.click(Id::new("share"))?;
    assert_eq!(ui.into_messages().count(), 0);
    Ok(())
}
