// Fails when a component has stories but no docs page, a page references a
// missing story, a story has no poster in both themes, or a keymap has no page.
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const site = join(dirname(fileURLToPath(import.meta.url)), "..");
const stories = JSON.parse(readFileSync(join(site, "src/generated/stories.json"), "utf8"));
const keymapsFile = join(site, "src/generated/keymaps.json");
const keymaps = existsSync(keymapsFile) ? JSON.parse(readFileSync(keymapsFile, "utf8")) : {};
const pagesDir = join(site, "src/content/components");

// Theme stories demonstrate a whole palette and showcase stories combine
// several components, so they appear in guides or on the landing page instead.
const guideOnly = new Set(["theme", "showcase"]);

const pages = readdirSync(pagesDir).filter((name) => name.endsWith(".md"));
const referenced = new Set();
const errors = [];

for (const file of pages) {
  const text = readFileSync(join(pagesDir, file), "utf8");
  const list = text.match(/^stories:\s*\[(.*)\]\s*$/m)?.[1] ?? "";
  const hero = text.match(/^hero:\s*(\S+)/m)?.[1];
  const ids = list.split(",").map((id) => id.trim()).filter(Boolean);
  if (hero) ids.push(hero);
  for (const id of ids) {
    referenced.add(id);
    if (!stories.some((story) => story.id === id)) errors.push(`${file}: unknown story "${id}"`);
  }
}

for (const story of stories) {
  if (!referenced.has(story.id) && !guideOnly.has(story.component)) {
    errors.push(`story "${story.id}" is not on any docs page`);
  }
  for (const theme of ["light", "dark"]) {
    const poster = join(site, "public/snapshots", `${story.id.replaceAll("/", "--")}-${theme}.png`);
    if (!existsSync(poster)) errors.push(`story "${story.id}" has no ${theme} snapshot`);
  }
}

for (const slug of Object.keys(keymaps)) {
  if (!pages.includes(`${slug}.md`)) errors.push(`keymap "${slug}" has no docs page`);
}

if (errors.length > 0) {
  console.error(errors.join("\n"));
  process.exit(1);
}
console.log(`Coverage ok: ${stories.length} stories, ${Object.keys(keymaps).length} keymaps.`);
