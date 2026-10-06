---
title: Avatar
description: A small picture of a person or team, with initials when there is no image.
group: Feedback
order: 5
module: feedback::avatar
imports: |
  use iced_cube::{avatar, avatar_group};
  use iced_cube::feedback::avatar::{Presence, Shape, Size};
keywords: [profile picture, user, initials, presence, status]
related: [badge]
hero: avatar/presence
stories: [avatar/fallbacks, avatar/sizes, avatar/presence, avatar/group]
api:
  - name: "avatar(name)"
    description: "Creates an avatar showing the name's initials. A name with no letters shows a person icon."
  - name: ".image(handle)"
    description: "Shows a picture instead of the initials, cropped to fill the shape."
  - name: ".icon(glyph)"
    description: "Shows a Lucide icon instead of the initials, such as a bot for an automated account."
  - name: ".size(Size)"
    description: "Sm, Md, Lg or Xl: 24, 32, 40 or 56 pixels across. Defaults to Md."
  - name: ".shape(Shape)"
    description: "Circle for people or Square, with rounded corners, for teams and organisations. Defaults to Circle."
  - name: ".presence(Presence)"
    description: "Adds a dot for Online, Away, Busy or Offline on the bottom right corner."
  - name: "avatar_group(avatars)"
    description: "Overlaps avatars from left to right, each ringed in the background colour."
  - name: ".size(Size) / .max_items(n)"
    description: "Sets the size of every avatar in a group, and shows at most n places, the last one counting the rest."
  - name: "avatar::initials(name)"
    description: "The initials an avatar shows: the first letters of the first and last words."
---

An avatar shows the first thing it has of an image, an icon and the name's initials, so the same code covers people who have uploaded a picture and those who have not. Initials and icons sit on the muted surface.

Images come from an iced `image::Handle`. The `avatar` feature turns on iced's image widget without any decoders, which is enough for pixels you already have, such as a picture fetched and decoded elsewhere. To load PNG or JPEG files directly, enable iced's `image` feature in your app. The GPU renderer crops images to the avatar's shape; iced's software fallback draws them with square corners.

The rings around grouped avatars and presence dots are drawn in the page background colour, so they read best on the page itself rather than on a raised card.

Presence colours come from the theme's status tokens: success for online, warning for away, destructive for busy and the muted foreground for offline. Pair the dot with text, as the presence example does, since colour alone does not say which state it is.
