---
title: Formatting Toolbar
description: Text size, bold, italic, lists, links and clear formatting for the rich text editor, showing what is on at the caret.
group: Application
order: 5
module: application::formatting_toolbar
imports: |
  use iced_cube::formatting_toolbar::{self, LINK_FIELD, Output, formatting_toolbar};
keywords: [rich text toolbar, editor toolbar, text formatting, wysiwyg]
related: [rich-text-editor, toolbar, dropdown-menu]
hero: formatting-toolbar/widths
stories: [formatting-toolbar/widths]
api:
  - name: "formatting_toolbar::State::new() / State::with_keymap(&keymap)"
    description: "The buttons, the size menu and the link field. with_keymap shows your editor keymap's chords in the tooltips."
  - name: "state.sync(&content)"
    description: "Matches the buttons to the formatting at the caret. Call it after every change to the content. Returns Output::EditLink when Ctrl+K was pressed in the editor."
  - name: "state.update(event, &content)"
    description: "Handles presses, the size menu and the link field. Returns Output::Format with an Action for your content, Output::EditLink when the link field opens, Output::Closed when the field, the size menu or the keyboard focus closes with nothing to apply, and Output::Unfocus when Ctrl+F10 moves the keyboard onto the buttons."
  - name: "formatting_toolbar::unfocus()"
    description: "A task that takes focus from the editor, for Output::Unfocus, so the arrow keys reach the toolbar."
  - name: "state.is_on(Tool) / state.size() / state.link()"
    description: "Whether a button is on, the size at the caret (None when the selection mixes sizes), and the address being edited."
  - name: "state.key_event(&keymap, &key)"
    description: "Resolves a key press from keys::subscription with the toolbar keymap: Ctrl+F10 focuses the buttons for the arrow keys."
  - name: "formatting_toolbar(&state)"
    description: "Renders the size menu, the buttons that fit with a More menu for the rest, and the link field while it is open. It renders disabled until .on_event is set."
  - name: ".on_event(fn) / .tooltip(Option<Position>)"
    description: "Maps toolbar events to your message, and moves or hides the tooltips, which sit below the buttons by default, as on the toolbar."
  - name: "LINK_FIELD / SIZE_WIDTH"
    description: "The id of the link address field, to focus on Output::EditLink, and the width of the size button."
---

The formatting toolbar is its own component, so you decide where it goes: above the [rich text editor](../rich-text-editor/), in a window's toolbar row, or under a compose form on a phone. The buttons are a [toolbar](../toolbar/), so on a narrow window the ones that do not fit move into its More menu, where toggles become checkbox items. The text size stays in view as a [dropdown menu](../dropdown-menu/).

Your app owns a `formatting_toolbar::State`. When `update` returns `Output::Format(action)`, apply the action to your content, call `sync` and focus the editor again, as the example does with `widget::operation::focus`. Do the same for `Output::Closed`, which comes when the size menu or the link field closes with nothing chosen. A click on the toolbar takes focus from the editor, and the selection waits in the content, shown quieter, until it comes back. After the editor's own actions, call `sync` so the buttons follow the caret.

The link button opens an address field under the strip, filled in when the caret is on a link; pressing it again, Close or Escape shuts the field. Enter or Apply links the selection, or the word at the caret; an address without a scheme gets `https://`, an email address gets `mailto:`, and anything else, such as a script or a single word, is refused and the field turns red. Remove takes the link off and an empty address does the same.

For the keyboard, route presses from `keys::subscription()` through `state.key_event` with the [toolbar](../toolbar/) keymap. Ctrl+F10 returns `Output::Unfocus`: run `formatting_toolbar::unfocus()` so the editor lets the arrow keys through, then Left and Right move through the buttons and Escape returns `Output::Closed`. The size menu is reached with the pointer or a tap. If a plain toolbar is on screen too, route Ctrl+F10 to only one of them.
