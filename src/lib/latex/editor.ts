// CodeMirror 6 setup for LaTeX. One EditorView; each open file keeps its own
// EditorState (so undo history survives tab switches).

import { EditorState, Compartment, StateEffect, type Extension } from "@codemirror/state";
import {
  EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection,
  dropCursor, rectangularSelection, highlightSpecialChars, hoverTooltip,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { StreamLanguage, bracketMatching, syntaxHighlighting, HighlightStyle, indentOnInput, foldGutter, foldKeymap } from "@codemirror/language";
import { stex } from "@codemirror/legacy-modes/mode/stex";
import { search, searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap, snippetCompletion, type CompletionContext, type Completion } from "@codemirror/autocomplete";
import { linter, lintGutter, forceLinting, type Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { tags as t } from "@lezer/highlight";
import { COMMANDS, ENVIRONMENTS, ENV_BODY, TYPST_FUNCTIONS } from "./completions";
import { typst } from "./typst-mode";
import { searchCitations } from "./citeSearch";
import { findMissingPackages, usepackageInsertPos, PACKAGE_INFO } from "./packages";
import type { Diagnostic, ProjectRefs } from "../types";

export interface EditorHost {
  onChange(path: string, text: string): void;
  getRefs(): ProjectRefs;
  /** Compile diagnostics for this file. */
  getDiagnostics(path: string): Diagnostic[];
  /** Preamble source for package checks (the main file's text), or null. */
  getPreamble(path: string): string | null;
  /** Called when the quick fix wants to edit another file (the main file). */
  addPackage(pkg: string, path: string): void;
}

const latex = StreamLanguage.define(stex);
const typstLang = StreamLanguage.define(typst);

function isTypst(path: string) {
  return /\.typ$/i.test(path);
}

const highlight = HighlightStyle.define([
  { tag: t.tagName, color: "var(--syn-command)" },
  { tag: t.keyword, color: "var(--syn-keyword)", fontWeight: "600" },
  { tag: [t.atom, t.bool], color: "var(--syn-atom)" },
  { tag: t.number, color: "var(--syn-number)" },
  { tag: t.special(t.variableName), color: "var(--syn-math)" },
  { tag: t.variableName, color: "var(--syn-math)" },
  { tag: t.comment, color: "var(--syn-comment)", fontStyle: "italic" },
  { tag: t.bracket, color: "var(--syn-bracket)" },
  { tag: t.string, color: "var(--syn-string)" },
  { tag: t.invalid, color: "var(--danger)" },
  { tag: t.strong, fontWeight: "700" },
  { tag: t.emphasis, fontStyle: "italic" },
]);

const baseTheme = EditorView.theme({
  "&": { height: "100%", backgroundColor: "var(--editor-bg)", color: "var(--text)" },
  ".cm-scroller": { fontFamily: "var(--font-mono)", lineHeight: "1.6" },
  ".cm-content": { caretColor: "var(--accent)", padding: "12px 0" },
  ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)", borderLeftWidth: "2px" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": { backgroundColor: "var(--selection) !important" },
  ".cm-gutters": { backgroundColor: "var(--editor-bg)", color: "var(--text-faint)", border: "none" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--text-muted)" },
  ".cm-activeLine": { backgroundColor: "var(--active-line)" },
  ".cm-matchingBracket": { backgroundColor: "var(--bracket-match)", outline: "none" },
  ".cm-searchMatch": { backgroundColor: "var(--search-match)" },
  ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "var(--search-match-selected)" },
  ".cm-selectionMatch": { backgroundColor: "var(--selection-match)" },
  ".cm-tooltip": { backgroundColor: "var(--surface-raised)", color: "var(--text)", border: "1px solid var(--border)", borderRadius: "8px", boxShadow: "var(--shadow-lg)", overflow: "hidden" },
  ".cm-tooltip-autocomplete > ul > li[aria-selected]": { backgroundColor: "var(--accent-soft)", color: "var(--text)" },
  ".cm-completionDetail": { color: "var(--text-muted)", fontStyle: "normal", marginLeft: "1em" },
  ".cm-completionInfo": { whiteSpace: "pre-line" },
  ".cm-panels": { backgroundColor: "var(--surface)", color: "var(--text)", borderColor: "var(--border)" },
  ".cm-panel.cm-search": { padding: "8px 10px", fontFamily: "var(--font-ui)" },
  ".cm-panel.cm-search input, .cm-panel.cm-search button": { fontFamily: "var(--font-ui)", fontSize: "13px" },
  ".cm-textfield": { backgroundColor: "var(--input-bg)", border: "1px solid var(--border)", borderRadius: "6px", color: "var(--text)", padding: "3px 6px" },
  ".cm-button": { backgroundImage: "none", backgroundColor: "var(--surface-raised)", border: "1px solid var(--border)", borderRadius: "6px", color: "var(--text)" },
  ".cm-diagnostic": { padding: "6px 10px", fontFamily: "var(--font-ui)", fontSize: "13px", maxWidth: "520px" },
  ".cm-diagnostic-error": { borderLeft: "3px solid var(--danger)" },
  ".cm-diagnostic-warning": { borderLeft: "3px solid var(--warning)" },
  ".cm-diagnostic-info": { borderLeft: "3px solid var(--info)" },
  ".cm-diagnosticAction": { backgroundColor: "var(--accent)", color: "var(--on-accent)", borderRadius: "6px", padding: "3px 8px", font: "inherit", fontSize: "12px" },
  ".cm-lintRange-error": { backgroundImage: "none", textDecoration: "underline wavy var(--danger)", textUnderlineOffset: "3px" },
  ".cm-lintRange-warning": { backgroundImage: "none", textDecoration: "underline wavy var(--warning)", textUnderlineOffset: "3px" },
  ".cm-lintRange-info": { backgroundImage: "none", textDecoration: "underline dotted var(--info)", textUnderlineOffset: "3px" },
  ".cm-foldGutter span": { color: "var(--text-faint)" },
  ".kx-pkg-tip": { padding: "8px 10px", fontFamily: "var(--font-ui)", fontSize: "13px", maxWidth: "380px" },
  ".kx-pkg-tip b": { fontFamily: "var(--font-mono)" },
});

