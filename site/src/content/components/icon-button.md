---
title: Icon Button
description: A square button that shows only an icon, with a label and an optional pressed state.
group: Actions
order: 2
module: primitives::icon_button
imports: |
  use iced_cube::{icon_button, lucide};
  use iced_cube::primitives::button::{Size, Variant};
keywords: [toggle, toolbar, icon]
related: [button, tooltip]
hero: icon-button/toolbar
stories: [icon-button/toolbar, icon-button/variants, icon-button/sizes]
api:
  - name: "icon_button(glyph)"
    description: "Creates a square ghost button showing a Lucide icon. It renders disabled until it has a message."
  - name: ".label(text)"
    description: "Names the action, such as \"Bold\". The label shows as a tooltip on hover."
  - name: ".tooltip(Option<Position>)"
    description: "Where the label's tooltip appears, or None to never show it. Defaults to Some(Position::Top)."
  - name: ".pressed(bool)"
    description: "Draws the button as switched on, for toolbar toggles. Pass the value your app holds."
  - name: ".variant(Variant)"
    description: "The same variants as a text button. Defaults to Ghost."
  - name: ".size(Size)"
    description: "Sm is 32 pixels square, Md and Icon are 36 and Lg is 40. Defaults to Icon."
  - name: ".on_press(message) / .on_press_maybe(option)"
    description: "Sets the message emitted on press. None disables the button."
  - name: ".id(id)"
    description: "Sets a widget id, for clicking the button in tests."
  - name: "colours(tokens, variant, pressed, status) / style(...)"
    description: "The resolved colours and iced style, for reuse in custom widgets."
---

Give every icon button a label. The icon alone rarely says enough, and the label shows as a tooltip so people can check what a button does before they press it. iced 0.14 has no screen reader support, so the label cannot reach assistive technology yet.

A pressed icon button is a toggle: the app holds the on or off value and passes it to `.pressed`, and the button's message flips it. Use pressed buttons for formatting in a toolbar, or for a group where exactly one option is on, such as text alignment.

Icon buttons share their variants, sizes and colours with [Button](../button/). The `icon-button` feature turns on `button` and `tooltip` for that reason.
