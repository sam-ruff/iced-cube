---
title: Button
description: Triggers an action, with consistent variants, sizes and states.
group: Actions
order: 1
module: primitives::button
imports: |
  use iced_cube::{button, icon_button, lucide};
  use iced_cube::primitives::button::{Size, Variant};
related: [tooltip]
hero: button/variants
stories: [button/variants, button/sizes, button/with-icon, button/counter]
api:
  - name: "button(label)"
    description: "Creates a text button. It renders disabled until it has a message."
  - name: "icon_button(glyph)"
    description: "Creates a square, icon-only ghost button."
  - name: ".variant(Variant)"
    description: "Primary, Secondary, Destructive, Outline, Ghost or Link. Defaults to Primary."
  - name: ".size(Size)"
    description: "Sm, Md, Lg or Icon. Defaults to Md."
  - name: ".icon(glyph) / .trailing_icon(glyph)"
    description: "Adds a Lucide icon before or after the label."
  - name: ".on_press(message) / .on_press_maybe(option)"
    description: "Sets the message emitted on press. None disables the button."
  - name: ".width(length)"
    description: "Overrides the width. Buttons shrink to fit by default."
---

Use one primary button per view for the main action, and secondary, outline or ghost buttons for everything else. Destructive buttons are for actions that delete or cannot be undone.

Icons come from Lucide through the `lucide!` macro, which resolves the icon at compile time so only the icons you use are included in your binary.
