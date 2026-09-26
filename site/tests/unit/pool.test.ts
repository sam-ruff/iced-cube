import assert from "node:assert/strict";
import { test } from "node:test";
import { plan } from "../../src/lib/pool.ts";

const distanceOf = (distances: Record<string, number>) => (item: string) => distances[item] ?? 0;

test("mounts wanted previews while under capacity", () => {
  const result = plan(["a", "b"], [], 8, distanceOf({}));
  assert.deepEqual(result, { mount: ["a", "b"], unmount: [] });
});

test("mounts nothing already mounted", () => {
  const result = plan(["a"], ["a"], 8, distanceOf({}));
  assert.deepEqual(result, { mount: [], unmount: [] });
});

test("evicts the farthest unwanted preview when full", () => {
  const result = plan(["c"], ["a", "b"], 2, distanceOf({ a: 100, b: 900, c: 0 }));
  assert.deepEqual(result, { mount: ["c"], unmount: ["b"] });
});

test("never evicts a wanted preview", () => {
  const result = plan(["a", "b", "c"], ["a", "b"], 2, distanceOf({}));
  assert.deepEqual(result, { mount: [], unmount: [] });
});

test("mounts the closest pending previews first", () => {
  const result = plan(["far", "near"], [], 1, distanceOf({ far: 500, near: 10 }));
  assert.deepEqual(result.mount, ["near"]);
});

test("zero capacity mounts nothing", () => {
  assert.deepEqual(plan(["a"], [], 0, distanceOf({})), { mount: [], unmount: [] });
});
