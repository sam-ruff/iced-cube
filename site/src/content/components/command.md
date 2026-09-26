---
title: Command
description: A searchable list of actions, grouped, with icons and shortcut hints.
group: Navigation
order: 2
module: navigation::command
imports: |
  use iced_cube::navigation::command::{self, Output, State, command, group, item};
keywords: [command menu, launcher, fuzzy search, quick open]
related: [combobox, tabs]
hero: command/inline
stories: [command/inline, command/keyboard, command/async-search]
api:
  - name: "State::new(groups)"
    description: "Holds the groups of items and highlights the first enabled one."
  - name: "group(label, items)"
    description: "Items under a heading. An empty label shows no heading."
  - name: "item(id, label)"
    description: "Creates an item. Add .icon(glyph), .shortcut(text), .keywords([...]), .disabled(true) or .destructive(true)."
  - name: ".with_visible_rows(n)"
    description: "How many rows show at once (eight by default). The window follows the highlight and scrolls with the wheel."
  - name: "state.update(Event)"
    description: "Handles typing, moving, running, clearing and results from the channel. Returns an Output."
  - name: "Output"
    description: "Ready(sender) once the channel is open, Search(query) when the query changes, Run(id) when an item is chosen, and Dismiss on Escape with an empty query."
  - name: "state.results() / state.highlighted() / state.query()"
    description: "The ranked results with their group labels, the highlighted item and the current query."
  - name: "command::score(query, text)"
    description: "The matcher: case-insensitive, in-order letters, preferring exact matches, prefixes and word starts. None when there is no match."
  - name: "command::subscription()"
    description: "Owns the results channel. Emits Ready(sender) once, then batches of up to BATCH_SIZE (32)."
  - name: "command::results(query, group, items)"
    description: "What a producer sends. Results for any query but the current one are dropped, and the rest are shown in arrival order under their group."
  - name: "command(&state)"
    description: "Renders the search field and results. It renders disabled until .on_event is set."
  - name: ".on_event(f) / .placeholder(text) / .empty(text)"
    description: "Maps events to your message, and sets the field and empty-state text."
  - name: ".loading(bool)"
    description: "Shows \"Searching...\" while a background search is running."
  - name: ".width(length) / .height(length) / .id(id)"
    description: "The width fills by default. A fixed height stops the list resizing as results change."
  - name: ".keymap(keymap)"
    description: "Replaces the default keymap. The list resolves its own key presses while its search field has focus."
  - name: "command::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.event(&state)"
    description: "Turns an Action into the Event it sends, or None when it would do nothing, for keys the app routes itself."
---

A command list puts every action behind one search field. Typing ranks the items: an exact match comes first, then prefixes, then matches at the start of a word, then anywhere, then letters in order. Keywords let an item be found by other names, such as "preferences" for Settings. Disabled items stay visible but are skipped by the keyboard.

The list is embedded where you place it. It has no overlay of its own, so a modal palette can wrap it later.

Results can also come from background work, such as a file index or a server. When `update` returns `Output::Search(query)`, start a producer with that query and a clone of the sender. It sends `command::results(query, group, items)` as matches arrive; anything tagged with an older query is dropped, so slow searches never overwrite newer ones. The async example waits with `futures-timer` rather than a thread, so it runs in the browser too.

While the search field has focus, the list handles its keys itself. Home and End then move the highlight rather than the text cursor. Escape clears the query, and a second press returns `Output::Dismiss`.
