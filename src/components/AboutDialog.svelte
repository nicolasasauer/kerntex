<script lang="ts">
  import Modal from "./Modal.svelte";
  import Bug from "@lucide/svelte/icons/bug";
  import CodeXml from "@lucide/svelte/icons/code-xml";
  import Shield from "@lucide/svelte/icons/shield";
  import Sparkles from "@lucide/svelte/icons/sparkles";
  import { ui } from "../lib/commands.svelte";
  import { openUrl } from "../lib/api";
  import { APP_VERSION, CREDITS, LINKS, reportProblemUrl } from "../lib/about";
  import logoSvg from "../../assets/logo/logo.svg?raw";
</script>

<Modal title="About KernTeX" width={480} onClose={() => (ui.about = false)}>
  <div class="body">
    <header>
      <span class="logo" aria-hidden="true">{@html logoSvg}</span>
      <div>
        <h3>KernTeX <span class="ver">{APP_VERSION}</span></h3>
        <p class="muted">A fast, modern, local LaTeX editor. Free and open source, written in Rust.</p>
      </div>
    </header>

    <div class="actions">
      <button class="btn primary" onclick={() => openUrl(reportProblemUrl())}>
        <Bug size={15} /> Report a problem
      </button>
      <button class="btn" onclick={() => openUrl(LINKS.repo)}><CodeXml size={15} /> Source on GitHub</button>
      <button class="btn" onclick={() => openUrl(LINKS.releases)}><Sparkles size={15} /> What's new</button>
      <button class="btn" onclick={() => openUrl(LINKS.privacy)}><Shield size={15} /> Privacy</button>
    </div>
    <p class="note muted">“Report a problem” opens GitHub with your KernTeX version, system and engine already filled in — nothing is sent before you submit it.</p>

    <section>
      <h4>Built with</h4>
      <ul>
        {#each CREDITS as c}
          <li>
            <button class="link" onclick={() => openUrl(c.url)}>{c.name}</button>
            <span class="muted">{c.what} · {c.license}</span>
          </li>
        {/each}
      </ul>
    </section>

    <footer class="muted">
      © 2026 Nicolas Sauer ·
      <button class="link" onclick={() => openUrl(LINKS.license)}>MIT License</button>
    </footer>
  </div>
</Modal>

<style>
  .body {
    padding: 4px 22px 20px;
    user-select: text;
  }
  header {
    display: flex;
    gap: 16px;
    align-items: center;
  }
  .logo {
    flex: none;
    width: 72px;
    height: 72px;
  }
  .logo :global(svg) {
    width: 72px;
    height: 72px;
  }
  h3 {
    margin: 0;
    font-size: 20px;
  }
  .ver {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
    margin-left: 4px;
  }
  header p {
    margin: 4px 0 0;
    line-height: 1.45;
  }
  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-top: 18px;
  }
  .actions .btn {
    justify-content: center;
  }
  .note {
    font-size: 12px;
    margin: 8px 0 0;
    line-height: 1.45;
  }
  section {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  h4 {
    margin: 0 0 6px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px 16px;
  }
  li {
    display: flex;
    flex-direction: column;
    font-size: 13px;
  }
  li .muted {
    font-size: 12px;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    font: inherit;
    text-align: left;
  }
  .link:hover {
    text-decoration: underline;
  }
  footer {
    margin-top: 16px;
    font-size: 12px;
  }
</style>
