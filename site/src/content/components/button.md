---
title: Button
description: Triggers an action, with consistent variants, sizes and states.
group: Actions
order: 1
module: primitives::button
imports: |
  use iced_cube::{button, lucide};
  use iced_cube::primitives::button::{Size, Variant};
related: [icon-button, tooltip]
hero: button/variants
stories: [button/variants, button/sizes, button/with-icon, button/counter, button/list]
api:
  - name: "button(label)"
    description: "Creates a text button. It renders disabled until it has a message."
  - name: ".variant(Variant)"
    description: "Primary, Secondary, Destructive, Outline, Ghost or Link. Defaults to Primary. Link has no side padding, so it lines up with nearby text."
  - name: ".size(Size)"
    description: "Sm, Md or Lg. Defaults to Md. The square Icon size is deprecated; use icon_button instead."
  - name: ".icon(glyph) / .trailing_icon(glyph)"
    description: "Adds a Lucide icon before or after the label."
  - name: ".on_press(message) / .on_press_maybe(option)"
    description: "Sets the message emitted on press. None disables the button."
  - name: ".width(length)"
    description: "Overrides the width. Buttons shrink to fit by default."
  - name: ".align_x(alignment)"
    description: "Where the icon and label sit in a button wider than them: Alignment::Start, Center or End. Defaults to Center."
keyboard:
  - keys: "Tab, Shift + Tab"
    action: "Moves keyboard focus to or from the button inside a dialog, or wherever the app runs iced's focus_next and focus_previous."
  - keys: "Enter, Space"
    action: "Presses the focused button."
---

Use one primary button per view for the main action, and secondary, outline or ghost buttons for everything else. Destructive buttons are for actions that delete or cannot be undone.

For a list of choices such as settings categories, give ghost buttons `.width(Length::Fill)` and `.align_x(Alignment::Start)` so their labels line up on the left, and mark the current one with the secondary variant.

For a button that shows only an icon, such as a toolbar toggle or a close button, use [Icon Button](../icon-button/).

An enabled button can take keyboard focus. A focused button draws a ring in the theme's ring colour, and Enter or Space presses it. Only the keyboard shows the ring: a click never focuses a button, and a click anywhere takes focus off it, as it does from a text field. Disabled buttons are skipped.

Icons come from Lucide through the `lucide!` macro, which resolves the icon at compile time so only the icons you use are included in your binary.
