---
title: Context menu
description: A menu that opens at the pointer when you right-click an area.
group: Overlays
order: 5
module: overlay::context_menu
imports: |
  use iced_cube::context_menu::{self, Event, State, context_menu, keyed};
  use iced_cube::dropdown_menu::{Output, checkbox_item, item, separator, submenu};
keywords: [right click, right-click menu, shortcut menu, row menu]
related: [dropdown-menu, popover]
hero: context-menu/default
stories: [context-menu/default, context-menu/keyboard]
api:
  - name: "State::new(entries)"
    description: "Holds the entries, closed. Build them with the dropdown menu's item, checkbox_item, radio_item, submenu, group_label and separator. State<Id> serves one area; State<Id, Key> serves many areas told apart by Key."
  - name: "state.update(Event)"
    description: "Open(key, point) opens at a right-click on the area key, OpenFromKeyboard(key) opens below that area on the first item, and Menu(event) navigates as a dropdown menu does. Returns the dropdown menu's Output when an item is chosen. The key is () for a single area."
  - name: "state.is_open() / state.is_open_on(&key) / state.target()"
    description: "Whether the menu is open, whether it is open on one area, and the area it last opened on, which stays set after it closes so you know which row a chosen item belongs to."
  - name: "state.position() / state.menu()"
    description: "Where a right-click opened the menu relative to the area, or None when it opened from the keyboard, and the underlying dropdown menu state."
  - name: "state.is_checked(id) / state.set_checked(id, bool) / state.set_disabled(id, bool)"
    description: "Reads and changes items, as on a dropdown menu."
  - name: "context_menu(&state, content)"
    description: "Makes the content a right-click area. Right-clicks do nothing until .on_event is set."
  - name: "keyed(&state, key, content)"
    description: "Makes the content one of many areas sharing one menu, such as a row in a list. The menu shows on it only while open on key, and its open events carry key."
  - name: ".on_event(f)"
    description: "Maps context menu events to your message."
  - name: ".width(f32)"
    description: "Width of the menu and its submenus. Defaults to 224."
  - name: ".keymap(keymap)"
    description: "Replaces the default keymap the open menu resolves its keys with."
  - name: "context_menu::default_keymap()"
    description: "The dropdown menu's shortcuts plus Open. Bind, unbind or clear chords to change them."
  - name: "state.key_event(&keymap, &key, target)"
    description: "Resolves a key press through the keymap and, while the menu is open, turns letters and digits into typeahead. The opening chord opens the menu on target, such as the focused row, or () for a single area."
  - name: "action.event(&state, target)"
    description: "Turns a resolved Action into the Event to pass to state.update. Open only works while the menu is closed, and the others only while it is open."
---

A context menu offers actions for whatever is under the pointer: a file in a list, a selection in an editor, a node on a canvas. It shares its entries, rows and navigation with the [dropdown menu](../dropdown-menu/), so everything on that page applies here too.

A right-click inside the area opens the menu with its top left corner at the pointer. It flips up or to the left near the edges of the window. Right-clicking somewhere else in the area moves the menu there, and a left-click anywhere outside the menu closes it. Clicks inside the menu never reach the area underneath.

Keyboard users open it with Shift+F10 or the Menu key, as in desktop apps: route presses from `keys::subscription()` through `state.key_event`, as the second example does. The menu then opens just below the area, lined up with its start so it never covers the area's label, with its first item highlighted. It handles the arrow keys, Enter and Escape itself until it closes. While it is closed, only the opening chords do anything, so a context menu shares the keyboard with the rest of your app.

For a list, keep one `State<Id, Key>` for every row rather than one per row. Wrap each row with `keyed(&state, row_key, row)`, and the open events tell you which row they came from. Before an open event reaches `update`, you can enable or check items for that row. After an item is chosen, `state.target()` still holds the row, so you know what to act on. Pass the focused row as the target of `key_event`, so Shift+F10 opens the menu below it.
