---
title: Radio
description: Picks exactly one option from a small, visible set.
group: Forms
order: 4
module: primitives::radio
imports: |
  use iced_cube::{radio, radio_group};
  use iced_cube::primitives::radio::Direction;
related: [select, checkbox]
hero: radio/default
stories: [radio/default, radio/horizontal, radio/disabled]
api:
  - name: "radio_group(options, selected)"
    description: "Creates one radio per option, labelled with its Display text. It renders disabled until it has a message."
  - name: ".on_select(f)"
    description: "Sets the message emitted with the option that was clicked."
  - name: ".direction(Direction)"
    description: "Vertical or Horizontal. Defaults to Vertical. Horizontal groups wrap when space runs out."
  - name: "radio(label, value, selected)"
    description: "Creates a single radio for building custom layouts. It is selected when selected holds its value."
  - name: "radio(...).on_select(f)"
    description: "Sets the message for a click, built from the radio's value. Without it the radio is disabled."
---

Use radios when there are only a handful of options and seeing them all at once helps the choice. For longer lists, a select takes less room.

Options are usually a small `Copy` enum with a `Display` implementation. Clicking a radio or its label emits its value; the app stores it and passes it back as `selected`.
