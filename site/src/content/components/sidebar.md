---
title: Sidebar
description: Persistent app navigation down one side of the window, which folds to an icon rail and becomes a drawer on small screens.
group: Navigation
order: 3
module: navigation::sidebar
imports: |
  use iced_cube::navigation::sidebar::{self, Event, Output, State, group, item, menu_button, sidebar};
keywords: [nav, navigation drawer, rail, side menu, app shell]
related: [resizable-panel, dropdown-menu, tabs]
hero: sidebar/default
stories: [sidebar/default, sidebar/rail, sidebar/drawer, sidebar/resizable]
api:
  - name: "group(label, items)"
    description: "A labelled group of items. An empty label draws no heading. Add .collapsible(true) to let the heading open and close it."
  - name: "item(id, label)"
    description: "A navigation item. Add .icon(glyph), .badge(text), .disabled(true) or .children(items) for one level of nested items."
  - name: "State::new(groups)"
    description: "Holds the navigation with every group open and nothing active."
  - name: ".with_active(id) / .with_rail(bool)"
    description: "Starts on an item, opening its parent, or collapsed to the icon rail."
  - name: "state.update(Event)"
    description: "Handles clicks, keyboard navigation, the rail and the drawer. Returns Output::Activated(id) when an item is chosen."
  - name: "state.shortcut(&keymap, &key)"
    description: "Resolves the app-wide Ctrl+B from keys::subscription. The sidebar handles its other keys while it has focus."
  - name: "state.active() / state.item(id) / state.is_compact()"
    description: "The active item, an item's details, and whether the rail is showing, for the header and footer buttons."
  - name: "sidebar(&state)"
    description: "Renders the navigation. Items render disabled until .on_event is set."
  - name: ".header(element) / .footer(element)"
    description: "Content above and below the groups, such as a workspace switcher and the signed in user."
  - name: ".content(element)"
    description: "The page beside the sidebar. With content, the sidebar becomes a drawer below the breakpoint; without it only the panel is drawn."
  - name: ".breakpoint(width)"
    description: "The window width below which the sidebar becomes a drawer. Defaults to 640."
  - name: ".trigger(bool)"
    description: "Whether narrow windows get a bar holding the drawer's trigger. Turn it off to put sidebar::trigger in your own header."
  - name: ".width(length)"
    description: "The expanded width, 256 by default. Use Length::Fill inside a resizable panel."
  - name: ".keymap(Keymap<Action>) / sidebar::default_keymap()"
    description: "The shortcuts the focused sidebar resolves."
  - name: "sidebar::trigger(message)"
    description: "The icon button that toggles the rail, or the drawer on a narrow window. Send Event::Toggle from it."
  - name: "menu_button(title)"
    description: "A header or footer button with .subtitle, .icon or .initials, and .compact(state.is_compact()) for the rail. Wrap it in a dropdown menu for a switcher or a user menu."
---

The app owns the state: which item is active, which groups and parents are open, whether the sidebar is folded to a rail, and whether the drawer is open. Clicking a parent item opens its children instead of activating it, and in the rail it expands the sidebar first. The item that holds the page keeps the accent background, and a closed parent keeps it while one of its children is active.

The sidebar is drawn on a surface a shade off the page, with a one pixel edge, so it sits comfortably beside cards and other content. Rows use the same look as menus and lists.

## Small screens

With `.content(...)`, the sidebar notices when its width falls below the breakpoint and sends `Event::Narrow(true)`. From then on it is an off-canvas drawer: a bar above the page holds the trigger, and the drawer slides over a scrim. A tap on the scrim, Escape or choosing an item closes it, and widening the window closes it too. `Event::Toggle` does whichever fits, so one trigger button works at every size.

## Keyboard focus

iced 0.14 has no general focus model, so the sidebar takes focus the way a text field does: a press inside it gives it focus, and a press outside or Escape takes it away. An open drawer always has focus. While focused, the sidebar resolves its own keymap and captures the keys it uses. Ctrl+B is meant to work anywhere, so route `keys::subscription` through `state.shortcut`, which only resolves that one action. Some browsers keep Ctrl+B for themselves, so the rail story also has a button.
