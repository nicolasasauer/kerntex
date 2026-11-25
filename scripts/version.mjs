// Keeps the app version in step across every file that states it.
//   node scripts/version.mjs 0.2.0          set it everywhere
//   node scripts/version.mjs --check v0.2.0 fail unless every file says 0.2.0 (used on release tags)
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));

const json = /("version":\s*")([^"]*)(")/;
const toml = /^(version\s*=\s*")([^"]*)(")/m;
const locked = (name) => new RegExp(`(name = "${name}"\\r?\\nversion = ")([^"]*)(")`);

/** Every place the version is written; each pattern captures prefix, version, suffix. */
export const places = [
  ["package.json", json],
  ["package-lock.json", json],
  ["package-lock.json", /("packages":\s*\{\s*"":\s*\{\s*"name":\s*"kerntex",\s*"version":\s*")([^"]*)(")/],
  ["src-tauri/tauri.conf.json", json],
  ["src-tauri/Cargo.toml", toml],
  ["src-tauri/mcp-proxy/Cargo.toml", toml],
  ["src-tauri/Cargo.lock", locked("kerntex")],
  ["src-tauri/Cargo.lock", locked("kerntex-mcp")],
];

function match(text, file, pattern) {
  const found = text.match(pattern);
  if (!found) throw new Error(`No version found in ${file}`);
  return found;
}

export function readVersions(dir = root) {
  return places.map(([file, pattern]) => ({ file, version: match(readFileSync(join(dir, file), "utf8"), file, pattern)[2] }));
}

export function setVersion(version, dir = root) {
  if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) throw new Error(`Not a version: ${version}`);
  for (const [file, pattern] of places) {
    const path = join(dir, file);
    const text = readFileSync(path, "utf8");
    match(text, file, pattern);
    writeFileSync(path, text.replace(pattern, (_, before, _old, after) => before + version + after));
  }
}

/** The files that don't match a tag such as "v0.2.0". */
export function mismatches(tag, dir = root) {
  const wanted = tag.replace(/^v/, "");
  return readVersions(dir).filter((p) => p.version !== wanted);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [first, second] = process.argv.slice(2);
  if (first === "--check" && second) {
    const wrong = mismatches(second);
    for (const p of wrong) console.error(`${p.file} says ${p.version}, but the tag is ${second}`);
    if (wrong.length) {
      console.error(`Run: npm run version:set -- ${second.replace(/^v/, "")}`);
      process.exit(1);
    }
    console.log(`All files match ${second}.`);
  } else if (first && !first.startsWith("-")) {
    setVersion(first.replace(/^v/, ""));
    console.log(`Version set to ${first.replace(/^v/, "")}.`);
  } else {
    console.error("Usage: node scripts/version.mjs <version> | --check <tag>");
    process.exit(1);
  }
}
