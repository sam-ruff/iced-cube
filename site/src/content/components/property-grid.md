---
title: Property grid
description: A compact two-column inspector for editing named properties, grouped into categories, with validation, reset to default and a description of the highlighted property.
group: Data
order: 3
module: data::property_grid
imports: |
  use iced_cube::data::property_grid::{
      self, Event, Output, State, choice, colour, custom, number, property_grid, read_only, text,
      toggle,
  };
keywords: [inspector, properties panel, property editor, settings grid, attributes]
related: [tree, data-table, input, select]
hero: property-grid/inspector
stories: [property-grid/inspector, property-grid/filter-sort, property-grid/validation, property-grid/keyboard]
api:
  - name: "State::new(properties)"
    description: "Holds the properties by id with their values, categorised with every category open. Ids must be unique."
  - name: "text(id, name, value) / number(id, name, value) / toggle(id, name, value)"
    description: "Free text, a number and a true or false value. Numbers take .min, .max, .range, .step, .decimals and .unit; a whole step also rejects values between steps. Toggles show a checkbox, or a switch with .switch()."
  - name: "choice(id, name, options, selected) / colour(id, name, colour)"
    description: "One of a list of options, chosen with a select, and a colour shown as a swatch and edited as hex."
  - name: "read_only(id, name, value) / custom(id, name, value)"
    description: "Text that cannot be edited, and a value the app draws itself with .custom on the grid."
  - name: ".category(name) / .description(text) / .default(value) / .disabled(true)"
    description: "Group a property, describe it under the rows, give it a default to reset to, or show it without letting it change. Text properties also take .validate(fn)."
  - name: ".with_sort(Sort) / .with_collapsed(categories) / .with_name_width(px)"
    description: "Sort::Categorised (the default) or Alphabetical, categories to start closed, and a fixed starting width for the name column."
  - name: "state.update(Event)"
    description: "Handles presses, editing, typing, keyboard navigation, the filter, sorting and resizing. Returns Output::Changed { id, value } whenever a value changes and Output::Reset { id, value } when one goes back to its default."
  - name: "state.value(id) / state.property(id) / state.set(id, value)"
    description: "Reads a value or property, and sets one from the app, such as from a custom editor or after an undo."
  - name: "state.editing() / state.draft(id) / state.error(id)"
    description: "The property being typed into, its text, and why that text is not valid yet."
  - name: "state.entries() / state.highlighted() / state.filter() / state.sort()"
    description: "The rows on screen in order, the row the keyboard acts on, the filter text and the sort."
  - name: "property_grid(&state)"
    description: "Renders the filter box, the rows and the description area. Editors render disabled until .on_event is set."
  - name: ".custom(id, render)"
    description: "Draws the value of one property with your own element, such as a slider. Store the result with state.set."
  - name: ".toolbar(bool) / .description(bool) / .placeholder(text)"
    description: "Show or hide the filter and sort buttons and the description area, and change the filter's placeholder. Errors show under the rows either way."
  - name: ".width(length) / .height(length)"
    description: "A fixed or filling height scrolls the rows, and only the rows in view are built."
  - name: ".keymap(keymap) / property_grid::default_keymap() / action.event(&state)"
    description: "Replaces the default shortcuts, and the Event a resolved Action sends. State::key_event does both."
  - name: "property_grid::row_style(tokens, Status) / name_width(requested, width)"
    description: "The row colours, and the width the name column gets in a row of a given width."
---

A property grid edits the settings of one thing at a time, such as the shape selected on a canvas or a widget in a form designer. The app owns a `property_grid::State` holding every property by id and routes the grid's events into it. Whenever `update` returns `Output::Changed`, copy the new value into your own model.

Text, numbers and colours show as plain text until clicked, or until Enter or F2 is pressed on the highlighted row. They then become a text field with their text selected. A valid edit changes the value as it is typed. An invalid one, such as a number above its maximum or a colour that is not hex, keeps what was typed, draws the field with a destructive border and explains the problem under the rows, without changing the value. Enter ends the edit once it is valid, and Escape puts back the value from before it. Leaving an invalid edit keeps its text in place until it is fixed or reset.

Toggles and choices change straight away: a checkbox or switch flips, a select picks an option, and Enter on the row does the same from the keyboard. A property with a default shows a dot and a bold name while its value differs, and a reset button at the end of its row.

The box above the rows filters properties by name, and the two buttons beside it switch between categories and one alphabetical list. Drag the line between the columns to give names more room. Names that still do not fit end in an ellipsis rather than wrapping, so a panel only 260 pixels wide keeps every row the same height.

The grid resolves its keys itself, and only while it has focus, which it takes when pressed. While a field is being typed into it keeps the keys it uses, such as the arrows that move its cursor, but Up and Down step a number up or down.
