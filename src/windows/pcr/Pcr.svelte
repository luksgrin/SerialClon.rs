<script lang="ts">
  import { currentPrefs, prefsReady } from '$lib/prefs';
  /**
   * Run a PCR (⌘3): a matrix and two primers with optional un-annealed
   * tails; the Sequence Window's context menu can send a selection here as
   * the forward or reverse primer.
   */
  import { onMount } from 'svelte';
    import { message } from '@tauri-apps/plugin-dialog';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api, type PrimerCheck, listenHere } from '$lib/doc';

  let docs = $state<[number, string][]>([]);
  let matrix = $state<number | null>(null);
  let forward = $state('');
  let reverse = $state('');
  let ta = $state(currentPrefs().pcr_ta_overhang);
  void prefsReady.then((p) => (ta = p.pcr_ta_overhang));
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
  const isOther = (c: PrimerCheck | string | null) => c !== null && typeof c !== 'string' && c.other_strand;
  const isOk = (c: PrimerCheck | string | null) => c !== null && typeof c !== 'string' && !c.other_strand;
  /** The pair is inverted relative to the file's top strand: forward on the bottom, reverse on the top. Valid. */
  const inverted = $derived(isOther(fwdCheck) && isOther(revCheck));
  const describe = (c: PrimerCheck | string | null, which: 'forward' | 'reverse') => {
    if (c === null) return '';
    if (typeof c === 'string') return c;
    const strand = which === 'forward' ? 'bottom' : 'top';
    const partner = which === 'forward' ? 'reverse' : 'forward';
    if (!c.other_strand) return `anneals at ${c.start}–${c.end}${c.tail ? `, ${c.tail} nt tail` : ''}, Tm ${c.tm.toFixed(1)} °C`;
    if (inverted) return `anneals to the ${strand} strand at ${c.start}–${c.end}. The pair is inverted relative to this file's top strand — fine: the product is given from the forward primer.`;
    if (isOk(which === 'forward' ? revCheck : fwdCheck))
      return `as written this matches the ${strand} strand at ${c.start}–${c.end}, the same strand as the ${partner} primer: no product. Use its reverse complement, or move it if it is your ${partner} primer.`;
    return `as written this matches the ${strand} strand at ${c.start}–${c.end}. Fine if the ${partner} primer is on the ${which === 'forward' ? 'top' : 'bottom'} strand (an inverted pair); otherwise use its reverse complement.`;
  };
  const wrongBox = (c: PrimerCheck | string | null) => isOther(c) && !inverted;
  /** A primer entered in the wrong box: exchange the two boxes. */
  function swap() {
    [forward, reverse] = [reverse, forward];
    scheduleCheck();
  }
  const COMP: Record<string, string> = { A: 'T', T: 'A', U: 'A', C: 'G', G: 'C', R: 'Y', Y: 'R', S: 'S', W: 'W', K: 'M', M: 'K', B: 'V', V: 'B', D: 'H', H: 'D', N: 'N' };
  /** Reverse complement, case preserved, non-letters dropped. */
  function reverseComplement(text: string): string {
    return [...text.replace(/[^A-Za-z]/g, '')]
      .reverse()
      .map((ch) => {
        const c = COMP[ch.toUpperCase()] ?? ch.toUpperCase();
        return ch === ch.toLowerCase() ? c.toLowerCase() : c;
      })
      .join('');
  }
  function flip(which: 'forward' | 'reverse') {
    if (which === 'forward') forward = reverseComplement(forward);
    else reverse = reverseComplement(reverse);
    scheduleCheck();
  }

  onMount(() => {
    void refreshDocs();
    const t = setInterval(() => void refreshDocs(), 3000);
    let unlisten: (() => void) | undefined;
    void listenHere<{ which: 'forward' | 'reverse'; text: string }>('pcr:primer', (e) => {
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
    <span class="hint" class:error={typeof fwdCheck === 'string'} class:warn={wrongBox(fwdCheck)}>{describe(fwdCheck, 'forward')}</span>
    {#if wrongBox(fwdCheck)}
      <span class="fixes">
        <button type="button" onclick={() => flip('forward')} data-help-name="Reverse-complement" data-help="Replaces this oligo by its reverse complement, i.e. the same site written 5'→3' on the top strand.">Reverse-complement</button>
        <button type="button" onclick={swap} data-help-name="Use as reverse primer" data-help="Moves this oligo to the Reverse primer box (and the reverse primer, if any, to this one).">Use as reverse primer</button>
      </span>
    {/if}
  </fieldset>
  <fieldset data-help-name="Reverse primer" data-help="The reverse primer, 5'→3' (it anneals to the bottom strand). From a Sequence Window, right-click a selection ▸ Use selection as reverse primer (it is antiparalleled for you).">
    <legend>Reverse primer</legend>
    <textarea rows="2" class="mono wide" bind:value={reverse} oninput={scheduleCheck} spellcheck="false"></textarea>
    <span class="hint" class:error={typeof revCheck === 'string'} class:warn={wrongBox(revCheck)}>{describe(revCheck, 'reverse')}</span>
    {#if wrongBox(revCheck)}
      <span class="fixes">
        <button type="button" onclick={() => flip('reverse')} data-help-name="Reverse-complement" data-help="Replaces this oligo by its reverse complement, i.e. the same site written 5'→3' on the bottom strand — what you order as a reverse primer.">Reverse-complement</button>
        <button type="button" onclick={swap} data-help-name="Use as forward primer" data-help="Moves this oligo to the Forward primer box (and the forward primer, if any, to this one).">Use as forward primer</button>
      </span>
    {/if}
  </fieldset>
  <div class="row">
    <label data-help-name="3' A overhang" data-help="Adds the single 3' A that Taq polymerase leaves, for TA cloning."><input type="checkbox" bind:checked={ta} /> 3' A overhang (TA cloning)</label>
    <button type="button" class="primary" onclick={run} disabled={matrix === null || !forward.trim() || !reverse.trim()} data-help-name="Run PCR" data-help="Amplifies: the product opens in a new Sequence Window with the primers' Anchor and Core as Features, the matrix Features inside, and the provenance in the comments.">Run PCR</button>
  </div>
</ToolWindow>

<style>
  .warn {
    color: var(--site-double);
    font-weight: 600;
  }
  .fixes {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }
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
