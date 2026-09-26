---
layout: ../../layouts/Guide.astro
title: Introduction
description: Themeable application components for iced, with Lucide icons built in.
---

iced-cube is a set of components for [iced](https://iced.rs), the Rust GUI library. It covers buttons, form controls (inputs, textareas, checkboxes, radios, switches, selects, sliders and labelled fields), layout pieces (cards, stacks, separators, scroll areas and accordions), tabs, tooltips, toasts, and feedback such as alerts, badges, progress bars and spinners. They all draw from one set of design tokens, so they look like they belong together in light and dark mode.

More components are on the way. The [Status](status/) page lists what is planned.

For AI agents and other tools, the whole documentation is also available as plain Markdown with the examples as code, indexed at [llms.txt](../llms.txt) with everything in one file at [llms-full.txt](../llms-full.txt).

## How it fits with iced

Every component is an ordinary iced widget, or a small composition of them. You build a component with a function, set options on it, and convert it into an `Element` like anything else:

```rust
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

let save = button("Save")
    .icon(lucide!(Save))
    .variant(Variant::Primary)
    .on_press(Message::Save);
```

There is no separate runtime and no global state. The only macro is `lucide!`, which picks an icon at compile time. Your application still owns its state and handles its own messages.

## Principles

- **You own the state.** Components that need state, such as tabs or toasts, give you a plain struct with a pure `update` function. You store it and pass it back in.
- **Colours come from the theme.** Every colour is read from the active theme, so custom palettes and dark mode need no extra work.
- **Channels for background work.** Toasts raised by other threads arrive through a subscription that owns a channel. See [Subscriptions](subscriptions/).
- **Tested.** Each component ships with unit tests, headless interaction tests and image snapshots in both themes.

## Live previews

The previews are the real Rust examples compiled to WebAssembly. The code under each preview is the exact file that produced it.
