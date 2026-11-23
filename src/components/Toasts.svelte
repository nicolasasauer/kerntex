<script lang="ts">
  import X from "@lucide/svelte/icons/x";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import Info from "@lucide/svelte/icons/info";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { toasts, dismiss } from "../lib/toasts.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each toasts as t (t.id)}
    <div class="toast {t.kind}" role={t.kind === "error" ? "alert" : "status"}>
      <span class="icon">
        {#if t.kind === "success"}<CircleCheck size={17} />{:else if t.kind === "error"}<CircleX size={17} />{:else if t.kind === "warning"}<TriangleAlert size={17} />{:else}<Info size={17} />{/if}
      </span>
      <div class="content">
        <div class="top"><span class="source">{t.source}</span><b>{t.title}</b></div>
        {#if t.body}<p>{t.body}</p>{/if}
        {#if t.action}
          {@const action = t.action}
          <button class="btn" onclick={() => { action.run(); dismiss(t.id); }}>{action.label}</button>
        {/if}
      </div>
      <button class="icon-btn" aria-label="Dismiss" onclick={() => dismiss(t.id)}><X size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 80;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 380px;
    max-width: calc(100vw - 32px);
    pointer-events: none;
  }
  .toast {
    display: flex;
    gap: 10px;
    padding: 10px 8px 10px 12px;
    background: var(--surface-raised);
    border: 1px solid var(--border);
    border-left: 3px solid var(--info);
    border-radius: 10px;
    box-shadow: var(--shadow-lg);
    pointer-events: auto;
    animation: pop-in 200ms var(--ease);
    user-select: text;
  }
  .success {
    border-left-color: var(--success);
  }
  .error {
    border-left-color: var(--danger);
  }
  .warning {
    border-left-color: var(--warning);
  }
  .icon {
    display: grid;
    padding-top: 1px;
  }
  .success .icon {
    color: var(--success);
  }
  .error .icon {
    color: var(--danger);
  }
  .warning .icon {
    color: var(--warning);
  }
  .info .icon {
    color: var(--info);
  }
  .content {
    flex: 1;
    min-width: 0;
  }
  .top {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }
  .source {
    font-size: 11px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }
  p {
    margin: 4px 0 0;
    color: var(--text-muted);
    white-space: pre-line;
    word-break: break-word;
    max-height: 160px;
    overflow: auto;
    line-height: 1.45;
  }
  .btn {
    margin-top: 8px;
    height: 26px;
  }
</style>
