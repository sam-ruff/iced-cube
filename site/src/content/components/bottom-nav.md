---
title: Bottom navigation
description: A bar of three to five destinations along the bottom of a phone screen.
group: Navigation
order: 5
module: navigation::bottom_nav
imports: |
  use iced_cube::{bottom_nav, lucide};
  use iced_cube::navigation::bottom_nav::{self, Action};
keywords: [tab bar, navigation bar, mobile, phone]
related: [tabs, sidebar, toast]
hero: bottom-nav/default
stories: [bottom-nav/default, bottom-nav/keyboard]
api:
  - name: "bottom_nav(active)"
    description: "Creates a bar with active as the current destination. Any Copy + PartialEq type works as the id."
  - name: ".item(id, glyph, label)"
    description: "Adds a destination: an icon above a short label. Use three to five; past five the rest are not shown."
  - name: ".on_select(f)"
    description: "Maps the chosen id to your message. It is sent for the active destination too. Without it the destinations are faded and do nothing."
  - name: ".inset(bottom)"
    description: "Extra space under the destinations, such as the height of a system navigation bar drawn over the window. Defaults to 0."
  - name: "bottom_nav::HEIGHT"
    description: "The bar's height above the inset: 80 pixels, its top border included."
  - name: "bottom_nav::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.apply(&ids)"
    description: "The id a resolved Action::Go(index) selects from your list of destinations, or None when there is no destination at that index."
---

The bar shares its width equally between its destinations, so on a phone each one is a wide touch target. The active destination's icon sits on a pill with its label in full strength; the others are muted. Put the bar at the bottom of a `column` under the screen it switches, and keep its id in your app state.

Selecting the destination that is already active still sends `on_select`. Apps use that to go back to the top of the screen, such as scrolling a list to its start or leaving a detail view.

Each destination responds to a tap without capturing the finger, as a [tappable](../tappable/) does, and shows a light wash while pressed. On a wide window the same bar works with the mouse; a [sidebar](../sidebar/) is usually the better choice there.

When a system bar is drawn over the bottom of the window, pass its height to `.inset(...)` so the labels stay clear of it. Raise the bottom inset of [toasts](../toast/) by `HEIGHT` plus the same amount to keep a snackbar above the bar.

The default shortcuts are Alt and a number, because Ctrl and a number belong to [document tabs](../document-tabs/). Some browsers keep Alt and a number for their own tabs, so a web build may never receive them; rebind them there.
