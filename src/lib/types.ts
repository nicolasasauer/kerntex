// Mirrors the serde types in src-tauri/src/core (camelCase).

export type ErrorSource = "fileSystem" | "engine" | "archive" | "app";

export interface AppError {
  source: ErrorSource;
  title: string;
  detail: string | null;
  hint: string | null;
}

export interface FileNode {
  name: string;
  path: string;
  isDir: boolean;
  generated: boolean;
  children: FileNode[];
}

export type Engine = "tectonic" | "latexmk";
export type TexFlavor = "pdflatex" | "xelatex" | "lualatex";

export interface ProjectSettings {
  version: number;
  mainFile?: string;
  engine?: Engine;
  flavor?: TexFlavor;
}

export interface BuildHints {
  flavor: TexFlavor | null;
  hasLatexmkrc: boolean;
}

export interface ProjectInfo {
  root: string;
  name: string;
  tree: FileNode[];
  mainFile: string | null;
  settings: ProjectSettings;
  hints: BuildHints;
}

export type Severity = "error" | "warning" | "info";

export interface Diagnostic {
  severity: Severity;
  source: string;
  file: string | null;
  line: number | null;
  /** 1-based exact position, where the engine reports it (Typst). */
  column?: number;
  endLine?: number;
  endColumn?: number;
  message: string;
  hint: string | null;
}

export interface CompileRequest {
  root: string;
  mainFile: string;
  engine: Engine;
  flavor: TexFlavor;
}

export interface PackageRef {
  name: string;
  date: string | null;
  version: string | null;
  description: string | null;
}

export interface BuildDetails {
  program: string | null;
  format: string | null;
  documentClass: PackageRef | null;
  packages: PackageRef[];
  passes: number;
  enginePath: string | null;
  outDir: string | null;
  logPath: string | null;
}

export interface CompileResult {
  ok: boolean;
  canceled: boolean;
  pdfPath: string | null;
  diagnostics: Diagnostic[];
  durationMs: number;
  log: string;
  engineLabel: string;
  details: BuildDetails;
}

export interface Progress {
  stage: string;
  detail: string | null;
  elapsedMs: number;
}

export interface EngineStatus {
  tectonic: string | null;
  latexmk: string | null;
  tectonicCacheReady: boolean;
  tectonicVersion: string | null;
  latexmkVersion: string | null;
  tectonicCacheDir: string | null;
  typstVersion: string;
}

export interface LabelRef {
  name: string;
  file: string;
  line: number;
}

export interface CiteRef {
  key: string;
  kind: string;
  title: string | null;
  /** Family names; a trailing "others" stays as it is. */
  authors: string[];
  year: string | null;
  file: string;
}

export interface ProjectRefs {
  labels: LabelRef[];
  citations: CiteRef[];
}

export type Template =
  | "blank" | "article" | "report" | "beamer" | "letter"
  | "typstBlank" | "typstArticle" | "typstReport" | "typstSlides" | "typstLetter";
