---
title: Pagination
description: Moves through a long list one page at a time.
group: Navigation
order: 4
module: navigation::pagination
imports: |
  use iced_cube::pagination;
  use iced_cube::navigation::pagination::{Event, State, Variant};
keywords: [pager, pages, paging]
related: [breadcrumb, tabs]
hero: pagination/numbers
stories: [pagination/numbers, pagination/compact, pagination/keyboard]
api:
  - name: "State::new(pages) / State::for_items(items, per_page)"
    description: "Starts on page 1. Pages count from 1 and there is always at least one."
  - name: ".with_page(page)"
    description: "Starts on a given page, kept within range."
  - name: "state.update(Event)"
    description: "Select(page), Next, Previous, First or Last. Returns the new page when it changed."
  - name: "state.page() / state.pages()"
    description: "The current page and the page count."
  - name: "state.range(items, per_page)"
    description: "The indices of the items on the current page, for slicing your list."
  - name: "state.set_pages(pages)"
    description: "Changes the page count, such as after a filter, and moves the current page back into range."
  - name: "pagination(&state)"
    description: "Renders the controls. They render disabled until .on_event is set."
  - name: ".variant(Variant)"
    description: "Numbers, page numbers between previous and next buttons, or Compact, \"Page 3 of 12\" between first, previous, next and last buttons. Defaults to Numbers."
  - name: ".siblings(n)"
    description: "How many pages to show on each side of the current one. Defaults to 1."
  - name: ".size(Size)"
    description: "The button Size: Sm, Md or Lg. Defaults to Md."
  - name: ".on_event(f)"
    description: "Maps pagination events to your message, such as Message::Page."
  - name: "pagination::slots(page, pages, siblings)"
    description: "The page numbers and gaps the numbered layout shows, for custom layouts."
  - name: "pagination::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.event(&state)"
    description: "Turns a resolved Action (Next, Previous, First or Last) into the Event to pass to state.update, or None at that end already."
---

The app owns a `pagination::State` and routes events into it, then shows the slice of its list that `state.range(items, per_page)` returns. The control only moves the page, so the list can be anything from table rows to search results.

The numbered layout always shows the first and last pages and the ones next to the current page, with a gap for the rest. Once there are enough pages for a gap, it always shows the same number of slots, so the buttons stay put under the pointer as the page changes. Page numbers of three or more digits widen their buttons a little. When the space is too narrow, as on a phone, it drops the neighbouring pages and then falls back to the compact layout.

Use the compact layout under short lists, in panels and in dialogs, where the page count matters more than jumping to a particular page.
