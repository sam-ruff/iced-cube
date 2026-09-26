# AGENTS.md

Guidance for anyone, human or agent, changing this repository.

## Before you start

- If `OTHER.md` exists at the repo root, read it. It holds maintainer-local instructions that are not published.
- If `JOURNAL.md` exists, skim the latest entries to see what previous sessions did. Before finishing a session, add a short entry at the top: date, what was asked, what changed, decisions, open threads.
- `OTHER.md`, `JOURNAL.md` and `iced-component-catalogue.md` are gitignored and must never be committed. The pre-commit hook enforces this.

## What this is

- `crates/iced-cube` is the published library: themeable components for iced 0.14 with Lucide icons.
- `crates/gallery` holds one runnable example ("story") per component variant. It runs natively as a browsable gallery (`cargo run -p gallery`) and compiles to wasm for the live previews on the docs site.
- `site` is the docs site (Astro, TypeScript, Pagefind), deployed to GitHub Pages.

## Commands

```sh
scripts/install-hooks.sh                       # once per clone
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace                         # unit, system and snapshot tests
cargo run -p gallery                           # native gallery
cargo run -p gallery -- button/variants --dark # open one story
scripts/build-wasm.sh                          # wasm previews into site/public/wasm
cd site && npm run dev                         # docs site; PUBLIC_PREVIEW_MODE=mock skips wasm
cd site && npm run test:e2e                    # Playwright, mobile and desktop
```

`.cargo/config.toml` sets `ICED_TEST_BACKEND=tiny-skia` so headless tests and snapshots render on the CPU and match across machines.

## Conventions

- UK English everywhere (colour, behaviour, organise). No emojis. No em dashes.
- Comments only where the code is not obvious. Describe current behaviour, never history.
- Add dependencies with `cargo add` or `npm i <pkg>@latest` so the latest versions are used.
- No `unwrap`/`expect` in library code. Use guard clauses (`let ... else`, early return) to keep nesting shallow.
- Conventional commits (`feat(button): ...`, `fix(theme): ...`). The `commit-msg` hook checks the format. `feat` releases a minor, `fix`/`perf`/`refactor` a patch. While on 0.x a breaking change releases a minor.
- Published text (README, rustdoc, site, commit messages) must not mention other UI libraries by name.

## Component API

- One module per component under the catalogue's groups: `primitives`, `forms`, `overlay`, `navigation`, `data`, `layout`, `feedback`, `application`. Re-export the constructor from the group's `mod.rs`.
- Constructors are lowercase functions that mirror iced (`button("Save")`, `badge("New")`) returning a builder. Builders convert into `iced::Element<'a, Message>` with `From`.
- Visual options are enums (`Variant`, `Size`) with `Default` and an `ALL` constant so tests and stories can iterate them.
- Colours only come from `theme::Tokens::of(theme)`. Never hard-code a colour in a component. Put the style resolution in a pure `pub fn style(tokens, variant, status) -> Style` (or similar) so it can be unit-tested without a renderer.
- Stateful components (tabs, accordion, tree, table, command palette and so on) expose a plain `State` struct with a pure `update(&mut self, Event) -> Option<Output>`. The view reads `&State`. The app owns the state and routes messages; components never hide state behind interior mutability.
- A component without a message renders disabled, as iced's own widgets do.
- Colours reach components only through `Tokens`. `theme::Config` builds the light, dark and custom themes; it writes secondary into the extended palette's `secondary.base`, accent into `secondary.weak` and border into `background.strong`, and `Tokens::of` reads them from there.
- The library makes no network calls. If that ever changes, the call goes behind a trait with `mockall` tests.

## Keyboard shortcuts

iced has no general focus model, so shortcuts are app-wide: the app subscribes with `keys::subscription()` and resolves each press through a component's `Keymap`.

- Every interactive component has an `Action` enum implementing `keys::Action` (`ALL`, `defaults`, `name`, `description`) and a `pub fn default_keymap() -> Keymap<Action>`, documented with a table of its chords.
- Each action maps to the component's existing events or values through a pure function (`action.event(&state)`, `action.events(&state)`, `action.apply(...)`). Unit-test the defaults, an override, unbinding and the mapping.
- Pick defaults that follow common desktop conventions, build chords with `Chord::named(...)` rather than parsing strings, and add the component's slug to `gallery::keymaps::all()` so the docs page shows the table.
- Give the component a keymap-driven story. Stories with subscriptions opt in with `subscription: true`.

