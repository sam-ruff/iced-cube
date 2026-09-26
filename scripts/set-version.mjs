// Sets the published crate's version in its manifest and in Cargo.lock.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export function setManifestVersion(manifest, version) {
  const updated = manifest.replace(/^(\[package\][\s\S]*?^version\s*=\s*)"[^"]*"/m, `$1"${version}"`);
  if (updated === manifest && !manifest.includes(`version = "${version}"`)) {
    throw new Error("no [package] version found");
  }
  return updated;
}

export function setLockVersion(lock, name, version) {
  const pattern = new RegExp(`(\\[\\[package\\]\\]\\nname = "${name}"\\nversion = )"[^"]*"`);
  if (!pattern.test(lock)) throw new Error(`${name} not found in Cargo.lock`);
  return lock.replace(pattern, `$1"${version}"`);
}

const isMain = process.argv[1] === fileURLToPath(import.meta.url);
if (isMain) {
  const version = process.argv[2];
  if (!/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(version ?? "")) {
    console.error("usage: set-version.mjs <semver>");
    process.exit(1);
  }
  const root = join(dirname(fileURLToPath(import.meta.url)), "..");
  const manifestPath = join(root, "crates/iced-cube/Cargo.toml");
  const lockPath = join(root, "Cargo.lock");
  writeFileSync(manifestPath, setManifestVersion(readFileSync(manifestPath, "utf8"), version));
  writeFileSync(lockPath, setLockVersion(readFileSync(lockPath, "utf8"), "iced-cube", version));
  console.log(`iced-cube set to ${version}`);
}
