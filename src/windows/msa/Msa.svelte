<script lang="ts">
  import { printPage } from '$lib/prefs';
  /**
   * Align Multiple Sequences (⌘8, M6b): pick open sequences, align them with
   * the pure-Rust MAFFT port, see the alignment with conservation colours,
   * each sequence's Features above its row, coordinates on click, then copy
   * it (Clustal), extract the consensus, or save it as FASTA / Clustal.
   */
  import { onMount } from 'svelte';
  import { message } from '@tauri-apps/plugin-dialog';
  import { api, requestSelect, type FeatureView, type MsaCandidate, type MsaMode, type MultipleAlignmentView } from '$lib/doc';

  const MODES: { value: MsaMode; label: string; help: string }[] = [
    { value: 'auto', label: 'Auto', help: "MAFFT's --auto: L-INS-i for up to 200 sequences of at most 2 kb, FFT-NS-2 otherwise." },
    { value: 'fft-ns-2', label: 'FFT-NS-2', help: 'Fast progressive method with two guide-tree rounds. Good for many or long sequences.' },
    { value: 'fft-nsi', label: 'FFT-NS-i', help: 'FFT-NS-2 followed by iterative refinement.' },
    { value: 'l-insi', label: 'L-INS-i', help: 'Local pairwise information and iterative refinement: the most accurate for a few hundred sequences with one alignable domain.' },
    { value: 'g-insi', label: 'G-INS-i', help: 'Global pairwise information and iterative refinement, for sequences alignable over their whole length.' },
    { value: 'e-insi', label: 'E-INS-i', help: 'Generalised affine gap costs, for sequences with long unalignable regions between conserved blocks.' },
  ];

  let candidates = $state<MsaCandidate[]>([]);
  let selected = $state<number[]>([]);
  let mode = $state<MsaMode>('auto');
  let adjustDirection = $state(true);
  let keepCase = $state(false);
  let busy = $state(false);
  let view = $state<MultipleAlignmentView | null>(null);
  let highlight = $state(false);
  let clickedCol = $state<number | null>(null);
  let chWidth = $state(7.2);
  let probe = $state<HTMLSpanElement | undefined>();

  const chosen = $derived(candidates.filter((c) => selected.includes(c.id)));
  const kinds = $derived(new Set(chosen.map((c) => c.protein ? 'Protein' : 'DNA')));
  const mixed = $derived(kinds.size > 1);
  const kindLabel = $derived(mixed ? 'DNA and protein — choose one kind' : kinds.size ? [...kinds][0] : '');

  async function refresh() {
    const list = await api.msaCandidates();
    const known = new Set(list.map((c) => c.id));
    const first = candidates.length === 0;
    candidates = list;
    selected = first ? list.map((c) => c.id) : selected.filter((id) => known.has(id));
  }

  function toggle(id: number, on: boolean) {
    selected = on ? [...new Set([...selected, id])] : selected.filter((x) => x !== id);
  }

  async function run() {
    if (selected.length < 2 || mixed) return;
    busy = true;
    clickedCol = null;
    try {
      // Keep the picker's order (document id order).
      const ids = candidates.filter((c) => selected.includes(c.id)).map((c) => c.id);
      view = await api.alignMultiple(ids, { mode, adjust_direction: adjustDirection, keep_case: keepCase });
    } catch (e) {
      view = null;
      await message(String(e), { title: 'Align Multiple Sequences', kind: 'error' });
    } finally {
      busy = false;
    }
  }

  /** Runs of consecutive columns sharing a class, for cheap rendering. */
  interface Run {
    text: string;
    cls: string;
  }
  function classOf(rowIndex: number, col: number): string {
    if (!view) return '';
    const mark = view.msa.marks[col] ?? ' ';
    const ch = view.msa.rows[rowIndex]!.aligned[col] ?? '-';
    if (highlight) {
      if (ch === '-') return 'gap';
      return mark === '*' ? '' : 'diff';
    }
    return mark === '*' ? 'same' : mark === ':' ? 'similar' : ch === '-' ? 'gap' : '';
  }
  const runsPerRow = $derived.by((): Run[][] => {
    if (!view) return [];
    void highlight;
    return view.msa.rows.map((r, ri) => {
      const runs: Run[] = [];
      let start = 0;
      let cls = classOf(ri, 0);
      for (let c = 1; c <= r.aligned.length; c++) {
        const k = c < r.aligned.length ? classOf(ri, c) : null;
        if (k !== cls) {
          runs.push({ text: r.aligned.slice(start, c), cls });
          start = c;
          cls = k ?? '';
        }
      }
      return runs;
    });
  });

  /** Alignment column (0-based) of each residue index (1-based) per row. */
  const colOfPos = $derived.by((): number[][] => {
    if (!view) return [];
    return view.msa.rows.map((r) => {
      const map: number[] = [0];
      for (let c = 0; c < r.aligned.length; c++) if (r.aligned[c] !== '-') map.push(c);
      return map;
    });
  });

  interface Bar {
    left: number;
    width: number;
    color: string;
    name: string;
  }
  const barsPerRow = $derived.by((): Bar[][] => {
    if (!view) return [];
    return view.msa.rows.map((r, ri) => {
      const map = colOfPos[ri] ?? [];
      const n = r.length;
      return (view!.rows[ri]?.features ?? [])
        .filter((f) => f.show)
        .map((f) => {
          let lo = Math.min(f.start, f.end);
          let hi = Math.max(f.start, f.end);
          if (r.reversed) [lo, hi] = [n - hi + 1, n - lo + 1];
          lo = Math.max(1, Math.min(n, lo));
          hi = Math.max(1, Math.min(n, hi));
          const c0 = map[lo] ?? 0;
          const c1 = map[hi] ?? c0;
          return { left: c0, width: Math.max(1, c1 - c0 + 1), color: f.color, name: f.name };
        });
    });
  });

  const ruler = $derived.by(() => {
    if (!view) return '';
    let s = '';
    for (let c = 0; c < view.msa.length; c += 10) {
      const label = String(c + 10);
      s += label.padStart(10, ' ');
    }
    return s.slice(0, view.msa.length);
  });

  function onResiduesClick(e: MouseEvent) {
    const el = e.currentTarget as HTMLElement;
    const x = e.clientX - el.getBoundingClientRect().left;
    const col = Math.floor(x / chWidth);
    if (view && col >= 0 && col < view.msa.length) clickedCol = col;
  }

  /** Coordinate (1-based, original orientation) of a row at a column, or null on a gap. */
  function coordinateAt(ri: number, col: number): number | null {
    if (!view) return null;
    const r = view.msa.rows[ri]!;
    if ((r.aligned[col] ?? '-') === '-') return null;
    let n = 0;
    for (let c = 0; c <= col; c++) if (r.aligned[c] !== '-') n++;
    return r.reversed ? r.length - n + 1 : n;
  }
  function featuresAt(ri: number, pos: number): FeatureView[] {
    return (view?.rows[ri]?.features ?? []).filter((f) => pos >= Math.min(f.start, f.end) && pos <= Math.max(f.start, f.end));
  }

  async function copyClustal() {
    if (!view) return;
    await api.clipboardWrite(await api.msaText(view.msa, true));
  }
  async function extract() {
    if (!view) return;
    await api.extractMsaConsensus(view.msa);
  }
  async function save(clustal: boolean) {
    if (!view) return;
    await api.exportMsa(view.msa, clustal);
  }

  onMount(() => {
    void refresh();
    const t = setInterval(() => void refresh(), 3000);
    if (probe) chWidth = probe.getBoundingClientRect().width / 10;
    return () => clearInterval(t);
  });
