---
title: Accordion
description: Stacked sections that expand and collapse under their headers.
group: Layout
order: 5
module: layout::accordion
imports: |
  use iced_cube::layout::accordion::{self, Mode, accordion};
keywords: [collapsible, disclosure]
related: [tabs, card]
hero: accordion/single
stories: [accordion/single, accordion/multiple]
api:
  - name: "State::new(Mode)"
    description: "Starts with every section closed. Single keeps at most one open; Multiple opens each independently."
  - name: ".with_open(id)"
    description: "Starts with a section open."
  - name: "state.update(Event)"
    description: "Toggle(id), Open(id) or Close(id). Returns Opened(id) or Closed(id) when something changed."
  - name: "state.is_open(id) / state.open()"
    description: "Checks one section, or lists the open ones in the order they opened."
  - name: "state.set_mode(Mode)"
    description: "Switches mode. Going to Single keeps only the first open section."
  - name: "accordion(&state)"
    description: "Renders the sections. Headers render disabled until .on_event is set."
  - name: ".item(id, title, content)"
    description: "Adds a section. Closed sections are not laid out or drawn, but their content is still built on every view, so check state.is_open(id) first if it is expensive."
  - name: "state.mode()"
    description: "The current Mode."
  - name: ".on_event(f)"
    description: "Maps accordion events to your message."
  - name: "accordion::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.events(&state, &sections)"
    description: "The events for a resolved Action, given every section id in display order. Pass each one to state.update."
---

Use an accordion to shorten long pages such as FAQs or settings, where people only need one or two sections at a time. The chevron points down while a section is closed and up while it is open.

Semibold headings separate each section from its muted body text. Content inherits the muted colour by default; text and controls with their own styles keep those colours.

Clicking the header of an open section closes it in both modes.

The state does not know which sections exist, so keyboard actions take the section ids in display order. `Next` and `Previous` move the most recently opened section along that list, and `ExpandAll` only applies in Multiple mode.
