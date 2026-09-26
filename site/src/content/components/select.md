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
stories: [select/default, select/states]
api:
  - name: "select(options, selected)"
    description: "Creates a select over a slice or Vec of options, labelled with their Display text. It renders disabled until it has a message."
  - name: ".on_select(f) / .on_select_maybe(option)"
    description: "Sets the message emitted with the chosen option. None disables the select."
  - name: ".placeholder(text)"
    description: "Text shown while nothing is selected."
  - name: ".width(length)"
    description: "Overrides the width. Defaults to 200 pixels."
  - name: "select::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.apply(&options, selected)"
    description: "The option a resolved Action (Next or Previous) selects, or None at either end. Send it through your on_select message."
---

Use a select when there are too many options to show as radios, or when space is tight. The list opens below the field, as wide as it, with a check mark on the selected option, and looks like every other menu and list. It closes when an option is chosen, on Escape, or when you click elsewhere; that click goes no further.

Options can be borrowed, such as a `const` slice, so the list is not copied on every view.
