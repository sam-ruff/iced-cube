import assert from "node:assert/strict";
import { test } from "node:test";
import { absoluteLinks, SITE, stripFrontmatter } from "../../src/lib/markdown-text.ts";

test("frontmatter is removed and the body kept", () => {
  const source = "---\nlayout: x\ntitle: Icons\n---\n\n## Using an icon\n\nText.";
  assert.equal(stripFrontmatter(source), "## Using an icon\n\nText.");
});

test("a source without frontmatter is only trimmed", () => {
  assert.equal(stripFrontmatter("\n# Title\n"), "# Title");
});

test("relative links point at the Markdown page", () => {
  assert.equal(
    absoluteLinks("[Tabs](../components/tabs/)", "docs/keyboard/"),
    `[Tabs](${SITE}/docs/components/tabs.md)`,
  );
  assert.equal(absoluteLinks("[Status](status/)", "docs/"), `[Status](${SITE}/docs/status.md)`);
});

test("links to files keep their extension", () => {
  assert.equal(absoluteLinks("[index](../llms.txt)", "docs/"), `[index](${SITE}/llms.txt)`);
});

test("links to the docs root map to the introduction", () => {
  assert.equal(absoluteLinks("[Intro](../)", "docs/theming/"), `[Intro](${SITE}/docs/index.md)`);
});

test("anchors survive and external links are untouched", () => {
  assert.equal(
    absoluteLinks("[API](../components/toast/#api)", "docs/subscriptions/"),
    `[API](${SITE}/docs/components/toast.md#api)`,
  );
  assert.equal(absoluteLinks("[iced](https://iced.rs)", "docs/"), "[iced](https://iced.rs)");
  assert.equal(absoluteLinks("[below](#usage)", "docs/"), "[below](#usage)");
});
