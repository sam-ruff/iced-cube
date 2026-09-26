use iced::Element;
use iced::keyboard::key::Named;
use iced::widget::Id;
use iced::widget::text_editor::{Action, Content, Edit};
use iced_cube::textarea;
use iced_test::{Error, simulator};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Edit(Action),
}

fn view(content: &Content, enabled: bool) -> Element<'_, Message> {
    textarea(content)
        .id("notes")
        .placeholder("Write a note")
        .on_action_maybe(enabled.then_some(Message::Edit))
        .into()
}

fn inserted(messages: &[Message]) -> String {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::Edit(Action::Edit(Edit::Insert(c))) => Some(*c),
            _ => None,
        })
        .collect()
}

#[test]
fn typing_emits_insert_actions() -> Result<(), Error> {
    let content = Content::new();
    let mut ui = simulator(view(&content, true));
    ui.click(Id::new("notes"))?;
    ui.typewrite("hi");

    let messages: Vec<_> = ui.into_messages().collect();
    assert_eq!(inserted(&messages), "hi");
    Ok(())
}

#[test]
fn enter_inserts_a_new_line() -> Result<(), Error> {
    let content = Content::with_text("first");
    let mut ui = simulator(view(&content, true));
    ui.click(Id::new("notes"))?;
    ui.tap_key(Named::Enter);

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(messages.contains(&Message::Edit(Action::Edit(Edit::Enter))));
    Ok(())
}

#[test]
fn applying_actions_updates_the_content() -> Result<(), Error> {
    let mut content = Content::new();
    let messages: Vec<_> = {
        let mut ui = simulator(view(&content, true));
        ui.click(Id::new("notes"))?;
        ui.typewrite("ok");
        ui.into_messages().collect()
    };

    for Message::Edit(action) in messages {
        content.perform(action);
    }
    assert_eq!(content.text().trim_end(), "ok");
    Ok(())
}

#[test]
fn disabled_textarea_emits_nothing() -> Result<(), Error> {
    let content = Content::with_text("Read only");
    let mut ui = simulator(view(&content, false));
    ui.click(Id::new("notes"))?;
    ui.typewrite("x");

    assert_eq!(ui.into_messages().count(), 0);
    Ok(())
}
