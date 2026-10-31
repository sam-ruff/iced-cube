---
title: Floating action button
description: The main action of a screen, raised above the content in a corner.
group: Actions
order: 5
module: primitives::fab
imports: |
  use iced::widget::{container, stack};
  use iced_cube::{fab, lucide};
keywords: [fab, floating button, compose, mobile]
related: [button, icon-button, bottom-nav]
hero: fab/over-list
stories: [fab/over-list, fab/extended]
api:
  - name: "fab(glyph)"
    description: "Creates a 56 pixel square button with a large radius, primary colours and a soft shadow. It renders disabled until it has a message."
  - name: ".label(text)"
    description: "Shows the text beside the icon, making the fab an extended pill with 16 pixels of padding each side."
  - name: ".tooltip(text)"
    description: "Names the action of an icon-only fab, shown as a tooltip on hover. An extended fab ignores it."
  - name: ".on_press(message) / .on_press_maybe(option)"
    description: "Sets the message for a click or tap. None disables the fab."
  - name: ".id(id)"
    description: "Sets a widget id, for finding the fab in tests."
  - name: "fab::style(tokens, status)"
    description: "The button style for a status, for drawing something to match."
---

A fab does not place itself. Lay it over the screen with an iced `stack`, in a `container` aligned to the corner you want, as the first example does. Pad the container so the fab clears the edge and anything else that floats there, such as a [bottom navigation](../bottom-nav/) bar.

Use one fab per screen, for the action people come to the screen for, such as writing a new note. Give an icon-only fab a `.tooltip(...)` so the action has a name. Use `.label(...)` when the icon alone is not obvious, or when there is room, such as at the top of a long list.

The fab grows its shadow on hover and flattens it a little while pressed. It answers a tap as well as a click.
