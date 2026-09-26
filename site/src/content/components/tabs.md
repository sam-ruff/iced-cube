---
title: Tabs
description: Switches between related views that share one place on screen.
group: Navigation
order: 1
module: navigation::tabs
imports: |
  use iced_cube::navigation::tabs::{self, Variant, tab, tabs};
related: [accordion]
hero: tabs/underline
stories: [tabs/underline, tabs/pills, tabs/keyboard]
api:
  - name: "State::new(tabs)"
    description: "Holds the tabs and selects the first enabled one."
  - name: ".with_selected(id)"
    description: "Starts on a given tab, if it exists and is enabled."
  - name: "tab(id, label)"
    description: "Creates a tab. Add .icon(glyph) or .disabled(true)."
  - name: "state.update(Event)"
    description: "Select(id), Next or Previous. Returns the new id when the selection changed."
  - name: "state.selected() / state.is_selected(id)"
    description: "The selected id, or None when every tab is disabled, and a check for one id."
  - name: "state.tabs()"
    description: "The tabs, in order."
  - name: "state.set_disabled(id, bool)"
    description: "Enables or disables a tab at runtime."
  - name: "tabs(&state)"
    description: "Renders the tab list. Tabs render disabled until .on_event is set."
  - name: ".variant(Variant)"
    description: "Underline or Pills. Defaults to Underline."
  - name: ".width(length)"
    description: "Overrides the width. Underline tabs fill the available width and pills shrink to fit by default."
  - name: ".on_event(f)"
    description: "Maps tab events to your message, such as Message::Tabs."
  - name: "tabs::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.event(&state)"
    description: "Turns a resolved Action (Next, Previous, First or Last) into the Event to pass to state.update."
---

The app owns a `tabs::State` and routes events into it, then renders the view for `state.selected()` itself. The tab list is only the switcher, so the content can be anything.

`Next` and `Previous` wrap round at the ends and skip disabled tabs. The widget does not listen to the keyboard itself: the app subscribes with `keys::subscription()` and resolves each key press through a keymap, as the keyboard example does. The arrow keys, Home and End are bound app-wide, so an app with text inputs or other arrow-driven controls may want to unbind them and keep `Ctrl+Tab`.

Use the underline variant for page sections and the pills variant for a compact switch between views of the same data.
