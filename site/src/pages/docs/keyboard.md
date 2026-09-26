---
layout: ../../layouts/Guide.astro
title: Keyboard shortcuts
description: Default shortcuts for interactive components, and how to change them with a keymap.
---

Interactive components such as [Tabs](../components/tabs/), [Slider](../components/slider/) and [Toast](../components/toast/) come with default keyboard shortcuts. Each component page lists its defaults under Keyboard.

iced has no focus model for most widgets, so these shortcuts are not tied to a focused control. Your app listens for key presses and decides which component they go to:

- `keys::subscription()` delivers every key press that no widget captured. A focused text input keeps the keys it uses, such as letters, arrows, Home and End, so typing in it never switches tabs.
- Each component has an `Action` enum and a `default_keymap()`, a `Keymap<Action>` from chords to actions.
- `keymap.resolve_event(&key)` returns the action for a key press, or `None`.
- A pure function on the action turns it into the component's own event or value, such as `action.event(&state)` for tabs or `action.apply(value, range, step)` for a slider.

## A complete example

This app keeps the tab shortcuts but drops the plain arrow keys, which it wants for something else, and adds `Ctrl+PageDown` and `Ctrl+PageUp`:

```rust
use iced::keyboard::key::Named;
use iced::widget::{column, text};
use iced::{Element, Subscription};
use iced_cube::keys::{self, Chord, Keymap};
use iced_cube::navigation::tabs::{self, Action, State, tab, tabs};

#[derive(Debug, Clone)]
enum Message {
    Key(keys::Event),
    Tabs(tabs::Event<Page>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Page {
    Account,
    Billing,
    Team,
}

struct App {
    tabs: State<Page>,
    keymap: Keymap<Action>,
}

impl Default for App {
    fn default() -> Self {
        let keymap = tabs::default_keymap()
            .unbind(&Chord::named(Named::ArrowLeft))
            .unbind(&Chord::named(Named::ArrowRight))
            .bind(Chord::named(Named::PageDown).ctrl(), Action::Next)
            .bind(Chord::named(Named::PageUp).ctrl(), Action::Previous);

        Self {
            tabs: State::new([
                tab(Page::Account, "Account"),
                tab(Page::Billing, "Billing"),
                tab(Page::Team, "Team"),
            ]),
            keymap,
        }
    }
}

impl App {
    fn update(&mut self, message: Message) {
        let event = match message {
            Message::Key(key) => self
                .keymap
                .resolve_event(&key)
                .and_then(|action| action.event(&self.tabs)),
            Message::Tabs(event) => Some(event),
        };
        if let Some(event) = event {
            let _ = self.tabs.update(event);
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let page = match self.tabs.selected() {
            Some(Page::Account) => "Account settings",
            Some(Page::Billing) => "Invoices and payment methods",
            Some(Page::Team) => "People with access",
            None => "",
        };
        column![tabs(&self.tabs).on_event(Message::Tabs), text(page)]
            .spacing(16)
            .padding(24)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        keys::subscription().map(Message::Key)
    }
}

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .subscription(App::subscription)
        .run()
}
```

The keymap lives in your app's state, so it can change at runtime, for example from a settings screen. Keymaps are plain values: `bind`, `unbind` and `clear` each return the changed keymap.

## Changing a keymap

- `bind(chord, action)` adds a chord. If the chord already did something, it now does this instead.
- `unbind(&chord)` removes one chord. `unbind_action(&action)` removes every chord for an action.
- `clear()` removes everything, so you can build the keymap from scratch.
- `grouped()` lists the chords for each action, which is handy for a help screen.

When two components want the same key, such as the arrow keys for tabs and a slider, decide in `update` which keymap to ask. For example, resolve through the slider's keymap while the slider is the current control, and fall back to the tabs keymap otherwise.

## Chord syntax

A `Chord` is a key plus the modifiers held with it. Build one in code with `Chord::named(Named::Tab).ctrl()` or `Chord::character('k')`, or parse it from a string, which suits settings files:

```rust
use iced_cube::keys::Chord;

let chord: Chord = "Ctrl+Shift+Tab".parse().expect("valid chord");
assert_eq!(chord.to_string(), "Ctrl+Shift+Tab");
```

- Join modifiers and the key with `+`. Case does not matter, so `ctrl+k` and `Ctrl+K` are the same chord.
- Modifiers are `Ctrl`, `Shift`, `Alt` (or `Option`) and `Cmd` (or `Super` or `Meta`). `Mod` means Cmd on macOS and Ctrl on Linux and Windows, for shortcuts that should feel native on each.
- A key is a single character, such as `K`, `/` or `+`, or a name: `Enter`, `Tab`, `Space`, `Escape`, `Backspace`, `Delete`, `Insert`, `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Home`, `End`, `PageUp`, `PageDown` and `F1` to `F12`. Short forms such as `Esc`, `Up` and `PgDn` also work.
- A chord matches only its exact modifiers: `Ctrl+K` does not fire for `Ctrl+Shift+K`.

Displaying a chord gives the canonical form, with modifiers in the order Ctrl, Alt, Shift, Cmd, which is what the component pages show.

## Built-in widget keys

Some keys are handled inside iced's own widgets rather than through a keymap, such as text editing in [Input](../components/input/) and [Textarea](../components/textarea/). Those are listed on their component pages and cannot be rebound. While one of these widgets has focus it captures the keys it handles, so they never reach `keys::subscription()`. Keys it ignores, such as `Ctrl+Tab`, still do.
