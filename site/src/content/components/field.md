---
title: Field and label
description: Puts a label above any form control, with helper text or an error message below it.
group: Forms
order: 8
module: forms::label
imports: |
  use iced_cube::{field, label};
  use iced_cube::forms::label::State;
keywords: [form field, field, label]
related: [input, textarea, select]
hero: label/field
stories: [label/field, label/validation]
api:
  - name: "field(label, control)"
    description: "Stacks a label above any control, such as an input, textarea or select."
  - name: ".description(text)"
    description: "Adds muted helper text below the control."
  - name: ".error(text) / .error_maybe(option)"
    description: "Shows an error below the control and colours the label to match. None clears it."
  - name: ".required(bool)"
    description: "Adds a required marker after the label."
  - name: ".disabled(bool)"
    description: "Mutes the label to match a disabled control."
  - name: ".width(length)"
    description: "Overrides the width of the whole field. Fields fill the available width by default."
  - name: "label(text)"
    description: "A standalone label, with .required(bool), .disabled(bool), .invalid(bool) and .state(State)."
  - name: "State"
    description: "Normal, Disabled or Invalid. Sets the label's colour."
---

Use `field` for every form control so labels, descriptions and errors line up the same way across your app. The control keeps its own messages; `field` only lays it out.

When a field has an error, also mark the control itself as invalid (for example `input(...).invalid(true)`) so its border matches the message.

Clicking a label does not focus its control, and iced does not yet expose labels to screen readers, so keep label text short and visible next to the control it names.
