import assert from "node:assert/strict";
import { test } from "node:test";
import { rank, score, type Entry } from "../../src/lib/rank.ts";

const entry = (title: string, description = "", keywords: string[] = []): Entry => ({
  title,
  description,
  keywords,
  href: `/${title}`,
  section: "s",
});

test("keywords find an entry by another name", () => {
  assert.ok(score(entry("Switch", "", ["toggle"]), "toggle") > 0);
  assert.ok(score(entry("Switch", "", ["toggle"]), "togg") > 0);
  assert.ok(score(entry("Input", "", ["text field"]), "field") > 0);
  assert.equal(score(entry("Switch"), "toggle"), 0);
});

test("an exact keyword beats a partial title match", () => {
  const entries = [entry("Tabs"), entry("Badge", "", ["tag", "chip"])];
  assert.deepEqual(
    rank(entries, "tag").map((e) => e.title),
    ["Badge"],
  );
  assert.ok(score(entry("Tooltip", "", ["hint"]), "hint") > score(entry("Card", "a hint inside"), "hint"));
});

test("title matches still come first", () => {
  assert.ok(score(entry("Tag input"), "tag") > score(entry("Badge", "", ["tag"]), "tag"));
  const entries = [entry("Field and label", "", ["form field"]), entry("Input", "", ["text field"])];
  assert.equal(rank(entries, "field")[0]?.title, "Field and label");
  assert.ok(score(entry("Spinner", "", ["loader"]), "spinner") > score(entry("Progress", "", ["spinner"]), "spinner"));
});

test("exact title beats prefix beats substring", () => {
  assert.ok(score(entry("Button"), "button") > score(entry("Button group"), "button"));
  assert.ok(score(entry("Button group"), "button") > score(entry("Icon button"), "button"));
  assert.ok(score(entry("Icon button"), "button") > score(entry("Card", "a button inside"), "button"));
});

test("matches word prefixes and loose subsequences", () => {
  assert.ok(score(entry("Scroll area"), "area") > 0);
  assert.ok(score(entry("Checkbox"), "chkbx") > 0);
  assert.equal(score(entry("Slider"), "zzz"), 0);
});

test("empty query keeps everything in index order", () => {
  const entries = [entry("B"), entry("A")];
  assert.deepEqual(rank(entries, "  "), entries);
});

test("rank drops non-matches and sorts by score", () => {
  const entries = [entry("Icon button"), entry("Slider"), entry("Button")];
  assert.deepEqual(
    rank(entries, "button").map((e) => e.title),
    ["Button", "Icon button"],
  );
});
