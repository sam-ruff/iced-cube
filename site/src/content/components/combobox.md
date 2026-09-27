---
title: Combobox
description: A text field with suggestions that filter as you type.
group: Forms
order: 9
module: forms::combobox
imports: |
  use iced_cube::forms::combobox::{self, State, combobox};
keywords: [autocomplete, typeahead, search select, suggestions]
related: [select, command, field]
hero: combobox/default
stories: [combobox/default, combobox/empty, combobox/field, combobox/keyboard]
api:
  - name: "State::new(options)"
    description: "Holds the options, labelled with their Display text. The list starts closed with nothing selected."
  - name: ".with_selected(&value) / .with_visible_rows(n)"
    description: "Starts with a value selected, and sets how many suggestions show at once (six by default)."
  - name: "state.update(Event)"
    description: "Input(query), Open, Close, Next, Previous, ActivateHighlighted, Highlight(index), Activate(index) or Scroll(rows). Returns the value when one is chosen."
  - name: "state.selected() / state.query() / state.is_open()"
    description: "The chosen value, the text typed so far and whether the list is showing."
  - name: "state.matches() / state.highlighted()"
    description: "The options that contain the query, ignoring case, in their original order, and the highlighted one."
  - name: "combobox(&state)"
    description: "Renders the field and, while open, the floating list. It renders disabled until .on_event is set."
  - name: ".on_event(f)"
    description: "Maps combobox events to your message, such as Message::Fruit."
  - name: ".placeholder(text) / .empty(text)"
    description: "Text shown in the empty field, and in the list when nothing matches. The empty text defaults to \"No results found.\""
  - name: ".width(length) / .size(Size) / .id(id)"
    description: "Overrides the width (240 pixels by default), takes the input sizes Sm, Md (the default) or Lg, and sets the text field's widget id."
  - name: ".invalid(bool)"
    description: "Draws the field with the destructive border, as input does, for example when a required value is missing."
  - name: ".keymap(keymap)"
    description: "Replaces the default keymap. The combobox resolves its own key presses while its field has focus or its list is open."
  - name: "combobox::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.event(&state)"
    description: "Turns an Action into the Event it sends, or None when it would do nothing, for apps that route keys themselves."
---

Use a combobox when the list is long enough that people would rather type than scroll, such as timezones or countries. For a handful of options, a [select](../select/) is simpler.

The app owns a `combobox::State` and passes every event to `update`. Clicking the field or pressing the down arrow opens the list; typing filters it. The selected value shows in the field while the list is closed, with the caret at its end, and as the placeholder while you type. In a [field](../field/) with an error, set `.invalid(true)` so the border matches the message, as the field example does. The list is as wide as the field, draws its rows like every other menu and list, and stays inside the window.

The combobox handles its own keys while its field has focus or its list is open, so there is no need to subscribe to key presses. Inside a dialog or popover, Escape closes the list first and leaves the dialog or popover open. The list closes when the field loses focus or when you click elsewhere; that click goes no further and the field gives up focus. Long lists show a window of rows that follows the highlight and scrolls with the mouse wheel.
