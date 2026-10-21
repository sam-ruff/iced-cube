---
title: Status bar
description: A thin bar along the bottom of a window, fed with status from background work.
group: Application
order: 3
module: application::status_bar
imports: |
  use iced_cube::status_bar::{
      self, Output, Section, State, Update, action, badge, progress, spinner, status_bar, text,
  };
keywords: [footer, status line, indicators]
related: [toast, progress, spinner, badge]
hero: status-bar/background
stories: [status-bar/background]
api:
  - name: "State::new() / .with_items(items)"
    description: "An empty bar, or one starting with items, as if each had been set in turn."
  - name: "text(id, label) / action(id, label) / spinner(id, label) / progress(id, value) / badge(id, label, variant)"
    description: "Muted text, a small clickable ghost button, a spinner with a label, a short progress bar and a badge."
  - name: ".section(Section) / .icon(glyph) / .label(text) / .tooltip(text)"
    description: "Places an item at the Start (the default), Centre or End, adds an icon, a label before a progress bar, or a tooltip."
  - name: "Update::Set(item) / Update::Remove(id) / Update::Clear"
    description: "What producers send. Set replaces the item with the same id where it stands, or adds it at the end of its section."
  - name: "status_bar::subscription()"
    description: "Owns the bounded channel under a stable id. Emits Ready(sender) once, then batches of up to BATCH_SIZE (16) updates."
  - name: "status_bar::animation(&state)"
    description: "Window frames while a spinner is showing, and nothing otherwise."
  - name: "state.update(Event)"
    description: "Handles Ready, Received, Frame and Press. Returns Output::Ready(sender) once the channel is open and Output::Pressed(id) for a clickable item."
  - name: "state.apply(update) / state.items() / state.section(section) / state.item(&id)"
    description: "Applies one update directly, and reads the items back."
  - name: "status_bar(&state)"
    description: "Renders the three sections. Clickable items render disabled until .on_event is set."
  - name: ".on_event(f) / .compact_below(f32)"
    description: "Maps status bar events to your message, and sets the width below which the items flow onto several lines (480 by default)."
---

Status usually comes from outside the UI thread: a sync job, a build, the language server or the editor's cursor. The status bar follows the same channel pattern as [toasts](../toast/). `status_bar::subscription()` creates a bounded channel under a stable id, so iced keeps one stream however often you ask for it, and sends `Event::Ready` with the sender first. Keep the sender and hand clones to producers, which send `Update`s. The receiver drains whatever is waiting in batches, so a burst of cursor moves costs one redraw.

Every item has an id. Sending `Update::Set` with an id that is already there replaces that item in place, so a spinner can become a check mark, or a progress bar a badge, without the bar jumping about. `Update::Remove` takes an item away. Items keep the order they were first set within their section.

Subscribe to `status_bar::animation(&state)` as well to turn the spinners. It asks for window frames only while a spinner is showing. The example's worker waits with `futures-timer` rather than a thread, so it also runs in the browser.

On a wide window the start section sits at the left, the end section at the right and the centre section between them. Below 480 pixels the items flow onto as many lines as they need instead, so nothing is cut off on a phone.
