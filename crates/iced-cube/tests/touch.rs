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
