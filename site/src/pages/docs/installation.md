---
layout: ../../layouts/Guide.astro
title: Installation
description: Add iced-cube to an iced 0.14 application and render your first component.
---

## Add the crate

```sh
cargo add iced@0.14 iced-cube
```

iced-cube is built for iced 0.14, so pin iced to that version. A newer iced would sit alongside the 0.14 copy that iced-cube uses, and their types would not match.

## Your first app

Replace `src/main.rs` with this and run `cargo run`:

```rust
use iced::Element;
use iced::widget::{column, row, text};
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide, theme};

#[derive(Debug, Clone)]
enum Message {
    Increment,
    Reset,
}

#[derive(Default)]
struct App {
    count: u32,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Increment => self.count += 1,
            Message::Reset => self.count = 0,
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // A button without a message renders disabled.
        let reset = (self.count > 0).then_some(Message::Reset);

        column![
            text(format!("Pressed {} times", self.count)),
            row![
                button("Press me")
                    .icon(lucide!(Plus))
                    .on_press(Message::Increment),
                button("Reset")
                    .variant(Variant::Secondary)
                    .on_press_maybe(reset),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .padding(24)
        .into()
    }
}

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .theme(|_: &App| theme::light())
        .run()
}
```

Components read their colours from whichever iced theme is active. `theme::light()` and `theme::dark()` are tuned for them, but any iced theme works. See [Theming](../theming/).

## Running an example from these docs

Every example on a component page is a single file with the same shape: a `Message` enum, an `Example` struct that implements `Default`, and `update` and `view` methods. Some also have a `subscription` method. To run one on your machine, start a project as above, open the example's Code tab, and save the code as `src/example.rs`. Then point `src/main.rs` at it:

```rust
mod example;

use example::Example;

fn main() -> iced::Result {
    iced::application(Example::default, Example::update, Example::view)
        .theme(|_: &Example| iced_cube::theme::light())
        .run()
}
```

If the example has a `subscription` method, such as the spinner or toast examples, register it too, or it will sit still:

```rust
fn main() -> iced::Result {
    iced::application(Example::default, Example::update, Example::view)
        .theme(|_: &Example| iced_cube::theme::light())
        .subscription(Example::subscription)
        .run()
}
```

The toast background job example also needs `cargo add futures-timer`.

## Features

- `x11`, `wayland` and `thread-pool` are on by default and turn on the iced features of the same name. They let the crate build on its own and on docs.rs. In your app, iced's own default features already turn them on, so switching them off in iced-cube changes nothing unless you also turn off iced's defaults.
- `tokio` turns on iced's `tokio` feature, which makes tokio iced's executor, and makes the toast timer use `iced::time::every`. Without it, the toast timer runs on a small thread that sleeps between ticks, so toasts close on time whichever executor you use. If your app already runs on tokio, add it with `cargo add iced-cube --features tokio`.

In the browser the toast timer always uses `iced::time::every`, and no feature is needed.

## Next steps

- Browse the components, starting with [Button](../components/button/).
- Match your brand with [Theming](../theming/).
- Pick icons in [Icons](../icons/).
- Send toasts from background work with [Subscriptions](../subscriptions/).
- Write headless tests for your UI with [Testing](../testing/).