## Subscriptions and channels

Anything fed from outside the UI thread (toasts, log viewers, status indicators, streaming lists, async search) uses channels. Nothing shares `Arc<Mutex<_>>` or atomics with worker threads.

- The component provides a `subscription()` built with `Subscription::run` or `Subscription::run_with(id, builder)`. The id must be stable and hashable so iced does not restart the stream on every call.
- The subscription creates the channel itself and owns the receiver. Its first item is `Event::Ready(Sender)`. The app stores the sender and hands clones to producers (threads, tokio tasks, callbacks).
- Channels are bounded (`futures::channel::mpsc::channel(n)`) so a fast producer gets backpressure. Receivers drain what is ready in batches (`ready_chunks`) so a burst does not cause one re-render per item.
- Timers are only subscribed while needed. For example, toasts use `time::every` only while at least one toast is visible; otherwise return `Subscription::none()`.
- The gallery's `bridge` module is a small working example: the browser's `message` listener owns the sender, the subscription owns the receiver.
- Every streaming component gets a story with a simulated producer, so the pattern is visible on its docs page.

## Tests (definition of done)

A component is not done until all of these exist and pass:

- **Unit tests** next to the code: builder defaults, every `Variant`/`Size`, style resolution in light and dark and in every status (hovered, pressed, disabled, focused), `State::update` transitions, bounds and edge cases (empty, disabled, overflow, min/max), channel batching where relevant.
- **System tests** in `crates/iced-cube/tests/<component>.rs` using `iced_test::simulator`: click, type and key sequences, asserting on the emitted messages and on what is rendered (`ui.find`). Overlays: open, close, dismiss.
- **Stories** in `crates/gallery/src/stories/<component>/`, registered in `stories/mod.rs`. Each story file is shown verbatim on the site, so write it as clean example code: `Message`, a `Default` `Example`, `update`, `view`. Optional settings after `file`, in order: `height: <px>` (default 280; about 160-200 for a single row, 320-360 for toasts and overlays), `subscription: true` and `theme: true` (a story `theme(&self) -> Option<Theme>` that replaces the gallery theme).
- **Snapshots**: `cargo test -p gallery --test snapshots` writes PNGs for new stories on first run, 720 pixels wide at the story's height. Look at them before committing. To accept an intentional change, delete the old PNGs and rerun.
- **Docs page**: `site/src/content/components/<slug>.md` (format below). The coverage test fails if a component has stories but no page, or a page references a missing story.

## Docs pages

`site/src/content/components/<slug>.md`:

```md
---
title: Button
description: One sentence on what it is for.
group: Actions               # Actions | Forms | Layout | Navigation | Overlays | Feedback
order: 1                     # position within the group
module: primitives::button
imports: |                   # optional, the use lines a reader needs; defaults to the module path
  use iced_cube::button;
  use iced_cube::primitives::button::{Size, Variant};
keywords: [cta]              # optional, other names people search for
related: [tooltip]           # optional, slugs shown under "See also"
hero: button/variants        # story shown at the top, with its title and description as a caption
stories: [button/variants, button/sizes]
api:
  - name: "button(label)"
    description: "Creates a text button."
keyboard:                    # optional, only for keys built into iced widgets
  - keys: "Enter"
    action: "Emits the on_submit message."
---

Short usage notes in markdown. Code examples come from the stories, not from here.
```

Component keymaps are exported from Rust to `site/src/generated/keymaps.json` by `npm run prepare-data` and rendered on the page automatically, so never copy them into `keyboard:`. Stories whose component is `theme` are exempt from the coverage check and are linked from the theming guide instead, because guides cannot embed previews.

## CI and releases

- CI runs on self-hosted runners only. Never switch a job to a GitHub-hosted runner. Every job that can run on a pull request must keep the same-repository guard so forks never reach the runners.
- Releases are cut by semantic-release from `main` after CI passes, and publish to crates.io through trusted publishing. Do not bump versions by hand.
