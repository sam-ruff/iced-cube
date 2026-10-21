---
title: Breadcrumb
description: Shows where the current view sits in a hierarchy, with a link to each level above it.
group: Navigation
order: 3
module: navigation::breadcrumb
imports: |
  use iced_cube::{breadcrumb, crumb, lucide};
  use iced_cube::navigation::breadcrumb::Separator;
keywords: [path, trail, hierarchy]
related: [tabs, pagination]
hero: breadcrumb/default
stories: [breadcrumb/default, breadcrumb/collapsed, breadcrumb/keyboard]
api:
  - name: "breadcrumb(crumbs)"
    description: "Creates a trail from the root to the current view. The last crumb is the current view and never responds to clicks."
  - name: "crumb(label)"
    description: "Creates a crumb. It is plain text until it has a message."
  - name: ".on_press(message) / .on_press_maybe(option)"
    description: "Makes a crumb a link that sends the message."
  - name: ".icon(glyph)"
    description: "Adds a Lucide icon before a crumb's label, such as a house for the root."
  - name: ".separator(Separator)"
    description: "Chevron or Slash between crumbs. Defaults to Chevron."
  - name: ".max_items(n)"
    description: "Shows at most n crumbs: the first and the last n - 1, with an ellipsis for the rest. n is at least 2."
  - name: ".on_expand(message)"
    description: "Makes the ellipsis a button that sends the message, such as one that shows the whole trail."
  - name: "breadcrumb::slots(len, max)"
    description: "The crumbs and ellipsis a trail of len shows for a limit, for custom layouts."
  - name: "breadcrumb::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.target(len)"
    description: "The index of the crumb a resolved Action (Parent or Root) goes to in a trail of len, or None at the top."
---

The app holds the path and builds the trail from it on every view, giving each level above the current one a message that opens it. The breadcrumb only draws the trail, so the levels can be folders, settings sections or records.

On a narrow screen the trail wraps onto a second line rather than squeezing its labels. For deep hierarchies, `.max_items` keeps the root and the levels nearest the current view, which are the ones people go back to most, and `.on_expand` lets them see the rest.

The keymap's actions return an index rather than an event, because the app owns the path: `action.target(len)` says which crumb to open, as the keyboard example shows.
