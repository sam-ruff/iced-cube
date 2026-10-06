---
title: Toggle
description: A button that stays pressed while an option is on.
group: Actions
order: 3
module: primitives::toggle
imports: |
  use iced_cube::{lucide, toggle};
  use iced_cube::primitives::toggle::{Size, Variant};
keywords: [toggle button, pressed, on off]
related: [toggle-group, icon-button, switch]
hero: toggle/default
stories: [toggle/default, toggle/variants, toggle/keyboard]
api:
  - name: "toggle(label)"
    description: "Creates a toggle with a text label, switched off. It renders disabled until it has a message."
  - name: ".pressed(bool)"
    description: "Whether the toggle is on. Pass the value your app holds."
  - name: ".on_toggle(f)"
    description: "Sets the message for a click. The closure gets the new pressed state."
  - name: ".icon(glyph)"
    description: "Adds a Lucide icon before the label."
  - name: ".variant(Variant)"
    description: "Default, transparent until hovered, or Outline, with a border. Defaults to Default."
  - name: ".size(Size)"
    description: "Sm, Md or Lg: 32, 36 or 40 pixels tall. Defaults to Md."
  - name: "toggle::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Give each toggle its own chord by clearing the map and binding one."
  - name: "Action::Toggle.apply(pressed)"
    description: "The pressed state a click would send. Pass it to your on_toggle message."
  - name: "colours(tokens, pressed, status) / style(tokens, variant, pressed, status)"
    description: "The resolved colours and iced style, for reuse in custom widgets."
---

A toggle is a button with an on state, for options that sit among other controls rather than in a form: showing a grid, pinning a panel, bold text. The app holds the bool, passes it to `.pressed`, and flips it in the `on_toggle` message, as with a switch.

Use a [switch](../switch/) for a setting that takes effect in a settings list, and a toggle where the option lives next to the content it changes. For an option with only an icon, such as bold in a compact toolbar, use a pressed [icon button](../icon-button/), which shows its label as a tooltip.

Shortcuts are app-wide, so each toggle needs its own chord. The keyboard example clears the default Space binding and binds Alt+B and Alt+I instead.
