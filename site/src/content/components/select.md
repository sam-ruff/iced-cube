---
title: Select
description: Chooses one value from a dropdown list.
group: Forms
order: 6
module: forms::select
imports: |
  use iced_cube::select;
keywords: [dropdown, combobox, picker]
related: [radio, field]
hero: select/default
stories: [select/default, select/states, select/swatches]
api:
  - name: "select(options, selected)"
    description: "Creates a select over a slice or Vec of options, labelled with their Display text. It renders disabled until it has a message."
  - name: ".on_select(f) / .on_select_maybe(option)"
    description: "Sets the message emitted with the chosen option. None disables the select."
  - name: ".placeholder(text)"
    description: "Text shown while nothing is selected."
  - name: ".width(length)"
    description: "Overrides the width. Defaults to 200 pixels."
  - name: ".leading(f)"
    description: "Draws the element f returns for an option before its label, in the list and in the field while that option is selected. Return None for options that show their label alone."
  - name: "select::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.apply(&options, selected)"
    description: "The option a resolved Action (Next or Previous) selects, or None at either end. Send it through your on_select message."
keyboard:
  - keys: "Tab, Shift + Tab"
    action: "Moves keyboard focus to or from the select inside a dialog, or wherever the app runs iced's focus_next and focus_previous."
  - keys: "Space, Enter, Down"
    action: "Opens the focused select's list, with the selected option highlighted."
  - keys: "Up, Down, Home, End"
    action: "Moves the highlight in the open list."
  - keys: "Space, Enter"
    action: "Chooses the highlighted option and closes the list."
  - keys: "Escape"
    action: "Closes the list without choosing."
---

Use a select when there are too many options to show as radios, or when space is tight. The list opens below the field, as wide as it, with a check mark on the selected option, and looks like every other menu and list. It closes when an option is chosen, on Escape, or when you click elsewhere; that click goes no further.

An enabled select can take keyboard focus, and rings while it has it. Space, Enter or Down then opens the list, the arrow keys move through it and Space or Enter chooses. While a select is focused it keeps Down for opening, so the app-wide `Next` shortcut in the keymap does not reach it. A click never leaves the ring showing.

Options can be borrowed, such as a `const` slice, so the list is not copied on every view.

To mark options with a colour or an icon, pass `.leading(...)`. A [`theme::swatch`](../../theming/) takes its colour from the theme each time it is drawn, so a status coloured with `Tokens::of(theme).warning` follows the light and dark themes.
