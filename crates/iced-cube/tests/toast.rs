#![cfg(feature = "toast")]

use std::thread;

use futures::executor::block_on;
use futures::{SinkExt, StreamExt};
use iced::keyboard::{Key, Modifiers};
use iced::widget::text;
use iced::{Element, Point, mouse};
use iced_cube::keys;
use iced_cube::overlay::toast::{self, Event, Output, State, Variant, toast, toasts};
use iced_test::simulator;

#[derive(Debug, Clone)]
enum Message {
    Toast(Event),
}

fn view(state: &State) -> Element<'_, Message> {
    toasts(state, text("Content"))
        .on_event(Message::Toast)
        .into()
}

fn dismissed(messages: impl IntoIterator<Item = Message>) -> Vec<toast::Id> {
    messages
        .into_iter()
        .filter_map(|Message::Toast(event)| match event {
            Event::Dismiss(id) => Some(id),
            _ => None,
        })
        .collect()
}

#[test]
fn close_button_emits_dismiss() {
    let mut state = State::new();
    let _ = state.push(toast("First"));
    let second = state.push(toast("Second").variant(Variant::Success));

    let mut ui = simulator(view(&state));
    ui.click(toast::close_button_id(second))
        .expect("close button is rendered");

    assert_eq!(dismissed(ui.into_messages()), vec![second]);
}

#[test]
fn action_button_emits_action() {
    let mut state = State::new();
    let id = state.push(toast("Deleted").action("Undo"));

    let mut ui = simulator(view(&state));
    ui.click("Undo").expect("action is rendered");
    let messages: Vec<_> = ui.into_messages().collect();

    let [Message::Toast(Event::Action(pressed))] = messages.as_slice() else {
        panic!("expected one action, got {messages:?}");
    };
    assert_eq!(*pressed, id);
    let Some(Output::Action(_, toast)) = state.update(Event::Action(id)) else {
        panic!("expected an action output");
    };
    assert_eq!(toast.title, "Deleted");
}

