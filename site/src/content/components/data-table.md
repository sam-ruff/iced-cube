---
title: Data table
description: Records in rows and columns that readers can sort, search, filter, select and page through, and that turn into cards on a phone.
group: Data
order: 2
module: data::data_table
imports: |
  use iced_cube::data::data_table::{self, Align, Event, Output, State, data_table};
keywords: [table, grid, data grid, records, rows, columns, sort, filter, pagination, bulk actions]
related: [tree, context-menu, checkbox, dropdown-menu]
hero: data-table/default
stories: [data-table/default, data-table/pagination, data-table/selection, data-table/row-actions, data-table/loading, data-table/empty, data-table/keyboard]
api:
  - name: "State::new(columns, rows, key)"
    description: "Holds your own row type, the columns that read it, and a key function giving each row an identity that survives sorting, such as its id."
  - name: "data_table::column(id, header, value)"
    description: "A sortable, searchable and hideable column. value reads a Value (text, a number or empty) from a row. Add .width(length), .align(Align::End), .format(fn) for the text shown, .filterable(true) for a filter of its values, .sortable(false), .hideable(false) or .searchable(false)."
  - name: ".with_selection(bool) / .with_page_sizes(sizes) / .with_sort(id, direction) / .with_hidden(ids)"
    description: "Row checkboxes, the page sizes offered (the first is used), a starting sort and columns hidden to start with."
  - name: "state.update(Event)"
    description: "Handles sorting, searching, filters, pages, selection, the keyboard and the Columns menu. Returns Output::Activated(key) or Output::Selected(keys)."
  - name: "state.page_rows() / state.selected() / state.highlighted()"
    description: "The rows on the current page, every selected key in row order, and the row the keyboard acts on."
  - name: "state.set_rows(rows) / state.push(row) / state.update_row(&key, f) / state.retain(f)"
    description: "Changes the rows. The sort, filters and the selection of rows that remain are kept."
  - name: "state.showing() / state.page() / state.page_count() / state.filtered_len()"
    description: "The footer text, such as \"Showing 1-10 of 243\", and the numbers behind it."
  - name: "data_table(&state)"
    description: "Renders the toolbar, the table and the footer. Controls render disabled until .on_event is set."
  - name: ".cell(id, f)"
    description: "Draws a column's cells with your own element, such as a badge or a progress bar."
  - name: ".toolbar(bool) / .placeholder(text)"
    description: "Shows or hides the search box, column filters and Columns menu, and sets the search placeholder."
  - name: ".loading(bool) / .empty(text)"
    description: "Placeholder rows while loading, and the message when nothing matches."
  - name: ".height(length) / .row_height(f32)"
    description: "A fixed or filling height scrolls the rows under a header that stays put. Rows are 44 pixels tall unless set, such as 32 for a compact table."
  - name: ".breakpoint(width)"
    description: "Below this width the rows become cards. Defaults to 560."
  - name: ".context_menu(&menu, f) / .row_actions(bool)"
    description: "Gives every row the same context menu, keyed by row key, and a button at the end of each row that opens it."
  - name: ".keymap(keymap) / .context_menu_keymap(keymap)"
    description: "Replaces the table's or the context menu's default keymap."
  - name: "data_table::default_keymap() / action.event(&state)"
    description: "The default shortcuts, and the Event a resolved Action sends. State::key_event does both."
  - name: "data_table::row_style(tokens, Status)"
    description: "The row colours: muted when selected, lighter under the pointer and a ring for the keyboard highlight."
---

A data table shows records that share the same fields, such as payments or invoices. `State` is generic over your own row type: the columns read each row through plain functions, so sorting, searching and filtering work on your data without copying it into strings first. The app owns the state, routes the table's events into it and acts on the outputs.

Pressing a header sorts by that column, then reverses the order, then restores the original order. The search box looks through every visible searchable column, and a filterable column adds a select of its values to the toolbar. The Columns menu hides and shows columns, but always leaves one. The footer changes the page size and turns the pages, and only the rows of the current page that are in view are built, so a table of thousands of rows stays quick.

With selection on, every row has a checkbox and the header has one that selects or clears the rows on the current page only. The selection belongs to row keys, so it survives sorting, filtering and turning pages. Build bulk actions from `state.selected()` next to the table, as the selection example does.

For actions on one row, pass one `context_menu::State` for every row with `.context_menu`. It opens on a right-click or a long press, and `.row_actions(true)` adds a visible button for touch screens. While the table has focus, Shift+F10 opens the menu on the highlighted row.

The table takes focus when pressed and resolves its keys only then, so its arrow keys, Space and Escape never clash with other components. The highlight moves on to the next page after the last row, and scrolls into view in a table with a fixed height.

Below its breakpoint the table becomes a stack of cards, one per row: the first column heads each card, and the other columns become labelled lines. A select above the cards takes over from the sortable headers. Nothing scrolls sideways or runs off the edge of a phone screen.
