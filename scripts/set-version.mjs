// Sets the version of every published crate in its manifest, in the
// dependency between them and in Cargo.lock. The crates are released together.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const crates = ["iced-cube-rules", "iced-cube"];

export function setManifestVersion(manifest, version) {
  const updated = manifest.replace(/^(\[package\][\s\S]*?^version\s*=\s*)"[^"]*"/m, `$1"${version}"`);
  if (updated === manifest && !manifest.includes(`version = "${version}"`)) {
    throw new Error("no [package] version found");
  }
  return updated;
}

export function setDependencyVersion(manifest, name, version) {
  const pattern = new RegExp(`^(${name}\\s*=\\s*\\{[^}\\n]*version\\s*=\\s*)"[^"]*"`, "m");
  if (!pattern.test(manifest)) throw new Error(`no dependency on ${name} found`);
  return manifest.replace(pattern, `$1"${version}"`);
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
  for (const name of crates) {
    const path = join(root, "crates", name, "Cargo.toml");
    let manifest = setManifestVersion(readFileSync(path, "utf8"), version);
    if (name === "iced-cube") manifest = setDependencyVersion(manifest, "iced-cube-rules", version);
    writeFileSync(path, manifest);
  }
  const lockPath = join(root, "Cargo.lock");
  let lock = readFileSync(lockPath, "utf8");
  for (const name of crates) lock = setLockVersion(lock, name, version);
  writeFileSync(lockPath, lock);
  console.log(`${crates.join(" and ")} set to ${version}`);
}
