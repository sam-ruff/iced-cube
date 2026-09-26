// Exports story metadata, default keymaps and snapshot posters from the Rust workspace into the site.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const site = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = join(site, "..");
const generated = join(site, "src/generated");

function runBin(name) {
  return execFileSync("cargo", ["run", "--quiet", "-p", "gallery", "--bin", name], {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  });
}

mkdirSync(generated, { recursive: true });
const stories = runBin("manifest");
writeFileSync(join(generated, "stories.json"), stories);
const keymaps = runBin("keymaps");
writeFileSync(join(generated, "keymaps.json"), keymaps);

const from = join(root, "crates/gallery/snapshots");
const to = join(site, "public/snapshots");
mkdirSync(to, { recursive: true });
let copied = 0;
for (const file of readdirSync(from)) {
  if (!file.endsWith("-tiny-skia.png")) continue;
  copyFileSync(join(from, file), join(to, file.replace("-tiny-skia.png", ".png")));
  copied += 1;
}

console.log(
  `Exported ${JSON.parse(stories).length} stories, ${Object.keys(JSON.parse(keymaps)).length} keymaps and ${copied} posters.`,
);
