---
title: Toggle group
description: A row of toggles where one, or several, can be on.
group: Actions
order: 4
module: primitives::toggle_group
imports: |
  use iced_cube::toggle_group;
  use iced_cube::primitives::toggle_group::{Event, State, Variant, item};
keywords: [segmented control, button group, toolbar, radio buttons]
related: [toggle, tabs, radio]
hero: toggle-group/single
stories: [toggle-group/single, toggle-group/multiple, toggle-group/keyboard]
api:
  - name: "State::single(items) / State::multiple(items)"
    description: "Holds the items, with nothing on. Single mode keeps at most one item on; multiple mode switches each one by itself."
  - name: ".required(true)"
    description: "Single mode only: the last item on stays on, and the first enabled item starts on when nothing else is."
  - name: ".with_selected(ids)"
    description: "Switches these items on, skipping unknown and disabled ones. In single mode the first that applies replaces the selection."
  - name: "item(id, label)"
    description: "Creates an item. Add .icon(glyph) or .disabled(true)."
  - name: "state.update(Event)"
    description: "Toggle(id), Next or Previous. Returns the ids that are on, if anything changed."
  - name: "state.selected() / state.selected_one() / state.is_selected(id)"
    description: "The ids that are on in item order, the first of them, and a check for one id."
  - name: "state.set_disabled(id, bool)"
    description: "Enables or disables an item at runtime. Whether it is on stays as it was."
  - name: "toggle_group(&state)"
    description: "Renders the items. They render disabled until .on_event is set."
  - name: ".variant(Variant)"
    description: "Default, separate toggles with a small gap, or Outline, attached segments inside one border. Defaults to Default."
  - name: ".size(Size)"
    description: "The toggle Size: Sm, Md or Lg. Defaults to Md."
  - name: ".on_event(f)"
    description: "Maps item events to your message, such as Message::Group."
  - name: "toggle_group::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.event(&state)"
    description: "Turns a resolved Action (Next or Previous) into the Event to pass to state.update. Multiple-mode groups get None."
---

The app owns a `toggle_group::State` and routes events into it, as with [tabs](../tabs/). In single mode pressing an item turns the others off; pressing the one that is on turns it off too, unless the group is `.required(true)`, which suits a choice that always has an answer, such as text alignment or a date range.

Multiple mode suits formatting options and filters, where each item is independent. `selected()` returns the items that are on in the order they are shown, whatever order they were pressed in.

Separate toggles wrap onto new lines on a narrow screen. An outlined group stays in one row, so keep it to two to four short items.

Use a single-choice group for a few short options shown side by side, and a [select](../select/) when there are more than four or five. When the options switch between whole views rather than settings, tabs are the clearer choice.

The arrow keys are bound by default because a single-choice group behaves like tabs. They are app-wide, so an app showing tabs, a slider or another group on the same screen should rebind one of them.
