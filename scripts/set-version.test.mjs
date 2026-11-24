import assert from "node:assert/strict";
import { test } from "node:test";
import { setLockVersion, setManifestVersion } from "./set-version.mjs";

const manifest = `[package]
name = "iced-cube"
version = "0.0.1"
edition.workspace = true

[dependencies]
iced = { version = "0.14.0" }
`;

const lock = `[[package]]
name = "iced"
version = "0.14.0"

[[package]]
name = "iced-cube"
version = "0.0.1"
dependencies = [
`;

test("updates only the package version in the manifest", () => {
  const out = setManifestVersion(manifest, "0.2.0");
  assert.match(out, /^version = "0.2.0"$/m);
  assert.match(out, /iced = \{ version = "0.14.0" \}/);
});

test("updates only the named crate in Cargo.lock", () => {
  const out = setLockVersion(lock, "iced-cube", "0.2.0");
  assert.match(out, /name = "iced-cube"\nversion = "0.2.0"/);
  assert.match(out, /name = "iced"\nversion = "0.14.0"/);
});

test("fails loudly when the crate is missing from the lock file", () => {
  assert.throws(() => setLockVersion(lock, "missing", "1.0.0"));
});
