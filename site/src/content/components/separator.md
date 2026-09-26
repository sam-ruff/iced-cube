---
title: Separator
description: A thin line that divides related groups of content, with optional inline text.
group: Layout
order: 3
module: primitives::separator
imports: |
  use iced_cube::{separator, vertical_separator};
  use iced_cube::primitives::separator::Orientation;
keywords: [divider, rule]
related: [stack]
hero: separator/with-label
stories: [separator/orientation, separator/with-label]
api:
  - name: "separator()"
    description: "Creates a horizontal line that fills the available width."
  - name: "vertical_separator()"
    description: "Creates a vertical line that fills the available height."
  - name: ".orientation(Orientation)"
    description: "Horizontal or Vertical. Defaults to Horizontal."
  - name: ".label(text)"
    description: "Shows short muted text, such as \"or\", in the middle of the line."
---

Separators are one pixel wide and take their colour from the theme's border token. Space around them comes from the parent's `spacing`.

A vertical separator fills the height of its row, so give the row a height (or let taller siblings set it) when the separator sits between short items such as links.
