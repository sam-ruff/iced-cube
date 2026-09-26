---
title: Spinner
description: An indeterminate loading indicator.
group: Feedback
order: 4
module: feedback::spinner
imports: |
  use iced_cube::spinner;
  use iced_cube::feedback::spinner::{Size, advance};
keywords: [loader, loading]
related: [progress]
hero: spinner/loading
stories: [spinner/loading, spinner/sizes]
api:
  - name: "spinner(phase)"
    description: "Creates a spinner at a phase measured in turns, from 0.0 to 1.0."
  - name: ".size(Size)"
    description: "Sm, Md or Lg. Defaults to Md."
  - name: "advance(phase, elapsed)"
    description: "Returns the phase moved on by the elapsed time. One turn takes PERIOD."
---

The spinner has no clock of its own. Your app keeps the phase in its state and moves it on with `advance` each frame, which keeps the spinner a pure function of state and makes it easy to test.

Subscribe to `iced::window::frames()` only while something is loading, and return `Subscription::none()` otherwise. That way an idle app does not redraw at all.

Several spinners can share one phase, as the rows of a job list do. The arc steps through `FRAMES` fixed positions per turn and is drawn as a cached image, so any number of spinners on screen stay cheap and look the same on every renderer, including WebGL in the browser.
