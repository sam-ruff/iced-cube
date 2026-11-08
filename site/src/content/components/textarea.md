---
title: Textarea
description: Multi-line text editing that matches the look of Input.
group: Forms
order: 2
module: forms::textarea
imports: |
  use iced::widget::text_editor;
  use iced_cube::textarea;
  use iced_cube::textarea::{Shortcut, Shortcuts};
keywords: [multiline]
related: [input, field]
hero: textarea/default
stories: [textarea/default, textarea/disabled]
api:
  - name: "textarea(&content)"
    description: "Creates a textarea editing a text_editor::Content owned by your app. It renders disabled until it has on_action."
  - name: ".on_action(fn) / .on_action_maybe(option)"
    description: "Sets the message built from each edit action. Apply it with content.perform(action) in update. None disables the textarea."
  - name: ".placeholder(text)"
    description: "Text shown while the content is empty. It is hidden while a phone keyboard or input method is composing a word in the focused textarea."
  - name: ".height(length)"
    description: "Sets the height. By default the textarea starts about three lines tall and grows with its content. A fixed height scrolls once the text overflows."
  - name: ".min_height(pixels)"
    description: "Sets the starting height used while the height is Shrink. Defaults to 80."
  - name: ".invalid(bool)"
    description: "Draws a destructive border."
  - name: ".id(id) / .width(length)"
    description: "Sets the widget id and overrides the width. Textareas fill the available width by default."
  - name: ".shortcuts(Shortcuts)"
    description: "Sets which shortcuts the textarea handles: undo, redo, word deletion and Escape. Shortcuts::all() is the default, Shortcuts::none() turns them all off, and .without(Shortcut::EscapeUnfocus) turns off one. The keys of a shortcut that is off reach your app."
  - name: ".keymap(keymap)"
    description: "Binds the shortcuts to other keys. Starts from input::default_keymap()."
keyboard:
  - keys: "Enter"
    action: "Inserts a new line."
  - keys: "Ctrl + A, C, X, V (Cmd on macOS)"
    action: "Selects all, copies, cuts and pastes."
  - keys: "Arrow keys, Home, End, Page Up, Page Down"
    action: "Moves the cursor. Hold Shift to select."
  - keys: "Ctrl + Arrow, Ctrl + Home / End (Alt on macOS)"
    action: "Moves by word, or to the start or end of the text."
---

The textarea wraps iced's text editor, so your app owns the `text_editor::Content` and applies every `Action` it receives. This keeps validation and limits in your own `update`, as the character count example shows.

Undo works with that as it is. The textarea keeps its own history and, on Ctrl+Z, sends the actions that put the earlier text back, which your `update` applies like any others. Typing undoes a word at a time. If your app replaces the content itself, for example to open another document, the history starts again.

Escape removes focus from the textarea and stops there, so a dialog around it stays open. Switch that off on its own with `.shortcuts(Shortcuts::all().without(Shortcut::EscapeUnfocus))`, or every shortcut with `Shortcuts::none()`; the keys then reach your app. `Shortcut` and `Shortcuts` are the same types the [input](../input/) takes.

Read the text back with `content.text()`. Wrap the textarea in [`field`](../field/) to give it a label and helper text.

On a touch screen a tap places the caret and a vertical drag scrolls the text, because the textarea wraps its editor in [touch clicks](../touch-clicks/) with scrolling on. A drag inside a textarea therefore scrolls the textarea, not a list around it.
