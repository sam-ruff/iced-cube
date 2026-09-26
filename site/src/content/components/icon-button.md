---
title: Icon Button
description: A square button that shows only an icon, with a label and an optional pressed state.
group: Actions
order: 2
module: primitives::icon_button
imports: |
  use iced_cube::{icon_button, lucide};
  use iced_cube::primitives::button::Size;
  use iced_cube::primitives::icon_button::Variant;
keywords: [toolbar, icon only, pressed]
related: [button, tooltip]
hero: icon-button/toolbar
stories: [icon-button/toolbar, icon-button/variants, icon-button/sizes, icon-button/disabled]
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
    description: "Primary, Secondary, Destructive, Outline or Ghost: the text button variants without Link. Defaults to Ghost."
  - name: ".size(Size)"
    description: "The button Size: Sm is 32 pixels square, Md is 36 and Lg is 40. Defaults to Md."
  - name: ".on_press(message) / .on_press_maybe(option)"
    description: "Sets the message emitted on press. None disables the button."
  - name: ".id(id)"
    description: "Sets a widget id, for clicking the button in tests."
  - name: "colours(tokens, variant, pressed, status) / style(...)"
    description: "The resolved colours and iced style, for reuse in custom widgets."
---

Give every icon button a label. The icon alone rarely says enough, and the label shows as a tooltip so people can check what a button does before they press it. iced 0.14 has no screen reader support, so the label cannot reach assistive technology yet.

A pressed icon button is a toggle: the app holds the on or off value and passes it to `.pressed`, and the button's message flips it. Use pressed buttons for formatting in a toolbar, or for a group where exactly one option is on, such as text alignment.

Icon buttons share their sizes and colours with [Button](../button/). There is no Link variant, because a link needs text to read as one; on an icon alone it would look the same as Ghost. The `icon-button` feature turns on `button` and `tooltip`.

A disabled icon button draws its icon at half opacity, like the label of a disabled text button.
