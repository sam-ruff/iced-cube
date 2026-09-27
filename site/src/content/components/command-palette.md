---
title: Command palette
description: The app-wide launcher, opened with Ctrl+K, with recent commands and nested pages.
group: Application
order: 4
module: application::command_palette
imports: |
  use iced_cube::command_palette::{self, Event, Output, State, command_palette, page};
  use iced_cube::navigation::command::{group, item};
keywords: [launcher, quick open, ctrl+k, go to anything, spotlight]
related: [command, dialog, menubar]
hero: command-palette/pages
stories: [command-palette/pages, command-palette/async-search]
api:
  - name: "State::new(groups)"
    description: "A closed palette listing command groups on its first page."
  - name: ".with_page(page(id, title, groups))"
    description: "A nested list. Choosing the item with the page's id opens it instead of running it. Add .placeholder(text) to the page."
  - name: ".with_recent(ids) / .with_recent_limit(n)"
    description: "Starts with recent commands, and keeps up to n of them (five by default, zero for none)."
  - name: ".with_visible_rows(n)"
    description: "How many rows show at once, eight by default."
  - name: "state.update(Event)"
    description: "Handles Open, Close, Toggle, Back and the command list's own events. Returns Output::Activated when a command runs, Search(query) when the query changes, and Ready(sender) once the channel is open."
  - name: "state.is_open() / state.page() / state.page_id() / state.depth()"
    description: "Whether the palette is open, and the nested page on show."
  - name: "state.recent() / state.query() / state.command()"
    description: "The recent commands, newest first, the query and the command list inside."
  - name: "state.set_groups(groups)"
    description: "Replaces the first page's groups, for labels that follow your app's state."
  - name: "state.key_event(&keymap, &key)"
    description: "Resolves a key press from keys::subscription, so the toggle chords open the palette anywhere."
  - name: "command_palette::subscription()"
    description: "The command list's results channel, wrapped in Event::Command."
  - name: "command_palette(&state, base)"
    description: "Wraps your app in the palette's dialog. The list renders disabled until .on_event is set."
  - name: ".on_event(f) / .title(text) / .placeholder(text) / .empty(text)"
    description: "Maps palette events to your message, and sets the first page's title, the search placeholder and the empty-state text."
  - name: ".loading(bool) / .max_height(f32) / .size(dialog::Size)"
    description: "Shows that a search is running, caps the list's height (400 by default), and sets the dialog's width."
  - name: ".keymap(keymap) / .command_keymap(keymap) / .dialog_keymap(keymap)"
    description: "Replace the toggle and Back chords, the list's keys and the dialog's keys."
  - name: "command_palette::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>."
---

The command palette puts a [command](../command/) list inside a [dialog](../dialog/) and adds what an app-wide launcher needs. The app owns a `command_palette::State`, routes presses from `keys::subscription()` through `state.key_event` so Ctrl+K or Ctrl+Shift+P opens it from anywhere, and runs whatever `update` returns as `Output::Activated`. The palette closes itself when a command runs, on Escape with an empty query, or on a click outside.

Commands you run are remembered, newest first, and listed under Recent at the top while the query is empty. Typing hides that group so nothing shows twice. Choosing an item whose id names a page, such as "Change theme...", opens that page's list with its own title and placeholder instead of running anything. Backspace on an empty query, Escape or the Back button returns to the page before, and the search field keeps its focus throughout.

Results can also come from background work through the command list's channel. Subscribe to `command_palette::subscription()`, keep the sender from `Output::Ready`, and when `update` returns `Output::Search(query)`, search `state.page_id()` for it and send `command::results(query, group, items)`. Results for an older query are dropped. Set `.loading(true)` while a search runs.

While the search field has focus, the list keeps its own keys, then the palette's Back and toggle chords, then the dialog's, so arrows, Enter and Escape work as they do in the list on its own.
