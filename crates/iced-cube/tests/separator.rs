use iced::widget::{column, row, text};
use iced::{Element, Size};
use iced_cube::primitives::separator::Orientation;
use iced_cube::{separator, vertical_separator};
use iced_test::simulator::Simulator;
use iced_test::{Error, simulator};

#[test]
fn inline_label_is_rendered() {
    let element: Element<'_, ()> =
        column![text("Above"), separator().label("or"), text("Below")].into();
    let mut ui = simulator(element);
    assert!(ui.find("or").is_ok());
    assert!(ui.find("Above").is_ok());
}

#[test]
fn label_sits_between_the_neighbours() -> Result<(), Error> {
    let element: Element<'_, ()> =
        column![text("Above"), separator().label("or"), text("Below")].into();
    let mut ui = simulator(element);
    let above = ui.find("Above")?.bounds();
    let or = ui.find("or")?.bounds();
    let below = ui.find("Below")?.bounds();
    assert!(above.y < or.y && or.y < below.y);
    Ok(())
}

#[test]
fn vertical_separator_divides_a_row() -> Result<(), Error> {
    let element: Element<'_, ()> = row![text("Left"), vertical_separator(), text("Right")]
        .spacing(8)
        .height(24)
        .into();
    let mut ui = simulator(element);
    let left = ui.find("Left")?.bounds();
    let right = ui.find("Right")?.bounds();
    assert!(right.x - (left.x + left.width) >= 16.0);
    Ok(())
}

#[test]
fn every_orientation_renders_with_a_label() {
    for orientation in Orientation::ALL {
        let element: Element<'_, ()> = separator().orientation(orientation).label("or").into();
        let mut ui =
            Simulator::with_size(iced::Settings::default(), Size::new(200.0, 200.0), element);
        assert!(ui.find("or").is_ok(), "{orientation:?}");
    }
}
