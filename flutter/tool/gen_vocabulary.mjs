// Regenerates lib/src/editor/vocabulary.dart from the Tauri editor's data:
//   node --experimental-strip-types flutter/tool/gen_vocabulary.mjs

import { writeFileSync } from "node:fs";
const root = new URL("../../", import.meta.url);
const c = await import(new URL("src/lib/latex/completions.ts", root));
const p = await import(new URL("src/lib/latex/packages.ts", root));
const q = (s) => "'" + s.replace(/\\/g, "\\\\").replace(/'/g, "\\'").replace(/\$/g, "\\$").replace(/\n/g, "\\n") + "'";
let o = "// Generated from src/lib/latex/completions.ts and packages.ts — the same\n// vocabulary as the Tauri editor. Regenerate rather than edit by hand.\n\n";
o += "/// (insert, cursor offset in insert, detail)\nconst latexCommands = <(String, int, String)>[\n";
for (const [t, d] of c.COMMANDS) { const i = t.indexOf("{}"); o += `  (${q(t)}, ${i < 0 ? t.length : i + 1}, ${q(d)}),\n`; }
o += "];\n\n/// (name, detail)\nconst latexEnvironments = <(String, String)>[\n";
for (const [t, d] of c.ENVIRONMENTS) o += `  (${q(t)}, ${q(d)}),\n`;
o += "];\n\n/// Body for \\begin{…} snippets: (text, cursor offset).\nconst latexEnvBodies = <String, (String, int)>{\n";
for (const [k, v0] of Object.entries(c.ENV_BODY)) { const v = v0.replace(/\t/g, "  "); const i = v.indexOf("|"); o += `  ${q(k)}: (${q(v.replace("|", ""))}, ${i}),\n`; }
o += "};\n\n/// (insert, cursor offset, detail)\nconst typstFunctions = <(String, int, String)>[\n";
for (const [t, d] of c.TYPST_FUNCTIONS) { const i = t.indexOf("{}"); const s = i < 0 ? t : t.replace("{}", ""); o += `  (${q(s)}, ${i < 0 ? s.length : i}, ${q(d)}),\n`; }
o += "];\n\n/// Package name -> what it does.\nconst packageInfo = <String, String>{\n";
for (const [k, v] of Object.entries(p.PACKAGE_INFO)) o += `  ${q(k)}: ${q(v)},\n`;
o += "};\n";
writeFileSync(new URL("flutter/lib/src/editor/vocabulary.dart", root), o);
