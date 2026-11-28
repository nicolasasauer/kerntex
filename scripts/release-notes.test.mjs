// node --test scripts/
import { test } from "node:test";
import assert from "node:assert/strict";
import { applyReleaseNotes, notesFromSubjects, toStoreText, whatsNewSection } from "./release-notes.mjs";

test("commit subjects become bullets without merges, version bumps or duplicates", () => {
  const notes = notesFromSubjects([
    "Icons for the Files and Git tabs",
    "Merge pull request #34 from nicolasasauer/x",
    "Release 0.1.8",
    "",
    "Icons for the Files and Git tabs",
    "Calmer auto-compile",
  ]);
  assert.equal(notes, "- Icons for the Files and Git tabs\n- Calmer auto-compile");
});

test("the What's new section is read up to the next heading", () => {
  const body = "## What's new\n\n- One\n- Two\n\n## Download\n\nGet it below.";
  assert.equal(whatsNewSection(body), "- One\n- Two");
  assert.equal(whatsNewSection("## What's New\r\n- Only\r\n"), "- Only");
  assert.equal(whatsNewSection("Just downloads."), "");
  assert.equal(whatsNewSection(null), "");
});

test("the Store gets plain text with bullets", () => {
  assert.equal(toStoreText("- **Bold** item\n* [Link](https://x.y) and `code`"), "• Bold item\n• Link and code");
});

test("long notes are cut at a whole line and stay within the Store limit", () => {
  const md = Array.from({ length: 100 }, (_, i) => `- Change number ${i} with some words`).join("\n");
  const text = toStoreText(md);
  assert.ok(text.length <= 1500);
  assert.ok(text.endsWith("\n…"));
  assert.match(text.split("\n").at(-2), /^• Change number \d+ with some words$/);
});

test("What's new is set in every language of the submission", () => {
  const submission = {
    listings: {
      "en-us": { baseListing: { description: "A", releaseNotes: "old" } },
      "de-de": { baseListing: { description: "B" } },
    },
  };
  assert.deepEqual(applyReleaseNotes(submission, "new"), ["en-us", "de-de"]);
  assert.equal(submission.listings["en-us"].baseListing.releaseNotes, "new");
  assert.equal(submission.listings["de-de"].baseListing.releaseNotes, "new");
  assert.equal(submission.listings["en-us"].baseListing.description, "A");
});
