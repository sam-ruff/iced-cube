#![cfg(feature = "breadcrumb")]

use iced::Element;
use iced::widget::column;
use iced_cube::navigation::breadcrumb::Separator;
use iced_cube::{breadcrumb, crumb, lucide};
use iced_test::simulator::{self, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Go(usize),
    Expand,
}

const TRAIL: [&str; 5] = ["Home", "Projects", "Website", "Pages", "Pricing"];

fn trail<'a>() -> Vec<iced_cube::navigation::breadcrumb::Crumb<'a, Message>> {
    TRAIL
        .iter()
        .enumerate()
        .map(|(index, label)| crumb(*label).on_press(Message::Go(index)))
        .collect()
}

#[test]
fn clicking_a_crumb_sends_its_message() {
    for separator in Separator::ALL {
        let element: Element<'_, Message> = breadcrumb(trail()).separator(separator).into();
        let mut ui = simulator(element);
        ui.click("Projects").expect("Projects is shown");
        ui.click("Home").expect("Home is shown");
        let messages: Vec<_> = ui.into_messages().collect();
        assert_eq!(
            messages,
            vec![Message::Go(1), Message::Go(0)],
            "{separator:?}"
        );
    }
}

#[test]
fn the_current_crumb_never_responds() {
    let element: Element<'_, Message> = breadcrumb(trail()).into();
    let mut ui = simulator(element);
    ui.click("Pricing").expect("the current crumb is shown");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn crumbs_without_a_message_are_plain_text() {
    let element: Element<'_, Message> = breadcrumb([
        crumb("Home").icon(lucide!(House)).on_press(Message::Go(0)),
        crumb("Archive"),
        crumb("2024"),
    ])
    .into();
    let mut ui = simulator(element);
    ui.click("Archive").expect("Archive is shown");
    assert_eq!(ui.into_messages().count(), 0);
}

#[test]
fn a_collapsed_trail_hides_the_middle_and_expands_through_the_ellipsis() {
    let element: Element<'_, Message> = breadcrumb(trail())
        .max_items(3)
        .on_expand(Message::Expand)
        .into();
    let mut ui = simulator(element);
    for shown in ["Home", "Pages", "Pricing"] {
        assert!(ui.find(shown).is_ok(), "{shown}");
    }
    for hidden in ["Projects", "Website"] {
        assert!(ui.find(hidden).is_err(), "{hidden}");
    }

    let home = ui.find("Home").expect("Home").bounds();
    let pages = ui.find("Pages").expect("Pages").bounds();
    let between = iced::Point::new((home.x + home.width + pages.x) / 2.0, home.center_y());
    ui.point_at(between);
    let _ = ui.simulate(simulator::click());
    assert_eq!(
        ui.into_messages().collect::<Vec<_>>(),
        vec![Message::Expand]
    );
}

#[test]
fn a_long_trail_wraps_rather_than_squeezing_its_labels() {
    let element: Element<'_, Message> = column![breadcrumb(trail())].width(160).into();
    let mut ui = simulator(element);
    let home = ui.find("Home").expect("Home").bounds();
    let pricing = ui.find("Pricing").expect("Pricing").bounds();
    assert!(pricing.y > home.y, "the trail wraps onto a second line");
    for label in TRAIL {
        let bounds = ui.find(label).expect("crumb").bounds();
        assert!(bounds.height < 24.0, "{label}: {}", bounds.height);
        assert!(bounds.x + bounds.width <= 160.5, "{label} stays inside");
    }
}
