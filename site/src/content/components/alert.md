---
title: Alert
description: An inline callout with a title, description and icon.
group: Feedback
order: 1
module: feedback::alert
imports: |
  use iced_cube::alert;
  use iced_cube::feedback::alert::Variant;
keywords: [callout, banner]
related: [toast, badge]
hero: alert/variants
stories: [alert/variants, alert/custom-icon]
api:
  - name: "alert(title)"
    description: "Creates an alert with a title. It fills the available width."
  - name: ".description(text)"
    description: "Adds supporting text under the title."
  - name: ".variant(Variant)"
    description: "Info, Success, Warning or Destructive. Defaults to Info."
  - name: ".icon(glyph)"
    description: "Replaces the variant's default icon with any Lucide icon."
  - name: ".width(length)"
    description: "Overrides the width."
---

Alerts sit in the flow of the page and stay until the content around them changes. Use them for messages that relate to what is on screen, such as a failed sync or a setting that needs attention.

Each variant has a default icon: info, a tick, a warning triangle and an exclamation mark. Status variants tint their background and border with the matching theme colour, while Info stays neutral.
