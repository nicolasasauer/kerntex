// Mirrors the serde types in src-tauri/src/core (camelCase).

export type ErrorSource = "fileSystem" | "engine" | "archive" | "git" | "app";

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
  /** 1-based exact position, where known (Typst; LaTeX missing-package errors). */
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

// --- Git (src-tauri/src/core/git.rs) ---

export type ChangeKind = "modified" | "added" | "deleted" | "renamed" | "untracked" | "conflicted";

export interface GitChange {
  path: string;
  oldPath: string | null;
  kind: ChangeKind;
}

export interface CommitInfo {
  hash: string;
  subject: string;
  /** Unix seconds. */
  time: number;
}

export interface GitStatus {
  gitVersion: string | null;
  repo: boolean;
  branch: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  remote: string | null;
  remoteUrl: string | null;
  hasCommits: boolean;
  lastCommit: CommitInfo | null;
  merging: boolean;
  identity: { name: string; email: string } | null;
  changes: GitChange[];
}

export interface PullResult {
  updated: boolean;
  conflicts: string[];
}

export interface GitProgress {
  op: "pull" | "push";
  line: string;
}

// --- MCP server (src-tauri/src/mcp) ---

export interface McpClient {
  id: number;
  name: string;
}

export interface McpSetup {
  /** Absolute path of the `kerntex-mcp` command, if found. */
  command: string | null;
  /** Why `command` is missing, in plain language. */
  problem: string | null;
  endpoint: string;
  /** The Claude Code CLI, if installed. */
  claudeCode: string | null;
}

/** A request from the MCP server that needs the editor. */
export interface McpRequest {
  id: number;
  op: "read" | "apply" | "buffers" | "created" | "compile";
  args: Record<string, unknown>;
}

export interface McpActivity {
  client: string;
  tool: string;
  detail: string;
  ok: boolean;
  error: string | null;
  /** Added by the UI. */
  at?: number;
}
