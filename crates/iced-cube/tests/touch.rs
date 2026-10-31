//! Every interactive component answers a finger as well as the mouse. The
//! windowing shell moves the cursor to a finger before delivering it, so
//! each tap points the simulator's cursor there first.

use iced::{Event, Point, touch};
use iced_test::simulator::Simulator;

#[allow(dead_code)]
fn tap<Message>(ui: &mut Simulator<'_, Message>, at: Point) {
    let id = touch::Finger(1);
    ui.point_at(at);
    let _ = ui.simulate([
        Event::Touch(touch::Event::FingerPressed { id, position: at }),
        Event::Touch(touch::Event::FingerLifted { id, position: at }),
    ]);
}

#[allow(dead_code)]
fn centre<Message>(ui: &mut Simulator<'_, Message>, label: &str) -> Point {
    match ui.find(label) {
        Ok(target) => target.bounds().center(),
        Err(error) => panic!("{label} is not rendered: {error:?}"),
    }
}

#[cfg(all(feature = "button", feature = "icon-button"))]
#[test]
fn buttons_and_icon_buttons() {
    use iced::widget::{Id, column};
    use iced_cube::{button, icon_button, lucide};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Save,
        Bold,
    }
    let view: iced::Element<'_, Message> = column![
        button("Save").on_press(Message::Save),
        icon_button(lucide!(Bold))
            .label("Bold")
            .id("bold")
            .on_press(Message::Bold),
    ]
    .into();
    let mut ui = iced_test::simulator(view);
    let save = centre(&mut ui, "Save");
    tap(&mut ui, save);
    let bold = ui
        .find(Id::new("bold"))
        .expect("rendered")
        .bounds()
        .center();
    tap(&mut ui, bold);
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Save, Message::Bold]);
}

#[cfg(all(feature = "checkbox", feature = "radio"))]
#[test]
fn checkboxes_and_radios() {
    use iced::widget::column;
    use iced_cube::primitives::checkbox::BOX_SIZE;
    use iced_cube::primitives::{checkbox, radio};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Terms(bool),
        Size(u8),
    }
    let view: iced::Element<'_, Message> = column![
        checkbox(false)
            .label("Accept terms")
            .on_toggle(Message::Terms),
        radio("Large", 3, Some(2)).on_select(Message::Size),
    ]
    .spacing(12)
    .into();
    let mut ui = iced_test::simulator(view);
    let label = centre(&mut ui, "Accept terms");
    tap(&mut ui, label);
    let terms = ui.find("Accept terms").expect("rendered").bounds();
    tap(&mut ui, Point::new(BOX_SIZE / 2.0, terms.center_y()));
    let large = centre(&mut ui, "Large");
    tap(&mut ui, large);
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages,
        vec![Message::Terms(true), Message::Terms(true), Message::Size(3)]
    );
}

#[cfg(feature = "input")]
#[test]
fn inputs_take_focus_from_a_tap() {
    use iced_cube::input;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Name(String),
    }
    let view: iced::Element<'_, Message> = input("Your name", "")
        .id("name")
        .on_input(Message::Name)
        .into();
    let mut ui = iced_test::simulator(view);
    let at = ui
        .find(iced::widget::Id::new("name"))
        .expect("rendered")
        .bounds()
        .center();
    tap(&mut ui, at);
    let _ = ui.typewrite("A");
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Name("A".into())]);
}

#[cfg(feature = "dropdown-menu")]
#[test]
fn dropdown_menu_items() {
    use iced::widget::{button, column, text};
    use iced_cube::overlay::dropdown_menu::{Event as MenuEvent, State, dropdown_menu, item};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Menu(MenuEvent<u8>),
    }
    let mut state = State::new([item(1, "Copy"), item(2, "Paste")]);
    let _ = state.update(MenuEvent::Open);
    let view: iced::Element<'_, Message> = column![
        dropdown_menu(
            &state,
            button(text("Actions")).on_press(Message::Menu(MenuEvent::Toggle))
        )
        .on_event(Message::Menu),
        iced::widget::space().height(300),
    ]
    .into();
    let mut ui = iced_test::simulator(view);
    let paste = centre(&mut ui, "Paste");
    tap(&mut ui, paste);
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(
        messages.last(),
        Some(&Message::Menu(MenuEvent::Activate(2))),
        "{messages:?}"
    );
}

