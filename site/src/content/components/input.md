---
title: Input
description: A single line of editable text, with an optional icon, password masking and an invalid state.
group: Forms
order: 1
module: primitives::input
imports: |
  use iced_cube::{input, lucide};
  use iced_cube::primitives::input::Size;
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
keyboard:
  - keys: "Enter"
    action: "Emits the on_submit message."
  - keys: "Escape"
    action: "Removes focus from the input."
  - keys: "Home / End"
    action: "Moves the cursor to the start or end of the value."
  - keys: "Shift + Arrow"
    action: "Extends the selection."
---

Inputs are controlled: the app stores the value, passes it to `input` and updates it from the `on_input` message. Leave out `on_input` to show a read-only value in the disabled style.

To name an input, wrap it in [`field`](../field/), which adds a label, a description and an error message. Set `.invalid(true)` on the input when the field has an error so the border matches.

Clicking an input focuses it. iced does not move focus on Tab by itself: listen for Tab in a keyboard subscription and return `iced::widget::operation::focus_next()`, or give the input an `.id(...)` and return `iced::widget::operation::focus(id)` from `update`.