#[derive(Debug, Clone)]
enum App {
    Toast(Event),
    Restore(Vec<&'static str>),
}

#[test]
fn undo_hands_the_app_its_own_message() {
    let mut state: State<App> = State::default();
    let _ = state.push(toast("Saved"));
    let _ = state.push_with(
        toast("Deleted 2 files").action("Undo"),
        App::Restore(vec!["a.txt", "b.txt"]),
    );

    let element: Element<'_, App> = toasts(&state, text("Content")).on_event(App::Toast).into();
    let mut ui = simulator(element);
    ui.click("Undo").expect("action is rendered");
    let messages: Vec<App> = ui.into_messages().collect();

    let follow_ups: Vec<App> = messages
        .into_iter()
        .filter_map(|message| match message {
            App::Toast(event) => match state.update(event) {
                Some(Output::Payload(message)) => Some(message),
                _ => None,
            },
            other => Some(other),
        })
        .collect();
    let [App::Restore(files)] = follow_ups.as_slice() else {
        panic!("expected the restore message, got {follow_ups:?}");
    };
    assert_eq!(files, &["a.txt", "b.txt"]);
    assert_eq!(state.len(), 1, "only the plain toast is left");
}

#[test]
fn renders_visible_toasts_over_the_content() {
    let mut state = State::new().with_limit(2);
    for title in ["One", "Two", "Three"] {
        let _ = state.push(toast(title).description("Details"));
    }

    let mut ui = simulator(view(&state));
    assert!(ui.find("Content").is_ok());
    assert!(ui.find("One").is_ok());
    assert!(ui.find("Two").is_ok());
    assert!(ui.find("Three").is_err());
}

#[test]
fn content_stays_clickable_under_the_overlay() {
    #[derive(Debug, Clone, PartialEq)]
    enum Local {
        Pressed,
        Toast,
    }

    let mut state = State::new();
    let _ = state.push(toast("Hello"));
    let content = iced_cube::button("Behind").on_press(Local::Pressed);
    let element: Element<'_, Local> = toasts(&state, content).on_event(|_| Local::Toast).into();

    let mut ui = simulator(element);
    ui.click("Behind").expect("content is rendered");
    assert_eq!(ui.into_messages().collect::<Vec<_>>(), vec![Local::Pressed]);
}

#[test]
fn producer_thread_sends_through_the_ready_channel() {
    let mut events = Box::pin(toast::stream());
    let Some(Event::Ready(sender)) = block_on(events.next()) else {
        panic!("the first event must be Ready");
    };

    let producer = thread::spawn(move || {
        let mut sender = sender;
        block_on(async {
            for n in 0..40 {
                sender
                    .send(toast(format!("Job {n}")))
                    .await
                    .expect("receiver is alive");
            }
        });
    });

    let mut state = State::new();
    let mut batches = 0;
    while state.len() < 40 {
        let Some(event) = block_on(events.next()) else {
            break;
        };
        let Event::Received(batch) = &event else {
            panic!("expected a batch, got {event:?}");
        };
        assert!(!batch.is_empty() && batch.len() <= toast::BATCH_SIZE);
        batches += 1;
        let _ = state.update(event);
    }
    producer.join().expect("producer finished");

    assert_eq!(state.len(), 40);
    assert!(batches >= 40 / toast::BATCH_SIZE);
    let first: Vec<_> = state
        .visible()
        .map(|(_, toast)| toast.title.clone())
        .collect();
    assert_eq!(first, ["Job 0", "Job 1", "Job 2"]);
}

fn move_to(ui: &mut iced_test::Simulator<'_, Message>, position: Point) {
    ui.point_at(position);
    let _ = ui.simulate([iced::Event::Mouse(mouse::Event::CursorMoved { position })]);
}

#[test]
fn hovering_the_toasts_pauses_them_and_leaving_resumes() -> Result<(), iced_test::Error> {
    let mut state = State::new();
    let _ = state.push(toast("Hover me"));

    let mut ui = simulator(view(&state));
    let over = ui.find("Hover me")?.bounds().center();
    move_to(&mut ui, over);
    move_to(&mut ui, Point::new(4.0, 4.0));
    let events: Vec<Event> = ui
        .into_messages()
        .map(|Message::Toast(event)| event)
        .collect();
    assert!(
        matches!(events.as_slice(), [Event::Pause, Event::Resume]),
        "{events:?}"
    );

    for event in events {
        let _ = state.update(event);
    }
    assert!(!state.is_paused());
    Ok(())
}

#[test]
fn an_undo_toast_shows_even_when_three_toasts_are_already_up() {
    let mut state = State::new();
    for title in ["One", "Two", "Three"] {
        let _ = state.push(toast(title));
    }
    let _ = state.push(toast("Deleted").action("Undo"));

    let mut ui = simulator(view(&state));
    assert!(ui.find("Undo").is_ok());
    assert!(ui.find("Deleted").is_ok());
    assert!(ui.find("Three").is_err(), "the newest plain toast waits");
}

#[test]
fn the_keymap_presses_undo_on_the_newest_action_toast() {
    let mut state: State<App> = State::default();
    let _ = state.push_with(toast("Deleted a").action("Undo"), App::Restore(vec!["a"]));
    let _ = state.push_with(toast("Deleted b").action("Undo"), App::Restore(vec!["b"]));
    let _ = state.push(toast("Saved"));

    let alt_z = keys::Event {
        key: Key::Character("z".into()),
        modifiers: Modifiers::ALT,
    };
    let Some(action) = toast::default_keymap().resolve_event(&alt_z) else {
        panic!("Alt+Z is bound");
    };
    let restored: Vec<App> = action
        .events(&state)
        .into_iter()
        .filter_map(|event| match state.update(event) {
            Some(Output::Payload(message)) => Some(message),
            _ => None,
        })
        .collect();
    assert!(matches!(restored.as_slice(), [App::Restore(files)] if files == &["b"]));
    assert_eq!(state.len(), 2);
}
