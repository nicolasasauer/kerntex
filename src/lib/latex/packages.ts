// Small built-in knowledge base: which package provides a command or
// environment, and what common packages do. Used for the "missing package"
// hint + quick fix and for hover info on \usepackage{…}.

/** Command (without backslash) or `env:name` → package. */
export const PROVIDED_BY: Record<string, string> = {
  includegraphics: "graphicx", graphicspath: "graphicx", rotatebox: "graphicx", scalebox: "graphicx", resizebox: "graphicx",
  toprule: "booktabs", midrule: "booktabs", bottomrule: "booktabs", cmidrule: "booktabs",
  SI: "siunitx", si: "siunitx", num: "siunitx", qty: "siunitx", unit: "siunitx",
  "env:align": "amsmath", "env:align*": "amsmath", "env:gather": "amsmath", "env:multline": "amsmath", "env:split": "amsmath",
  "env:pmatrix": "amsmath", "env:bmatrix": "amsmath", "env:vmatrix": "amsmath", "env:cases": "amsmath",
  eqref: "amsmath", text: "amsmath", DeclareMathOperator: "amsmath", dfrac: "amsmath", tfrac: "amsmath", binom: "amsmath",
  mathbb: "amssymb", mathfrak: "amssymb", varnothing: "amssymb", leqslant: "amssymb", geqslant: "amssymb",
  "env:tikzpicture": "tikz", tikz: "tikz", usetikzlibrary: "tikz",
  url: "url", href: "hyperref", hypersetup: "hyperref", autoref: "hyperref",
  textcolor: "xcolor", color: "xcolor", colorbox: "xcolor", definecolor: "xcolor", rowcolor: "xcolor",
  cite: "biblatex", printbibliography: "biblatex", addbibresource: "biblatex", parencite: "biblatex", textcite: "biblatex", autocite: "biblatex",
  "env:lstlisting": "listings", lstinputlisting: "listings", lstset: "listings",
  "env:minted": "minted",
  "env:subfigure": "subcaption", subcaption: "subcaption",
  "env:tabularx": "tabularx", "env:longtable": "longtable", multirow: "multirow",
  "env:wrapfigure": "wrapfig",
  cref: "cleveref", Cref: "cleveref",
  todo: "todonotes", listoftodos: "todonotes",
  lipsum: "lipsum", blindtext: "blindtext",
  "env:enumerate*": "enumitem", setlist: "enumitem",
  geometry: "geometry", newgeometry: "geometry",
  fancyhead: "fancyhdr", fancyfoot: "fancyhdr", pagestyle: "",
  sisetup: "siunitx", ce: "mhchem",
  qquad: "", mathrm: "", frac: "", label: "", ref: "",
};

/** `\cite` works without biblatex (plain BibTeX); only nudge when no bibliography setup exists. */
const ALTERNATIVES: Record<string, string[]> = {
  biblatex: ["natbib", "biblatex", "cite", "apacite"],
  url: ["url", "hyperref", "xurl"],
  xcolor: ["xcolor", "color", "tikz", "pgf", "beamer"],
  amssymb: ["amssymb", "amsfonts", "unicode-math"],
  amsmath: ["amsmath", "mathtools", "unicode-math"],
  tikz: ["tikz", "pgfplots"],
  graphicx: ["graphicx", "graphics", "tikz", "beamer"],
  cleveref: ["cleveref"],
};

/** Classes that already load these packages. */
const CLASS_PROVIDES: Record<string, string[]> = {
  beamer: ["xcolor", "graphicx", "hyperref", "amsmath", "amssymb", "url", "tikz"],
  memoir: ["graphicx", "url"],
  scrartcl: [], scrreprt: [], scrbook: [],
};

