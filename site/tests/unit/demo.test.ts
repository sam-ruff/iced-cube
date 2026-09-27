import assert from "node:assert/strict";
import { test } from "node:test";
import { appTheme, assetAvailability } from "../../src/lib/demo.ts";

const file = "iced-cube-demo-linux-x86_64.tar.gz";

test("a release that lists the file makes it available", () => {
  const body = { assets: [{ name: "other.zip" }, { name: file }] };
  assert.equal(assetAvailability(200, body, file), "available");
});

test("a release without the file, or no release at all, means it is missing", () => {
  assert.equal(assetAvailability(200, { assets: [] }, file), "missing");
  assert.equal(assetAvailability(404, { message: "Not Found" }, file), "missing");
});

test("anything unexpected is unknown, so the link is left alone", () => {
  assert.equal(assetAvailability(403, {}, file), "unknown");
  assert.equal(assetAvailability(200, null, file), "unknown");
  assert.equal(assetAvailability(200, { assets: "none" }, file), "unknown");
});

test("reads the app's theme messages and ignores everything else", () => {
  assert.equal(appTheme({ type: "app-theme", value: "dark" }), "dark");
  assert.equal(appTheme({ type: "app-theme", value: "system" }), "system");
  assert.equal(appTheme({ type: "app-theme", value: "sepia" }), null);
  assert.equal(appTheme({ type: "size", height: 300 }), null);
  assert.equal(appTheme("dark"), null);
  assert.equal(appTheme(null), null);
});
