---
title: Dropdown menu
description: A menu of actions that opens from a button, with check marks, radio choices and submenus.
group: Overlays
order: 4
module: overlay::dropdown_menu
imports: |
  use iced_cube::dropdown_menu::{
      self, Event, Output, State, checkbox_item, dropdown_menu, group_label, item,
      radio_item, separator, submenu,
  };
keywords: [menu, actions, overflow menu, more]
related: [context-menu, popover, select]
hero: dropdown-menu/default
stories: [dropdown-menu/default, dropdown-menu/checkboxes, dropdown-menu/radio-items, dropdown-menu/submenu, dropdown-menu/keyboard]
api:
  - name: "State::new(entries)"
    description: "Holds the entries, closed. Item ids must be unique, including those in submenus."
  - name: "item(id, label)"
    description: "An action. Add .icon(glyph), .shortcut(\"Ctrl+S\"), .disabled(true) or .destructive(true)."
  - name: "checkbox_item(id, label, checked)"
    description: "An item with a check mark that toggles when chosen."
  - name: "radio_item(id, label, checked)"
    description: "One choice among the radio items next to it. Choosing it unchecks the rest of that run."
  - name: "submenu(id, label, entries)"
    description: "Opens a nested menu to its right, on hover, click, Enter or the right arrow key."
  - name: "group_label(text) / separator()"
    description: "A heading for the items below it, and a line between groups."
  - name: "state.update(Event)"
    description: "Handles Open, Close, Toggle, Highlight, Next, Previous, First, Last, OpenSubmenu, CloseSubmenu, Activate, ActivateHighlighted and Typeahead. Returns Output::Activated, Toggled or Selected when an item is chosen, and closes the menu."
  - name: "state.is_open() / state.highlighted() / state.depth()"
    description: "Whether the menu is open, the highlighted item's id, and how many menus are open including submenus."
  - name: "state.item(id) / state.is_checked(id)"
    description: "An item by id, and whether a checkbox or radio item is checked."
  - name: "state.set_checked(id, bool) / state.set_disabled(id, bool)"
    description: "Changes an item from your own state, for example when a setting changes elsewhere."
  - name: "dropdown_menu(&state, trigger)"
    description: "Renders the trigger with the menu below it while it is open. Items render disabled until .on_event is set."
  - name: ".on_event(f)"
    description: "Maps menu events to your message. The trigger sends Event::Toggle itself."
  - name: ".side(Side) / .align(Align) / .gap(f32)"
    description: "Where the menu opens. Defaults to below the trigger, lined up with its left edge."
  - name: ".width(f32)"
    description: "Width of the menu and its submenus. Defaults to 224."
  - name: "row_style(tokens, RowStatus, destructive)"
    description: "The colours of a row when idle, highlighted or disabled, for reuse in custom menus."
  - name: "dropdown_menu::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "state.key_event(&keymap, &key)"
    description: "Resolves a key press through the keymap and, while the menu is open, turns letters and digits into typeahead."
  - name: "action.event(&state)"
    description: "Turns a resolved Action into the Event to pass to state.update, or None when it does nothing."
---

The app owns a `dropdown_menu::State` and routes its events into it. When `update` returns an `Output`, the menu has already closed and updated its own check marks, so the app only acts on the choice. Read checkbox and radio values back with `state.is_checked(id)`, or keep your own copy and call `set_checked` when it changes elsewhere.

The trigger is any element you pass in, usually a button whose message carries `Event::Toggle`. Escape and a click outside send `Event::Close` through `on_event`, and Escape inside a submenu closes just that submenu. Clicks inside the menu never reach the widgets underneath it.

Moving the pointer over an item highlights it, and the keyboard moves the same highlight. The menu does not listen to the keyboard itself: the app subscribes with `keys::subscription()` and passes each press to `state.key_event`, as the keyboard example does. Because shortcuts are app-wide, the arrow keys, Enter and Space open a closed menu; route keys to the menu only while it is the one in use if other parts of your app use them.

Shortcut hints are only labels. Bind the keys themselves in your own keymap. The [context menu](../context-menu/) uses the same entries and rows, opened with a right-click.