export const PACKAGE_INFO: Record<string, string> = {
  amsmath: "Advanced math: align, gather, matrices, \\eqref, \\text …",
  amssymb: "Extra math symbols and fonts like \\mathbb{R}.",
  amsthm: "Theorem environments (\\newtheorem, proof).",
  mathtools: "Fixes and extensions for amsmath.",
  graphicx: "Include images with \\includegraphics.",
  booktabs: "Professional table rules: \\toprule, \\midrule, \\bottomrule.",
  siunitx: "Numbers and units: \\SI{3}{\\meter}, \\num{1e3}.",
  tikz: "Draw vector graphics directly in LaTeX.",
  pgfplots: "Plots and charts based on TikZ.",
  hyperref: "Clickable links, PDF bookmarks and metadata. Load near the end of the preamble.",
  url: "Typeset URLs with line breaks: \\url{…}.",
  xcolor: "Colours: \\textcolor, \\colorbox, named colours.",
  biblatex: "Modern bibliographies: \\addbibresource, \\printbibliography.",
  natbib: "Author–year citations for BibTeX: \\citet, \\citep.",
  listings: "Source code listings with syntax highlighting.",
  minted: "Code highlighting via Pygments (needs -shell-escape).",
  geometry: "Page size and margins: \\usepackage[margin=2.5cm]{geometry}.",
  fontenc: "Font encoding; T1 enables proper hyphenation of accented words (pdfLaTeX).",
  inputenc: "Input encoding (UTF-8 is the default since 2018).",
  lmodern: "Latin Modern fonts — a scalable version of the default font.",
  microtype: "Subtle typographic improvements (protrusion, expansion).",
  babel: "Language support: hyphenation, captions, quotes.",
  csquotes: "Context-sensitive quotation marks: \\enquote{…}.",
  fontspec: "System fonts with XeLaTeX/LuaLaTeX: \\setmainfont{…}.",
  caption: "Customise figure and table captions.",
  subcaption: "Sub-figures and sub-tables with their own captions.",
  float: "Extra float control, e.g. [H] placement.",
  tabularx: "Tables with auto-width X columns.",
  longtable: "Tables that break across pages.",
  multirow: "Cells spanning several rows.",
  enumitem: "Customise itemize/enumerate/description lists.",
  cleveref: "Smart references: \\cref{fig:x} → “Figure 3”.",
  fancyhdr: "Custom headers and footers.",
  setspace: "Line spacing: \\onehalfspacing, \\doublespacing.",
  parskip: "Paragraphs separated by space instead of indentation.",
  todonotes: "Margin notes: \\todo{…}.",
  lipsum: "Dummy text: \\lipsum[1-3].",
  wrapfig: "Figures with text flowing around them.",
  xurl: "Like url, but allows line breaks anywhere in URLs.",
  iftex: "Detect the engine (\\ifPDFTeX, \\ifXeTeX, \\ifLuaTeX).",
  mhchem: "Chemical formulae: \\ce{H2O}.",
  glossaries: "Glossaries and lists of acronyms.",
  acro: "Acronyms that are expanded on first use.",
  "unicode-math": "Unicode math fonts with XeLaTeX/LuaLaTeX.",
};

export interface MissingPackage {
  pkg: string;
  /** The command or environment that triggered it (for the message). */
  trigger: string;
  from: number;
  to: number;
}

/** Loaded packages + document class, from the preamble. */
export function loadedPackages(doc: string): { packages: Set<string>; docClass: string | null; preambleEnd: number } {
  const end = doc.indexOf("\\begin{document}");
  const preamble = stripComments(end >= 0 ? doc.slice(0, end) : doc);
  const packages = new Set<string>();
  for (const m of preamble.matchAll(/\\(?:usepackage|RequirePackage)\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}/g)) {
    m[1].split(",").map((s) => s.trim()).filter(Boolean).forEach((p) => packages.add(p));
  }
  const cls = preamble.match(/\\documentclass\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}/);
  return { packages, docClass: cls ? cls[1].trim() : null, preambleEnd: end };
}

function stripComments(s: string) {
  return s.replace(/(^|[^\\])%.*$/gm, "$1");
}

/**
 * Finds uses of commands/environments whose package isn't loaded. Only runs on
 * a file with a preamble (the main file); included files would need the main
 * file's preamble, which we pass in as `preambleSource`.
 */
export function findMissingPackages(doc: string, preambleSource: string = doc): MissingPackage[] {
  const { packages, docClass, preambleEnd } = loadedPackages(preambleSource);
  if (!docClass && preambleSource === doc) return [];
  const provided = new Set([...packages, ...(CLASS_PROVIDES[docClass ?? ""] ?? [])]);
  // biblatex also counts as "bibliography set up" when using plain \bibliography.
  const hasBib = /\\bibliography\{/.test(preambleSource) || /\\bibliography\{/.test(doc);
  const out: MissingPackage[] = [];
  const seen = new Set<string>();
  const body = preambleSource === doc && preambleEnd >= 0 ? preambleEnd : 0;
  const re = /\\begin\{([a-zA-Z*]+)\}|\\([a-zA-Z]+)/g;
  const text = doc;
  for (const m of text.matchAll(re)) {
    const idx = m.index ?? 0;
    if (idx < body) continue;
    // skip comments
    const lineStart = text.lastIndexOf("\n", idx) + 1;
    if (/(^|[^\\])%/.test(text.slice(lineStart, idx))) continue;
    const key = m[1] ? `env:${m[1]}` : m[2];
    const pkg = PROVIDED_BY[key];
    if (!pkg || seen.has(pkg)) continue;
    if (pkg === "biblatex" && (hasBib || provided.has("natbib") || provided.has("biblatex"))) continue;
    const alts = ALTERNATIVES[pkg] ?? [pkg];
    if (alts.some((a) => provided.has(a))) continue;
    if (pkg === "amsmath" && provided.has("amssymb") && key === "text") continue;
    seen.add(pkg);
    out.push({ pkg, trigger: m[1] ? `\\begin{${m[1]}}` : `\\${m[2]}`, from: idx, to: idx + m[0].length });
  }
  return out;
}

/** Where to insert a new \usepackage line: after the last one, else after \documentclass. */
export function usepackageInsertPos(doc: string): number {
  const end = doc.indexOf("\\begin{document}");
  const pre = end >= 0 ? doc.slice(0, end) : doc;
  let pos = -1;
  for (const m of pre.matchAll(/^\\usepackage.*$/gm)) pos = (m.index ?? 0) + m[0].length;
  if (pos < 0) {
    const cls = pre.match(/^\\documentclass.*$/m);
    if (cls) pos = (cls.index ?? 0) + cls[0].length;
  }
  return pos < 0 ? 0 : pos;
}