</script>

<div class="msa">
  <div class="setup no-print">
    <div class="picker" data-help-name="Sequences" data-help="Tick the open sequences to align (at least two, all DNA or all protein). Newly opened windows appear here automatically.">
      <div class="picker-head">Sequences <span class="hint">{chosen.length} selected{kindLabel ? ' · ' + kindLabel : ''}</span></div>
      <div class="picker-list">
        {#each candidates as c (c.id)}
          <label><input type="checkbox" checked={selected.includes(c.id)} onchange={(e) => toggle(c.id, e.currentTarget.checked)} /> {c.name} <span class="hint">{c.seq_type} · {c.length}</span></label>
        {:else}
          <p class="hint">No open sequences.</p>
        {/each}
      </div>
    </div>
    <div class="options">
      <label data-help-name="Strategy" data-help={MODES.find((m) => m.value === mode)?.help ?? ''}>
        Strategy
        <select bind:value={mode}>{#each MODES as m (m.value)}<option value={m.value}>{m.label}</option>{/each}</select>
      </label>
      <label data-help-name="Adjust direction" data-help="Reverse-complements sequences that are on the other strand before aligning (MAFFT --adjustdirection). Flipped sequences are marked in the alignment."><input type="checkbox" bind:checked={adjustDirection} /> Adjust direction</label>
      <label data-help-name="Keep case" data-help="Keeps the letter case of each sequence in the alignment (MAFFT otherwise writes DNA in lower case and protein in upper case)."><input type="checkbox" bind:checked={keepCase} /> Keep case</label>
      <button type="button" class="primary" onclick={run} disabled={busy || selected.length < 2 || mixed} data-help-name="Align" data-help="Runs MAFFT in the background. All the science is in-process: nothing leaves your computer.">{busy ? 'Aligning…' : 'Align'}</button>
      {#if busy}<span class="spinner" aria-label="aligning"></span>{/if}
    </div>
  </div>

  {#if view}
    {@const m = view.msa}
    <div class="summary" data-help-name="Summary" data-help="The strategy MAFFT used (Auto resolved), the number of columns, the score, the time it took, and the sequences that were reverse-complemented.">
      <b>{MODES.find((x) => x.value === m.mode)?.label ?? m.mode}</b> · {m.rows.length} sequences · {m.length} columns · score {m.score.toFixed(1)} · {view.elapsed_ms} ms
      {#if m.rows.some((r) => r.reversed)}· reverse-complemented: {m.rows.filter((r) => r.reversed).map((r) => r.name).join(', ')}{/if}
    </div>
    <div class="tools no-print">
      <label data-help-name="Highlight differences" data-help="Colours residues that differ from the column's consensus instead of the conservation colouring."><input type="checkbox" bind:checked={highlight} /> Highlight differences</label>
      <button type="button" onclick={copyClustal} data-help-name="Copy alignment" data-help="Copies the alignment in Clustal format.">Copy alignment</button>
      <button type="button" onclick={extract} data-help-name="Extract consensus" data-help="Opens the majority consensus (ties as IUPAC codes) in a new Sequence Window.">Extract consensus</button>
      <button type="button" onclick={() => void save(false)} data-help-name="Save as FASTA" data-help="Saves the aligned sequences as FASTA (gaps as -).">Save as FASTA…</button>
      <button type="button" onclick={() => void save(true)} data-help-name="Save as Clustal" data-help="Saves the alignment in Clustal format.">Save as Clustal…</button>
      <button type="button" onclick={() => void printPage()}>Print…</button>
    </div>
    <div class="alignment" data-help-name="Alignment" data-help="Green columns are identical, yellow similar. Coloured bars above a row are that sequence's Features. Click a column to see the coordinate of every sequence there.">
      <div class="line ruler"><span class="gutter"></span><span class="residues">{ruler}</span></div>
      {#each m.rows as r, ri (ri)}
        <div class="track"><span class="gutter"></span><span class="bars" style:width="{m.length}ch">
          {#each barsPerRow[ri] ?? [] as b, bi (bi)}
            <span class="bar" style:left="{b.left}ch" style:width="{b.width}ch" style:background={b.color} title={b.name}></span>
          {/each}
        </span></div>
        <div class="line">
          <span class="gutter" title={r.name}>{r.reversed ? '⟲ ' : ''}{r.name.slice(0, 16)}</span>
          <span class="residues" role="button" tabindex="-1" onclick={onResiduesClick} onkeydown={() => {}}>{#each runsPerRow[ri] ?? [] as run, k (k)}<span class={run.cls}>{run.text}</span>{/each}</span>
        </div>
      {/each}
      <div class="line marks"><span class="gutter"></span><span class="residues">{m.marks}</span></div>
      <span class="probe" bind:this={probe} aria-hidden="true">0000000000</span>
    </div>
    {#if clickedCol !== null}
      <div class="clicked">
        <b>Column {clickedCol + 1}</b>
        {#each m.rows as r, ri (ri)}
          {@const pos = coordinateAt(ri, clickedCol)}
          <span>{r.name}: {pos === null ? 'gap' : pos}{pos !== null && featuresAt(ri, pos).length ? ' — ' + featuresAt(ri, pos).map((f) => f.name).join(', ') : ''}
            {#if pos !== null}<button type="button" class="tiny" onclick={() => void requestSelect(view!.rows[ri]!.id, pos - 1, pos)}>select</button>{/if}
          </span>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .msa {
    display: flex;
    flex-direction: column;
    height: 100vh;
    box-sizing: border-box;
    padding: 8px;
    gap: 6px;
  }
  .setup {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 10px;
    align-items: start;
  }
  .picker {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
    max-height: 160px;
    display: flex;
    flex-direction: column;
  }
  .picker-head {
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    font-weight: 600;
  }
  .picker-list {
    overflow: auto;
    padding: 4px 8px;
  }
  .picker-list label {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .options {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 200px;
  }
  .options label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  select {
    font: inherit;
  }
  .primary {
    background: var(--accent);
    color: #fff;
    padding: 4px 16px;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .summary {
    padding: 4px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
  }
  .tools {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .tools label {
    display: flex;
    align-items: center;
    gap: 4px;
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
    position: relative;
  }
  .line,
  .track {
    white-space: pre;
    line-height: 1.25;
    display: flex;
  }
  .track {
    height: 7px;
  }
  .gutter {
    position: sticky;
    left: 0;
    z-index: 1;
    display: inline-block;
    width: 16ch;
    min-width: 16ch;
    padding-right: 1ch;
    background: #fff;
    color: #555;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .residues {
    display: inline-block;
    cursor: default;
  }
  .bars {
    position: relative;
    display: inline-block;
    height: 7px;
  }
  .bar {
    position: absolute;
    top: 1px;
    height: 5px;
    border-radius: 2px;
    opacity: 0.85;
  }
  .ruler,
  .marks {
    color: #888;
  }
  .same {
    background: #dcf5dc;
  }
  .similar {
    background: #fff3c4;
  }
  .diff {
    background: #ffcc80;
  }
  .gap {
    color: #aaa;
  }
  .probe {
    position: absolute;
    visibility: hidden;
    white-space: pre;
  }
  .clicked {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
    padding: 4px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
  }
  .tiny {
    padding: 0 6px;
    font-size: var(--fs-small);
  }
  .hint {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  @media print {
    .no-print {
      display: none !important;
    }
    .msa {
      height: auto;
    }
    .alignment {
      border: none;
      overflow: visible;
      font-size: 8px;
    }
  }
</style>
