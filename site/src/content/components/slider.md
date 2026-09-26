---
title: Slider
description: Chooses a number within a range by dragging a handle.
group: Forms
order: 7
module: primitives::slider
imports: |
  use iced_cube::slider;
related: [progress, field]
hero: slider/default
stories: [slider/default, slider/steps, slider/keyboard, slider/disabled]
api:
  - name: "slider(range, value)"
    description: "Creates a slider over an inclusive range. The value is clamped to the range. It renders disabled until it has a message."
  - name: ".on_change(f)"
    description: "Sets the message emitted with the new value while dragging."
  - name: ".on_release(message)"
    description: "Sets a message emitted once the handle is released, useful for saving the final value."
  - name: ".step(value)"
    description: "Snaps the value to multiples of the step. Zero or negative steps are ignored."
  - name: ".label(text)"
    description: "Shows a label above the slider."
  - name: ".show_value() / .format_value(f)"
    description: "Shows the current value above the slider, as plain text or formatted by a closure."
  - name: ".width(length)"
    description: "Overrides the width. Sliders fill the available width by default."
  - name: "slider::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.apply(value, range, step)"
    description: "The value after an Action (Increase, Decrease, Min or Max), snapped to the step grid. Send it through your on_change message."
  - name: "slider::stepped(value, range, step, steps)"
    description: "Moves a value by whole steps within a range. Without a valid step it moves by 1, as dragging does."
---

Sliders work with `f32`, `f64`, `u8`, `u16`, `u32`, `i16` and `i32`: any `Copy` number that converts from `u8` and into `f64` without loss. That rules out `i8`, 64-bit integers and `usize`.

Use sliders where an approximate value is fine, such as volume or opacity. When the exact number matters, show the value with `.show_value()` or `.format_value(f)`.

iced sliders do not take keyboard focus, so shortcuts are app-wide. With several sliders on screen, the app decides which one is current and applies the resolved action to that value.
