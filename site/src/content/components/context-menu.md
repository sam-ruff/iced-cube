---
title: Context menu
description: A menu that opens at the pointer when you right-click an area.
group: Overlays
order: 5
module: overlay::context_menu
imports: |
  use iced_cube::context_menu::{self, Event, State, context_menu};
  use iced_cube::dropdown_menu::{Output, checkbox_item, item, separator, submenu};
keywords: [right click, right-click menu, shortcut menu]
related: [dropdown-menu, popover]
hero: context-menu/default
stories: [context-menu/default, context-menu/keyboard]
api:
  - name: "State::new(entries)"
    description: "Holds the entries, closed. Build them with the dropdown menu's item, checkbox_item, radio_item, submenu, group_label and separator."
  - name: "state.update(Event)"
    description: "Open(point) opens at a right-click, OpenFromKeyboard opens at the area's top left corner on the first item, and Menu(event) navigates as a dropdown menu does. Returns the dropdown menu's Output when an item is chosen."
  - name: "state.is_open() / state.position() / state.menu()"
    description: "Whether the menu is open, where it opened relative to the area, and the underlying dropdown menu state."
  - name: "state.is_checked(id) / state.set_checked(id, bool) / state.set_disabled(id, bool)"
    description: "Reads and changes items, as on a dropdown menu."
  - name: "context_menu(&state, content)"
    description: "Makes the content a right-click area. Right-clicks do nothing until .on_event is set."
  - name: ".on_event(f)"
    description: "Maps context menu events to your message."
  - name: ".width(f32)"
    description: "Width of the menu and its submenus. Defaults to 224."
  - name: ".keymap(keymap)"
    description: "Replaces the default keymap the open menu resolves its keys with."
  - name: "context_menu::default_keymap()"
    description: "The dropdown menu's shortcuts plus Open. Bind, unbind or clear chords to change them."
  - name: "state.key_event(&keymap, &key)"
    description: "Resolves a key press through the keymap and, while the menu is open, turns letters and digits into typeahead."
  - name: "action.event(&state)"
    description: "Turns a resolved Action into the Event to pass to state.update. Open only works while the menu is closed, and the others only while it is open."
---

A context menu offers actions for whatever is under the pointer: a file in a list, a selection in an editor, a node on a canvas. It shares its entries, rows and navigation with the [dropdown menu](../dropdown-menu/), so everything on that page applies here too.

A right-click inside the area opens the menu with its top left corner at the pointer. It flips up or to the left near the edges of the window. Right-clicking somewhere else in the area moves the menu there, and a left-click anywhere outside the menu closes it. Clicks inside the menu never reach the area underneath.

Keyboard users open it with Shift+F10 or the Menu key, as in desktop apps: route presses from `keys::subscription()` through `state.key_event`, as the keyboard example does. The menu then opens at the area's top left corner with its first item highlighted, and handles the arrow keys, Enter and Escape itself until it closes. While it is closed, only the opening chords do anything, so a context menu shares the keyboard with the rest of your app.
