---
title: Touch clicks
description: Lets mouse-only content, such as rich text links or a text editor, respond to touch.
group: Actions
order: 7
module: primitives::touch_clicks
imports: |
  use iced_cube::touch_clicks;
keywords: [touch, tap, links, markdown, rich text, text editor, mobile]
related: [tappable, textarea]
hero: touch-clicks/rich-text
stories: [touch-clicks/rich-text]
api:
  - name: "touch_clicks(content)"
    description: "Wraps content. It gets every event unchanged, and a tap also reaches it as a left click at the touch point."
  - name: ".slop(pixels)"
    description: "How far a finger may move and still be a tap. Defaults to DEFAULT_SLOP (8)."
  - name: ".scroll(true)"
    description: "For content that scrolls itself, such as a text editor: a vertical drag that starts on it becomes wheel scrolling for the content, and the finger's moves are captured so an outer scrollable stays put."
  - name: ".line_height(pixels)"
    description: "The content's line height, so a scroll-mode drag moves its text with the finger. Defaults to DEFAULT_LINE_HEIGHT (21), a textarea's."
  - name: "touch_clicks::wheel_delta(dy, line_height)"
    description: "The wheel delta a scroll-mode drag of dy pixels sends."
---

Some of iced's own widgets read only the mouse: the text editor, rich text and so markdown. On a touch screen you cannot place the caret in them or tap a link. Turning every touch into a mouse event for the whole window would fix those and break the rest, because widgets that already read touch, such as buttons and [tappables](../tappable/), would then react twice. Wrap just the mouse-only content instead.

For a finger that goes down and lifts within the slop, the content gets the cursor moved to the touch point, then a left press and release there, so a link opens or the caret moves. A tap anywhere in the window is passed on this way, so tapping outside a focused editor unfocuses it, as a click would. A tap on a layer drawn over the content, such as a menu, reaches it as a click elsewhere, so a link under the menu does not open. A finger that moves further is a drag. By default the content gets nothing extra, so a list around rich text scrolls as usual and no link opens.

Markdown views are wrapped the same way as the rich text example: `touch_clicks(markdown::view(&items, settings).map(Message::Link))`. A [textarea](../textarea/) is already wrapped, with scrolling on.

The touch press and lift are never captured, and only the first finger counts: a second finger cancels the tap. Some platforms already turn touches into mouse events for an app; there, wrapped content would get each tap twice, so leave it unwrapped.
