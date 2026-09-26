---
title: Textarea
description: Multi-line text editing that matches the look of Input.
group: Forms
order: 2
module: forms::textarea
imports: |
  use iced::widget::text_editor;
  use iced_cube::textarea;
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
    description: "Text shown while the content is empty."
  - name: ".height(length)"
    description: "Sets the height. By default the textarea starts about three lines tall and grows with its content. A fixed height scrolls once the text overflows."
  - name: ".min_height(pixels)"
    description: "Sets the starting height used while the height is Shrink. Defaults to 80."
  - name: ".invalid(bool)"
    description: "Draws a destructive border."
  - name: ".id(id) / .width(length)"
    description: "Sets the widget id and overrides the width. Textareas fill the available width by default."
keyboard:
  - keys: "Enter"
    action: "Inserts a new line."
  - keys: "Escape"
    action: "Removes focus from the textarea."
  - keys: "Arrow keys, Home, End, Page Up, Page Down"
    action: "Moves the cursor. Hold Shift to select."
---

The textarea wraps iced's text editor, so your app owns the `text_editor::Content` and applies every `Action` it receives. This keeps undo, validation and limits in your own `update`, as the character count example shows.

Read the text back with `content.text()`. Wrap the textarea in [`field`](../field/) to give it a label and helper text.
