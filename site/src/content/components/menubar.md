---
title: Menubar
description: File, Edit, View and Help menus in a strip along the top of a desktop window.
group: Application
order: 1
module: application::menubar
imports: |
  use iced_cube::dropdown_menu::{checkbox_item, item, radio_item, separator, submenu};
  use iced_cube::menubar::{self, Output, State, menu, menubar};
keywords: [menu bar, application menu, file menu, mnemonic]
related: [dropdown-menu, toolbar, command-palette]
hero: menubar/editor
stories: [menubar/editor]
api:
  - name: "State::new(menus)"
    description: "Holds the menus, all closed. Item ids must be unique across every menu."
  - name: "menu(title, entries)"
    description: "One menu, built from dropdown menu entries: items with icons and shortcut hints, checkbox and radio items, submenus, labels and separators."
  - name: ".mnemonic(letter)"
    description: "The letter that opens the menu with Alt, or on its own while the bar has focus. Defaults to the first letter of the title."
  - name: "state.update(Event)"
    description: "Handles Toggle, Hover, Menu, Focus, Next, Previous, Open, OpenMenu, Close and Dismiss. Returns Output::Activated, Toggled or Selected when an item is chosen, then closes the menu and leaves the bar."
  - name: "state.open() / state.focused() / state.highlighted()"
    description: "The open menu's position, the title with the keyboard focus, and the highlighted item in the open menu."
  - name: "state.is_checked(id) / state.set_checked(id, bool) / state.set_disabled(id, bool)"
    description: "Reads and changes items in whichever menu holds them."
  - name: "state.key_event(&keymap, &menu_keymap, &key)"
    description: "Resolves a key press from keys::subscription. A closed bar claims only Focus and Alt with a mnemonic until it has focus."
  - name: "menubar(&state)"
    description: "Renders the titles, with the open menu below its title. Titles render disabled until .on_event is set."
  - name: ".on_event(f) / .width(f32)"
    description: "Maps menubar events to your message, and sets the width of each menu (224 by default)."
  - name: ".keymap(keymap) / .menu_keymap(keymap)"
    description: "Replace the bar's keys and the keys inside an open menu. The open menu resolves both itself."
  - name: "menubar::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>."
  - name: "action.event(&state)"
    description: "Turns a resolved Action into the Event to pass to state.update, or None when it does nothing in the bar's current state."
  - name: "trigger_style(tokens, TriggerStatus)"
    description: "The colours of a title at rest, under the pointer, open or disabled."
---

A menubar is a row of [dropdown menus](../dropdown-menu/) that behave as one. Each menu takes the same entries, so shortcut hints, checkbox and radio items and submenus work exactly as they do there, drawn in the same rows. The app owns a `menubar::State`, routes its events into it and acts on the `Output`, which has already updated the menu's own check marks.

Clicking a title opens its menu. Once one menu is open, moving the pointer onto another title switches to that menu, as desktop menus do, and clicking the open title closes it. A click anywhere else closes the menu and goes no further. On a touch screen there is no pointer to move, so the first tap outside closes the open menu and a second tap opens the next.

While a menu is open it resolves its own keys: Left and Right move to the neighbouring menu, or open and close a submenu when the highlight is on one, and the arrows, Enter, Escape and typeahead work inside the menu. A closed bar claims no keys by itself. Route presses from `keys::subscription()` through `state.key_event`, and F10 or Alt focuses the first title; the arrows then move along the bar, Down opens a menu and Escape leaves. While the bar has focus from the keyboard, each title underlines its mnemonic, and that letter opens the menu. Alt with the letter works at any time. The [keyboard guide](../../keyboard/) explains how keys reach each layer.

Bare Alt focuses the bar on the key press, so a chord such as Alt+Z in another component also leaves the bar focused. Unbind Alt from the keymap if that clashes with your app, and keep F10.
