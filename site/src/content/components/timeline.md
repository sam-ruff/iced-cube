---
title: Timeline
description: A track of steps coloured by outcome, with a playhead the reader scrubs and lanes for work that spans several steps.
group: Data
order: 7
module: data::timeline
imports: |
  use iced_cube::data::timeline::{self, Action, Span, Tone, lane};
keywords: [scrubber, playhead, steps, trace, history, gantt, lanes]
related: [slider, image-compare]
hero: timeline/default
stories: [timeline/default, timeline/lanes, timeline/keyboard, timeline/disabled]
api:
  - name: "timeline(steps, current)"
    description: "Creates a timeline with one Tone per step and the current step selected. The current step is clamped to the steps. It renders disabled until it has a message."
  - name: ".on_select(f)"
    description: "Sets the message sent with the step under the pointer while pressing or dragging. The current step is never sent again."
  - name: ".lane(lane(label, spans)) / .lanes(iter)"
    description: "Adds labelled rows below the track. Each Span covers a range of steps, inclusive, with its own tone."
  - name: "Span::new(start, end).tone(tone)"
    description: "A run of steps in a lane. The ends may be given in either order."
  - name: ".label_width(px)"
    description: "Width of the lane labels, 96 by default. Only used when there are lanes."
  - name: ".label(text)"
    description: "The name tests and assistive tools know it by, Timeline by default."
  - name: ".width(length)"
    description: "Overrides the width. Timelines fill the available width by default, and are as tall as their track and lanes."
  - name: "timeline::default_keymap()"
    description: "The default shortcuts as a Keymap<Action>. Bind, unbind or clear chords to change them."
  - name: "action.apply(current, count)"
    description: "The step after an Action (Previous, Next, PageBack, PageForward, First or Last), kept within the steps. Send it through your on_select message."
  - name: "timeline::index_at(x, width, count) / position_of(index, width, count)"
    description: "The pure geometry behind the track: the step nearest a point, and where a step sits."
---

Tones come from the theme: neutral steps use the muted foreground, and success, warning and destructive steps use the matching theme colours, drawn a little taller so problems stand out in a long run.

Steps are spread evenly across the track, so a timeline with thousands of steps still fits, though individual ticks merge. Pair it with a list of the steps when the reader needs to pick one exactly.

Timelines do not take keyboard focus, so shortcuts are app-wide. With several on screen, the app decides which one is current and applies the resolved action to its step.
