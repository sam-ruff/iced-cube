---
title: Tree
description: Nested items that expand and collapse, such as folders and files, with selection, checkboxes and children that load later.
group: Data
order: 1
module: data::tree
imports: |
  use iced_cube::data::tree::{self, Event, Mode, Output, State, loaded, node, tree};
keywords: [tree view, file explorer, outline, hierarchy, folders, nested list, lazy loading]
related: [data-table, context-menu, checkbox]
hero: tree/file-explorer
stories: [tree/file-explorer, tree/checkbox, tree/lazy-loading, tree/keyboard]
api:
  - name: "State::new(nodes)"
    description: "Holds the nodes, keyed by id, all collapsed. Ids must be unique across the whole tree."
  - name: "node(id, label)"
    description: "A leaf. Add .children(nodes), .lazy() for children that load later, .folder() or .file() for the usual icons, .icon(glyph) or .icons(closed, open), .badge(text) or .trailing(text) at the end of the row, and .disabled(true)."
  - name: ".with_mode(Mode) / .with_expanded(ids)"
    description: "Mode::Single (the default), Multiple, Checkbox or None, and the nodes to start expanded."
  - name: "state.update(Event)"
    description: "Handles presses, expanding, keyboard navigation, checkboxes and arriving children. Returns Output::Load(ids) when lazy nodes need their children, Activated(id), Selected(ids) or Checked(ids)."
  - name: "state.selected() / state.checked() / state.check_state(id)"
    description: "The selected or checked nodes in tree order, and whether a node's box is checked, unchecked or indeterminate."
  - name: "state.visible() / state.highlighted() / state.is_expanded(id)"
    description: "The nodes on screen in order, the node the keyboard acts on, and whether a node is open."
  - name: "state.node(id) / state.children(id) / state.parent(id)"
    description: "Reads the nodes and their structure."
  - name: "tree::subscription() / tree::Sender / loaded(parent, children)"
    description: "The channel lazy children arrive on. It sends Event::Ready(sender) first; producers send loaded(parent, nodes) through clones of the sender, and batches arrive as Event::Received."
  - name: "state.set_loading(id)"
    description: "Shows a node as loading, such as while the app refreshes it."
  - name: "tree(&state)"
    description: "Renders the rows. Rows render disabled until .on_event is set."
  - name: ".guides(bool)"
    description: "Draws a vertical line at each level of indentation."
  - name: ".width(length) / .height(length)"
    description: "A fixed or filling height scrolls the rows. Only the rows in view are built."
  - name: ".context_menu(&menu, f)"
    description: "Gives every node the same context menu, keyed by node id, opened by a right-click, a long press or Shift+F10 on the highlighted node."
  - name: ".keymap(keymap) / .context_menu_keymap(keymap)"
    description: "Replaces the tree's or the context menu's default keymap."
  - name: "tree::default_keymap() / action.event(&state)"
    description: "The default shortcuts, and the Event a resolved Action sends. State::key_event does both, with typeahead."
  - name: "tree::row_style(tokens, Status)"
    description: "The row colours, taken from the shared menu rows: selected rows use the accent, the keyboard highlight a ring."
---

A tree shows data with a parent and child structure: folders and files, an outline, groups of permissions. The app owns a `tree::State` holding every node by id and routes the tree's events into it. Nodes are plain data, so the app can read them back with `state.node(id)` when a node is activated or chosen from its menu.

Pressing a row highlights it, and in single or multiple mode selects it. In multiple mode Ctrl or Cmd adds or removes one node and Shift selects the range from the last one pressed. In checkbox mode every row has a checkbox: checking a parent checks its children, and a parent with only some children checked shows a dash. Disabled nodes can be seen but not selected, checked, expanded or activated, and a disabled child keeps its state when its parent changes.

A node made with `.lazy()` has children that are not known yet. Expanding it shows a loading line and returns `Output::Load` with its id. Fetch the children however you like, on a thread or a task, and send `loaded(id, nodes)` through the sender from `tree::subscription()`. The lazy loading example simulates a server that answers after a short wait.

The tree resolves its keys itself, and only while it has focus, which it takes when pressed and gives up on a press elsewhere. That keeps its arrow keys from clashing with tabs, sliders or anything else on the page. Letters and digits jump to the next node starting with them, and `*` expands the highlighted node and all of its siblings. When the tree has a fixed height it scrolls the highlighted row into view as the highlight moves.

Rows are 32 pixels tall, as in menus, and long labels end in an ellipsis rather than wrapping, so a narrow sidebar on a phone keeps every row the same height and easy to tap.
