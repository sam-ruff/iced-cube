---
title: Image compare
description: Two images of the same scene side by side, blended, or split by a divider, with outlines marking regions of either.
group: Data
order: 8
module: data::image_compare
imports: |
  use iced_cube::data::image_compare::{self, Mode, Outline, Tone};
keywords: [before and after, diff, screenshot, visual diff, comparison, divider, swipe]
related: [timeline, avatar]
hero: image-compare/slider
stories: [image-compare/slider, image-compare/modes, image-compare/outlines, image-compare/keyboard]
api:
  - name: "image_compare(before, after)"
    description: "Compares two images, in Mode::Slider with the divider in the middle. Both take an image::Handle."
  - name: "image_compare::single(image)"
    description: "Shows one image, with outlines and pointer reports but no divider."
  - name: ".mode(mode)"
    description: "Mode::Slider, Mode::SideBySide or Mode::Overlay."
  - name: ".divider(fraction) / .on_divider(f)"
    description: "Where the divider sits, from 0 at the left edge to 1 at the right, and the message sent with its new place while it is dragged. Without on_divider it stays put."
  - name: ".opacity(fraction)"
    description: "How opaque the before image is over the after image in Mode::Overlay. 0.5 by default."
  - name: ".outline(outline) / .outlines(iter)"
    description: "Rectangles drawn over every image, in image pixels. Outline::new(rect).tone(tone) picks the colour: Primary, Success, Warning or Destructive."
  - name: ".on_hover(f) / .on_press(f)"
    description: "The image pixel under the pointer when it changes (None once it leaves), and under a press away from the divider."
  - name: ".width(length) / .height(length)"
    description: "Fills the width and follows the images' shape by default. A fixed height letterboxes the images, centred."
  - name: ".pixelated(bool)"
    description: "Draws pixels as hard squares rather than smoothing them, for screenshots compared at a large zoom."
  - name: ".label(text)"
    description: "The name tests and assistive tools know it by, Image compare by default."
  - name: "image_compare::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.apply(divider)"
    description: "The divider position after an Action (Left, Right, Start or End). Send it through your on_divider message."
---

Both images are fitted into the same frame using the after image's size, so they should share a shape. In side by side mode each gets half the width, with the before image on the left.

Outlines and pointer positions use image pixels rather than screen points, so an app that knows where something is in a screenshot can mark it, and a press can be matched against the same regions.

Handles built with `Handle::from_rgba` need nothing more. To show PNG or JPEG files, enable iced's `image` feature in your app. The divider shortcuts are app-wide, as images do not take keyboard focus.
