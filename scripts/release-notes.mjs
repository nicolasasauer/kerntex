#!/usr/bin/env node
// "What's new" for a release: written into the GitHub release, then copied into
// the Microsoft Store listing ("What's new in this version").
//
//   node scripts/release-notes.mjs from-git <tag>
//       Markdown list of the commits since the previous tag (for the release body).
//   node scripts/release-notes.mjs store <tag>
//       Reads the "## What's new" section of the published GitHub release <tag> (so
//       edits made to the draft count) and puts it into the pending Store submission
//       of every language. Needs AZURE_AD_TENANT_ID, AZURE_AD_CLIENT_ID,
//       AZURE_AD_CLIENT_SECRET, MSSTORE_PRODUCT_ID and the gh CLI.
//       With --commit, it then sends the submission to certification.

import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const HEADING = "## What's new";
/** The Store's limit for "What's new in this version". */
export const STORE_LIMIT = 1500;

/** Commit subjects → markdown bullets, without merges, version bumps and noise. */
export function notesFromSubjects(subjects) {
  const keep = subjects
    .map((s) => s.trim())
    .filter((s) => s && !/^Merge (pull request|branch|remote-tracking)/i.test(s) && !/^Release v?\d+\.\d+\.\d+/i.test(s));
  return [...new Set(keep)].map((s) => `- ${s}`).join("\n");
}

/** The text under "## What's new" in a release body, up to the next heading of the same level. */
export function whatsNewSection(body) {
  const lines = (body ?? "").replace(/\r\n/g, "\n").split("\n");
  const start = lines.findIndex((l) => l.trim().toLowerCase() === HEADING.toLowerCase());
  if (start < 0) return "";
  const rest = lines.slice(start + 1);
  const end = rest.findIndex((l) => /^#{1,2}\s/.test(l));
  return (end < 0 ? rest : rest.slice(0, end)).join("\n").trim();
}

/** Markdown → the Store's plain text: bullets as "•", no emphasis or link syntax, within the limit. */
export function toStoreText(markdown, limit = STORE_LIMIT) {
  const text = markdown
    .split("\n")
    .map((l) => l.replace(/^\s*[-*+]\s+/, "• "))
    .map((l) => l.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1").replace(/(\*\*|__|`)/g, ""))
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
  if (text.length <= limit) return text;
  // Cut at a whole line so no bullet ends mid-sentence.
  const cut = text.slice(0, limit - 2);
  return cut.slice(0, Math.max(cut.lastIndexOf("\n"), 0)).trimEnd() + "\n…";
}

/** Sets "What's new" in every language of a Store submission; returns the languages changed. */
export function applyReleaseNotes(submission, text) {
  const changed = [];
  for (const [lang, listing] of Object.entries(submission.listings ?? {})) {
    if (!listing?.baseListing) continue;
    listing.baseListing.releaseNotes = text;
    changed.push(lang);
  }
  return changed;
}

// --- Store submission API (the same one the msstore CLI uses) ------------------

const API = "https://manage.devcenter.microsoft.com/v1.0/my";

async function token(env) {
  const res = await fetch(`https://login.microsoftonline.com/${env.AZURE_AD_TENANT_ID}/oauth2/token`, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "client_credentials",
      client_id: env.AZURE_AD_CLIENT_ID,
      client_secret: env.AZURE_AD_CLIENT_SECRET,
      resource: "https://manage.devcenter.microsoft.com",
    }),
  });
  if (!res.ok) throw new Error(`Sign-in failed: ${res.status} ${await res.text()}`);
  return (await res.json()).access_token;
}

async function api(auth, method, path, body) {
  const res = await fetch(`${API}${path}`, {
    method,
    headers: { Authorization: `Bearer ${auth}`, "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  if (!res.ok) throw new Error(`${method} ${path}: ${res.status} ${text}`);
  return text ? JSON.parse(text) : {};
}

async function store(tag, commit, env = process.env) {
  for (const k of ["AZURE_AD_TENANT_ID", "AZURE_AD_CLIENT_ID", "AZURE_AD_CLIENT_SECRET", "MSSTORE_PRODUCT_ID"]) {
    if (!env[k]) throw new Error(`${k} is not set.`);
  }
  const app = env.MSSTORE_PRODUCT_ID;
  const auth = await token(env);
  const pending = (await api(auth, "GET", `/applications/${app}`)).pendingApplicationSubmission;
  if (!pending?.id) throw new Error("No pending submission. Upload the package first (msstore publish --noCommit).");
  const path = `/applications/${app}/submissions/${pending.id}`;

  // The notes are a nice-to-have: if they fail, the update still goes out.
  try {
    const body = execFileSync("gh", ["release", "view", tag, "--json", "body", "--jq", ".body"], { encoding: "utf8" });
    const notes = toStoreText(whatsNewSection(body));
    if (notes) {
      const submission = await api(auth, "GET", path);
      const langs = applyReleaseNotes(submission, notes);
      await api(auth, "PUT", path, submission);
      console.log(`What's new set for ${langs.join(", ") || "no language"}:\n${notes}`);
    } else {
      console.log(`Release ${tag} has no "${HEADING}" section; the Store keeps the previous text.`);
    }
  } catch (e) {
    console.log(`::warning::Couldn't set What's new in the Store: ${e.message}`);
  }

  if (commit) {
    await api(auth, "POST", `${path}/commit`);
    console.log("Submitted for certification.");
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [cmd, tag, ...flags] = process.argv.slice(2);
  try {
    if (cmd === "from-git" && tag) {
      let range = tag;
      try {
        const previous = execFileSync("git", ["describe", "--tags", "--abbrev=0", `${tag}^`], { encoding: "utf8" }).trim();
        range = `${previous}..${tag}`;
      } catch {
        // No earlier tag: every commit up to this one.
      }
      const log = execFileSync("git", ["log", "--no-merges", "--format=%s", range], { encoding: "utf8" });
      console.log(notesFromSubjects(log.split("\n").reverse()));
    } else if (cmd === "store" && tag) {
      await store(tag, flags.includes("--commit"));
    } else {
      console.error("Usage: node scripts/release-notes.mjs from-git <tag> | store <tag> [--commit]");
      process.exit(1);
    }
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
