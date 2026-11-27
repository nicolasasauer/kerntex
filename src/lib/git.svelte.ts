// Git panel state: status of the open project's repository and the actions
// behind the panel's buttons. All Git work happens in the backend
// (src-tauri/src/core/git.rs), which calls the system `git`.

import { api, toAppError } from "./api";
import { ws } from "./workspace.svelte";
import { settings } from "./settings.svelte";
import { toast, toastError } from "./toasts.svelte";
import type { AppError, GitChange, GitStatus } from "./types";

export interface GitOp {
  label: string;
  /** Latest progress line from Git ("Writing objects: 45% (9/20)"). */
  line: string | null;
  startedAt: number;
  /** Network operations can be stopped. */
  cancelable: boolean;
  /** Which way data flows, for the animated Git tab icon. */
  direction?: "push" | "pull";
}

class GitState {
  status: GitStatus | null = $state(null);
  /** Status couldn't be read (shown in the panel). */
  error: AppError | null = $state(null);
  loading = $state(false);
  message = $state("");
  /** The user typed in the message field: don't replace it with a suggestion. */
  edited = $state(false);
  suggesting = $state(false);
  op: GitOp | null = $state(null);
  /** File whose diff is shown. */
  diffFor: GitChange | null = $state(null);

  #root: string | null = null;
  #timer: ReturnType<typeof setTimeout> | undefined;
  #unlisten: (() => void) | null = null;

  get changeCount() {
    return this.status?.repo ? this.status.changes.length : 0;
  }

  get conflicts() {
    return this.status?.changes.filter((c) => c.kind === "conflicted") ?? [];
  }

  /** Something can be committed right now. */
  get canCommit() {
    const s = this.status;
    return !!s?.repo && !!s.identity && !this.op && (s.changes.length > 0 || s.merging) && this.conflicts.length === 0;
  }

