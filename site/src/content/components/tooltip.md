---
title: Tooltip
description: A short label that appears while the pointer rests on an element.
group: Overlays
order: 1
module: overlay::tooltip
imports: |
  use iced_cube::tooltip;
  use iced_cube::overlay::tooltip::Position;
keywords: [hint]
related: [button]
hero: tooltip/toolbar
stories: [tooltip/toolbar, tooltip/positions]
api:
  - name: "tooltip(content, label)"
    description: "Shows the label in a bubble while the pointer is over the content."
  - name: ".position(Position)"
    description: "Top, Bottom, Left, Right or FollowCursor. Defaults to Top."
  - name: ".delay(duration)"
    description: "How long the pointer has to rest on the element before the bubble appears. Defaults to zero, so it appears straight away."
  - name: "style(tokens)"
    description: "The bubble style, for reuse in custom overlays."
---

Use a tooltip to name an icon-only button or to add a short hint. Keep the label to a few words, and never put anything in it that people need to finish a task, because touch screens have no hover.

The bubble uses the primary colours, so it reads as dark on a light page and light on a dark one. It moves to stay inside the window when there is no room on the requested side.
