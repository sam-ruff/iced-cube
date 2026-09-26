---
title: Scroll area
description: A scrollable region with a thin, rounded scrollbar that follows the theme.
group: Layout
order: 4
module: primitives::scroll_area
imports: |
  use iced_cube::scroll_area;
  use iced_cube::primitives::scroll_area::Direction;
keywords: [scrollable]
related: [stack, card]
hero: scroll-area/vertical
stories: [scroll-area/vertical, scroll-area/horizontal, scroll-area/both]
api:
  - name: "scroll_area(content)"
    description: "Wraps content in a vertically scrolling area."
  - name: ".direction(Direction)"
    description: "Vertical, Horizontal or Both. Defaults to Vertical."
  - name: ".width(length) / .height(length)"
    description: "Sets the size of the visible region. Give the scrolling axis a fixed or fill length."
  - name: ".id(id)"
    description: "Sets the widget id, so the area can be scrolled with iced's scrolling tasks."
---

A scroll area only scrolls when its content is larger than the space it is given, so set a height for vertical scrolling and a width for horizontal scrolling.

In a single direction the scrollbar sits beside the content and never covers it. When scrolling both ways the two scrollbars float over the right and bottom edges, so leave a little padding there.

The mouse wheel, trackpad and dragging the scrollbar all work as usual.
