<script lang="ts">
  import GitBranch from "@lucide/svelte/icons/git-branch";
  import GitMerge from "@lucide/svelte/icons/git-merge";
  import GitCommit from "@lucide/svelte/icons/git-commit-horizontal";
  import ArrowDownToLine from "@lucide/svelte/icons/arrow-down-to-line";
  import ArrowUpFromLine from "@lucide/svelte/icons/arrow-up-from-line";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import WandSparkles from "@lucide/svelte/icons/wand-sparkles";
  import CloudUpload from "@lucide/svelte/icons/cloud-upload";
  import Link from "@lucide/svelte/icons/link";
  import Download from "@lucide/svelte/icons/download";
  import FilePen from "@lucide/svelte/icons/file-pen";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import SideTabs from "./SideTabs.svelte";
  import GitDiff from "./GitDiff.svelte";
  import { git, remoteLabel, timeAgo } from "../lib/git.svelte";
  import { ws } from "../lib/workspace.svelte";
  import { openUrl } from "../lib/api";
  import type { ChangeKind, GitChange } from "../lib/types";

  const st = $derived(git.status);
  let spinning = $state(false);
  let editingRemote = $state(false);
  let remoteUrl = $state("");
  let name = $state("");
  let email = $state("");

  // Elapsed time of the running operation.
  let now = $state(performance.now());
  $effect(() => {
    if (!git.op) return;
    now = performance.now();
    const t = setInterval(() => (now = performance.now()), 250);
    return () => clearInterval(t);
  });

  const BADGE: Record<ChangeKind, { letter: string; label: string; cls: string }> = {
    modified: { letter: "M", label: "Modified", cls: "mod" },
    added: { letter: "A", label: "Added", cls: "add" },
    untracked: { letter: "U", label: "New file", cls: "add" },
    deleted: { letter: "D", label: "Deleted", cls: "del" },
    renamed: { letter: "R", label: "Renamed", cls: "ren" },
    conflicted: { letter: "!", label: "Conflict", cls: "del" },
  };

  function split(path: string) {
    const i = path.lastIndexOf("/");
    return i < 0 ? { name: path, dir: "" } : { name: path.slice(i + 1), dir: path.slice(0, i) };
  }

  async function refresh() {
    spinning = true;
    await git.refresh();
    setTimeout(() => (spinning = false), 300);
  }

  function openChange(c: GitChange) {
    if (c.kind === "deleted") git.diffFor = c;
    else ws.openFile(c.path);
  }

  async function saveRemote(e: SubmitEvent) {
    e.preventDefault();
    if (await git.setRemote(remoteUrl)) editingRemote = false;
  }

  async function saveIdentity(e: SubmitEvent) {
    e.preventDefault();
    await git.setIdentity(name, email);
  }

  function messageKey(e: KeyboardEvent) {
    // Ctrl+Enter = the primary button.
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      e.stopPropagation();
      if (git.canCommit) git.commit(!!st?.remote);
    }
  }

  function secs(ms: number) {
    return Math.max(0, Math.round(ms / 1000)) + " s";
  }
</script>

