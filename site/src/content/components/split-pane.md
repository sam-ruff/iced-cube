---
title: Split pane
description: Two views, a start and an end, with one handle between them and a ratio that says how the space is shared.
group: Layout
order: 7
module: layout::split_pane
imports: |
  use iced_cube::layout::split_pane::{Axis, Event, Extent, State, split_pane};
keywords: [splitter, divider, two panes]
related: [resizable-panel]
hero: split-pane/editor-preview
stories: [split-pane/editor-preview, split-pane/vertical]
api:
  - name: "State::new(ratio)"
    description: "A split with the start view taking the given share, from 0.0 to 1.0. Out of range values are clamped. State::default() is an even split."
  - name: ".min(start, end) / .max(start, end)"
    description: "The smallest and largest each view may get, as Extent::Fraction or Extent::Pixels."
  - name: ".collapsible(start, end)"
    description: "Lets either view snap shut when dragged below its threshold."
  - name: "State::with_panels(start, end)"
    description: "Builds the split from two resizable panel settings, for anything the shortcuts above do not cover."
  - name: ".with_ratio(ratio) / state.set_ratio(ratio)"
    description: "Restores a saved ratio, fitted to the limits."
  - name: "state.ratio()"
    description: "The start view's share, ready to save."
  - name: "state.update(Event)"
    description: "Applies the events the view sends. Returns Output::Resized, Collapsed or Expanded."
  - name: "state.is_start_collapsed() / state.is_end_collapsed()"
    description: "Whether either view is collapsed."
  - name: "split_pane(&state, start, end)"
    description: "Shows the two views with a handle between them. The handle does nothing until .on_event is set."
  - name: ".axis(Axis) / .stack_below(width)"
    description: "Side by side by default, or stacked with Axis::Vertical. stack_below stacks a horizontal split on narrow screens."
  - name: ".grip(bool)"
    description: "Shows a grip on the handle. On by default."
  - name: ".id(id) / .keymap(Keymap<Action>)"
    description: "The handle's id for focusing it, and the shortcuts it resolves while focused."
---

A split pane is a resizable panel group with exactly two panels, so it is a thin wrapper rather than a widget of its own. The state wraps the group's state and adds the ratio, and the view is the same widget. Dragging, keyboard resizing, collapsing and stacking on small screens all behave exactly as they do for resizable panels, and the events, keymap and actions are the same types.

Use a split pane when there are two views and the ratio is all you need to keep. For three or more panels, or for a sidebar with a pixel width beside a flexible page, use a resizable panel.
