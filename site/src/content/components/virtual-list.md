---
title: Virtual list
description: A long list of equally tall rows that only builds the rows in view, with selection, keyboard navigation and items streamed in from other threads.
group: Data
order: 3
module: data::virtual_list
imports: |
  use iced_cube::data::virtual_list::{self, Event, Mode, Output, State, virtual_list};
keywords: [list, long list, windowing, virtualised list, infinite list, feed, stream, follow tail]
related: [log-viewer, data-table, tree]
hero: virtual-list/large
stories: [virtual-list/large, virtual-list/streaming, virtual-list/keyboard]
api:
  - name: "State::new(items)"
    description: "Holds the items in Mode::Single with nothing selected. State::default() starts empty."
  - name: ".with_mode(Mode) / .with_limit(n) / .with_follow(bool)"
    description: "Mode::Single (the default), Multiple or None; the most items kept, dropping the oldest; and whether the list sticks to its end as items arrive."
  - name: "state.update(Event)"
    description: "Handles presses, keyboard navigation, following and arriving items. Returns Output::Ready(sender) once the channel is open, Activated(index) or Selected(indices)."
  - name: "state.push(item) / state.extend(items) / state.set_items(items) / state.clear()"
    description: "Adds or replaces items directly. Replacing clears the selection and highlight."
  - name: "state.get(index) / state.len() / state.iter()"
    description: "Reads the items, oldest first."
  - name: "state.selected() / state.highlighted() / state.is_following()"
    description: "The selected indices in order, the index the keyboard acts on, and whether the list follows its end."
  - name: "state.scroll_to(index)"
    description: "Scrolls an item into view without highlighting it."
  - name: "state.id(index) / state.index(id)"
    description: "Converts between an index and the RowId events carry, which stays the same when older items are dropped."
  - name: "virtual_list::subscription() / virtual_list::Sender"
    description: "The channel items arrive on. It sends Event::Ready(sender) first; producers send items through clones of the sender, and they arrive in batches as Event::Received."
  - name: "virtual_list(&state, |index, item| row)"
    description: "Renders the rows, each drawn by the closure. Rows render disabled until .on_event is set."
  - name: ".row_height(f32) / .width(length) / .height(length)"
    description: "Every row's height, 32 by default, and the list's size. A fixed or filling height scrolls the rows."
  - name: ".keymap(keymap)"
    description: "Replaces the default keymap the focused list resolves keys with."
  - name: ".context_menu(&menu, on_event) / .context_menu_keymap(keymap)"
    description: "Gives every row a context menu keyed by its RowId, opened with a right-click, a long press or Shift+F10 on the highlighted row, and replaces the menu's keymap. The row it opened on carries the ring while it is open."
  - name: "virtual_list::default_keymap() / action.event(&state)"
    description: "The default shortcuts, and the Event a resolved Action sends. State::key_event does both."
  - name: "virtual_list::row_style(tokens, Status) / frame_style(tokens)"
    description: "The row colours, as a tree's rows: the accent behind selected rows, a lighter tint under the pointer and a ring around the keyboard highlight while the list has focus, inside a bordered frame."
---

A virtual list shows thousands of items without building thousands of widgets. Every row is the same height, so the list knows where each one sits and builds only the rows in view, plus a few either side so scrolling never shows a gap. The app owns a `virtual_list::State` holding the items and routes the list's events into it. The closure passed to `virtual_list` draws one row from its index and item, and only runs for rows in view.

Pressing a row highlights it, and in single or multiple mode selects it. In multiple mode Ctrl or Cmd adds or removes one item and Shift selects the range from the last one pressed. Space selects the highlighted item, and a double click or Enter activates it. `.context_menu(&menu, on_event)` gives every row a menu, keyed by its `RowId`; `state.index(id)` turns that back into an index. Give the list a fixed or filling height: one that shrinks to fit has to build every row.

Items can stream in from threads or tasks. `virtual_list::subscription()` owns a bounded channel and sends `Event::Ready` with its sender first. Hand clones of the sender to producers, and items arrive in batches, so a burst costs one update rather than one per item. A list made `with_follow(true)` stays scrolled to its newest item. Scrolling up stops following, so the rows stay still while you read, and scrolling back to the end or pressing End starts again. `with_limit(n)` drops the oldest items once the list holds that many, and the rows in view stay where they are as older ones go.

The list resolves its keys itself, and only while it has focus, which it takes when pressed and gives up on a press elsewhere. Page Up and Page Down move by the number of rows that fit, which the list reports as its size changes.
