<script lang="ts">
  /**
   * Run a PCR (⌘3): a matrix and two primers with optional un-annealed
   * tails; the Sequence Window's context menu can send a selection here as
   * the forward or reverse primer.
   */
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { message } from '@tauri-apps/plugin-dialog';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api, type PrimerCheck } from '$lib/doc';

  let docs = $state<[number, string][]>([]);
  let matrix = $state<number | null>(null);
  let forward = $state('');
  let reverse = $state('');
  let ta = $state(false);
  let fwdCheck = $state<PrimerCheck | string | null>(null);
  let revCheck = $state<PrimerCheck | string | null>(null);

  async function refreshDocs() {
    docs = await api.listDocuments();
    if (matrix === null && docs[0]) matrix = docs[0][0];
  }
  let timer: ReturnType<typeof setTimeout> | undefined;
  function scheduleCheck() {
    clearTimeout(timer);
    timer = setTimeout(check, 200);
  }
  async function check() {
    if (matrix === null) return;
    fwdCheck = forward.trim() ? await api.checkPrimer(matrix, forward, false).catch((e) => String(e)) : null;
    revCheck = reverse.trim() ? await api.checkPrimer(matrix, reverse, true).catch((e) => String(e)) : null;
  }
  async function run() {
    if (matrix === null) return;
    try {
      await api.runPcr(matrix, forward, reverse, ta);
    } catch (e) {
      await message(String(e), { title: 'Run a PCR', kind: 'error' });
    }
  }
  const describe = (c: PrimerCheck | string | null) =>
    c === null ? '' : typeof c === 'string' ? c : `anneals at ${c.start}–${c.end}${c.tail ? `, ${c.tail} nt tail` : ''}, Tm ${c.tm.toFixed(1)} °C`;

  onMount(() => {
    void refreshDocs();
    const t = setInterval(() => void refreshDocs(), 3000);
    let unlisten: (() => void) | undefined;
    void listen<{ which: 'forward' | 'reverse'; text: string }>('pcr:primer', (e) => {
      if (e.payload.which === 'forward') forward = e.payload.text;
      else reverse = e.payload.text;
      scheduleCheck();
    }).then((u) => (unlisten = u));
    return () => {
      clearInterval(t);
      unlisten?.();
    };
  });
</script>

<ToolWindow title="Run a PCR" subtitle="primers 5'→3'; the 3' end must anneal (≥ 10 nt), the rest is a tail">
  <fieldset data-help-name="Matrix" data-help="The template: one of the open sequences. Open a file first if it is not listed.">
    <legend>Matrix</legend>
    <select bind:value={matrix} onchange={scheduleCheck}>
      {#each docs as [id, n] (id)}<option value={id}>{n}</option>{/each}
    </select>
    <button type="button" onclick={async () => { await api.openFiles(); await refreshDocs(); }}>Open matrix…</button>
  </fieldset>
  <fieldset data-help-name="Forward primer" data-help="The forward primer, 5'→3'. Its 3' end must match the top strand; a 5' portion that does not anneal becomes an un-annealed tail added to the product. From a Sequence Window, right-click a selection ▸ Use selection as forward primer.">
    <legend>Forward primer</legend>
    <textarea rows="2" class="mono wide" bind:value={forward} oninput={scheduleCheck} spellcheck="false"></textarea>
    <span class="hint" class:error={typeof fwdCheck === 'string'}>{describe(fwdCheck)}</span>
  </fieldset>
  <fieldset data-help-name="Reverse primer" data-help="The reverse primer, 5'→3' (it anneals to the bottom strand). From a Sequence Window, right-click a selection ▸ Use selection as reverse primer (it is antiparalleled for you).">
    <legend>Reverse primer</legend>
    <textarea rows="2" class="mono wide" bind:value={reverse} oninput={scheduleCheck} spellcheck="false"></textarea>
    <span class="hint" class:error={typeof revCheck === 'string'}>{describe(revCheck)}</span>
  </fieldset>
  <div class="row">
    <label data-help-name="3' A overhang" data-help="Adds the single 3' A that Taq polymerase leaves, for TA cloning."><input type="checkbox" bind:checked={ta} /> 3' A overhang (TA cloning)</label>
    <button type="button" class="primary" onclick={run} disabled={matrix === null || !forward.trim() || !reverse.trim()} data-help-name="Run PCR" data-help="Amplifies: the product opens in a new Sequence Window with the primers' Anchor and Core as Features, the matrix Features inside, and the provenance in the comments.">Run PCR</button>
  </div>
</ToolWindow>

<style>
  .wide {
    width: 100%;
    box-sizing: border-box;
  }
  .row {
    display: flex;
    gap: 12px;
    align-items: center;
  }
</style>
