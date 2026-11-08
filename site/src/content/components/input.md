---
title: Input
description: A single line of editable text, with an optional icon, password masking and an invalid state.
group: Forms
order: 1
module: primitives::input
imports: |
  use iced_cube::{input, lucide};
  use iced_cube::primitives::input::{Shortcut, Shortcuts, Size};
keywords: [text field, text input]
related: [field, textarea]
hero: input/with-icon
stories: [input/default, input/with-icon, input/password, input/states]
api:
  - name: "input(placeholder, value)"
    description: "Creates a text input showing value, with the placeholder while it is empty. It renders disabled until it has on_input."
  - name: ".on_input(fn) / .on_input_maybe(option)"
    description: "Sets the message built from the new value on every edit. None disables the input."
  - name: ".on_submit(message) / .on_submit_maybe(option)"
    description: "Sets the message emitted when Enter is pressed. A disabled input never submits."
  - name: ".icon(glyph)"
    description: "Adds a Lucide icon at the start of the input."
  - name: ".secure(bool)"
    description: "Masks the value, for passwords and other secrets."
  - name: ".invalid(bool)"
    description: "Draws a destructive border. Pair it with an error message from field."
  - name: ".size(Size)"
    description: "Sm, Md or Lg (32, 36 and 40 pixels tall). Defaults to Md."
  - name: ".id(id) / .width(length)"
    description: "Sets the widget id, used to focus the input from a task or find it in tests, and overrides the width. Inputs fill the available width by default."
  - name: ".shortcuts(Shortcuts)"
    description: "Sets which shortcuts the input handles: undo, redo, word deletion and Escape. Shortcuts::all() is the default, Shortcuts::none() turns them all off, and .without(Shortcut::Undo) turns off one. The keys of a shortcut that is off reach your app."
  - name: ".keymap(keymap)"
    description: "Binds the shortcuts to other keys. Starts from input::default_keymap()."
keyboard:
  - keys: "Enter"
    action: "Emits the on_submit message."
  - keys: "Ctrl + A, C, X, V (Cmd on macOS)"
    action: "Selects all, copies, cuts and pastes."
  - keys: "Ctrl + Arrow (Alt on macOS)"
    action: "Moves the cursor by word. Hold Shift to select."
  - keys: "Home / End"
    action: "Moves the cursor to the start or end of the value."
  - keys: "Shift + Arrow"
    action: "Extends the selection."
---

Inputs are controlled: the app stores the value, passes it to `input` and updates it from the `on_input` message. Leave out `on_input` to show a read-only value in the disabled style.

Undo comes built in. The input remembers its own edits and sends the earlier value through `on_input`, so there is nothing to store. Typing undoes a word at a time, and a paste, cut or deleted word is a step of its own. If your app changes the value itself, for example by loading another record, the history starts again.

Each shortcut is a `Shortcut`, and a `Shortcuts` set says which ones a field handles. Turn off the ones your app handles itself, for example `.shortcuts(Shortcuts::all().without(Shortcut::Undo).without(Shortcut::Redo))` when the app has its own undo, or `.shortcuts(Shortcuts::none())` for iced's plain behaviour. The textarea, and every component with a search or filter field (combobox, command, command palette, data table, log viewer, JSON viewer and property grid), takes the same set. A key typed with Ctrl or Cmd held never types its character, whatever the set.

To name an input, wrap it in [`field`](../field/), which adds a label, a description and an error message. Set `.invalid(true)` on the input when the field has an error so the border matches.

Clicking an input focuses it. iced does not move focus on Tab by itself: listen for Tab in a keyboard subscription and return `iced::widget::operation::focus_next()`, or give the input an `.id(...)` and return `iced::widget::operation::focus(id)` from `update`.
