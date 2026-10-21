---
title: Log viewer
description: A live log that streams entries from threads or tasks, with level filters, search, follow, pause and copy, and a cap on how many lines it keeps.
group: Data
order: 4
module: data::log_viewer
imports: |
  use iced_cube::data::log_viewer::{self, Event, Level, Output, State, entry, log_viewer};
keywords: [logs, console, output, terminal, tail, stream, trace, search]
related: [virtual-list, status-bar, toast]
hero: log-viewer/live
stories: [log-viewer/live, log-viewer/filtering, log-viewer/keyboard]
api:
  - name: "State::new()"
    description: "An empty log that keeps 10,000 lines, shows every level and follows the newest line."
  - name: ".with_limit(n) / .with_levels(levels) / .with_wrap(bool) / .with_search_id(id)"
    description: "The most lines kept, the levels shown, whether long lines wrap, and the search field's widget id when several logs share a screen."
  - name: "entry(level, message)"
    description: "One line. Add .timestamp(text) and .target(text); the timestamp is shown as given, so the app picks the format."
  - name: "state.update(Event)"
    description: "Handles arriving entries, presses, the search, filters, follow, pause, wrap, clear and the keyboard. Returns Output::Ready(sender) once the channel is open, Copy(text) for the clipboard and FocusSearch."
  - name: "state.extend(entries)"
    description: "Appends entries directly, as the channel does. While paused they are held back."
  - name: "log_viewer::subscription() / log_viewer::Sender"
    description: "The channel entries arrive on. It sends Event::Ready(sender) first; producers send entries through clones of the sender, and they arrive in batches as Event::Received."
  - name: "log_viewer::focus_search(&state)"
    description: "The task that focuses the search field, for answering Output::FocusSearch."
  - name: "state.entries() / state.visible() / state.entry(id)"
    description: "Every kept entry, the ids of the lines shown, and one entry by id."
  - name: "state.count(level) / state.matches() / state.current_match()"
    description: "How many kept lines have a level, the lines matching the search, and the match the search last moved to with its position."
  - name: "state.selected() / state.can_copy() / state.copy_text()"
    description: "The selected lines, whether there is anything to copy, and the text Copy returns: the selection, or the highlighted line."
  - name: "state.is_following() / state.is_paused() / state.pending() / state.unseen()"
    description: "Whether the log follows its newest line, whether it is paused, how many lines are held back, and how many arrived since it stopped following."
  - name: "log_viewer(&state)"
    description: "Renders the toolbar and the lines. Everything renders disabled until .on_event is set."
  - name: ".width(length) / .height(length)"
    description: "The log's size, toolbar included. It fills by default."
  - name: ".toolbar(bool) / .placeholder(text) / .empty(text)"
    description: "Hides the toolbar, which leaves its actions on the keyboard, and sets the search placeholder and what shows with no lines."
  - name: ".keymap(keymap)"
    description: "Replaces the default keymap the focused log resolves keys with."
  - name: "log_viewer::default_keymap() / action.event(&state)"
    description: "The default shortcuts, and the Event a resolved Action sends. State::key_event does both."
  - name: "log_viewer::level_colour(tokens, level) / line_style(tokens, level, current, Status)"
    description: "The colour of each level's tag, and a line's colours: list row colours, a tint on error lines and the search marks, stronger on the current match."
---

A log viewer shows output from a running program as it happens. The app owns a `log_viewer::State` and routes the viewer's events into it. `log_viewer::subscription()` owns a bounded channel and sends `Event::Ready` with its sender first; hand clones of the sender to threads or tasks and send `entry(level, message)` values through them. Entries arrive in batches, so a burst of lines costs one update rather than one per line.

The log keeps the newest 10,000 lines by default and drops the oldest after that, so memory stays bounded however long the program runs. Only the lines in view are built. It follows the newest line until you scroll up, then shows a button to jump back with a count of the lines that arrived since. Pausing holds new lines back, bounded by the same limit, until you resume.

The level buttons show how many kept lines have each level and hide or show them. The search matches messages, ignoring the case of Latin letters, marks every match and counts them, and Enter or F3 moves to the next match, Shift with either to the previous, wrapping at the ends. Long lines are cut at the edge unless wrapping is on. Below 560 pixels the toolbar stacks and each message sits under its timestamp, level and target, so it keeps the full width.

Click, Shift and Ctrl select lines, and Ctrl+C or the copy button returns `Output::Copy` with them as plain text, one per line. The library never touches the clipboard itself: write the text with `iced::clipboard::write` or your own platform code. Ctrl+F returns `Output::FocusSearch`; answer it with `log_viewer::focus_search(&state)`.

The log resolves its keys while any part of it has focus, the search field included, which keeps the keys typing needs. Timestamps and levels sit in fixed columns so lines line up, and every part uses the font the app registers with `theme::set_font`.