#[cfg(feature = "dialog")]
#[test]
fn dialog_close_button_and_alert_dialog_buttons() {
    use iced::widget::text;
    use iced_cube::overlay::dialog::{CLOSE_BUTTON_ID, alert_dialog, dialog};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Dismiss,
        Cancel,
        Delete,
    }
    let view: iced::Element<'_, Message> = dialog(text("Page"))
        .open(true)
        .title("Edit profile")
        .on_dismiss(Message::Dismiss)
        .into();
    let mut ui = iced_test::simulator(view);
    let close = ui
        .find(CLOSE_BUTTON_ID)
        .expect("rendered")
        .bounds()
        .center();
    tap(&mut ui, close);
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Dismiss]);

    let view: iced::Element<'_, Message> =
        alert_dialog(text("Page"), "Delete project?", "This cannot be undone.")
            .open(true)
            .confirm("Delete")
            .on_cancel(Message::Cancel)
            .on_confirm(Message::Delete)
            .into();
    let mut ui = iced_test::simulator(view);
    let cancel = centre(&mut ui, "Cancel");
    tap(&mut ui, cancel);
    let delete = centre(&mut ui, "Delete");
    tap(&mut ui, delete);
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Cancel, Message::Delete]);
}

/// A tapped control is drawn as it was before the tap once the finger
/// lifts, rather than hovered, and a mouse brings the hover back.
#[cfg(all(feature = "button", feature = "fab", feature = "toggle-group"))]
#[test]
fn a_tap_leaves_no_hover_on_buttons_fabs_or_toggle_items() {
    use iced::time::Instant;
    use iced::widget::column;
    use iced::{mouse, window};
    use iced_cube::primitives::toggle_group::{State, Variant, item, toggle_group};
    use iced_cube::{button, fab, lucide};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Save,
        New,
        Mode,
    }
    let modes = State::single([item(0, "Light"), item(1, "Dark")]);
    let view: iced::Element<'_, Message> = column![
        button("Save").on_press(Message::Save),
        fab(lucide!(Plus)).label("New").on_press(Message::New),
        toggle_group(&modes)
            .variant(Variant::Outline)
            .width(iced::Length::Fill)
            .on_event(|_| Message::Mode),
    ]
    .spacing(24)
    .padding(24)
    .into();
    let mut ui = iced_test::simulator(view);

    let directory = std::env::temp_dir().join(format!("iced-cube-touch-{}", std::process::id()));
    let resting = directory.join("resting");
    let same_as_resting = |ui: &mut Simulator<'_, Message>| {
        let redraw = Event::Window(window::Event::RedrawRequested(Instant::now()));
        let _ = ui.simulate([redraw]);
        match ui
            .snapshot(&iced_cube::theme::light())
            .and_then(|frame| frame.matches_hash(&resting))
        {
            Ok(same) => same,
            Err(error) => panic!("snapshot failed: {error:?}"),
        }
    };
    assert!(same_as_resting(&mut ui));

    for label in ["Save", "New", "Dark"] {
        let at = centre(&mut ui, label);
        tap(&mut ui, at);
        assert!(same_as_resting(&mut ui), "{label} still looks hovered");

        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: at })]);
        assert!(!same_as_resting(&mut ui), "{label} shows no hover");
        let away = Point::new(2.0, 2.0);
        ui.point_at(away);
        let _ = ui.simulate([Event::Mouse(mouse::Event::CursorMoved { position: away })]);
    }
    let _ = std::fs::remove_dir_all(&directory);
    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(messages, vec![Message::Save, Message::New, Message::Mode]);
}
