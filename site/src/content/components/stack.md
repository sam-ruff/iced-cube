---
title: Stack
description: Vertical and horizontal stacks that take their spacing from the theme's scale.
group: Layout
order: 2
module: layout::stack
imports: |
  use iced_cube::{hstack, vstack};
  use iced_cube::layout::stack::Gap;
keywords: [column, row]
related: [card, separator]
hero: stack/gaps
stories: [stack/gaps, stack/alignment, stack/wrap]
api:
  - name: "vstack(children) / hstack(children)"
    description: "Stacks elements top to bottom or left to right."
  - name: ".push(element)"
    description: "Adds a child at the end."
  - name: ".gap(Gap)"
    description: "None, Xs, Sm, Md, Lg or Xl. Defaults to Md."
  - name: ".align(Alignment)"
    description: "Aligns children across the stack: horizontally in a vstack, vertically in an hstack."
  - name: ".wrap()"
    description: "Moves children onto a new line when they run out of room."
  - name: ".padding(padding) / .width(length) / .height(length)"
    description: "Sizing, as on any iced column or row."
  - name: "Gap::pixels()"
    description: "The gap in pixels: 0, 4, 8, 12, 16 or 24."
  - name: ".len() / .is_empty()"
    description: "How many children the stack has."
---

Stacks are a thin layer over iced's `column` and `row`. The only difference is that spacing comes from a fixed scale, which keeps gaps consistent across a whole app.

Reach for a plain `column` or `row` whenever you need something the stacks do not expose.
