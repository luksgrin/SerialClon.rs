<script lang="ts">
  /**
   * Align two Sequences (⌘7; User Manual §III.E, changelog 1.2–2.6): pick two
   * open sequences, align them locally (Smith-Waterman) or at the NCBI
   * (BLAST2Seq), see the alignment with match marks, % identity, optional
   * 3-frame translation, the features of both, highlight differences, extract
   * the consensus, switch DNA ↔ protein.
   */
  import { onMount } from 'svelte';
  import { message } from '@tauri-apps/plugin-dialog';
  import { api, requestSelect, type AlignView, type FeatureView } from '$lib/doc';

  let docs = $state<[number, string][]>([]);
  let aId = $state<number | null>(null);
  let bId = $state<number | null>(null);
  let protein = $state(false);
  let short = $state(false);
  let highlight = $state(false);
  let showTranslation = $state(false);
  let view = $state<AlignView | null>(null);
  let frames = $state<[string, string, string] | null>(null);
  let busy = $state(false);
  let remote = $state<{ rid: string; status: string; text: string; wait: number } | null>(null);
  let clicked = $state<{ a: number | null; b: number | null; aFeatures: FeatureView[]; bFeatures: FeatureView[] } | null>(null);
  const WIDTH = 60;

  async function refreshDocs() {
    docs = await api.listDocuments();
    if (aId === null && docs[0]) aId = docs[0][0];
    if (bId === null && docs[1]) bId = docs[1][0];
  }

  async function run() {
    if (aId === null || bId === null) return;
    busy = true;
    remote = null;
    clicked = null;
    try {
      view = await api.alignDocuments(aId, bId, protein, short);
      frames = showTranslation && view.alignment.kind === 'Dna' ? await api.translateFrames(view.alignment.a_line.replace(/-/g, '')) : null;
    } catch (e) {
      view = null;
      await message(String(e), { title: 'Align', kind: 'error' });
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    void showTranslation;
    if (view && showTranslation && view.alignment.kind === 'Dna' && !frames) {
      const v = view;
      void api.translateFrames(v.alignment.a_line.replace(/-/g, '')).then((f) => (frames = f));
    }
  });

  /** Sequence coordinate (1-based) at an alignment column, or null on a gap. */
  function coordAt(line: string, startPos: number, col: number, reverse: boolean, len: number): number | null {
    if (line[col] === '-') return null;
    let n = 0;
    for (let i = 0; i <= col; i++) if (line[i] !== '-') n++;
    return reverse ? len - (startPos + n - 1) : startPos + n;
  }

  function featuresAt(list: FeatureView[], pos: number): FeatureView[] {
    return list.filter((f) => {
      const lo = Math.min(f.start, f.end);
      const hi = Math.max(f.start, f.end);
      return pos >= lo && pos <= hi;
    });
  }

  function clickColumn(col: number) {
    if (!view) return;
    const a = coordAt(view.alignment.a_line, view.alignment.a_start, col, false, view.a_len);
    const bRaw = coordAt(view.alignment.b_line, view.alignment.b_start, col, false, view.b_len);
    const b = bRaw === null ? null : view.alignment.b_reverse ? view.b_len - bRaw + 1 + view.alignment.b_start - view.alignment.b_start : bRaw;
    const bPos = view.alignment.b_reverse && bRaw !== null ? view.alignment.b_end - (bRaw - view.alignment.b_start) : b;
    clicked = { a, b: bPos, aFeatures: a ? featuresAt(view.a_features, a) : [], bFeatures: bPos ? featuresAt(view.b_features, bPos) : [] };
  }

  function selectIn(which: 'a' | 'b') {
    if (!view) return;
    const id = which === 'a' ? view.a_id : view.b_id;
    const from = which === 'a' ? view.alignment.a_start : view.alignment.b_start;
    const to = which === 'a' ? view.alignment.a_end : view.alignment.b_end;
    const factor = protein && view.alignment.kind === 'Protein' ? 3 : 1;
    void requestSelect(id, from * factor, to * factor);
  }

  async function extractConsensus() {
    if (!view) return;
    await api.alignConsensus(view.alignment, view.a_name, view.b_name);
  }

  // ---- remote (BLAST2Seq) ----
  let pollTimer: ReturnType<typeof setTimeout> | undefined;
  async function runRemote() {
    if (aId === null || bId === null) return;
    busy = true;
    try {
      const job = await api.blastSubmit(aId, 0, 0, protein ? 'blastp' : 'blastn', null, bId);
      remote = { rid: job.rid, status: 'submitted', text: '', wait: Math.max(5, job.rtoe) };
      schedulePoll(Math.max(5, job.rtoe));
    } catch (e) {
      await message(String(e), { title: 'BLAST2Seq', kind: 'error' });
    } finally {
      busy = false;
    }
  }
  function schedulePoll(seconds: number) {
    clearTimeout(pollTimer);
    pollTimer = setTimeout(poll, seconds * 1000);
  }
  async function poll() {
    if (!remote) return;
    try {
      const s = await api.blastStatus(remote.rid);
      remote = { ...remote, status: s.status, wait: s.wait };
      if (s.status === 'ready') remote = { ...remote, text: await api.blastResult(remote.rid) };
      else if (s.status === 'waiting') schedulePoll(Math.max(s.wait, 15));
    } catch (e) {
      remote = { ...remote, status: `error: ${e}` };
    }
  }

  onMount(() => {
    void refreshDocs();
    const t = setInterval(() => void refreshDocs(), 3000);
    return () => {
      clearInterval(t);
      clearTimeout(pollTimer);
    };
  });

  const rows = $derived.by(() => {
    if (!view) return [] as { start: number; a: string; m: string; b: string; f?: [string, string, string] }[];
    const out = [];
    const al = view.alignment;
    for (let i = 0; i < al.length; i += WIDTH) {
      out.push({
        start: i,
        a: al.a_line.slice(i, i + WIDTH),
        m: al.midline.slice(i, i + WIDTH),
        b: al.b_line.slice(i, i + WIDTH),
      });
    }
    return out;
  });