  async init() {
    this.#unlisten ??= await api.onGitProgress((p) => {
      if (this.op) this.op = { ...this.op, line: p.line };
    });
  }

  /** Re-reads the status soon (coalesces bursts of saves). */
  schedule(ms = 400) {
    clearTimeout(this.#timer);
    this.#timer = setTimeout(() => this.refresh(), ms);
  }

  async refresh() {
    const root = ws.project?.root ?? null;
    if (root !== this.#root) {
      // Another project: forget everything about the previous one.
      this.#root = root;
      this.status = null;
      this.error = null;
      this.message = "";
      this.edited = false;
    }
    if (!root) return;
    this.loading = true;
    try {
      const st = await api.gitStatus(root);
      if (ws.project?.root !== root) return;
      this.status = st;
      this.error = null;
      if (!this.edited) this.suggest();
    } catch (e) {
      this.error = toAppError(e);
    } finally {
      this.loading = false;
    }
  }

  /** Pre-fills the message from what changed (rule-based, see core/git.rs). */
  async suggest(force = false) {
    const root = this.#root;
    const st = this.status;
    if (!root || !st?.repo || (this.edited && !force)) return;
    if (st.changes.length === 0 && !st.merging) {
      if (!this.edited) this.message = "";
      return;
    }
    this.suggesting = true;
    try {
      const msg = await api.gitSuggestMessage(root);
      if (force || !this.edited) {
        this.message = msg;
        this.edited = false;
      }
    } catch {
      /* no suggestion; the field stays as it is */
    } finally {
      this.suggesting = false;
    }
  }

  setMessage(text: string) {
    this.message = text;
    this.edited = text.trim().length > 0;
  }

  async #run<T>(label: string, cancelable: boolean, f: () => Promise<T>, direction?: "push" | "pull"): Promise<T> {
    this.op = { label, line: null, startedAt: performance.now(), cancelable, direction };
    try {
      return await f();
    } finally {
      this.op = null;
    }
  }

  /** Saves open files first, so the commit contains what the user sees. */
  async #saveFirst(): Promise<boolean> {
    if (ws.dirtyCount === 0) return true;
    this.op = { label: "Saving files…", line: null, startedAt: performance.now(), cancelable: false };
    const ok = await ws.saveAll();
    this.op = null;
    return ok;
  }

  get remoteName() {
    return this.status?.remote ?? "remote";
  }

  /** Commit, then (with `push`) pull and push: the usual "sync" in one click. */
  async commit(push: boolean) {
    const root = this.#root;
    if (!root || this.op) return;
    if (!(await this.#saveFirst())) return;
    await this.refresh();
    const st = this.status;
    if (!st?.repo) return;
    if (st.changes.length === 0 && !st.merging) {
      if (push && st.ahead > 0) return this.push();
      toast({ kind: "info", source: "Git", title: "Nothing to commit", body: "All changes are already committed." });
      return;
    }
    if (!this.message.trim()) await this.suggest(true);
    const message = this.message.trim();
    if (!message) {
      toast({ kind: "warning", source: "Git", title: "The commit message is empty", body: "Describe in a few words what you changed." });
      return;
    }
    try {
      const c = await this.#run("Committing…", false, () => api.gitCommit(root, message));
      this.message = "";
      this.edited = false;
      await this.refresh();
      if (!push || !st.remote) {
        toast({ kind: "success", source: "Git", title: `Committed “${c.subject}”`, body: st.remote ? "Not pushed yet — use “Commit & Push” or Push to upload it." : null });
        return;
      }
      if (await this.#sync(root, st.upstream != null)) {
        toast({ kind: "success", source: "Git", title: `Committed and pushed “${c.subject}”`, body: `to ${st.remote}${st.branch ? `/${st.branch}` : ""}` });
      }
    } catch (e) {
      toastError(toAppError(e));
    } finally {
      await this.refresh();
    }
  }

  /** Pull (if the branch is already on the remote), then push. False if stopped. */
  async #sync(root: string, pullFirst: boolean): Promise<boolean> {
    if (pullFirst) {
      const r = await this.#run(`Getting changes from ${this.remoteName}…`, true, () => api.gitPull(root), "pull");
      if (r.updated) await this.#afterPull();
      if (r.conflicts.length) {
        this.#conflictToast(r.conflicts);
        return false;
      }
    }
    await this.#run(`Pushing to ${this.remoteName}…`, true, () => api.gitPush(root), "push");
    return true;
  }

  async push() {
    const root = this.#root;
    if (!root || this.op) return;
    try {
      if (await this.#sync(root, this.status?.upstream != null)) {
        toast({ kind: "success", source: "Git", title: `Pushed to ${this.remoteName}` });
      }
    } catch (e) {
      toastError(toAppError(e));
    } finally {
      await this.refresh();
    }
  }

  async pull() {
    const root = this.#root;
    if (!root || this.op) return;
    if (!(await this.#saveFirst())) return;
    try {
      const r = await this.#run(`Getting changes from ${this.remoteName}…`, true, () => api.gitPull(root), "pull");
      if (r.updated) await this.#afterPull();
      if (r.conflicts.length) this.#conflictToast(r.conflicts);
      else toast({ kind: "success", source: "Git", title: r.updated ? "Got the latest changes" : "Already up to date" }, 2500);
    } catch (e) {
      toastError(toAppError(e));
    } finally {
      await this.refresh();
    }
  }

  /** Files on disk changed: show the new text in open tabs, recompile. */
  async #afterPull() {
    await ws.reloadFromDisk();
    if (settings.compileOnSave) ws.compile();
  }

  #conflictToast(files: string[]) {
    toast(
      {
        kind: "warning",
        source: "Git",
        title: files.length === 1 ? `Conflict in ${files[0]}` : `Conflicts in ${files.length} files`,
        body: "You and someone else changed the same lines. Open the file, keep the right text between the <<<<<<< and >>>>>>> markers, save, then commit.",
        action: { label: "Open", run: () => ws.openFile(files[0]) },
      },
      12000,
    );
  }

  async cancel() {
    await api.gitCancel();
  }

  async setUp() {
    const root = this.#root;
    if (!root) return;
    try {
      await this.#run("Setting up Git…", false, () => api.gitInit(root));
      toast({ kind: "success", source: "Git", title: "Git is set up for this project", body: "Build output (.kerntex/out/) is ignored. Make your first commit below." });
      await ws.refreshTree();
    } catch (e) {
      toastError(toAppError(e));
    } finally {
      await this.refresh();
    }
  }

  async setRemote(url: string): Promise<boolean> {
    const root = this.#root;
    if (!root) return false;
    try {
      await api.gitSetRemote(root, url);
      toast({ kind: "success", source: "Git", title: "Remote saved", body: url.trim() });
      return true;
    } catch (e) {
      toastError(toAppError(e));
      return false;
    } finally {
      await this.refresh();
    }
  }

  async setIdentity(name: string, email: string): Promise<boolean> {
    const root = this.#root;
    if (!root) return false;
    try {
      await api.gitSetIdentity(root, name, email);
      return true;
    } catch (e) {
      toastError(toAppError(e));
      return false;
    } finally {
      await this.refresh();
    }
  }

  async abortMerge() {
    const root = this.#root;
    if (!root || this.op) return;
    try {
      await this.#run("Aborting the merge…", false, () => api.gitAbortMerge(root));
      await ws.reloadFromDisk();
      toast({ kind: "info", source: "Git", title: "Merge aborted", body: "Your files are back to how they were before the pull." });
    } catch (e) {
      toastError(toAppError(e));
    } finally {
      await this.refresh();
    }
  }
}

export const git = new GitState();

/** "github.com/you/thesis" from an https or ssh remote URL. */
export function remoteLabel(url: string | null): string {
  if (!url) return "";
  const m = url.match(/^(?:[a-z+]+:\/\/)?(?:[^@/]+@)?([^/:]+)[:/](.+?)(?:\.git)?\/?$/i);
  return m ? `${m[1]}/${m[2]}` : url;
}

export function timeAgo(unixSecs: number): string {
  const s = Math.max(0, Date.now() / 1000 - unixSecs);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  if (s < 86400 * 30) return `${Math.floor(s / 86400)} d ago`;
  return new Date(unixSecs * 1000).toLocaleDateString();
}
