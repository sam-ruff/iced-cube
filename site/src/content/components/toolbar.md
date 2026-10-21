---
title: Toolbar
description: A compact strip of icon buttons that moves whatever does not fit into a More menu.
group: Application
order: 2
module: application::toolbar
imports: |
  use iced_cube::toolbar::{
      self, Output, State, button, choice, separator, spacer, toggle, toolbar,
  };
keywords: [action bar, formatting toolbar, overflow, toggle group]
related: [icon-button, menubar, dropdown-menu, tooltip]
hero: toolbar/formatting
stories: [toolbar/formatting]
api:
  - name: "State::new(entries)"
    description: "Holds the entries and whether each toggle and choice is on. Buttons are small by default."
  - name: "button(id, icon, label) / toggle(id, icon, label, on) / choice(id, icon, label, selected)"
    description: "An action, a button that stays pressed while on, and one of a single-choice group. Adjacent choices form a group. The label is the tooltip and the overflow row."
  - name: "separator() / spacer()"
    description: "A short line between groups, and a gap that pushes the entries after it to the far end while everything fits."
  - name: ".shortcut(hint) / .disabled(bool)"
    description: "A hint added to the tooltip and the overflow row, and a disabled button."
  - name: ".with_size(Size)"
    description: "The button size: Sm (32 pixels) by default, or Md and Lg."
  - name: "state.update(Event)"
    description: "Handles presses, the overflow menu and the keyboard highlight. Returns Output::Activated, Toggled or Selected, and updates its own toggles and choices."
  - name: "state.is_on(id) / state.set_on(id, bool) / state.set_disabled(id, bool)"
    description: "Reads and changes buttons from your own state, such as after undo."
  - name: "state.visible() / state.highlighted()"
    description: "How many entries fit at the last measured width, and where the keyboard highlight is."
  - name: "toolbar::fit(entries, width, size)"
    description: "The pure measuring rule: how many entries fit before the rest move into More."
  - name: "state.key_event(&keymap, &key)"
    description: "Resolves a key press from keys::subscription. Only Focus claims a key until the toolbar has focus."
  - name: "toolbar(&state)"
    description: "Renders the entries that fit and a More menu with the rest. Buttons render disabled until .on_event is set."
  - name: ".on_event(f) / .tooltip(Option<Position>) / .more_label(text)"
    description: "Maps toolbar events to your message, moves or hides the tooltips (below by default), and names the More button."
  - name: ".menu_keymap(keymap)"
    description: "Replaces the keys the open overflow menu resolves."
  - name: "toolbar::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>."
---

A toolbar measures itself as it lays out. The entries that fit stay in the strip, in order, and the rest move into a More [dropdown menu](../dropdown-menu/) at its end, so the same toolbar works in a wide window and on a phone. In the menu, toggles become checkbox items and choices become radio items, so a format that has overflowed still shows whether it is on. Drag the slider in the example to watch buttons move in and out.

The app owns a `toolbar::State`. When `update` returns `Output::Toggled` or `Output::Selected`, the toolbar has already updated the button, so the app only changes its own model. When the model changes elsewhere, such as after undo or a Ctrl+B shortcut, call `set_on` and `set_disabled` to match.

iced has no focus for a strip of buttons, so the toolbar keeps a keyboard highlight in its state and draws it as a ring. Route presses from `keys::subscription()` through `state.key_event`: Ctrl+F10 focuses the toolbar, Left and Right move through the buttons in view and then More, Enter or Space presses the highlighted one and Escape leaves. More opens its menu on the first item, which handles its own keys. A click on a button ends the keyboard focus. Each button's label shows as a [tooltip](../tooltip/) below it, with its shortcut hint.
