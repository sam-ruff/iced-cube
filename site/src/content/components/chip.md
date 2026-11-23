---
title: Chip
description: A compact pill for a chosen value, such as a guest or a tag, that can be removed with its x.
group: Feedback
order: 6
module: feedback::chip
imports: |
  use iced_cube::chip;
  use iced_cube::feedback::chip::{Size, Variant};
keywords: [tag, token, pill, removable, filter chip]
related: [badge, multi-combobox, avatar]
hero: chip/removable
stories: [chip/removable, chip/variants, chip/leading]
api:
  - name: "chip(label)"
    description: "Creates a pill with a text label that never wraps."
  - name: ".on_remove(message) / .on_remove_maybe(Option)"
    description: "Draws a small x after the label that sends the message when pressed. Without it the chip has no x."
  - name: ".variant(Variant)"
    description: "The badge variants: Default, Secondary, Outline, Destructive, Success or Warning. Defaults to Secondary."
  - name: ".size(Size)"
    description: "Sm, which matches a badge, or Md (the default)."
  - name: ".icon(glyph)"
    description: "Adds a Lucide icon before the label."
  - name: ".leading(element)"
    description: "Adds an element before the label, such as a small avatar, against the chip's left edge. Replaces any icon."
  - name: ".id(key)"
    description: "Gives the x the widget id chip::remove_id(key), so tests can find and click it. Keys must be unique on screen."
  - name: "chip::style / chip::remove_style"
    description: "The chip and x styles for a variant, as pure functions of the theme tokens."
---

Chips show values someone has picked, such as the guests of a meeting or the labels on an issue, and let them take one back with the x. For a label that only reports a state, use a [badge](../badge/) instead; chips share its colours and its small size.

The label stays on one line however narrow the space, so put a row of chips in `row(...).wrap()` to let them move onto new lines. The x is a plain button, so the app decides what removing means: drop the value from its list in `update`. To pick values from a long list, the [multi-select combobox](../multi-combobox/) draws its chosen values as chips.

A medium chip is 26 pixels tall, so a small avatar (24 pixels) fits as its leading element.
