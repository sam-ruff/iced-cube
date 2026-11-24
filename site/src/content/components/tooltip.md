---
title: Tooltip
description: A short label that appears while the pointer rests on an element or while it has keyboard focus.
group: Overlays
order: 1
module: overlay::tooltip
imports: |
  use iced_cube::tooltip;
  use iced_cube::overlay::tooltip::Position;
keywords: [hint, help text]
related: [button, input]
hero: tooltip/toolbar
stories: [tooltip/toolbar, tooltip/positions, tooltip/hints]
api:
  - name: "tooltip(content, label)"
    description: "Shows the label in a bubble while the pointer is over the content, or while something inside it has keyboard focus."
  - name: ".position(Position)"
    description: "Top, Bottom, Left, Right or FollowCursor. Defaults to Top. A side with no room in the window gives way to the opposite one, so a bubble near an edge never covers its own control. FollowCursor sits on top while the bubble shows for focus. Icon buttons take it through .tooltip(Some(Position::Left))."
  - name: ".delay(duration)"
    description: "How long the pointer has to rest on the element before the bubble appears. Defaults to zero, so it appears straight away. Keyboard focus always shows it straight away."
  - name: ".max_width(pixels)"
    description: "The widest the bubble grows. A longer label wraps onto more lines. Without it the bubble is as wide as the label, up to the window's width."
  - name: ".text_size(pixels)"
    description: "The label's size. By default a label of up to LONG_LABEL (40) characters is 12 pixels and a longer one 14, as label_size(label, None) works out."
  - name: "style(tokens)"
    description: "The bubble style, for reuse in custom overlays."
  - name: "Focus::next(focused, pointer_press)"
    description: "How the tooltip tracks focus: focus that comes with a pointer press does not show the bubble, any other focus does."
---

Use a tooltip to name an icon-only button or to add a hint. Keep it to a few words or a short sentence, and never put anything in it that people need to finish a task, because touch screens have no hover. A finger held on the element shows the bubble, and it closes when the finger lifts.

For a sentence, set `.max_width(...)` so the bubble wraps rather than stretching across the window. Longer labels read a size up on their own; set `.text_size(...)` to choose.

The bubble also shows while something inside the element has keyboard focus, such as a text field reached with Tab. iced 0.14 has no general focus model, so this works for widgets that report focus: text fields, text editors and the focusable parts of this library, but not plain buttons. Focus from a click does not show the bubble, so it does not cover a field while people type into it.

The bubble uses the primary colours, so it reads as dark on a light page and light on a dark one. It moves to stay inside the window when there is no room on the requested side. While it shows, its text can be found with `Simulator::find` in tests like any other text.
