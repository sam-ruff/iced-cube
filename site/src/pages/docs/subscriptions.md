---
layout: ../../layouts/Guide.astro
title: Subscriptions
description: Sending toasts from threads and async tasks through a channel, and keeping timers idle.
---

Some UI updates start outside the UI: an export finishing on a worker thread, a sync task failing, a download completing. In iced-cube these reach the app through a channel owned by a subscription, never through shared `Arc<Mutex<_>>` state.

[Toast](../components/toast/) is currently the only component fed this way. Later components that stream data in will use the same pattern.

## How the toast channel works

- `toast::subscription()` creates a bounded channel and keeps the receiving end.
- Its first event is `toast::Event::Ready(sender)`. Passing it to `state.update` returns `toast::Output::Ready(sender)`, and your app stores the sender.
- You give a clone of the sender to each producer: a thread, an async task, a callback.
- Whatever is waiting in the channel is delivered as one `Event::Received(Vec<Toast>)` of up to `toast::BATCH_SIZE` (16) toasts, so a burst causes a handful of redraws rather than one per toast.

The channel holds `toast::CHANNEL_CAPACITY` (32) toasts, plus one slot per sender clone. Nothing else is shared between threads, and the UI thread never waits on a worker.

## A complete example

This app has two buttons. Export runs on a thread, Sync runs as an async task, and both report back with a toast.

```rust
use std::thread;
use std::time::Duration;

use iced::futures::SinkExt;
use iced::widget::{column, text};
use iced::{Element, Subscription, Task};
use iced_cube::overlay::toast::{self, Variant, toast, toasts};
use iced_cube::button;

#[derive(Debug, Clone)]
enum Message {
    Toast(toast::Event),
    Export,
    Sync,
}

#[derive(Default)]
struct App {
    toasts: toast::State,
    sender: Option<toast::Sender>,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Toast(event) => {
                if let Some(toast::Output::Ready(sender)) = self.toasts.update(event) {
                    self.sender = Some(sender);
                }
                Task::none()
            }
            Message::Export => {
                let Some(mut sender) = self.sender.clone() else {
                    return Task::none();
                };
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(2));
                    // try_send drops the toast if the channel is full.
                    let _ = sender.try_send(toast("Export finished").variant(Variant::Success));
                });
                Task::none()
            }
            Message::Sync => {
                let Some(mut sender) = self.sender.clone() else {
                    return Task::none();
                };
                Task::future(async move {
                    // Await the real work here, then report back.
                    let _ = sender.send(toast("Sync finished")).await;
                })
                .discard()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let ready = self.sender.is_some();
        let content = column![
            text("Background work reports back through toasts."),
            button("Export").on_press_maybe(ready.then_some(Message::Export)),
            button("Sync").on_press_maybe(ready.then_some(Message::Sync)),
        ]
        .spacing(12)
        .padding(24);

        toasts(&self.toasts, content)
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

The buttons stay disabled until `Ready` has arrived, because until then there is no sender to hand out.

## Sending from a thread

`try_send` never blocks. It fails when the channel is full or the app has closed, and the toast is dropped. For notifications that is usually what you want: a worker should not stall because nobody is reading.

When every message must arrive, block the worker until there is room instead. Add the `futures` crate (`cargo add futures`) and use `block_on` with `SinkExt::send`:

```rust
use futures::SinkExt;
use futures::executor::block_on;

thread::spawn(move || {
    for n in 1..=100 {
        // Waits while the channel is full instead of dropping.
        if block_on(sender.send(toast(format!("Item {n} done")))).is_err() {
            break;
        }
    }
});
```

`send` only fails once the receiver is gone, which means the app has shut down, so that is the moment to stop.

## Sending from async code

In async code, `SinkExt::send` waits for room without blocking a thread. The Sync branch above runs it as an iced `Task`. The same works in a `tokio::spawn` task or any other executor, because the sender does not depend on a runtime.

## Stable identity

iced compares subscriptions by identity to decide whether a stream is new. `toast::subscription()` is built with `Subscription::run` from a plain function, so it keeps the same identity every time `subscription` is called, and the channel and sender survive for the life of the app. If you write a similar subscription yourself, use `Subscription::run` with a function, or `Subscription::run_with` with a stable, hashable id.

## Timers only when needed

`toast::timer(&state)` returns `Subscription::none()` unless a visible toast is counting down. While one is, it emits `Event::Tick` every `toast::TICK` (250 ms), and the tick closes any toast whose time is up. Persistent toasts and toasts waiting behind the visible ones do not keep it running, so an app with no toasts on screen is idle.

The [Spinner](../components/spinner/) follows the same idea without a timer of its own. It has no clock: your app keeps its phase, subscribes to `iced::window::frames()` only while something is loading, and moves the phase on with `spinner::advance`.
