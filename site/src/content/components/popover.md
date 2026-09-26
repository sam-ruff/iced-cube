---
title: Popover
description: A floating panel of any content, anchored to the button that opens it.
group: Overlays
order: 3
module: overlay::popover
imports: |
  use iced_cube::popover;
  use iced_cube::overlay::popover::{Align, Side};
keywords: [floating panel, flyout, popup]
related: [dropdown-menu, tooltip]
hero: popover/settings
stories: [popover/settings, popover/placement, popover/keyboard]
api:
  - name: "popover(trigger, content)"
    description: "Shows the content in a panel next to the trigger while it is open. The trigger opens it itself, usually a button that sets a flag in your state."
  - name: ".open(bool)"
    description: "Whether the panel is showing. Defaults to false."
  - name: ".on_dismiss(message)"
    description: "Sent on Escape and on a press outside the panel and the trigger. Without it, only your app closes the popover."
  - name: ".side(Side) / .align(Align)"
    description: "Top, Bottom, Left or Right, and Start, Center or End. Defaults to below the trigger, centred."
  - name: ".gap(f32)"
    description: "Space between the trigger and the panel. Defaults to 4."
  - name: ".width(length) / .padding(f32)"
    description: "The panel is 288 wide with 16 of padding by default. Length::Shrink fits the content."
  - name: ".keymap(keymap)"
    description: "Replaces the default keymap. The open popover closes on the chords bound to Action::Close, after its content has had the key."
  - name: "style(tokens)"
    description: "The panel style, the floating surface shared with menus and dialogs."
  - name: "popover::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind a chord to Action::Toggle to give a popover an app-wide shortcut."
  - name: "action.apply(open)"
    description: "Whether the popover should be open after a resolved Action (Toggle or Close), or None when nothing changes."
---

A popover holds a small piece of UI that belongs to one button, such as a settings form, a colour picker or a short explanation. It is not modal: the rest of the window stays usable, and clicking elsewhere closes it through `on_dismiss` while the click still reaches what it landed on. Menus, comboboxes and command lists inside a popover close first: Escape or a click outside closes the inner layer and leaves the popover open. For choosing one action from a list, use a [dropdown menu](../dropdown-menu/) instead, and for a label that appears on hover, a [tooltip](../tooltip/).

Your app owns whether the popover is open. Toggle a flag from the trigger's message, clear it on dismiss and pass it to `.open`. Clicks on the trigger are left to the trigger, so it can close the popover as well as open it.

The panel moves to the opposite side when there is no room on the one you asked for, and slides along its side to stay inside the window. Clicks inside it never reach the widgets underneath.
