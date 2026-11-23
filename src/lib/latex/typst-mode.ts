// Syntax highlighting for Typst (.typ) as a CodeMirror stream language.
// Covers markup (headings, *strong*, _emph_, `raw`, labels, references),
// math ($…$) and code (#keywords, function calls, strings, numbers, comments).
// Token names map onto the same highlight tags as the LaTeX mode, so both
// themes work unchanged.

import type { StreamParser, StringStream } from "@codemirror/language";

type Mode = "markup" | "math" | "code";

interface State {
  stack: Mode[];
  /** Bracket depth per code/content nesting, to know when `#expr` ends. */
  depth: number[];
  inBlockComment: boolean;
  inRaw: boolean;
  rawFence: string;
  strong: boolean;
  emph: boolean;
}

const KEYWORDS = new Set([
  "let", "set", "show", "import", "include", "if", "else", "for", "in", "while",
  "return", "break", "continue", "as", "and", "or", "not", "context",
]);
const ATOMS = new Set(["none", "auto", "true", "false"]);

function mode(s: State): Mode {
  return s.stack[s.stack.length - 1];
}

function comment(stream: StringStream, state: State): string | null {
  if (state.inBlockComment) {
    if (stream.skipTo("*/")) {
      stream.match("*/");
      state.inBlockComment = false;
    } else stream.skipToEnd();
    return "comment";
  }
  if (stream.match("//")) {
    stream.skipToEnd();
    return "comment";
  }
  if (stream.match("/*")) {
    state.inBlockComment = true;
    return "comment";
  }
  return null;
}

function codeToken(stream: StringStream, state: State): string | null {
  if (stream.eatSpace()) return null;
  const c = comment(stream, state);
  if (c) return c;
  if (stream.match(/^"(?:[^"\\]|\\.)*"?/)) return "string";
  if (stream.match(/^\d+(\.\d+)?(pt|mm|cm|in|em|fr|%|deg|rad)?\b/)) return "number";
  if (stream.match("[")) {
    // Content block inside code: back to markup until the matching "]".
    state.stack.push("markup");
    state.depth.push(0);
    return "bracket";
  }
  if (stream.match("$")) {
    state.stack.push("math");
    return "keyword";
  }
  const id = stream.match(/^[a-zA-Z_][\w-]*/) as RegExpMatchArray | null;
  if (id) {
    const w = id[0];
    if (KEYWORDS.has(w)) return "keyword";
    if (ATOMS.has(w)) return "atom";
    if (stream.peek() === "(" || stream.peek() === "[") return "tagName";
    return "variableName";
  }
  const ch = stream.next();
  if (ch === "(" || ch === "{") {
    state.depth[state.depth.length - 1]++;
    return "bracket";
  }
  if (ch === ")" || ch === "}") {
    const d = --state.depth[state.depth.length - 1];
    if (d <= 0 && state.stack.length > 1) {
      state.stack.pop();
      state.depth.pop();
    }
    return "bracket";
  }
  if (ch === "]" && state.stack.length > 1) {
    state.stack.pop();
    state.depth.pop();
    return "bracket";
  }
  return null;
}

function markupToken(stream: StringStream, state: State): string | null {
  if (state.inRaw) {
    if (stream.skipTo(state.rawFence)) {
      stream.match(state.rawFence);
      state.inRaw = false;
    } else stream.skipToEnd();
    return "string";
  }
  const c = comment(stream, state);
  if (c) return c;
  if (stream.sol() && stream.match(/^\s*=+\s/)) {
    stream.skipToEnd();
    return "keyword";
  }
  if (stream.sol() && stream.match(/^\s*(-|\+|\/)\s/)) return "bracket";
  const fence = stream.match(/^`{3,}/) as RegExpMatchArray | null;
  if (fence) {
    state.inRaw = true;
    state.rawFence = fence[0];
    stream.skipToEnd();
    return "string";
  }
  if (stream.match(/^`[^`]*`?/)) return "string";
  if (stream.match(/^<[\w\-.:]+>/)) return "atom";
  if (stream.match(/^@[\w\-.:]+/)) return "atom";
  if (stream.match(/^\\./)) return "string";
  if (stream.match("$")) {
    state.stack.push("math");
    return "keyword";
  }
  if (stream.peek() === "#") {
    stream.next();
    // `#expr` switches to code for one expression.
    if (stream.match(/^[a-zA-Z_][\w-]*/, false)) {
      const w = (stream.match(/^[a-zA-Z_][\w-]*/) as RegExpMatchArray)[0];
      if (KEYWORDS.has(w)) {
        state.stack.push("code");
        state.depth.push(0);
        return "keyword";
      }
      if (stream.peek() === "(") {
        state.stack.push("code");
        state.depth.push(0);
      } else if (stream.peek() === "[") {
        stream.next();
        state.stack.push("markup");
        state.depth.push(0);
      }
      return "tagName";
    }
    if (stream.peek() === "(" || stream.peek() === "{") {
      state.stack.push("code");
      state.depth.push(0);
    }
    return "tagName";
  }
  if (stream.peek() === "]" && state.stack.length > 1) {
    stream.next();
    state.stack.pop();
    state.depth.pop();
    return "bracket";
  }
  if (stream.match(/^\*(?=\S)/) || (state.strong && stream.match("*"))) {
    state.strong = !state.strong;
    return "keyword";
  }
  if (stream.match(/^_(?=\S)/) || (state.emph && stream.match("_"))) {
    state.emph = !state.emph;
    return "keyword";
  }
  stream.next();
  if (state.strong) return "strong";
  if (state.emph) return "emphasis";
  return null;
}

function mathToken(stream: StringStream, state: State): string | null {
  if (stream.match("$")) {
    state.stack.pop();
    return "keyword";
  }
  if (stream.eatSpace()) return null;
  if (stream.match(/^"(?:[^"\\]|\\.)*"?/)) return "string";
  if (stream.match(/^\d+(\.\d+)?/)) return "number";
  if (stream.match(/^#[a-zA-Z_][\w-]*/)) return "tagName";
  if (stream.match(/^[a-zA-Z][a-zA-Z.]+/)) return "variableName";
  stream.next();
  return "variableName";
}

export const typst: StreamParser<State> = {
  name: "typst",
  startState: () => ({ stack: ["markup"], depth: [0], inBlockComment: false, inRaw: false, rawFence: "", strong: false, emph: false }),
  copyState: (s) => ({ ...s, stack: [...s.stack], depth: [...s.depth] }),
  token(stream, state) {
    // A one-expression `#call(...)` ends at the end of its line if unbalanced.
    if (stream.sol() && mode(state) === "code" && state.depth[state.depth.length - 1] === 0 && state.stack.length > 1 && !state.inBlockComment) {
      state.stack.pop();
      state.depth.pop();
    }
    switch (mode(state)) {
      case "math":
        return mathToken(stream, state);
      case "code": {
        const t = codeToken(stream, state);
        // `#let x = 1` / `#set text(...)` end at line end; a bare `#name` ends after the name.
        return t;
      }
      default:
        return markupToken(stream, state);
    }
  },
  blankLine(state) {
    state.strong = false;
    state.emph = false;
  },
  languageData: { commentTokens: { line: "//", block: { open: "/*", close: "*/" } } },
};
