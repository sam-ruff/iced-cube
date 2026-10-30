---
title: Tappable
description: Makes any content respond to taps and long presses without stopping a list from scrolling.
group: Actions
order: 6
module: primitives::tappable
imports: |
  use iced_cube::tappable;
  use iced_cube::primitives::tappable::Status;
keywords: [touch, tap, long press, press and hold, list row, clickable]
related: [touch-clicks, button, bottom-nav]
hero: tappable/list
stories: [tappable/list]
api:
  - name: "tappable(content)"
    description: "Wraps content so it reports taps and long presses. Without a message it only draws the content."
  - name: ".on_tap(message)"
    description: "Sent when a finger or the left button is pressed and lifted without moving further than the slop."
  - name: ".on_long_press(message)"
    description: "Sent once when a press is held for the hold time, or on a right click. The lift that follows is ignored."
  - name: ".hold(duration)"
    description: "How long a press is held to count as a long press. Defaults to DEFAULT_HOLD (450 ms)."
  - name: ".slop(pixels)"
    description: "How far a press may move and still be a tap. Defaults to DEFAULT_SLOP (8)."
  - name: ".style(fn(&Theme, Status) -> container::Style)"
    description: "Replaces the default look, which is transparent at rest, lightly tinted on hover and more strongly while pressed. Status is Idle, Hovered or Pressed."
  - name: ".width(length) / .height(length) / .padding(padding)"
    description: "Sizes the pressable area around the content."
  - name: "tappable::Tracker"
    description: "The gesture logic on its own, for apps that want the same rules in a widget of their own."
---

A button captures the finger that presses it, so in a list made of buttons a drag that starts on a row goes nowhere: the list only scrolls from the gaps between rows. A tappable never captures a finger. It watches the press, and if the finger moves further than the slop before it lifts, the tap is cancelled and the scrollable around it scrolls as usual.

Use it for list rows, cards and anything else people tap on a phone. A press held for the hold time sends `on_long_press` while the finger is still down, which suits a context menu. With a mouse, a right click does the same.

The pressed tint clears as soon as the finger lifts or the press is cancelled. A touch leaves the cursor where the finger was, so after a touch the hover tint stays off until the mouse really moves.

The content gets every event first. A press the content captures, such as one on a button inside the row, belongs to that button: the row does not send its own tap as well. Such a button also captures the finger, so a drag that starts on it does not scroll; keep nested controls small, or make them tappables too.
