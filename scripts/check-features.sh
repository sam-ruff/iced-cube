#!/usr/bin/env bash
# Checks iced-cube with no components, then with each component, group and
# `full` feature on its own. Extra arguments are passed to `cargo check`.
# Needs node to read the feature list from `cargo metadata`.
set -euo pipefail

cd "$(dirname "$0")/.."

export RUSTFLAGS="${RUSTFLAGS:--D warnings}"
platform="x11,wayland,thread-pool"

features="$(
    cargo metadata --no-deps --format-version 1 |
        node -e '
            const skip = new Set(["default", "x11", "wayland", "thread-pool", "tokio"]);
            let s = "";
            process.stdin.on("data", (d) => (s += d)).on("end", () => {
                const pkg = JSON.parse(s).packages.find((p) => p.name === "iced-cube");
                for (const f of Object.keys(pkg.features)) {
                    if (!skip.has(f)) console.log(f);
                }
            });
        '
)"

check() {
    local set="$1"
    shift
    echo "==> $set"
    # The library alone first, because dev-dependencies can enable iced
    # features that the library would otherwise be missing.
    cargo check -p iced-cube --lib --no-default-features --features "$set" "$@"
    cargo check -p iced-cube --all-targets --no-default-features --features "$set" "$@"
}

check "$platform" "$@"
for feature in $features; do
    check "$platform,$feature" "$@"
done
