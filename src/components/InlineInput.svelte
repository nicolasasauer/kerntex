<script lang="ts">
  import { onMount } from "svelte";
  let { value, placeholder = "", selectStem = false, cursorStart = false, onSubmit, onCancel }: {
    value: string;
    placeholder?: string;
    selectStem?: boolean;
    cursorStart?: boolean;
    onSubmit: (v: string) => void;
    onCancel: () => void;
  } = $props();
  let el: HTMLInputElement | undefined = $state();
  // svelte-ignore state_referenced_locally
  let v = $state(value); // initial value only; the input owns it afterwards
  let done = false;

  onMount(() => {
    el!.focus();
    if (cursorStart) el!.setSelectionRange(0, 0);
    else if (selectStem && v.includes(".")) el!.setSelectionRange(0, v.lastIndexOf("."));
    else el!.select();
  });

  function finish(ok: boolean) {
    if (done) return;
    done = true;
    const t = v.trim();
    if (ok && t && t !== ".tex") onSubmit(t);
    else onCancel();
  }
</script>

<input
  class="input inline"
  bind:this={el}
  bind:value={v}
  {placeholder}
  spellcheck="false"
  onkeydown={(e) => {
    if (e.key === "Enter") finish(true);
    else if (e.key === "Escape") finish(false);
    e.stopPropagation();
  }}
  onblur={() => finish(true)}
/>

<style>
  .inline {
    flex: 1;
    height: 24px;
    min-width: 0;
    padding: 0 6px;
  }
</style>
