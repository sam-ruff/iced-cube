## [0.0.3](https://github.com/sam-ruff/iced-cube/compare/v0.0.2...v0.0.3) (2026-10-06)

### Features

* add resizable panel, split pane and sidebar ([8a7e61c](https://github.com/sam-ruff/iced-cube/commit/8a7e61c47ec1e0e71b07cdbbce2902f11f202dac))
* **application:** add a menubar, toolbar, status bar and command palette ([f611d48](https://github.com/sam-ruff/iced-cube/commit/f611d487260536f844d2c79521fc2cd96d035eab))
* **data:** add tree and data table components ([289e4f8](https://github.com/sam-ruff/iced-cube/commit/289e4f81fef43d7e4eb95c2d050b769833193eeb))

### Bug Fixes

* correct component state and preview behaviour ([09cf3c1](https://github.com/sam-ruff/iced-cube/commit/09cf3c1b6d0d472967c5836f9bdae7abda744e37))
* **layout:** refine card elevation and accordion hierarchy ([8dd7fa9](https://github.com/sam-ruff/iced-cube/commit/8dd7fa995dba89d638e060fb6ec8ccf6cb75e819))

## [0.0.2](https://github.com/sam-ruff/iced-cube/compare/v0.0.1...v0.0.2) (2026-09-27)

### ⚠ BREAKING CHANGES

* **toast:** toast::Output has a new Payload variant, so exhaustive
matches on it need another arm.
* **context-menu:** Event::Open and Event::OpenFromKeyboard carry the area
key (() for a single area), key_event and Action::event take a target,
and State::position returns Option<Point>.
* **icon:** icon_button().variant() takes icon_button::Variant.
* **overlay:** build every overlay on one anchored layer with one menu look

### Features

* **combobox:** add a searchable combobox ([c821b20](https://github.com/sam-ruff/iced-cube/commit/c821b200a7e5150331983b961d75f56380a9007c))
* **combobox:** add an invalid state and move the caret to the end after a pick ([c16af78](https://github.com/sam-ruff/iced-cube/commit/c16af786737558f48e39873e2f405495e1bd88c0))
* **command:** add an inline searchable command list ([c82a208](https://github.com/sam-ruff/iced-cube/commit/c82a208026f3a42f039684ae6a77f5e940d4c888))
* **command:** add max_height so a palette shrinks to its results ([b95e1ff](https://github.com/sam-ruff/iced-cube/commit/b95e1ff756fa102e8961d3a0961a9f02cba5ddc3))
* **context-menu:** add context menus that open at the pointer ([8a02fd6](https://github.com/sam-ruff/iced-cube/commit/8a02fd69631eb9a9f1170675cb592e34002b44e4))
* **context-menu:** open on a long press on touch screens ([32fac0e](https://github.com/sam-ruff/iced-cube/commit/32fac0efc43be17a18af8ea6e8effb617faca77d))
* **context-menu:** share one menu across many rows and open below from the keyboard ([9c9c307](https://github.com/sam-ruff/iced-cube/commit/9c9c307366851ac56464365ebaa0da40d828201a))
* **dialog:** add modal dialogs and destructive confirmations ([8e52e90](https://github.com/sam-ruff/iced-cube/commit/8e52e907fbccb3d936ab514ed4739fea35613bef))
* **dialog:** confirm with Enter through the keymap ([ed6a684](https://github.com/sam-ruff/iced-cube/commit/ed6a68405911bf410cecd619311b3e55b658a800))
* **dropdown-menu:** add dropdown menus with checkbox, radio and submenu items ([abca20a](https://github.com/sam-ruff/iced-cube/commit/abca20ad668ef837950f8fdbc3b7d43d6f028a58))
* **gallery:** let stories fill the preview edge to edge ([615f7eb](https://github.com/sam-ruff/iced-cube/commit/615f7ebd3fc43dfff5f4d820cfb42d028b7c76da))
* **gallery:** let stories return tasks and talk theme and clipboard with the page ([5d6bb08](https://github.com/sam-ruff/iced-cube/commit/5d6bb0820d9d8a8cb1c2e8d25e7fd71d9efc2cf6))
* **icon-button:** promote icon buttons to their own component ([5de68aa](https://github.com/sam-ruff/iced-cube/commit/5de68aa9a0fd8721663f4013771118c2e3bcadbc))
* **keys:** parse and name the Menu key in chords ([8729b44](https://github.com/sam-ruff/iced-cube/commit/8729b44f7ae16aaba014291f4331ad69ee576092))
* **popover:** add popovers on a shared anchored overlay layer ([8a8ffd6](https://github.com/sam-ruff/iced-cube/commit/8a8ffd6a5d684ad67cacb483f19fa2bfe5b125fc))
* put every component behind its own Cargo feature ([a818b6b](https://github.com/sam-ruff/iced-cube/commit/a818b6b420c4f7c270760e9ff8b456883ebf7751))
* **showcase:** add a demo app that uses every component together ([e11a519](https://github.com/sam-ruff/iced-cube/commit/e11a519d200c24efa7c157d90b5bf8359fb5832e))
* **site:** add a full-window demo page with desktop downloads ([d92269b](https://github.com/sam-ruff/iced-cube/commit/d92269b0bc1d381fdd3723427cfa4d97db09ec96))
* **site:** disable demo downloads a release does not carry yet and follow the app's theme ([31b279d](https://github.com/sam-ruff/iced-cube/commit/31b279df46fe10b7886763e92b01369b877d69ea))
* **site:** grow a preview frame when its story wraps on a narrow screen ([cdf43df](https://github.com/sam-ruff/iced-cube/commit/cdf43df98b90dfceedb87ee6cf9691d5811b60ba))
* **theme:** add a popover surface token lifted in dark mode ([bb86053](https://github.com/sam-ruff/iced-cube/commit/bb8605339c82cf79127984778228331083454c71))
* **theme:** register a base font for semibold text ([fa63d0d](https://github.com/sam-ruff/iced-cube/commit/fa63d0d125a37ed1975826db592d9e989f8c1499))
* **toast:** keep action toasts in view, pause on hover and add ActivateLatest ([33b77a7](https://github.com/sam-ruff/iced-cube/commit/33b77a730bb1a12267dfe56b0ae031ba289c62f9))
* **toast:** let an action button hand back the app's own message ([da6791f](https://github.com/sam-ruff/iced-cube/commit/da6791fed83e6fbe9d1737b346131f3f2fefbaef))

### Bug Fixes

* **button:** drop the side padding from link buttons so they line up with text ([a542dbb](https://github.com/sam-ruff/iced-cube/commit/a542dbb025632ac3d94dea3c27f933a1b6020424))
* **button:** keep button, badge and tab labels on one line at their natural width ([956c2d4](https://github.com/sam-ruff/iced-cube/commit/956c2d43eef87b5088cc0636664d921057949d4c))
* **card:** keep headers and content inside the card border ([74d3a8f](https://github.com/sam-ruff/iced-cube/commit/74d3a8f8cd51d5ab7c996b916a87af39457cb760))
* **command:** only draw the highlight while the field has focus or after navigating ([c97a702](https://github.com/sam-ruff/iced-cube/commit/c97a702661f822742cedcbe56db8fef9b13d930c))
* **docs:** disambiguate doc links to the button and input functions ([55cdd71](https://github.com/sam-ruff/iced-cube/commit/55cdd71602c2b31aecf4d91e93414f2b8d166955))
* **dropdown-menu:** give every row one leading slot so labels line up ([d76e434](https://github.com/sam-ruff/iced-cube/commit/d76e434465d778b3e4c0e0def731cd6867b51834))
* **examples:** draw the times table in the bundled font ([217e286](https://github.com/sam-ruff/iced-cube/commit/217e286e1fcd994f24f771e8bf19f3251fa88f89))
* **examples:** stop the radio example shifting when the selection changes ([27c21cf](https://github.com/sam-ruff/iced-cube/commit/27c21cf34a088a2cf6ba9ec27d9d8ae6fe1d31a7))
* **gallery:** wrap rows of cards, alerts and buttons in stories on narrow screens ([dd3297e](https://github.com/sam-ruff/iced-cube/commit/dd3297eb0c12310ae9ecc18755024ef469bfbc3f))
* **icon:** draw disabled icons at half opacity instead of a faded tint ([420e519](https://github.com/sam-ruff/iced-cube/commit/420e5194f44ef57e3733836157741e81c143a94f))
* **showcase:** fix the demo findings from the soak review ([65f12b7](https://github.com/sam-ruff/iced-cube/commit/65f12b723ed91353da38cb87a2843169a4b1a744))
* **showcase:** put undone jobs back where they were in the demo ([4f1e537](https://github.com/sam-ruff/iced-cube/commit/4f1e537905ab5cef7d1c74f80b06ab30d3eb9527))
* **spinner:** draw the arc as cached svg frames so every spinner shows on WebGL ([f65c1c7](https://github.com/sam-ruff/iced-cube/commit/f65c1c79185a7371f1a8bede167fd6bf00593fb9))
* **stack:** let an unset width or height follow the children ([b01f79d](https://github.com/sam-ruff/iced-cube/commit/b01f79d8629a080cb7e83ba2141c6876ae08c9b2))

### Code Refactoring

* **overlay:** build every overlay on one anchored layer with one menu look ([587891f](https://github.com/sam-ruff/iced-cube/commit/587891f23585b4cc6f58869805a7b03bb7ea0ecd))
