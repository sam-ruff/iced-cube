---
title: Multi-select combobox
description: A searchable field that collects several values, each shown as a removable chip.
group: Forms
order: 10
module: forms::multi_combobox
imports: |
  use iced_cube::forms::multi_combobox::{self, Output, State, multi_combobox};
keywords: [multi select, tags input, token field, guests, chips, autocomplete]
related: [combobox, chip, field]
hero: multi-combobox/default
stories: [multi-combobox/default, multi-combobox/field, multi-combobox/keyboard]
api:
  - name: "State::new(options)"
    description: "Holds the options, labelled with their Display text. The list starts closed with nothing chosen."
  - name: ".with_chosen(&[values]) / .with_visible_rows(n)"
    description: "Starts with values chosen, in that order (unknown values and repeats are skipped), and sets how many suggestions show at once (six by default)."
  - name: "state.update(Event)"
    description: "Input(query), Open, Close, Next, Previous, ActivateHighlighted, Highlight(index), Activate(index), Scroll(rows), Remove(index) or RemoveLast. Returns Output::Added(value) or Output::Removed(value) when the chosen values change."
  - name: "state.chosen() / state.chosen_indices() / state.is_chosen(index)"
    description: "The chosen values in the order they were chosen, their indices in the options, and whether one option is chosen."
  - name: "state.query() / state.is_open() / state.matches() / state.highlighted()"
    description: "The text typed so far, whether the list is showing, the options that contain the query, and the highlighted one."
  - name: "multi_combobox(&state)"
    description: "Renders the chips, the field under them and, while open, the floating list. It renders disabled, with no x on its chips, until .on_event is set."
  - name: ".on_event(f)"
    description: "Maps events to your message, such as Message::Guests."
  - name: ".placeholder(text) / .empty(text)"
    description: "Text shown in the empty field, and in the list when nothing matches."
  - name: ".width(length) / .size(Size)"
    description: "Overrides the width (320 pixels by default) and takes the input sizes Sm, Md (the default) or Lg. Small fields get small chips."
  - name: ".id(key)"
    description: "Sets the text field's widget id to key and gives each chip's x the id multi_combobox::remove_id(key, index), for focus and tests."
  - name: ".invalid(bool)"
    description: "Draws the field with the destructive border, for example when at least one value is required."
  - name: ".shortcuts(Shortcuts)"
    description: "Sets which editing shortcuts the field handles, as on input. All by default; Shortcuts::none() turns them off."
  - name: ".keymap(keymap)"
    description: "Replaces the default keymap. The combobox resolves its own key presses while its field has focus or its list is open."
  - name: "multi_combobox::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.event(&state)"
    description: "Turns an Action into the Event it sends, or None when it would do nothing, for apps that route keys themselves."
---

Use a multi-select combobox to pick several values from a list too long to show at once, such as the guests of a meeting or the members of a group. To pick one value, use the [combobox](../combobox/).

It shares the combobox's field, list and filtering. Choosing a suggestion adds it as a [chip](../chip/) above the field, clears the query so the next name can be typed straight away, and keeps the list open. Chosen suggestions stay in the list with a tick, in their usual place, so the rows do not jump; choosing a ticked one removes it. `update` returns `Output::Added` or `Output::Removed`, so the app can send an invitation or save the group as values change.

The chips keep the order they were chosen in and wrap onto new lines when the field is narrow. A chip's x removes that value. Backspace in an empty field removes the last chip; while there is text in the field it deletes text as usual. Pressing an x takes focus from the field, so an open list closes.

The list closes on Escape, when the field loses focus, or on a click elsewhere. Inside a dialog or popover, Escape closes the list first.