<div class="git">
  <div class="head">
    <SideTabs />
    <div class="tools">
      {#if st?.repo}
        <button class="icon-btn" title={st.upstream ? `Pull: get changes from ${st.upstream}` : "Pull (available after the first push)"} aria-label="Pull" disabled={!st.upstream || !!git.op} onclick={() => git.pull()}><ArrowDownToLine size={15} /></button>
        <button class="icon-btn" title={st.remote ? `Push to ${st.remote}` : "Push (add a remote first)"} aria-label="Push" disabled={!st.remote || !st.hasCommits || !!git.op} onclick={() => git.push()}><ArrowUpFromLine size={15} /></button>
      {/if}
      <button class="icon-btn" class:spin={spinning || git.loading} title="Refresh Git status" aria-label="Refresh Git status" onclick={refresh}><RefreshCw size={15} /></button>
    </div>
  </div>

  <div class="body">
    {#if !st}
      {#if git.error}
        <div class="empty">
          <TriangleAlert size={22} />
          <p class="title">{git.error.title}</p>
          {#if git.error.hint}<p class="muted">{git.error.hint}</p>{/if}
          <button class="btn" onclick={refresh}>Try again</button>
        </div>
      {:else}
        <div class="loading muted"><span class="spinner"></span>Reading Git status…</div>
      {/if}
    {:else if !st.gitVersion}
      <div class="empty">
        <GitBranch size={24} />
        <p class="title">Git isn't installed</p>
        <p class="muted">KernTeX uses the Git on your computer. On Windows, install “Git for Windows” — it also signs you in to GitHub through your browser.</p>
        <button class="btn primary" onclick={() => openUrl("https://git-scm.com/downloads")}><Download size={15} />Download Git</button>
        <button class="btn ghost" onclick={refresh}>I installed it — check again</button>
      </div>
    {:else if !st.repo}
      <div class="empty">
        <GitBranch size={24} />
        <p class="title">Keep every version of your project</p>
        <p class="muted">Git saves snapshots (commits) of your project and syncs them with GitHub, GitLab or Overleaf.</p>
        <button class="btn primary" disabled={!!git.op} onclick={() => git.setUp()}><GitBranch size={15} />Set up Git for this project</button>
      </div>
    {:else}
      <div class="branch" title={st.remoteUrl ?? "No remote"}>
        <GitBranch size={14} />
        <span class="name">{st.branch ?? "detached HEAD"}</span>
        {#if st.ahead > 0}<span class="sync" title="{st.ahead} commit(s) not pushed yet">↑{st.ahead}</span>{/if}
        {#if st.behind > 0}<span class="sync" title="{st.behind} commit(s) on the remote you don't have yet">↓{st.behind}</span>{/if}
        {#if st.remoteUrl && !editingRemote}
          <button class="remote" title="{st.remoteUrl} — click to change" onclick={() => { remoteUrl = st.remoteUrl ?? ""; editingRemote = true; }}>{remoteLabel(st.remoteUrl)}</button>
        {/if}
      </div>

      {#if !st.remote || editingRemote}
        <form class="card" onsubmit={saveRemote}>
          <p class="card-title"><Link size={14} />{st.remote ? "Change remote" : "Connect to GitHub, GitLab or Overleaf"}</p>
          <p class="muted small">Paste the repository URL, e.g. of a new, empty GitHub repository or your Overleaf Git URL (Overleaf: Menu → Git).</p>
          <!-- svelte-ignore a11y_autofocus -->
          <input class="input" placeholder="https://github.com/you/thesis.git" bind:value={remoteUrl} autofocus={editingRemote} spellcheck="false" aria-label="Remote URL" />
          <div class="row">
            <button class="btn primary" type="submit" disabled={!remoteUrl.trim()}>Save</button>
            {#if editingRemote}<button class="btn ghost" type="button" onclick={() => (editingRemote = false)}>Cancel</button>{/if}
          </div>
        </form>
      {/if}

      {#if !st.identity}
        <form class="card" onsubmit={saveIdentity}>
          <p class="card-title">Who's committing?</p>
          <p class="muted small">Git writes your name and email into every commit. They're saved in your Git settings for all projects.</p>
          <input class="input" placeholder="Your name" bind:value={name} aria-label="Name" />
          <input class="input" placeholder="you@example.com" type="email" bind:value={email} aria-label="Email" />
          <div class="row"><button class="btn primary" type="submit" disabled={!name.trim() || !email.includes("@")}>Save</button></div>
        </form>
      {/if}

      {#if st.merging}
        <div class="card warn">
          <p class="card-title"><GitMerge size={14} />{git.conflicts.length ? "Merge conflict" : "Merge ready to commit"}</p>
          {#if git.conflicts.length}
            <p class="small">You and someone else changed the same lines. In each file below, keep the right text between <code>&lt;&lt;&lt;&lt;&lt;&lt;&lt;</code> and <code>&gt;&gt;&gt;&gt;&gt;&gt;&gt;</code>, delete the markers and save. Then commit.</p>
            {#each git.conflicts as c (c.path)}
              <button class="conflict" onclick={() => ws.openFile(c.path)}><FilePen size={13} />{c.path}</button>
            {/each}
          {:else}
            <p class="small">All conflicts are resolved. Commit to finish the merge.</p>
          {/if}
          <div class="row"><button class="btn" disabled={!!git.op} onclick={() => git.abortMerge()}>Abort merge</button></div>
        </div>
      {/if}

      <div class="compose">
        <div class="msg">
          <textarea
            class="input"
            rows="3"
            placeholder={st.changes.length || st.merging ? "Commit message — what did you change?" : "No changes to commit"}
            value={git.message}
            oninput={(e) => git.setMessage(e.currentTarget.value)}
            onkeydown={messageKey}
            disabled={!st.identity}
            aria-label="Commit message"
            spellcheck="true"
          ></textarea>
          <button
            class="icon-btn suggest"
            class:busy={git.suggesting}
            title="Suggest a message from the changes"
            aria-label="Suggest a commit message"
            disabled={!st.changes.length || !!git.op}
            onclick={() => git.suggest(true)}
          ><WandSparkles size={14} /></button>
        </div>
        <div class="actions">
          {#if st.remote}
            <button class="btn primary grow" disabled={!git.canCommit} title="Commit all changes, get the latest from {st.remote}, then push (Ctrl+Enter)" onclick={() => git.commit(true)}>
              <CloudUpload size={15} />Commit & Push
            </button>
            <button class="btn small" disabled={!git.canCommit} title="Commit locally without pushing" onclick={() => git.commit(false)}>Commit</button>
          {:else}
            <button class="btn primary grow" disabled={!git.canCommit} title="Commit all changes (Ctrl+Enter)" onclick={() => git.commit(false)}>
              <GitCommit size={15} />Commit
            </button>
          {/if}
        </div>
        {#if git.op}
          <div class="op" role="status">
            <span class="spinner"></span>
            <div class="op-text">
              <span>{git.op.label} <span class="muted">{secs(now - git.op.startedAt)}</span></span>
              {#if git.op.line}<span class="op-line muted">{git.op.line}</span>{/if}
            </div>
            {#if git.op.cancelable}<button class="btn ghost small" onclick={() => git.cancel()}>Cancel</button>{/if}
          </div>
        {/if}
      </div>

      <div class="section">
        <span>Changes</span>
        {#if st.changes.length}<span class="count">{st.changes.length}</span>{/if}
      </div>
      {#if st.changes.length}
        <ul class="changes">
          {#each st.changes as c (c.path)}
            {@const b = BADGE[c.kind]}
            {@const f = split(c.path)}
            <li>
              <button class="change" title="{b.label}: {c.oldPath ? `${c.oldPath} → ` : ''}{c.path} — click to see the changes, double-click to open" onclick={() => (git.diffFor = c)} ondblclick={() => openChange(c)}>
                <span class="file" class:deleted={c.kind === "deleted"}>{f.name}</span>
                {#if f.dir}<span class="dir">{f.dir}</span>{/if}
                <span class="kind {b.cls}" aria-label={b.label}>{b.letter}</span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="none muted">No changes since the last commit.</p>
      {/if}

      {#if st.lastCommit}
        <div class="last muted" title="{st.lastCommit.hash} · {new Date(st.lastCommit.time * 1000).toLocaleString()}">
          <GitCommit size={13} /><span class="subj">{st.lastCommit.subject}</span><span class="when">{timeAgo(st.lastCommit.time)}</span>
        </div>
      {/if}
    {/if}
  </div>
</div>

{#if git.diffFor}
  <GitDiff change={git.diffFor} onClose={() => (git.diffFor = null)} />
{/if}

<style>
  .git {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
    padding: 0 6px;
    border-bottom: 1px solid var(--border);
  }
  .tools {
    display: flex;
  }
  .tools .icon-btn {
    width: 26px;
    height: 26px;
  }
  .icon-btn:disabled {
    opacity: 0.4;
    cursor: default;
    transform: none;
    background: transparent;
  }
  .spin :global(svg) {
    animation: spin 0.6s linear infinite;
  }
  .body {
    flex: 1;
    overflow: auto;
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .loading {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 12px 6px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 24px 10px;
    text-align: center;
    color: var(--text-muted);
  }
  .empty p {
    margin: 0;
    font-size: 12.5px;
  }
  .empty .title {
    color: var(--text);
    font-weight: 600;
    font-size: 13px;
  }
  .branch {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 4px;
    color: var(--text-muted);
    min-width: 0;
  }
  .branch .name {
    color: var(--text);
    font-weight: 600;
  }
  .sync {
    font-size: 11.5px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .remote {
    margin-left: auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    border: none;
    background: transparent;
    color: var(--text-faint);
    font-size: 11.5px;
    padding: 2px 4px;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .remote:hover {
    color: var(--text);
    background: var(--hover);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .card.warn {
    border-color: var(--warning);
    background: var(--warning-soft);
  }
  .card p {
    margin: 0;
  }
  .card-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .small {
    font-size: 12px;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  code {
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .conflict {
    display: flex;
    align-items: center;
    gap: 6px;
    border: none;
    background: transparent;
    color: var(--text);
    padding: 3px 4px;
    border-radius: var(--radius-sm);
    text-align: left;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .conflict:hover {
    background: var(--hover);
  }
  .compose {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .msg {
    position: relative;
  }
  textarea.input {
    width: 100%;
    height: auto;
    min-height: 62px;
    padding: 7px 30px 7px 9px;
    resize: vertical;
    font: inherit;
    line-height: 1.4;
  }
  .suggest {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 24px;
    height: 24px;
  }
  .suggest.busy :global(svg) {
    animation: pulse 0.9s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
  .actions {
    display: flex;
    gap: 6px;
  }
  .grow {
    flex: 1;
    justify-content: center;
  }
  .btn.small {
    padding: 0 9px;
    font-size: 12px;
  }
  .op {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    background: var(--accent-soft);
    animation: fade-in var(--dur) var(--ease);
  }
  .op-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    font-size: 12px;
  }
  .op-line {
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .section {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    padding: 0 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .count {
    padding: 0 5px;
    border-radius: 999px;
    background: var(--surface-2);
    letter-spacing: 0;
  }
  .changes {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .change {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 26px;
    padding: 0 6px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }
  .change:hover {
    background: var(--hover);
  }
  .change:active {
    background: var(--pressed);
  }
  .file {
    white-space: nowrap;
  }
  .file.deleted {
    text-decoration: line-through;
    color: var(--text-muted);
  }
  .dir {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-faint);
    font-size: 11.5px;
  }
  .kind {
    margin-left: auto;
    width: 16px;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
  }
  .kind.mod {
    color: var(--warning);
  }
  .kind.add {
    color: var(--success);
  }
  .kind.del {
    color: var(--danger);
  }
  .kind.ren {
    color: var(--info);
  }
  .none {
    margin: 0;
    padding: 2px 6px;
    font-size: 12px;
  }
  .last {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: auto;
    padding: 6px 4px 0;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
  }
  .subj {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .when {
    flex: none;
  }
</style>
