---
layout: ../../layouts/Guide.astro
title: Icons
description: The full Lucide icon set, resolved at compile time.
---

## Finding an icon

Browse the set at [lucide.dev/icons](https://lucide.dev/icons). Names there are kebab-case; in Rust they are PascalCase, so `chevron-right` becomes `ChevronRight` and `arrow-down-0-1` becomes `ArrowDown01`. The icons come from the version of Lucide bundled with the crate, so an icon added to Lucide very recently may not be there yet. The compiler will tell you if a name does not exist.

## Using an icon

The `lucide!` macro turns a name into a `Glyph`, the embedded SVG for one icon:

```rust
use iced_cube::icon::icon;
use iced_cube::{button, lucide};

let save = lucide!(Save);

let alone = icon(save, 16.0);
let on_a_button = button("Save").icon(save).on_press(Message::Save);
```

Components that show icons, such as buttons, badges, alerts and tabs, take a `Glyph` directly.

## `Icon` and `icon()`

Two items have nearly the same name:

- `Icon` (re-exported as `iced_cube::Icon`) is the enum of every Lucide icon. `lucide!(Save)` is shorthand for `Glyph::new(Icon::Save)` evaluated in a constant.
- `icon()` (in `iced_cube::icon`) is a function that renders a `Glyph` as an iced `svg` widget at a given size.

You rarely need `Icon` yourself. It is useful when you want to name a glyph once as a constant:

```rust
use iced_cube::icon::Glyph;
use iced_cube::Icon;

const TRASH: Glyph = Glyph::new(Icon::Trash);
```

## Only what you use

Each `lucide!` call expands to a constant, so only the SVGs you reference end up in your binary. The icons are embedded in the crate, so nothing is downloaded at build time and offline builds work.

## Colour

`icon()` draws in the theme's foreground colour. `icon::tinted(glyph, size, Some(colour))` sets a fixed colour instead. Components tint their own icons to match their text.