/** Typst: `@` references (labels and citation keys) and `#` functions. */
function typstCompletions(host: EditorHost) {
  const fnOptions: Completion[] = TYPST_FUNCTIONS.map(([tpl, detail]) => {
    const label = "#" + tpl.replace(/[([\s].*$/, "");
    return snippetCompletion("#" + tpl.replace(/\{\}/g, "${}"), { label, detail, type: "function" });
  });
  return (ctx: CompletionContext) => {
    const ref = ctx.matchBefore(/@[\p{L}\p{N}_\-.:]*$/u);
    if (ref && (ref.from === 0 || !/\w/.test(ctx.state.sliceDoc(ref.from - 1, ref.from)))) {
      const refs = host.getRefs();
      // No validFor: citations are searched anew on every keystroke.
      const query = ref.text.slice(1);
      const lower = query.toLowerCase();
      return {
        from: ref.from + 1,
        filter: false,
        options: [
          ...refs.labels.filter((l) => l.name.toLowerCase().includes(lower)).map((l) => ({ label: l.name, detail: `${l.file}:${l.line}`, type: "variable" })),
          ...searchCitations(refs.citations, query),
        ],
      };
    }
    const word = ctx.matchBefore(/#[a-zA-Z.]*$/);
    if (!word || (word.from === word.to && !ctx.explicit)) return null;
    return { from: word.from, validFor: /^#[a-zA-Z.]*$/, options: fnOptions };
  };
}

/** Completions for \commands, \begin{env}, \ref{label}, \cite{key}. */
function latexCompletions(host: EditorHost) {
  const cmdOptions: Completion[] = COMMANDS.map(([c, detail]) => {
    const name = c.replace(/\{.*$|\[.*$|\s$/, "");
    if (c.includes("{")) {
      const tpl = "\\" + c.replace(/\{\}/g, () => "{${}}").replace(/\[width=\\linewidth\]/, "[width=\\linewidth]");
      return snippetCompletion(tpl, { label: "\\" + name, detail, type: "function" });
    }
    return { label: "\\" + name, apply: "\\" + c, detail, type: "keyword" };
  });
  const envOptions: Completion[] = ENVIRONMENTS.map(([env, detail]) => {
    const body = (ENV_BODY[env] ?? "\t|").replace(/\{\|\}/g, "{${}}").replace(/\|/, "${}").replace(/\{\}/g, "{}");
    return snippetCompletion(`\\begin{${env}}\n${body}\n\\end{${env}}`, { label: `\\begin{${env}}`, detail, type: "class" });
  });

  return (ctx: CompletionContext) => {
    // Inside \ref{…} / \cite{…} (also with optional args and comma lists).
    const arg = ctx.matchBefore(/\\([a-zA-Z]*(?:ref|cite[a-z]*|Cref|cref))\*?(?:\[[^\]]*\])*\{([^}]*)$/);
    if (arg) {
      const cmd = arg.text.match(/^\\([a-zA-Z]+)/)![1];
      const inner = arg.text.slice(arg.text.lastIndexOf("{") + 1);
      const lastComma = inner.lastIndexOf(",");
      const from = arg.to - (inner.length - (lastComma + 1));
      const refs = host.getRefs();
      if (/cite/i.test(cmd)) {
        // The query may contain spaces ("knuth 1984"); the chosen key replaces
        // all of it. filter: false because we match more than the key.
        const query = inner.slice(lastComma + 1);
        const start = from + query.length - query.trimStart().length;
        return { from: start, filter: false, options: searchCitations(refs.citations, query) };
      }
      return {
        from, validFor: /^[^,}\s]*$/,
        options: refs.labels.map((l) => ({ label: l.name, detail: `${l.file}:${l.line}`, type: "variable" })),
      };
    }
    const beginArg = ctx.matchBefore(/\\(begin|end)\{[a-zA-Z*]*$/);
    if (beginArg) {
      const from = beginArg.text.indexOf("{") + 1 + beginArg.from;
      return { from, validFor: /^[a-zA-Z*]*$/, options: ENVIRONMENTS.map(([e, detail]) => ({ label: e, detail, type: "class" })) };
    }
    const word = ctx.matchBefore(/\\[a-zA-Z]*$/);
    if (!word || (word.from === word.to && !ctx.explicit)) return null;
    return { from: word.from, validFor: /^\\[a-zA-Z]*$/, options: [...envOptions, ...cmdOptions] };
  };
}

/** Signals that compile diagnostics changed, so the linter must run again
 *  even though the document text didn't change. */
const diagnosticsChanged = StateEffect.define<null>();

/** Compile diagnostics + missing-package checks with a one-click fix. */
function latexLinter(host: EditorHost, getPath: () => string) {
  return linter((view) => {
    const path = getPath();
    const doc = view.state.doc;
    const text = doc.toString();
    const out: CmDiagnostic[] = [];
    for (const d of host.getDiagnostics(path)) {
      if (!d.line || d.line < 1 || d.line > doc.lines) continue;
      const line = doc.line(d.line);
      // Underline the near-snippet if we can find it, else the whole line.
      let from = line.from, to = line.to;
      if (d.column) {
        // Exact position (Typst reports start and end).
        from = Math.min(line.to, line.from + d.column - 1);
        const endLine = d.endLine && d.endLine <= doc.lines ? doc.line(d.endLine) : line;
        to = d.endColumn ? Math.min(endLine.to, endLine.from + d.endColumn - 1) : line.to;
        if (to <= from) to = Math.min(doc.length, from + 1);
      }
      const near = d.message.match(/near “(.+?)”/);
      if (near) {
        const i = line.text.indexOf(near[1].split(/\s/)[0]);
        if (i >= 0) { from = line.from + i; to = from + near[1].split(/\s/)[0].length; }
      }
      if (from === to) to = Math.min(doc.length, from + 1);
      out.push({ from, to, severity: d.severity, source: d.source, message: d.message + (d.hint ? `\n\n${d.hint}` : "") });
    }
    if (path.endsWith(".tex")) {
      const preamble = host.getPreamble(path);
      for (const m of findMissingPackages(text, preamble ?? text)) {
        out.push({
          from: m.from, to: m.to, severity: "warning", source: "KernTeX",
          message: `${m.trigger} needs the “${m.pkg}” package, which isn't loaded.`,
          actions: [{
            name: `Add \\usepackage{${m.pkg}} to preamble`,
            apply: (v) => {
              if (preamble === null || preamble === text) {
                const pos = usepackageInsertPos(v.state.doc.toString());
                v.dispatch({ changes: { from: pos, insert: `${pos > 0 ? "\n" : ""}\\usepackage{${m.pkg}}${pos > 0 ? "" : "\n"}` } });
              } else {
                host.addPackage(m.pkg, path);
              }
            },
          }],
        });
      }
    }
    return out;
  }, { delay: 400, needsRefresh: (u) => u.transactions.some((t) => t.effects.some((e) => e.is(diagnosticsChanged))) });
}

/** Hover over \usepackage{name} → short description. */
const packageHover = hoverTooltip((view, pos) => {
  const line = view.state.doc.lineAt(pos);
  for (const m of line.text.matchAll(/\\(?:usepackage|RequirePackage)\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}/g)) {
    const start = line.from + (m.index ?? 0) + m[0].indexOf("{") + 1;
    let offset = start;
    for (const name of m[1].split(",")) {
      const from = offset, to = offset + name.length;
      offset = to + 1;
      if (pos < from || pos > to) continue;
      const pkg = name.trim();
      return {
        pos: from, end: to, above: true,
        create() {
          const dom = document.createElement("div");
          dom.className = "kx-pkg-tip";
          const b = document.createElement("b");
          b.textContent = pkg;
          dom.append(b, document.createTextNode(" — " + (PACKAGE_INFO[pkg] ?? "No description available. See ctan.org/pkg/" + pkg)));
          return { dom };
        },
      };
    }
  }
  return null;
});

export interface EditorController {
  view: EditorView;
  open(path: string, text: string): void;
  has(path: string): boolean;
  close(path: string): void;
  rename(from: string, to: string): void;
  text(path: string): string | null;
  replaceAll(path: string, text: string): void;
  gotoLine(line: number): void;
  refreshLint(): void;
  setOptions(o: { fontSize: number; wrap: boolean }): void;
  insertAt(path: string, pos: number, insert: string): void;
  /** Inserts at the cursor of the current file (on a fresh line) and selects nothing. */
  insertAtCursor(text: string): void;
  focus(): void;
  destroy(): void;
}

export function createEditor(parent: HTMLElement, host: EditorHost, opts: { fontSize: number; wrap: boolean; onSave: () => void }): EditorController {
  const states = new Map<string, EditorState>();
  let current = "";
  const wrapC = new Compartment();
  const fontC = new Compartment();
  const fontTheme = (px: number) => EditorView.theme({ ".cm-scroller": { fontSize: `${px}px` } });

  const extensions = (path: string): Extension[] => [
    lineNumbers(), highlightActiveLineGutter(), highlightSpecialChars(), history(), foldGutter(), drawSelection(), dropCursor(),
    EditorState.allowMultipleSelections.of(true), indentOnInput(), bracketMatching(), closeBrackets(), rectangularSelection(),
    highlightActiveLine(), highlightSelectionMatches(), search({ top: true }),
    isTypst(path) ? typstLang : latex, syntaxHighlighting(highlight), baseTheme,
    autocompletion({ override: [isTypst(path) ? typstCompletions(host) : latexCompletions(host)], icons: false, activateOnTyping: true }),
    latexLinter(host, () => current), lintGutter(), packageHover,
    wrapC.of(opts.wrap ? EditorView.lineWrapping : []),
    fontC.of(fontTheme(opts.fontSize)),
    keymap.of([
      { key: "Mod-s", preventDefault: true, run: () => { opts.onSave(); return true; } },
      ...closeBracketsKeymap, ...defaultKeymap, ...searchKeymap, ...historyKeymap, ...foldKeymap, ...completionKeymap, indentWithTab,
    ]),
    EditorView.updateListener.of((u) => {
      if (u.docChanged) host.onChange(current, u.state.doc.toString());
    }),
  ];

  const view = new EditorView({ parent, state: EditorState.create({ doc: "", extensions: extensions("") }) });
  let lastOptions = { ...opts };

  function syncOptions(state: EditorState): EditorState {
    return state.update({ effects: [wrapC.reconfigure(lastOptions.wrap ? EditorView.lineWrapping : []), fontC.reconfigure(fontTheme(lastOptions.fontSize))] }).state;
  }

  return {
    view,
    open(path, text) {
      if (current) states.set(current, view.state);
      let st = states.get(path);
      if (!st) st = EditorState.create({ doc: text, extensions: extensions(path) });
      current = path;
      view.setState(syncOptions(st));
      forceLinting(view);
    },
    has: (p) => states.has(p) || current === p,
    close(path) {
      states.delete(path);
      if (current === path) current = "";
    },
    rename(from, to) {
      if (current === from) current = to;
      const st = states.get(from);
      if (st) { states.delete(from); states.set(to, st); }
    },
    text(path) {
      if (path === current) return view.state.doc.toString();
      return states.get(path)?.doc.toString() ?? null;
    },
    replaceAll(path, text) {
      if (path === current) view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
      else states.delete(path);
    },
    insertAt(path, pos, insert) {
      if (path === current) {
        view.dispatch({ changes: { from: pos, insert } });
      } else {
        const st = states.get(path);
        if (st) states.set(path, st.update({ changes: { from: pos, insert } }).state);
      }
    },
    insertAtCursor(text) {
      const { doc } = view.state;
      let line = doc.lineAt(view.state.selection.main.head);
      let at: number, insert: string;
      if (line.text.trim() === "") {
        // On an empty line: insert right here.
        at = line.from;
        insert = text;
      } else {
        // Inside a paragraph: never split it — insert after its last line.
        while (line.number < doc.lines && doc.line(line.number + 1).text.trim() !== "") line = doc.line(line.number + 1);
        at = line.to;
        insert = "\n\n" + text.replace(/\n$/, "");
      }
      view.dispatch({ changes: { from: at, insert }, selection: { anchor: at + insert.length }, scrollIntoView: true });
      view.focus();
    },
    gotoLine(line) {
      const doc = view.state.doc;
      const l = doc.line(Math.max(1, Math.min(line, doc.lines)));
      view.dispatch({ selection: { anchor: l.from }, effects: EditorView.scrollIntoView(l.from, { y: "center" }) });
      view.focus();
    },
    refreshLint() {
      view.dispatch({ effects: diagnosticsChanged.of(null) });
      forceLinting(view);
    },
    setOptions(o) {
      lastOptions = { ...lastOptions, ...o };
      view.dispatch({ effects: [wrapC.reconfigure(o.wrap ? EditorView.lineWrapping : []), fontC.reconfigure(fontTheme(o.fontSize))] });
    },
    focus: () => view.focus(),
    destroy: () => view.destroy(),
  };
}
