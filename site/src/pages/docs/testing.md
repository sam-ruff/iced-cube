---
layout: ../../layouts/Guide.astro
title: Testing
description: Headless tests for your own UI with iced_test.
---

iced-cube components are ordinary iced widgets, so you can test a UI built with them the same way iced-cube tests itself: with the headless simulator from `iced_test`. It lays out your view, lets you click and type, and hands back the messages your app would have received. No window or GPU is needed.

## Setup

```sh
cargo add --dev iced_test@0.14
```

The simulator renders with whichever backend iced picks. To render on the CPU, so tests behave the same on every machine and in CI, add this to `.cargo/config.toml`:

```toml
[env]
ICED_TEST_BACKEND = "tiny-skia"
```

## What the simulator can do

- `simulator(element)` builds a simulator from any `Element`, usually `app.view()`.
- `ui.click(selector)` clicks a widget found by its text or by its widget `Id`.
- `ui.typewrite(text)` types into whichever input has focus, and `ui.tap_key(key)` presses one key.
- `ui.find(selector)` checks that something is on screen.
- `ui.into_messages()` ends the run and returns the messages it produced, in order.

The simulator never calls your `update`. Feed the messages back into your app yourself, then build a new simulator from the new view to check the result.

## An example

A form with a name input and a Save button that raises a toast. The input has an `id`, so the test can click it to give it focus, and `toast::close_button_id` gives each toast's close button an id to click.

The app and its tests share one file here, such as `tests/app.rs`, so the example stands on its own. In a real project, put the tests in a `#[cfg(test)] mod tests` next to your `App`, or move the app into a library crate that the files under `tests/` can import.

```rust
use iced::Element;
use iced::widget::{Id, column};
use iced_cube::overlay::toast::{self, toast, toasts};
use iced_cube::{button, input};
use iced_test::{Error, simulator};

#[derive(Debug, Clone)]
enum Message {
    Name(String),
    Save,
    Toast(toast::Event),
}

#[derive(Default)]
struct App {
    name: String,
    toasts: toast::State,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::Name(name) => self.name = name,
            Message::Save => {
                self.toasts.push(toast(format!("Saved {}", self.name)));
            }
            Message::Toast(event) => {
                let _ = self.toasts.update(event);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let save = (!self.name.is_empty()).then_some(Message::Save);
        let form = column![
            input("Your name", &self.name)
                .id("name")
                .on_input(Message::Name),
            button("Save").on_press_maybe(save),
        ];

        toasts(&self.toasts, form)
            .on_event(Message::Toast)
            .into()
    }
}

/// Feeds every message a simulator recorded back into the app.
fn apply(app: &mut App, messages: impl IntoIterator<Item = Message>) {
    for message in messages {
        app.update(message);
    }
}

#[test]
fn typing_a_name_and_saving_shows_a_toast() -> Result<(), Error> {
    let mut app = App::default();

    let mut ui = simulator(app.view());
    ui.click(Id::new("name"))?;
    ui.typewrite("Ada");
    let messages: Vec<_> = ui.into_messages().collect();
    apply(&mut app, messages);
    assert_eq!(app.name, "Ada");

    let mut ui = simulator(app.view());
    ui.click("Save")?;
    let messages: Vec<_> = ui.into_messages().collect();
    apply(&mut app, messages);

    let mut ui = simulator(app.view());
    assert!(ui.find("Saved Ada").is_ok());
    Ok(())
}

#[test]
fn save_is_disabled_until_a_name_is_typed() -> Result<(), Error> {
    let app = App::default();
    let mut ui = simulator(app.view());
    ui.click("Save")?;
    assert_eq!(ui.into_messages().count(), 0);
    Ok(())
}

#[test]
fn closing_a_toast_removes_it() -> Result<(), Error> {
    let mut app = App::default();
    let id = app.toasts.push(toast("Saved Ada"));

    let mut ui = simulator(app.view());
    ui.click(toast::close_button_id(id))?;
    let messages: Vec<_> = ui.into_messages().collect();
    apply(&mut app, messages);

    assert!(app.toasts.is_empty());
    Ok(())
}
```

## Testing state without a view

Stateful components keep their logic in plain structs, so much of it can be tested without the simulator at all. `tabs::State`, `accordion::State` and `toast::State` each have an `update` you can call directly. Toasts take the current time through `Event::Tick` (or `state.tick(now)`), so expiry can be tested with any `Instant` you like instead of waiting.
