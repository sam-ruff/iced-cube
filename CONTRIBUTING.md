# Contributing

Thanks for helping. [AGENTS.md](AGENTS.md) holds the full conventions and applies to people as much as to tools. The short version:

- **Hooks**: run `scripts/install-hooks.sh` once after cloning. The pre-commit hook runs `cargo fmt` and clippy; the commit-msg hook checks the message format.
- **Commits**: use conventional commits, such as `feat(button): add a loading state` or `fix(theme): ...`. Releases are cut from them, so do not bump versions by hand.
- **Tests**: `cargo test --workspace` runs the unit, system and snapshot tests. A new component needs all three, as described under "Tests" in AGENTS.md.
- **Stories**: each example lives in `crates/gallery/src/stories/<component>/` and is registered in `stories/mod.rs`. The file is shown verbatim on the docs site, so write it as example code.
- **Snapshots**: `cargo test -p gallery --test snapshots` writes PNGs for new stories. Look at them before committing. To accept a deliberate visual change, delete the old PNGs and run it again.
- **Docs pages**: every component has a page in `site/src/content/components/<slug>.md`. The format is in AGENTS.md. Check the site with `cd site && npm run check`.

UK English throughout, no emojis, no em dashes.
