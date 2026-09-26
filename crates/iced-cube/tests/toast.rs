use std::thread;

use futures::executor::block_on;
use futures::{SinkExt, StreamExt};
use iced::Element;
use iced::widget::text;
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
