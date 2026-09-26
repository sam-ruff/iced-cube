import assert from "node:assert/strict";
import { test } from "node:test";
import { MAX_FRAME_HEIGHT, frameHeight } from "../../src/lib/frame.ts";

test("keeps the story height when the content fits", () => {
  assert.equal(frameHeight(280, 190), 280);
});

test("grows to fit content that wrapped onto more lines", () => {
  assert.equal(frameHeight(280, 540.2), 541);
});

test("never grows past the cap", () => {
  assert.equal(frameHeight(280, 5000), MAX_FRAME_HEIGHT);
});

test("ignores reports that are not a finite number", () => {
  assert.equal(frameHeight(280, "600"), 280);
  assert.equal(frameHeight(280, Number.NaN), 280);
  assert.equal(frameHeight(280, undefined), 280);
});
