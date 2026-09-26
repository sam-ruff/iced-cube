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

A card is a container with a border, rounded corners and consistent padding. In light themes it sits on the page background with a faint shadow; in dark themes the surface is a shade lighter than the page so it still reads as raised.

The footer takes any element, so you decide how actions are laid out. Buttons inside a card work exactly as they do anywhere else.
