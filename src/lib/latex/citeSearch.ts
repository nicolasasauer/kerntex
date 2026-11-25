// Searching citation keys by key, author, year and title, so an entry can be
// found without knowing the key a reference manager gave it.
import type { Completion } from "@codemirror/autocomplete";
import type { CiteRef } from "../types";

/** Lower case without diacritics, so "muller" finds "Müller". */
const fold = (s: string) => s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();

/** `Knuth`, `Knuth & Lamport`, `Knuth et al.` */
function shortAuthors(authors: string[]): string {
  const [first, second] = authors;
  if (!first) return "";
  if (authors.length === 1) return first;
  return authors.length === 2 && second !== "others" ? `${first} & ${second}` : `${first} et al.`;
}

interface Entry {
  key: string;
  /** Authors and year. */
  who: string;
  /** Everything searchable. */
  all: string;
  option: Completion;
}

// A library can hold thousands of entries: prepare them once per scan.
const cache = new WeakMap<CiteRef[], Entry[]>();

function entries(citations: CiteRef[]): Entry[] {
  let list = cache.get(citations);
  if (!list) {
    list = citations.map((c) => {
      const who = [shortAuthors(c.authors), c.year].filter(Boolean).join(" ");
      const authors = c.authors.filter((a) => a !== "others").join(", ");
      return {
        key: fold(c.key),
        who: fold(`${authors} ${c.year ?? ""}`),
        all: fold(`${c.key} ${authors} ${c.year ?? ""} ${c.title ?? ""}`),
        option: {
          label: c.key,
          detail: [who, c.title].filter(Boolean).join(" · ") || c.kind,
          type: "text",
          info: [c.title, authors, `@${c.kind} · ${c.file}`].filter(Boolean).join("\n"),
        },
      };
    });
    cache.set(citations, list);
  }
  return list;
}

/**
 * Entries containing every word of `query`: matching keys first, then authors
 * and year, then titles. Use with `filter: false`.
 */
export function searchCitations(citations: CiteRef[], query: string): Completion[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  const ranked: Completion[][] = [[], [], [], []];
  for (const e of entries(citations)) {
    if (!words.every((w) => e.all.includes(w))) continue;
    const rank = words.every((w, i) => (i ? e.key.includes(w) : e.key.startsWith(w))) ? 0
      : words.every((w) => e.key.includes(w)) ? 1
      : words.every((w) => e.key.includes(w) || e.who.includes(w)) ? 2
      : 3;
    ranked[rank].push(e.option);
  }
  return ranked.flat();
}
