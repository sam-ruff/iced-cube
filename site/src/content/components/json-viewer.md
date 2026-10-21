---
title: JSON Viewer
description: A JSON document as rows that expand and collapse, with values coloured by type, search and copying of values and paths.
group: Data
order: 3
module: data::json_viewer
imports: |
  use iced_cube::data::json_viewer::{self, Event, MenuItem, NodeId, Output, State, json_viewer};
keywords: [json, inspector, api response, payload, object viewer, structured data, serde_json]
related: [tree, data-table, context-menu]
hero: json-viewer/api-response
stories: [json-viewer/api-response, json-viewer/search, json-viewer/copy, json-viewer/keyboard, json-viewer/large-document, json-viewer/parse-errors]
api:
  - name: "State::new(value)"
    description: "Holds a serde_json::Value with everything collapsed. The members of an object or array are the top rows; any other value is a single row."
  - name: "State::parse(text)"
    description: "Parses JSON text. Invalid text gives a viewer that shows the error, with its line and column, in place of the rows; state.error() returns it."
  - name: ".with_depth(n) / .with_filter(bool)"
    description: "Starts with every object and array fewer than n levels deep expanded, and with the search filter on."
  - name: "state.update(Event)"
    description: "Handles presses, expanding and collapsing (one node, a subtree, everything or to a depth), keyboard navigation, search and copying. Returns Output::CopyValue(json), CopyPath(path) or FocusSearch."
  - name: "Event::Search(text) / NextMatch / PreviousMatch / Filter(bool)"
    description: "Searches keys and values ignoring case, opens the parents of every match and highlights the first. The filter shows only matches, their parents and what is inside them."
  - name: "Event::ExpandAll / CollapseAll / ExpandToDepth(n)"
    description: "Expands or collapses every object and array, or opens those fewer than n levels deep."
  - name: "state.path(id) / state.pretty(id) / state.value_at(id)"
    description: "A node's path such as $.users[2].name, with [\"key\"] for keys that are not plain identifiers; its value as pretty-printed JSON; and the value itself."
  - name: "state.visible() / state.highlighted() / state.matches() / state.current_match()"
    description: "The rows on screen in order, the node the keyboard acts on, every match and the one the search moved to last."
  - name: "json_viewer(&state)"
    description: "Renders the rows. Without .on_event it renders disabled."
  - name: ".search(bool) / .placeholder(text) / .search_id(id)"
    description: "Shows a search box above the rows with the match count, previous and next buttons and a filter toggle. Its id defaults to json_viewer::SEARCH."
  - name: ".height(length) / .width(length) / .key_width(px)"
    description: "A fixed or filling height scrolls the rows, and only the rows in view are built. Keys longer than key_width end in an ellipsis."
  - name: ".context_menu(&menu, f)"
    description: "Gives every row the same context menu, keyed by node. json_viewer::menu() builds one with Copy value, Copy path, Expand all and Collapse all; MenuItem::event(id) turns the chosen entry into an Event."
  - name: ".keymap(keymap) / .context_menu_keymap(keymap)"
    description: "Replaces the viewer's or the context menu's default keymap."
  - name: "json_viewer::default_keymap() / action.event(&state)"
    description: "The default shortcuts, and the Event a resolved Action sends. State::key_event does both."
  - name: "json_viewer::syntax(tokens) / value_colour(tokens, kind) / row_style(tokens, Status)"
    description: "The colours of keys and of each kind of value, taken from the theme's success, warning and destructive tokens, and the row colours for the highlight and search matches."
---

The JSON viewer shows structured data such as an API response, a configuration file or an event payload. The app owns a `json_viewer::State` built from a `serde_json::Value`, or from text with `State::parse`, and routes the viewer's events into it.

Objects and arrays show how many members they hold, as `{3}` or `[12]`, and expand with their chevron, a double click or the keyboard. Strings, numbers, booleans and `null` each have their own colour, taken from the theme's tokens so they follow light, dark and custom themes. A long key or value ends in an ellipsis on one line rather than wrapping; copy it to read it in full.

The highlighted row shows two buttons: one copies its value as pretty-printed JSON and the other its path, such as `$.users[2].name`. The viewer never writes to the clipboard itself. It returns `Output::CopyValue` or `Output::CopyPath`, and the app writes the text, for example with `iced::clipboard::write`. The same outputs come from the context menu and from Ctrl+C and Ctrl+Shift+C.

Searching looks in keys and values, ignoring case. Every match is tinted, the parents of each one open, and the first match is highlighted. Enter in the search box, the arrow buttons or F3 move between matches, and the filter toggle hides everything that is not a match or a parent of one. Ctrl+F returns `Output::FocusSearch`; focus the box with `iced::widget::operation::focus(json_viewer::SEARCH)`.

The viewer resolves its keys itself, and only while it has focus, which it takes when pressed and gives up on a press elsewhere. Only the rows in view are built, so a document of tens of thousands of nodes scrolls and expands as quickly as a small one.
