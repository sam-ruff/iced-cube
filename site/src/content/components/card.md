---
title: Card
description: Groups related content on a raised surface, with an optional header and footer.
group: Layout
order: 1
module: layout::card
imports: |
  use iced_cube::card;
related: [stack]
hero: card/basic
stories: [card/basic, card/stats]
api:
  - name: "card()"
    description: "Creates an empty card. Every section is optional."
  - name: ".title(text) / .description(text)"
    description: "Sets the header: a heading and a muted line under it."
  - name: ".body(element)"
    description: "Sets the main content."
  - name: ".footer(element)"
    description: "Sets the content under the body, usually a row of buttons."
  - name: ".width(length)"
    description: "Overrides the width. Cards shrink to fit by default."
---

A card is a container with a soft border, rounded corners and consistent padding. A broad, faint shadow separates it from the page; in dark themes the surface is also a shade lighter so it still reads as raised.

The footer takes any element, so you decide how actions are laid out. Buttons inside a card work exactly as they do anywhere else. A link button has no padding at its sides, so a "View all" link in the footer lines up with the body above it.

The body fills the card when it contains something that fills, such as a row with a spacer. A `vstack` or `hstack` follows its children the way an iced `column` does, so a stat card whose header row pushes an icon to the right needs no extra width settings. Give the card itself a width, or put it in a layout that sizes it, since a card on its own shrinks to fit.
