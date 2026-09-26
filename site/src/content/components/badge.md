---
title: Badge
description: A short status or category marker.
group: Feedback
order: 2
module: feedback::badge
imports: |
  use iced_cube::badge;
  use iced_cube::feedback::badge::Variant;
keywords: [tag, chip, pill]
related: [alert]
hero: badge/variants
stories: [badge/variants, badge/with-icon, badge/in-a-list]
api:
  - name: "badge(label)"
    description: "Creates a pill-shaped badge with a text label."
  - name: ".variant(Variant)"
    description: "Default, Secondary, Outline, Destructive, Success or Warning. Defaults to Default."
  - name: ".icon(glyph)"
    description: "Adds a small Lucide icon before the label."
---

Badges are for short labels that sit next to other content: a status in a table row, a tag on a card, a version number. Keep the text to a word or two.

Success, Warning and Destructive map to the theme's status colours, so use them only when the badge actually reports a state. For neutral tags, Secondary and Outline stay out of the way.

Badges are not interactive. If a label needs to respond to clicks, use a button instead.
