---
title: Selectable text
description: Rich text that can be selected with the mouse across paragraphs and copied, with links that still open.
group: Data
order: 5
module: data::selectable_text
imports: |
  use iced_cube::data::selectable_text::{self, selectable_text, selection_area};
keywords: [select text, copy, highlight, read-only text, markdown, rich text, drag select, clipboard]
related: [context-menu, scroll-area, rich-text-editor]
hero: selectable-text/article
stories: [selectable-text/article, selectable-text/menu]
api:
  - name: "selectable_text(spans)"
    description: "Rich text made of the spans iced's rich text takes. On its own it draws the text and opens links; inside a selection area it can be selected. Text without links needs the link type named: selectable_text::<String, _>(...)."
  - name: ".size(pixels) / .line_height(height) / .font(font) / .color(colour)"
    description: "The size, line height, font and colour of spans that set none."
  - name: ".width(length)"
    description: "The width of the text. Defaults to Shrink, which wraps at the width available."
  - name: ".on_link_click(fn(Link) -> Message)"
    description: "Sent when a link is clicked. A drag that starts on a link selects instead. Without it links are drawn but do nothing."
  - name: "selection_area(content)"
    description: "Lets every selectable text in the content be selected together: drag, double click for a word, triple click for a block. A press outside the area drops the selection."
  - name: ".on_copy(fn(String) -> Message)"
    description: "Sent with the selected text when the Copy shortcut is pressed. Blocks are joined with a line break. Nothing is sent while nothing is selected."
  - name: ".on_select(fn(String) -> Message)"
    description: "Sent with the selected text each time a selection is finished, for a primary selection or a status line."
  - name: ".style(fn(&Theme) -> Style)"
    description: "On a selectable text, replaces the selection colour."
  - name: ".double_click(duration)"
    description: "On the area, how soon a press must follow the last to count as a double or triple click. Defaults to DOUBLE_CLICK (400 ms)."
  - name: ".id(id)"
    description: "The id the selected and select_all tasks find the area by."
  - name: ".keymap(keymap)"
    description: "Replaces the default shortcuts."
  - name: "selectable_text::selected(id)"
    description: "A task that produces the text selected in the area with that id, empty when nothing is. For a Copy item in a menu."
  - name: "selectable_text::select_all(id)"
    description: "A task that selects everything in the area with that id and produces it. For a Select all item in a menu."
  - name: "style(&Tokens) -> Style"
    description: "The selection colour, the same as a text field's."
  - name: "SCROLL_SPEED / SCROLL_GAIN / MAX_SCROLL_SPEED"
    description: "How fast a drag past the edge of a scrollable scrolls it: 200 pixels a second at the edge, 12 more for each pixel further out, up to 2400."
---

iced draws rich text and markdown that cannot be selected. Build the same spans into `selectable_text` and put a `selection_area` around all of them. The area owns the selection, so a drag that starts in a heading can end three paragraphs later, in a list or in a table cell. Buttons and other controls inside the area work as usual.

The area never touches the clipboard. It sends the text and the app writes it, with `iced::clipboard::write` for `on_copy` and, on Linux, `iced::clipboard::write_primary` for `on_select`.

Put the scrollable inside the area, not around it. A drag past its top or bottom edge then scrolls it, faster the further out the pointer is, and the selection follows.

Select all only answers once the area has been clicked, so it does not compete with a text field. Typing, Tab or another Ctrl or Cmd chord drops the selection, because those keys are meant for something else, and a focused field inside the area keeps the chords it uses. A field that a task gives focus, with no key or press, is the one case the area cannot see: clear the selection then if the two share chords.

Escape drops the selection and is used up doing so. With nothing selected it passes on, so one Escape in a dialog clears the selection and the next closes the dialog.

Text scrolled out of view is never under the pointer. A drag that leaves a scrollable selects up to the edge of what it shows, or on into the text after it.

For a right-click menu, give the area an id and wrap it in a [context menu](../context-menu/). A right-click leaves the selection as it is, and the menu's items run the `selected` and `select_all` tasks.

Selection is by mouse. A finger scrolls the page as before.
