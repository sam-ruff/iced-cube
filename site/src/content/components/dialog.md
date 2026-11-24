---
title: Dialog
description: A modal window over a dimmed scrim, for a focused task or a confirmation.
group: Overlays
order: 3
module: overlay::dialog
imports: |
  use iced_cube::{alert_dialog, dialog};
  use iced_cube::overlay::dialog::{self, Size, columns, section};
keywords: [modal, alert dialog, confirm, popup, sections, two columns]
related: [button, icon-button, toast, scroll-area]
hero: dialog/default
stories: [dialog/default, dialog/form, dialog/destructive, dialog/long, dialog/sections]
api:
  - name: "dialog(base)"
    description: "Wraps the content the dialog opens over. The dialog stays closed until .open(true)."
  - name: ".open(bool)"
    description: "Whether the dialog shows. Your app holds this value."
  - name: ".title(text) / .description(text)"
    description: "The heading, and a line of muted text under it."
  - name: ".body(element)"
    description: "Content between the header and the footer, such as a form. It scrolls when the dialog does not fit the window."
  - name: ".action(element)"
    description: "Adds a button to the footer, which lines them up on the right. Add the main action last."
  - name: ".on_dismiss(message)"
    description: "Emitted by Escape, a click on the scrim and the close button. Without it the close button renders disabled."
  - name: ".on_confirm(message)"
    description: "Emitted by Enter, through the keymap's Confirm action, usually the same message as the main footer action. Without it Enter does nothing. A field inside that submits on Enter keeps the key."
  - name: ".dismiss_on_escape(bool) / .dismiss_on_scrim(bool)"
    description: "Turns either way of dismissing off. Both default to true."
  - name: ".close_button(bool)"
    description: "Shows or hides the close button in the top right corner. Defaults to true."
  - name: ".size(Size) / .width(pixels)"
    description: "The maximum width: Sm (384), Md (512, the default) or Lg (640). The dialog shrinks to fit smaller windows."
  - name: ".max_height(pixels)"
    description: "Caps the height below the window's. The dialog is never taller than the window less the scrim's padding either way."
  - name: ".keymap(Keymap<Action>)"
    description: "Replaces the default shortcuts."
  - name: ".pass_through(chords)"
    description: "Chords the open dialog lets through to keys::subscription, such as the shortcut that toggles a command palette. Every other key press stops at the dialog."
  - name: ".id(id)"
    description: "The surface's widget id, which scopes focus. Give nested dialogs different ids."
  - name: "alert_dialog(base, title, description)"
    description: "A confirmation with Cancel and a destructive button. It has no close button, and by default the scrim does not dismiss it."
  - name: ".cancel(label) / .confirm(label)"
    description: "The button labels. They default to Cancel and Continue."
  - name: ".on_cancel(message) / .on_confirm(message)"
    description: "Emitted by the buttons. Escape also cancels unless .dismiss_on_escape(false) is set."
  - name: ".confirm_on_enter(bool)"
    description: "Whether Enter runs the destructive action while no button is focused. Defaults to false, so a stray Enter never destroys anything; keyboard users Tab to the button and press Enter or Space instead."
  - name: "alert.dismiss_on_scrim(bool)"
    description: "Whether a click on the scrim cancels an alert dialog, sending on_cancel. Defaults to false."
  - name: "section(title, body)"
    description: "A headed group for a dialog body, titled in the dialog's heading style one size smaller. Add .description(text) for a muted line under the title."
  - name: "columns(children)"
    description: "Lays children out in two equal columns, row by row, and in one column where the space is narrower than .breakpoint(pixels), which defaults to dialog::BREAKPOINT (400). .spacing(pixels) sets the gap, 24 by default."
  - name: "dialog::cells(width, breakpoint, spacing)"
    description: "How many columns fit in a width and how wide each is, as columns works it out."
  - name: "dialog::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.effect(dismiss_on_escape)"
    description: "What a resolved Action does to an open dialog: Effect::Dismiss, Effect::Confirm or Effect::Focus(direction)."
  - name: "dialog::CLOSE_BUTTON_ID"
    description: "The widget id of the close button, for clicking it in tests."
  - name: "surface_style(tokens) / scrim_style(tokens)"
    description: "The surface and scrim styles, for reuse in custom overlays."
keyboard:
  - keys: "Space, Enter"
    action: "Presses the focused button, flips the focused switch or toggle, or toggles the focused checkbox."
  - keys: "Space, Enter, Down"
    action: "Opens the focused select."
---

Wrap the root of your view in `dialog`, so the scrim covers the whole window. While the dialog is open, the content underneath still draws but gets no clicks, hover or key presses. It keeps its widget state, such as scroll positions, when the dialog opens and closes.

A dialog is never taller than the window. When the body does not fit, the title, description and actions stay where they are and the body scrolls, so Save and Cancel are always in reach. Tab scrolls a field below the fold into view as it focuses it. Use `.max_height(...)` for a dialog that should stay shorter than the window.

A longer form reads better in headed groups. Wrap each group in `section(title, body)` and put them in `columns([...])` to place them side by side in a large dialog. Below the breakpoint, on a phone for example, the sections stack in one column.

Use a dialog for a short task that needs attention before people go back to what they were doing. For a destructive confirmation, use `alert_dialog`: it has no close button and ignores clicks on the scrim, so the only ways out are Cancel, Escape or the destructive action. Turn on `.dismiss_on_scrim(true)` when a click outside should cancel, as for a confirmation that only discards a draft. Enter does nothing in it by default, because a key press meant for something else should never delete anything; turn it on with `.confirm_on_enter(true)` when the action is easy to undo. Turn Escape off with `.dismiss_on_escape(false)` when even that is too easy.

Keyboard users answer an alert dialog with Tab: nothing is focused when it opens, the first Tab rings Cancel, the next rings the destructive action, and Enter or Space presses the ringed button. Shift+Tab goes back, and a click anywhere clears the ring. To confirm with a single chord instead, bind one to `Action::Confirm` in the keymap and turn on `.confirm_on_enter(true)`.

The dialog handles its own keys, so it needs no subscription. When it opens, it focuses its first text field and unfocuses anything underneath. Tab and Shift+Tab then move through everything inside it that takes focus, in layout order, and wrap at either end, so focus never leaves the dialog. That covers text fields and text editors, and this library's buttons, icon buttons, switches, checkboxes, radios, toggles and selects, including the actions in the footer and the close button. Tab scrolls the body to show whatever it focuses. A focused control draws a ring: Space or Enter presses a button, flips a switch or toggles a checkbox, and Space, Enter or Down opens a select. A click never leaves a ring behind. The dialog captures every key press its content leaves, so app-wide shortcuts from `keys::subscription` cannot change tabs, sliders or menus behind the scrim. Let a chosen shortcut through with `.pass_through(...)`.

Menus, selects, comboboxes and command lists inside a dialog get keys and clicks first. Escape or a click on the scrim closes an open menu or list and leaves the dialog open; the next one closes the dialog, even while a text field has focus. The [command palette](../command/) example puts a command list in a dialog.

Enter goes to whatever has focus first: a focused button presses, and a field with an `on_submit` submits. Otherwise Enter sends `.on_confirm(...)`, if the dialog has one.

Some limits come from iced 0.14 itself:

- Widgets from other crates, and iced's own buttons, checkboxes and togglers, cannot take keyboard focus, so Tab skips them. Use this library's versions inside a dialog.
- There is no screen reader support, so the dialog's title and description are not announced.
