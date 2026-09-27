---
title: Resizable panel
description: Two or more panels in a row or column, separated by handles that resize them.
group: Layout
order: 6
module: layout::resizable_panel
imports: |
  use iced_cube::layout::resizable_panel::{self, Event, Extent, State, panel, resizable_panel};
keywords: [splitter, panes, resize, drag handle, panel group]
related: [split-pane, sidebar, card]
hero: resizable-panel/horizontal
stories: [resizable-panel/horizontal, resizable-panel/vertical, resizable-panel/nested, resizable-panel/collapsible, resizable-panel/keyboard]
api:
  - name: "panel(Extent)"
    description: "One panel's constraints, starting at a default size given as Extent::Fraction or Extent::Pixels."
  - name: ".min(Extent) / .max(Extent)"
    description: "The smallest and largest the panel may get. The maximum is never below the minimum."
  - name: ".collapsible(bool) / .collapsed_size(Extent) / .collapse_below(Extent)"
    description: "Lets the panel snap shut. It collapses to its collapsed size, 0 by default, when dragged below the threshold, which defaults to halfway between that size and the minimum."
  - name: ".collapsed(bool)"
    description: "Starts a collapsible panel collapsed."
  - name: "State::new(panels)"
    description: "Holds the panels and their sizes. Sizes are worked out when the group first measures itself."
  - name: ".with_sizes(sizes) / state.set_sizes(sizes)"
    description: "Restores saved sizes. They are scaled to add up to 1 and fitted to the limits."
  - name: ".step(Extent) / .large_step(Extent)"
    description: "How far the arrow keys and Shift with an arrow move a handle. Default 5% and 20%."
  - name: "state.update(Event)"
    description: "Applies a drag, a keyboard step, Collapse, Expand, Toggle or Reset. Returns Output::Resized, Collapsed(i) or Expanded(i) when something changed."
  - name: "state.sizes()"
    description: "Each panel's share of the space, adding up to 1, ready to save."
  - name: "state.resolve(total) / state.is_collapsed(i) / state.dragging()"
    description: "The sizes in pixels for a length, whether a panel is collapsed, and the handle being dragged."
  - name: "resizable_panel(&state, panes)"
    description: "Lays out one element per panel with handles between them. The handles do nothing until .on_event is set."
  - name: ".axis(Axis)"
    description: "Horizontal for a row, the default, or Vertical for a column."
  - name: ".stack_below(width)"
    description: "Turns a horizontal group into a column while it is narrower than the given width."
  - name: ".grip(bool)"
    description: "Draws a grip on each handle."
  - name: ".id(id)"
    description: "Gives the first handle an id, so widget::operation::focus can focus it."
  - name: ".keymap(Keymap<Action>) / resizable_panel::default_keymap()"
    description: "The shortcuts a focused handle resolves. Bind, unbind or clear chords to change them."
  - name: "action.event(&state, handle)"
    description: "The Event an Action sends for a handle, for apps that route keys themselves."
  - name: "resizable_panel::resize / fit / defaults"
    description: "The pure size maths the state uses, for tests or custom layouts."
---

The app owns the sizes. They live in `State` as fractions of the space the panels share, so saving `state.sizes()` and passing it back to `with_sizes` restores the layout at any window size. Pixel limits are applied against the current size every time the group is laid out.

Dragging a handle moves the panels on either side of it. When the panel that is shrinking reaches its minimum, the panels beyond it shrink too. A collapsible panel snaps shut once it is dragged below its threshold and snaps open again, at its minimum, once it is dragged back past it. `Event::Toggle` collapses a panel or brings it back to the size it had before.

The group sends `Event::Measure` when its size changes, followed by the drag events. Pass them all to `state.update`.

## Keyboard focus

iced 0.14 has no general focus model, but widgets can take part in its focus operations. Each handle is focusable: pressing it focuses it, just as a text field is focused, and `iced::widget::operation::focus(id)` or `focus_next()` reach it too. While a handle has focus the group resolves its own keymap and captures the keys it uses, so no subscription is needed. Escape or a press anywhere else takes the focus away.

## Small screens

When the minimums do not fit, collapsible panels collapse, starting with the last one, and if that is not enough every panel shrinks in proportion. For a row that would be too cramped on a phone, `.stack_below(480.0)` turns it into a column below that width, keeping the same shares.
