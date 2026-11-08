---
title: Rich Text Editor
description: Formatted text editing, drawn as it will look, with HTML and plain-text output for mail.
group: Forms
order: 10
module: forms::rich_text_editor
imports: |
  use iced_cube::rich_text_editor::{self, Action, Content, rich_text_editor};
  use iced_cube::rich_text_editor::{Shortcut, Shortcuts};
keywords: [wysiwyg, formatted text, compose, mail body, html editor, bold, italic, underline]
related: [formatting-toolbar, textarea, input]
hero: rich-text-editor/compose
stories: [rich-text-editor/compose, rich-text-editor/shortcuts, rich-text-editor/placeholder, rich-text-editor/output]
api:
  - name: "Content::new() / Content::with_text(text) / Content::with_html(html)"
    description: "The document, selection and undo history your app owns. with_html reads a saved draft or a reply; unknown elements keep their text and scripts are dropped."
  - name: "content.perform(action)"
    description: "Applies an Action from the editor, the formatting toolbar or your own code. Every change, formatting included, can be undone."
  - name: "content.html() / content.plain_text() / content.text()"
    description: "The HTML fragment to send, the plain-text alternative for multipart/alternative mail, and the bare text."
  - name: "content.formatting() / content.has_formatting()"
    description: "The formatting at the caret or across the selection, and whether anything is formatted at all, so plain mail can stay plain."
  - name: "rich_text_editor(&content)"
    description: "Creates the editor. It renders disabled until it has on_action."
  - name: ".on_action(fn) / .on_action_maybe(option)"
    description: "Sets the message built from each Action. Apply it with content.perform(action) in update."
  - name: ".on_link(fn)"
    description: "Sets the message sent with a link's address when it is clicked with Ctrl, or Cmd on macOS, held."
  - name: ".placeholder(text) / .id(id) / .invalid(bool)"
    description: "Text shown while the document is empty, the widget id for focusing, and a destructive border."
  - name: ".height(length) / .min_height(px) / .max_height(px)"
    description: "By default the editor starts about four lines tall and grows up to max_height. A fixed or filling height scrolls once the text overflows."
  - name: ".width(length) / .text_size(px) / .padding(padding)"
    description: "The width (Fill by default), the size of normal text, which the other sizes scale from, and the inner padding."
  - name: ".shortcuts(Shortcuts) / .keymap(keymap)"
    description: "Which shortcuts the editor handles, all by default, and the keys they are bound to, starting from rich_text_editor::default_keymap()."
  - name: "Format::Toggle(Mark) / Size(Size) / Block(BlockKind) / Link(address) / Unlink / Clear"
    description: "The formatting actions, as Action::Format. They apply to the selection, or to what is typed next when nothing is selected."
  - name: "html::to_html(&doc) / html::from_html(&str) / plain::to_plain(&doc)"
    description: "The conversions on their own, for a Document you hold outside a Content."
keyboard:
  - keys: "Arrow keys, Home, End, Page Up, Page Down"
    action: "Moves the caret. Hold Shift to select."
  - keys: "Ctrl + Arrow, Ctrl + Home / End (Alt and Cmd on macOS)"
    action: "Moves by word, or to the start or end of the text."
  - keys: "Ctrl + A, C, X, V (Cmd on macOS)"
    action: "Selects all, copies, cuts and pastes. Copies are plain text; pasting one back into the editor keeps its formatting."
  - keys: "Enter"
    action: "Starts a new line. In an empty list item it ends the list."
  - keys: "Backspace at the start of a list item"
    action: "Turns the item back into a plain line."
---

The editor shows formatting as it will look: no marks or tags appear in the text. Bold, italic, underline and strikethrough apply to any selection, even part of a word. Text comes in four sizes (small, normal, large and huge), lines can be bulleted or numbered, and links show underlined. Quoted lines from a reply load as a quote and stay one.

Your app owns a `Content` and applies each `Action` it receives, as with the [textarea](../textarea/). The content keeps the undo history too, so a format applied from the [formatting toolbar](../formatting-toolbar/) or from your own code is undone like typing. Typing undoes a word at a time.

For mail, send `content.html()` as the HTML part and `content.plain_text()` as the plain-text alternative: bold becomes `*bold*`, italic and underline `_text_`, links are followed by their address and list items keep a `- ` or their number. `Content::with_html` reads the same HTML back for drafts, and reads what other mail clients write closely enough to keep their formatting. The output is a small, safe subset: only `http`, `https` and `mailto` links survive, and every other attribute is dropped.

The shortcuts are the same `Shortcut` set the [input](../input/) and textarea take, with the formatting chords added. Turn one off with `.shortcuts(Shortcuts::all().without(Shortcut::Bold))`, or all of them with `Shortcuts::none()`; their keys then reach your app, as the shortcuts example shows. Ctrl+K sends `Action::RequestLink`, which the formatting toolbar turns into its address field.

While the editor has focus it keeps its chords, so a few app-wide defaults wait until focus leaves it: Ctrl+K also toggles the [command palette](../command-palette/) and Ctrl+B the [sidebar](../sidebar/) rail. If your app needs those keys while someone types, turn the editor's shortcut off, for example `.shortcuts(Shortcuts::all().without(Shortcut::Link))`, or bind it elsewhere with `.keymap(...)`.

Links are drawn in the theme's primary colour and underlined. With the default themes that colour is close to the text colour, so a link and underlined text look alike; a theme with a coloured primary tells them apart.

Bold is drawn with `theme::semibold()` and italic with the italic face of `theme::font()`, so register an italic face of your font alongside the regular and semibold ones, or italic text will stand upright. The examples here load Inter's italic faces.

On a touch screen a tap places the caret and a vertical drag scrolls the text.
