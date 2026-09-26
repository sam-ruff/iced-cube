---
title: Checkbox
description: Turns an option on or off, with an indeterminate state for mixed selections.
group: Forms
order: 3
module: primitives::checkbox
imports: |
  use iced_cube::checkbox;
  use iced_cube::primitives::checkbox::CheckState;
related: [switch, radio]
hero: checkbox/states
stories: [checkbox/default, checkbox/states, checkbox/select-all]
api:
  - name: "checkbox(state)"
    description: "Creates a checkbox from a bool or a CheckState. It renders disabled until it has a message."
  - name: ".label(text)"
    description: "Adds a label after the box. Clicking the label toggles the checkbox too."
  - name: ".on_toggle(f) / .on_toggle_maybe(option)"
    description: "Sets the message for a click. The closure gets the new checked value, so an indeterminate box reports true. None disables the checkbox."
  - name: "CheckState"
    description: "Unchecked, Checked or Indeterminate. toggled() gives the state after a click."
  - name: "checkbox::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>, for apps that track a current checkbox."
  - name: "Action::Toggle.apply(state)"
    description: "The checked value a click would send. Pass it to your on_toggle message."
---

Use a checkbox for independent options that take effect when a form is submitted. For a setting that applies straight away, a switch reads better.

The indeterminate state is for a parent box whose children are partly selected. The app works out the parent's state from its children, as the select-all example shows, and a click on an indeterminate box always checks it.