</script>

<div class="align">
  <div class="controls">
    <label data-help-name="Sequence 1" data-help="The first of the two open sequences to align. The front-most windows are proposed by default.">
      Sequence 1
      <select bind:value={aId}>{#each docs as [id, name] (id)}<option value={id}>{name}</option>{/each}</select>
    </label>
    <label data-help-name="Sequence 2" data-help="The second sequence. DNA is also tried as its reverse complement; the better alignment is shown.">
      Sequence 2
      <select bind:value={bId}>{#each docs as [id, name] (id)}<option value={id}>{name}</option>{/each}</select>
    </label>
    <label data-help-name="Protein" data-help="Rapid switch between a DNA and a Protein alignment: DNA sequences are translated (frame 1, chosen genetic code) and aligned with BLOSUM62."><input type="checkbox" bind:checked={protein} /> Protein</label>
    <label data-help-name="Align short sequences" data-help="For short sequences: the smaller one becomes Sequence 1."><input type="checkbox" bind:checked={short} /> Align short sequences</label>
    <button type="button" onclick={run} disabled={busy || aId === null || bId === null} data-help-name="Local alignment" data-help="Aligns the two sequences locally (Smith-Waterman with affine gaps) — no network needed.">Local alignment</button>
    <button type="button" onclick={runRemote} disabled={busy || aId === null || bId === null} data-help-name="NCBI BLAST2Seq" data-help="Submits the two sequences to the NCBI BLAST server (network). Results are polled at the rate the NCBI asks for.">NCBI BLAST2Seq</button>
  </div>

  {#if view}
    {@const al = view.alignment}
    <div class="summary" data-help-name="Alignment summary" data-help="Score, aligned region of each sequence, strand, and the percentage of identical positions over the aligned columns.">
      <b>{view.a_name}</b> {al.a_start + 1}–{al.a_end} vs <b>{view.b_name}</b> {al.b_start + 1}–{al.b_end}{al.b_reverse ? ' (reverse strand)' : ''}
      · score {al.score} · <b>{al.identity.toFixed(1)}%</b> identity ({al.matches}/{al.length}){view.swapped ? ' · sequences swapped' : ''}
    </div>
    <div class="options">
      <label data-help-name="Highlight differences" data-help="Toggles the highlighting of mismatches and gaps in the aligned sequences."><input type="checkbox" bind:checked={highlight} /> Highlight differences</label>
      {#if al.kind === 'Dna'}<label data-help-name="3-frame translation" data-help="Shows the three-frame translation of the aligned region of Sequence 1 under the alignment."><input type="checkbox" bind:checked={showTranslation} /> 3-frame translation</label>{/if}
      <button type="button" onclick={extractConsensus} data-help-name="Extract consensus" data-help="Opens the consensus of the aligned region (mismatches as IUPAC codes) in a new Sequence Window.">Extract consensus</button>
      <button type="button" onclick={() => selectIn('a')}>Select in {view.a_name}</button>
      <button type="button" onclick={() => selectIn('b')}>Select in {view.b_name}</button>
    </div>
    <div class="alignment" role="presentation" data-help-name="Alignment" data-help="Click a column to see the coordinates and the Features of both sequences at that position.">
      {#each rows as r (r.start)}
        <div class="block">
          <div class="line"><span class="label">{view.a_name.slice(0, 12)}</span>{#each r.a.split('') as ch, i (i)}<button type="button" class="col" class:diff={highlight && r.m[i] !== '|'} onclick={() => clickColumn(r.start + i)}>{ch}</button>{/each}</div>
          <div class="line"><span class="label"></span>{#each r.m.split('') as ch, i (i)}<span class="col mid">{ch}</span>{/each}</div>
          <div class="line"><span class="label">{view.b_name.slice(0, 12)}</span>{#each r.b.split('') as ch, i (i)}<button type="button" class="col" class:diff={highlight && r.m[i] !== '|'} onclick={() => clickColumn(r.start + i)}>{ch}</button>{/each}</div>
          {#if showTranslation && frames}
            {#each frames as f, k (k)}
              <div class="line frame"><span class="label">frame {k + 1}</span>{f.slice(Math.floor((r.start - k) / 3), Math.floor((r.start - k) / 3) + WIDTH / 3).split('').join('  ')}</div>
            {/each}
          {/if}
        </div>
      {/each}
    </div>
    {#if clicked}
      <div class="clicked">
        <span>{view.a_name}: {clicked.a ?? 'gap'}{clicked.aFeatures.length ? ' — ' + clicked.aFeatures.map((f) => f.name).join(', ') : ''}</span>
        <span>{view.b_name}: {clicked.b ?? 'gap'}{clicked.bFeatures.length ? ' — ' + clicked.bFeatures.map((f) => f.name).join(', ') : ''}</span>
      </div>
    {/if}
  {/if}

  {#if remote}
    <div class="remote">
      <div class="hint">BLAST2Seq request {remote.rid}: {remote.status}{remote.status === 'waiting' ? ` — next check in ${Math.max(remote.wait, 15)} s` : ''}</div>
      {#if remote.text}<pre>{remote.text}</pre>{/if}
    </div>
  {/if}
</div>

<style>
  .align {
    display: flex;
    flex-direction: column;
    height: 100vh;
    box-sizing: border-box;
    padding: 8px;
    gap: 6px;
  }
  .controls,
  .options {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .controls label,
  .options label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  select {
    font: inherit;
    max-width: 220px;
  }
  .summary {
    padding: 4px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
  }
  .alignment {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: #fff;
    color: #111;
    padding: 6px;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 12px;
  }
  .block {
    margin-bottom: 10px;
  }
  .line {
    white-space: pre;
    line-height: 1.2;
  }
  .label {
    display: inline-block;
    width: 110px;
    color: #666;
  }
  .col {
    display: inline-block;
    width: 0.62em;
    padding: 0;
    margin: 0;
    border: none;
    background: transparent;
    font: inherit;
    color: inherit;
    cursor: default;
    text-align: center;
  }
  .col.diff {
    background: #ffd54f;
  }
  .mid {
    color: #888;
  }
  .frame {
    color: #2f6fbf;
  }
  .clicked {
    display: flex;
    gap: 20px;
    padding: 4px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
  }
  .remote pre {
    max-height: 40vh;
    overflow: auto;
    background: #fff;
    color: #111;
    padding: 6px;
    font-size: 11px;
  }
  .hint {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
</style>
