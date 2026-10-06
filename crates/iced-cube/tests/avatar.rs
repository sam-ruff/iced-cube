#![cfg(feature = "avatar")]

use iced::Element;
use iced::widget::{column, image};
use iced_cube::feedback::avatar::{Presence, Shape, Size};
use iced_cube::{avatar, avatar_group, lucide};
use iced_test::simulator;

#[test]
fn every_size_and_shape_shows_the_initials() {
    for size in Size::ALL {
        for shape in Shape::ALL {
            let element: Element<'_, ()> = avatar("Ada Lovelace").size(size).shape(shape).into();
            let mut ui = simulator(element);
            let bounds = ui.find("AL").expect("initials are shown").bounds();
            assert!(bounds.width < size.metrics().diameter, "{size:?} {shape:?}");
        }
    }
}

#[test]
fn an_image_or_icon_replaces_the_initials() {
    let picture = image::Handle::from_rgba(2, 2, vec![128; 16]);
    let element: Element<'_, ()> = avatar("Ada Lovelace").image(picture).into();
    let mut ui = simulator(element);
    assert!(ui.find("AL").is_err());

    let element: Element<'_, ()> = avatar("Build bot").icon(lucide!(Bot)).into();
    let mut ui = simulator(element);
    assert!(ui.find("BB").is_err());
}

#[test]
fn presence_keeps_the_avatar_size() {
    for presence in Presence::ALL {
        let element: Element<'_, ()> = column![
            avatar("Ada Lovelace").presence(presence),
            avatar("Grace Hopper"),
        ]
        .into();
        let mut ui = simulator(element);
        let first = ui.find("AL").expect("first").bounds();
        let second = ui.find("GH").expect("second").bounds();
        assert_eq!(second.y - first.y, 32.0, "{presence:?}");
    }
}

#[test]
fn a_group_overlaps_its_avatars_and_counts_the_rest() {
    let names = [
        "Ada Lovelace",
        "Grace Hopper",
        "Alan Turing",
        "Edsger Dijkstra",
        "Barbara Liskov",
    ];
    let element: Element<'_, ()> = avatar_group(names.map(avatar)).max(4).into();
    let mut ui = simulator(element);
    let ada = ui.find("AL").expect("first avatar").bounds();
    let grace = ui.find("GH").expect("second avatar").bounds();
    let step = grace.x - ada.x;
    assert!(step > 0.0 && step < 32.0, "avatars overlap, step {step}");
    assert!(ui.find("ED").is_err(), "the fourth avatar is counted");
    assert!(ui.find("+2").is_ok(), "the count covers the rest");
}

#[test]
fn avatars_keep_their_size_in_a_narrow_container() {
    let element: Element<'_, ()> =
        column![avatar_group(["Ada Lovelace", "Grace Hopper"].map(avatar))]
            .width(10)
            .into();
    let mut ui = simulator(element);
    let ada = ui.find("AL").expect("first").bounds();
    let grace = ui.find("GH").expect("second").bounds();
    assert!(grace.x > ada.x + 10.0, "the group is not squeezed");
}
