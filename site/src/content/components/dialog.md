---
title: Dialog
description: A modal window over a dimmed scrim, for a focused task or a confirmation.
group: Overlays
order: 3
module: overlay::dialog
imports: |
  use iced_cube::{alert_dialog, dialog};
  use iced_cube::overlay::dialog::{self, Size};
keywords: [modal, alert dialog, confirm, popup]
related: [button, icon-button, toast]
hero: dialog/default
stories: [dialog/default, dialog/form, dialog/destructive]
api:
  - name: "dialog(base)"
    description: "Wraps the content the dialog opens over. The dialog stays closed until .open(true)."
  - name: ".open(bool)"
    description: "Whether the dialog shows. Your app holds this value."
  - name: ".title(text) / .description(text)"
    description: "The heading, and a line of muted text under it."
  - name: ".body(element)"
    description: "Content between the header and the footer, such as a form."
  - name: ".action(element)"
    description: "Adds a button to the footer, which lines them up on the right. Add the main action last."
  - name: ".on_dismiss(message)"
    description: "Emitted by Escape, a click on the scrim and the close button. Without it the close button renders disabled."
  - name: ".dismiss_on_escape(bool) / .dismiss_on_scrim(bool)"
    description: "Turns either way of dismissing off. Both default to true."
  - name: ".close_button(bool)"
    description: "Shows or hides the close button in the top right corner. Defaults to true."
  - name: ".size(Size) / .width(pixels)"
    description: "The maximum width: Sm (384), Md (512, the default) or Lg (640). The dialog shrinks to fit smaller windows."
  - name: ".keymap(Keymap<Action>)"
    description: "Replaces the default shortcuts."
  - name: ".pass_through(chords)"
    description: "Chords the open dialog lets through to keys::subscription, such as the shortcut that toggles a command palette. Every other key press stops at the dialog."
  - name: ".id(id)"
    description: "The surface's widget id, which scopes focus. Give nested dialogs different ids."
  - name: "alert_dialog(base, title, description)"
    description: "A confirmation with Cancel and a destructive button. The scrim does not dismiss it and it has no close button."
  - name: ".cancel(label) / .confirm(label)"
    description: "The button labels. They default to Cancel and Continue."
  - name: ".on_cancel(message) / .on_confirm(message)"
    description: "Emitted by the buttons. Escape also cancels unless .dismiss_on_escape(false) is set."
  - name: "dialog::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.effect(dismiss_on_escape)"
    description: "What a resolved Action does to an open dialog: Effect::Dismiss or Effect::Focus(direction)."
  - name: "dialog::CLOSE_BUTTON_ID"
    description: "The widget id of the close button, for clicking it in tests."
  - name: "surface_style(tokens) / scrim_style(tokens)"
    description: "The surface and scrim styles, for reuse in custom overlays."
---

Wrap the root of your view in `dialog`, so the scrim covers the whole window. While the dialog is open, the content underneath still draws but gets no clicks, hover or key presses. It keeps its widget state, such as scroll positions, when the dialog opens and closes.

Use a dialog for a short task that needs attention before people go back to what they were doing. For a destructive confirmation, use `alert_dialog`: it has no close button and ignores clicks on the scrim, so the only ways out are Cancel, Escape or the destructive action. Turn Escape off with `.dismiss_on_escape(false)` when even that is too easy.

The dialog handles its own keys, so it needs no subscription. When it opens, it focuses its first text field and unfocuses anything underneath. Tab and Shift+Tab then move between the text fields inside it and wrap at either end, so focus never leaves the dialog. It captures every key press its content leaves, so app-wide shortcuts from `keys::subscription` cannot change tabs, sliders or menus behind the scrim. Let a chosen shortcut through with `.pass_through(...)`.

Menus, comboboxes and command lists inside a dialog get keys and clicks first. Escape or a click on the scrim closes an open menu or list and leaves the dialog open; the next one closes the dialog. The [command palette](../command/) example puts a command list in a dialog.

Some limits come from iced 0.14 itself:

- Only text fields and text editors can take keyboard focus. Tab moves between those, but it skips buttons, and Enter or Space cannot press a button. Give a form an `on_submit` on its last field if people should be able to save from the keyboard.
- A focused text field takes the first Escape to lose focus, so it takes a second Escape to dismiss the dialog.
- There is no screen reader support, so the dialog's title and description are not announced.
