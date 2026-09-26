---
title: Toast
description: Short notifications that stack in a corner and close by themselves.
group: Overlays
order: 2
module: overlay::toast
imports: |
  use iced_cube::overlay::toast::{self, Position, Variant, toast, toasts};
keywords: [notification, snackbar]
related: [alert]
hero: toast/variants
stories: [toast/variants, toast/background-job, toast/action, toast/keyboard]
api:
  - name: "toast(title)"
    description: "Creates a toast that closes five seconds after it becomes visible."
  - name: ".description(text) / .variant(Variant)"
    description: "Adds a second line. Variants are Default, Success and Destructive."
  - name: ".action(label)"
    description: "Adds a button. Pressing it closes the toast and returns Output::Action."
  - name: ".duration(duration) / .persistent()"
    description: "Changes how long the toast stays, or keeps it until it is closed."
  - name: "toast::State"
    description: "The queue. At most three toasts show at once by default; change it with .with_limit(n). State::new() makes a queue without payloads; State::<Message>::default() makes one whose action buttons can hand back your messages."
  - name: "state.update(Event)"
    description: "Handles Ready, Received, Dismiss, Action and Tick. Returns Output::Ready(sender), Output::Payload(value) for a toast queued with push_with, or Output::Action(id, toast) for any other action."
  - name: "state.push(toast)"
    description: "Queues a toast from the UI thread without the channel, and returns its Id."
  - name: "state.push_with(toast, payload)"
    description: "Queues a toast whose action button hands back payload, such as the message that undoes what it reports. Dismissing or expiring drops the payload."
  - name: "state.dismiss(id)"
    description: "Removes a toast, visible or waiting, and returns it."
  - name: "state.tick(now)"
    description: "Closes toasts whose time is up at now and starts the countdown of newly visible ones. Returns the ids that closed."
  - name: "state.visible() / state.waiting() / state.len()"
    description: "The visible toasts with their ids, how many are queued behind them, and the total."
  - name: "toast::subscription()"
    description: "Owns the channel. Emits Ready(sender) once, then batches of up to BATCH_SIZE (16) toasts."
  - name: "toast::Sender"
    description: "The sending half of the channel, which holds CHANNEL_CAPACITY (32) toasts. Clone it for each producer."
  - name: "toast::timer(&state)"
    description: "Ticks every TICK (250 ms) while a visible toast is counting down, and is idle otherwise."
  - name: "toasts(&state, content)"
    description: "Renders the toasts over your content. Toast buttons render disabled until .on_event(f) is set."
  - name: ".position(Position)"
    description: "TopLeft, TopCenter, TopRight, BottomLeft, BottomCenter or BottomRight. Defaults to BottomRight."
  - name: "toast::close_button_id(id)"
    description: "The widget id of a toast's close button, for focusing it or clicking it in tests."
  - name: "toast::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.events(&state)"
    description: "The Dismiss events for a resolved Action (DismissLatest or DismissAll). Pass each one to state.update."
  - name: "state.ids()"
    description: "Every queued toast id, visible first."
---

Toasts often come from work running off the UI thread, such as a download or a sync. They reach the app through a bounded channel rather than shared state:

- `toast::subscription()` creates the channel and owns the receiver. Its first event is `Ready` with the sender.
- The app keeps the sender and gives a clone to each producer.
- The subscription delivers whatever is waiting in the channel in batches of up to 16, so a burst causes a handful of redraws rather than one per toast.
- `toast::timer` only runs while a toast is counting down.

A whole app that raises a toast from a worker thread:

```rust
use iced::{Element, Subscription};
use iced_cube::button;
use iced_cube::overlay::toast::{self, toast, toasts};

#[derive(Debug, Clone)]
enum Message {
    Toast(toast::Event),
    Export,
}

#[derive(Default)]
struct App {
    toasts: toast::State,
    sender: Option<toast::Sender>,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Toast(event) => {
                if let Some(toast::Output::Ready(sender)) = self.toasts.update(event) {
                    self.sender = Some(sender);
                }
            }
            Message::Export => {
                let Some(mut sender) = self.sender.clone() else {
                    return;
                };
                std::thread::spawn(move || {
                    // Do the work, then report back. try_send drops the
                    // toast if the channel is full.
                    let _ = sender.try_send(toast("Export finished"));
                });
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        toasts(&self.toasts, button("Export").on_press(Message::Export))
            .on_event(Message::Toast)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([toast::subscription(), toast::timer(&self.toasts)])
            .map(Message::Toast)
    }
}

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .subscription(App::subscription)
        .run()
}
```

[Subscriptions](../../subscriptions/) covers sending from async code, waiting for room instead of dropping, and how the timer stays idle.

An Undo button needs to know what to undo. Rather than keeping toast ids and matching them, queue the toast with `state.push_with(toast, message)` on a `toast::State<Message>`. When the button is pressed, `update` returns `Output::Payload(message)`, and you handle it like any other message, as the action example does. If the toast is dismissed or times out, the message is dropped with it. Payloads only go through `push_with` on the UI thread; toasts from the channel have none.

Time only enters the queue through `Tick`, so the queue is easy to test: push toasts, call `state.tick(now)` with any instant, and check what is left. A toast starts counting down when it becomes visible, not when it arrives.

The background job example uses an async stream that waits with `futures-timer` instead of a thread, so it also runs in the browser.
