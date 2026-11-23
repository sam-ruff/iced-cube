import assert from "node:assert/strict";
import { test } from "node:test";
import { setDependencyVersion, setLockVersion, setManifestVersion } from "./set-version.mjs";

const manifest = `[package]
name = "iced-cube"
version = "0.0.1"
edition.workspace = true

[dependencies]
iced = { version = "0.14.0" }
iced-cube-rules = { version = "0.0.1", path = "../iced-cube-rules", optional = true }
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

test("updates the version of the dependency on the rules crate only", () => {
  const out = setDependencyVersion(manifest, "iced-cube-rules", "0.2.0");
  assert.match(out, /iced-cube-rules = \{ version = "0.2.0", path/);
  assert.match(out, /^version = "0.0.1"$/m);
  assert.match(out, /iced = \{ version = "0.14.0" \}/);
});

test("fails loudly when the dependency is missing", () => {
  assert.throws(() => setDependencyVersion(manifest, "missing", "1.0.0"));
});

test("fails loudly when the crate is missing from the lock file", () => {
  assert.throws(() => setLockVersion(lock, "missing", "1.0.0"));
});
