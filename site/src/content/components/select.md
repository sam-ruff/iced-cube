---
title: Select
description: Chooses one value from a dropdown list.
group: Forms
order: 6
module: forms::select
imports: |
  use iced_cube::select;
keywords: [dropdown, combobox, picker, scroll, search, filter, type-ahead]
related: [radio, field, combobox]
hero: select/default
stories: [select/default, select/states, select/swatches, select/long-list, select/near-bottom, select/searchable, select/custom-value]
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
  - name: ".max_visible_rows(n)"
    description: "The most rows the open list shows before it scrolls. Defaults to eight (MAX_VISIBLE_ROWS)."
  - name: ".scrollable(bool)"
    description: "Whether a long list scrolls. On by default; turned off, the list is as tall as its options."
  - name: ".searchable(bool)"
    description: "Adds a search field to the open list that filters the options as you type. Off by default."
  - name: ".on_custom(f)"
    description: "Lets the user type a value that is not in the list. Enter with text that matches no option calls f with the text; return the message for a valid value, or None to keep the text with the invalid look."
  - name: ".search_placeholder(text) / .empty(text) / .not_valid(text)"
    description: "The text in the empty search field, the text shown when nothing matches, and the text shown when on_custom turns a typed value down."
  - name: ".shortcuts(shortcuts)"
    description: "The editing shortcuts of the search field, as on an input. Escape always closes the list."
  - name: ".type_ahead_pause(duration)"
    description: "How long a pause in typing starts a new type-ahead search. Defaults to one second."
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
    action: "Moves the highlight in the open list, scrolling it into view. With a search field, Home and End move the caret instead."
  - keys: "Page Up, Page Down"
    action: "Moves the highlight by one visible page."
  - keys: "Letters"
    action: "Moves the highlight to the first option starting with what you type, and Space while typing adds to the search. The same letter again moves to the next option starting with it. With a search field, typing goes into the field instead."
  - keys: "Space, Enter"
    action: "Chooses the highlighted option and closes the list. With a search field, Space types a space."
  - keys: "Enter"
    action: "With custom values on, sends text that matches no option to the app."
  - keys: "Tab"
    action: "Closes the list without choosing and moves focus on."
  - keys: "Escape"
    action: "Closes the list without choosing."
---

Use a select when there are too many options to show as radios, or when space is tight. The list opens below the field, as wide as it, with a check mark on the selected option, and looks like every other menu and list. It closes when an option is chosen, on Escape, or when you click elsewhere; that click goes no further.

An enabled select can take keyboard focus, and rings while it has it. Space, Enter or Down then opens the list, the arrow keys move through it and Space or Enter chooses. While a select is focused it keeps Down for opening, so the app-wide `Next` shortcut in the keymap does not reach it. A click never leaves the ring showing.

Options can be borrowed, such as a `const` slice, so the list is not copied on every view.

A long list shows eight rows at a time and scrolls by wheel, scrollbar, a finger drag or the keys. It opens with the selected option in the middle, and is never taller than the room the window has: near the bottom of the window it opens above the field. Scrolling stops at either end of the list rather than moving the page behind it. A list that fits shows no scrollbar.

For lists people search, such as timezones or countries, turn on `.searchable(true)`. The open list then starts with a search field that has the same editing keys as an input, such as paste, undo and deleting a word. To accept values that are not in the list, such as a time off the quarter hours, pass `.on_custom(...)`: when nothing matches, the list offers the typed text as its only row, and Enter or a click hands it to your app, which decides whether it is valid. A value turned down stays in the field in red, with the `.not_valid(...)` text below it, until it is edited. Both are off unless you ask for them, and the query is cleared whenever the list closes.

When the options are known up front and you want the field itself to take typing, with the query and list in your app's state, use a [combobox](../combobox/) instead.

To mark options with a colour or an icon, pass `.leading(...)`. A [`theme::swatch`](../../theming/) takes its colour from the theme each time it is drawn, so a status coloured with `Tokens::of(theme).warning` follows the light and dark themes.
