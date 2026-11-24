---
title: Switch
description: Turns a setting on or off with immediate effect.
group: Forms
order: 5
module: primitives::switch
imports: |
  use iced_cube::switch;
keywords: [toggle]
related: [checkbox]
hero: switch/settings
stories: [switch/default, switch/states, switch/settings]
api:
  - name: "switch(is_on)"
    description: "Creates a switch. It renders disabled until it has a message."
  - name: ".label(text)"
    description: "Adds a label after the switch. Clicking the label toggles the switch too."
  - name: ".on_toggle(f) / .on_toggle_maybe(option)"
    description: "Sets the message for a click. The closure gets the new value. None disables the switch."
  - name: "switch::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>, for apps that track a current switch."
  - name: "Action::Toggle.apply(is_on)"
    description: "The value a click would send. Pass it to your on_toggle message."
keyboard:
  - keys: "Tab, Shift + Tab"
    action: "Moves keyboard focus to or from the switch inside a dialog, or wherever the app runs iced's focus_next and focus_previous."
  - keys: "Space, Enter"
    action: "Turns the focused switch on or off."
---

Use a switch for settings that apply as soon as they change, such as turning notifications on. For choices that wait for a submit button, use a checkbox.

An enabled switch can take keyboard focus. Its track then rings, and Space or Enter flips it. A click never leaves the ring showing. The app-wide `Toggle` shortcut in the keymap is for an app that tracks which switch is current itself.

In a settings list, pass the switch without a label and put the title and description in your own layout, as the settings example does.
