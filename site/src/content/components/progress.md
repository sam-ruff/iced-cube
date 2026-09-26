---
title: Progress
description: A determinate progress bar with an optional label and percentage.
group: Feedback
order: 3
module: feedback::progress
imports: |
  use iced_cube::progress;
  use iced_cube::feedback::progress::{Size, Variant};
related: [spinner, slider]
hero: progress/variants
stories: [progress/variants, progress/sizes, progress/interactive]
api:
  - name: "progress(value)"
    description: "Creates a bar from a value between 0.0 and 1.0. Values outside that range are clamped, and NaN counts as 0.0."
  - name: ".label(text)"
    description: "Shows a caption above the bar."
  - name: ".show_percentage(bool)"
    description: "Shows the rounded percentage above the bar, on the right."
  - name: ".variant(Variant)"
    description: "Default, Success, Warning or Destructive. Defaults to Default."
  - name: ".size(Size)"
    description: "Sm, Md or Lg thickness. Defaults to Md."
  - name: ".width(length)"
    description: "Overrides the width. Bars fill the available width by default."
---

Use a progress bar when you know how much work is left. When you do not, show a spinner instead.

The bar is stateless: keep the value in your app and pass it in on every view. The `clamp` and `percentage` helpers in the module are there if you need the same maths elsewhere.
