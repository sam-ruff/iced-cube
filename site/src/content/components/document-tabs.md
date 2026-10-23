---
title: Document tabs
description: A strip of open documents above an editor, with unsaved changes, pinning, reordering and a menu for tabs that do not fit.
group: Navigation
order: 4
module: navigation::document_tabs
imports: |
  use iced_cube::navigation::document_tabs::{self, Event, MenuItem, Output, State, document, document_tabs};
keywords: [editor tabs, file tabs, open files, tab strip, tab bar, workspace tabs, dirty, unsaved, pinned tabs, sticky tabs]
related: [tabs, context-menu, dropdown-menu, dialog]
hero: document-tabs/editor
stories: [document-tabs/editor, document-tabs/overflow, document-tabs/confirm-close, document-tabs/keyboard]
api:
  - name: "State::new(documents)"
    description: "Opens the documents, pinned ones first, with the first one active. Ids must be unique."
  - name: "document(id, label)"
    description: "A saved, unpinned document. Add .icon(glyph), .modified(true) or .pinned(true)."
  - name: ".closable(false)"
    description: "A document the user cannot close: no close button, no Close item, left out of Close others, Close to the right, Close saved and Close all, and Ctrl+W does nothing on it."
  - name: ".locked(true)"
    description: "Keeps a document pinned or unpinned as it is: its menu has no Pin or Unpin item and Event::Pin leaves it alone. It still moves within its group."
  - name: ".with_active(id) / .with_menu_items(entries)"
    description: "The document to start on, and app items added to every tab's context menu, built with dropdown_menu::item(MenuItem::Custom(\"name\"), label)."
  - name: "state.update(Event)"
    description: "Activates, walks recent documents, moves, pins and drives both menus. Returns Output::Activated(id), CloseRequested(ids), Moved { id, from, to }, Pinned(id, bool), NewTab or Custom(name, id)."
  - name: "state.close(&ids)"
    description: "Closes documents. When the active one closes, the most recently used one left becomes active and is returned."
  - name: "state.open(document)"
    description: "Activates a document, adding it at the end of its group when it is not open yet."
  - name: "state.set_modified(id, bool) / state.set_label(id, label)"
    description: "Marks unsaved changes, or renames a tab."
  - name: "state.documents() / state.document(id) / state.active() / state.recent()"
    description: "The documents in strip order, one document, the active id, and the ids most recently used first."
  - name: "document_tabs(&state)"
    description: "Renders the strip. Tabs render disabled until .on_event is set."
  - name: ".new_tab(bool)"
    description: "Shows a new tab button after the last tab, and lets a double click on the empty strip ask for a new document."
  - name: ".max_label_width(f32)"
    description: "The widest a label grows before it ends in an ellipsis. Defaults to 200 pixels."
  - name: ".context_menu_keymap(keymap) / .overflow_keymap(keymap)"
    description: "Replaces the default keymap of the open tab context menu or of the list of every tab."
  - name: "document_tabs::cycle_end()"
    description: "A subscription that reports when every modifier is released, which ends a walk through recent documents. Map it with Event::from."
  - name: "document_tabs::default_keymap() / action.event(&state)"
    description: "The default shortcuts, and the Event a resolved Action sends."
  - name: "document_tabs::tab_style(tokens, active, hovered) / strip_style(tokens)"
    description: "The colours of one tab and of the strip behind it."
---

Document tabs are the strip of open files above an editor. The app owns a `document_tabs::State` with the documents in order and routes the strip's events into it. The strip only switches between documents: the app renders the active one below it, and the active tab takes the page background so it joins that content.

Closing never happens by itself. The close button, a middle click, `Ctrl+W` and the context menu all return `Output::CloseRequested` with the ids to close, so the app can save, or ask with an alert dialog when a document has unsaved changes, before it calls `state.close(&ids)`. The confirmation example shows the pattern.

Each tab reserves a slot after its label. The active tab shows its close button there, other tabs show it on hover, and a document with unsaved changes shows a dot that turns into the close button under the pointer, so the label never moves. Pinned tabs sit before the others, show only their icon (or a short label when they have none), have no close button, and are left alone by Close others, Close to the right, Close saved and Close all. Dragging a tab moves it, with a line marking where it will land, and a pinned tab only moves among pinned tabs.

A tab's context menu only lists what would do something on it. Close saved, which closes the tabs without unsaved changes, appears only while some of those tabs have unsaved changes and some do not. For a document the app always keeps open, such as a list the other tabs are opened from, use `.pinned(true).closable(false).locked(true)`: it then has no close button, no Close, Pin or Unpin item, and no close gesture or shortcut asks to close it.

When the tabs do not fit, the pinned tabs stay at the start of the strip and only the others scroll sideways, with the wheel or a finger, in the room after them. A button at the end of the strip lists every tab. When pinned tabs would take more than half that room, they scroll with the others instead, so a narrow window always has room to show the active tab. The strip keeps the active tab in view when it first appears, when the active tab changes and when the window is resized. The list opens below its button, marks the active tab and scrolls to it. Labels stay on one line, in the strip and in the list, and end in an ellipsis when they do not fit. A tab's context menu opens below the tab, whether from a right-click, a long press or the keyboard, and scrolls when the window is too short for it.

`Ctrl+Tab` walks the documents in the order they were last used, as editors do: each press goes one further back until Ctrl is released, which the app learns from `document_tabs::cycle_end()`. `Ctrl+PageDown` and `Ctrl+PageUp` move through the strip in order. The segmented [tabs](../tabs/) also bind `Ctrl+Tab` and `Ctrl+Shift+Tab`, so an app showing both should unbind them from one of them, usually the segmented tabs. Browsers keep these chords for their own tabs, which is why the keyboard example adds chords such as `Ctrl+]` that reach the page.

`Action::OpenMenu`, which opens the active tab's context menu, has no default chord. The strip never has keyboard focus, so an app-wide `Shift+F10` would also open any other [context menu](../context-menu/) on screen. Bind it with `default_keymap().bind(Chord::named(Named::F10).shift(), Action::OpenMenu)` when the tab strip is the only context menu, or resolve it only while the editor has focus. See [Keyboard shortcuts](../../keyboard/) for how components share keys.
