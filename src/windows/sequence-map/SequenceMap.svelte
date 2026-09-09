<script lang="ts">
  /**
   * The Sequence Map (User Manual §IV.D; reference/ui/08): the text
   * restriction map with multi-frame translation, driven by four control
   * panels — search, Translation, Restriction sites, Display — plus Features.
   */
  import { onMount } from 'svelte';
  import { api, docIdFromUrl, onDocChanged, onMenu, type DocumentView, type EnzymeEntry, type TextMapRequest } from '$lib/doc';
  import { currentPrefs, prefs, prefsReady, printPage } from '$lib/prefs';

  const id = docIdFromUrl();
  const params = new URLSearchParams(window.location.search);
  const initialFrom = params.get('from');
  const initialTo = params.get('to');

  let doc = $state<DocumentView | null>(null);
  let enzymes = $state<EnzymeEntry[]>([]);
  let text = $state('');
  let query = $state('');
  let enzymeQuery = $state('');
  let matchIndex = $state(-1);
  let translateAll = $state(!initialFrom);
  let from = $state(initialFrom ? Number(initialFrom) : 1);
  let to = $state(initialTo ? Number(initialTo) : 0);
  let frames = $state<[boolean, boolean, boolean]>([true, true, true]);
  let codons = $state(false);
  let uppercaseOnly = $state(false);
  let siteMode = $state<'none' | 'all' | 'max' | 'particular'>('max');
  let maxCuts = $state(1);
  let particular = $state<string[]>([]);
  let chooser = $state(false);
  let showReverse = $state(currentPrefs().smap_show_reverse);
  let showCoords = $state(true);
  let charSize = $state(currentPrefs().smap_char_size);
  let perLine = $state(currentPrefs().smap_per_line);
  let showFeatures = $state(true);
  let pre = $state<HTMLPreElement | undefined>();

  const singleFrame = $derived(frames.filter(Boolean).length === 1);

  let timer: ReturnType<typeof setTimeout> | undefined;
  function schedule() {
    clearTimeout(timer);
    timer = setTimeout(() => void refresh(), 60);
  }
  async function refresh() {
    if (!doc) return;
    const request: TextMapRequest = {
      per_line: Math.max(10, perLine),
      show_reverse: showReverse,
      show_coords: showCoords,
      translate: translateAll ? 'all' : 'range',
      from: translateAll ? null : from,
      to: translateAll ? null : to || doc.bases.length,
      frames,
      codons: codons && singleFrame,
      uppercase_only: uppercaseOnly,
      sites: siteMode,
      max_cuts: maxCuts,
      particular,
      features: showFeatures,
      aa_middle: $prefs.smap_aa_middle,
    };
    try {
      text = await api.sequenceMap(id, request);
    } catch (e) {
      text = String(e);
    }
  }
  $effect(() => {
    void [translateAll, from, to, frames, codons, uppercaseOnly, siteMode, maxCuts, particular, showReverse, showCoords, perLine, showFeatures, doc];
    schedule();
  });

  /** Search: sub-sequence, amino acids ("V  A  L") or an enzyme name; each click finds the next. */
  function search() {
    const q = (query || enzymeQuery).trim();
    if (!q || !text) return;
    const hay = text.toUpperCase();
    const needle = q.toUpperCase();
    let i = hay.indexOf(needle, matchIndex + 1);
    if (i < 0) i = hay.indexOf(needle);
    matchIndex = i;
    if (i >= 0) queueMicrotask(() => pre?.querySelector('mark')?.scrollIntoView({ block: 'center' }));
  }
  const segments = $derived.by(() => {
    const q = (query || enzymeQuery).trim();
    if (matchIndex < 0 || !q) return [{ t: text, m: false }];
    return [
      { t: text.slice(0, matchIndex), m: false },
      { t: text.slice(matchIndex, matchIndex + q.length), m: true },
      { t: text.slice(matchIndex + q.length), m: false },
    ];
  });

  function toggleParticular(name: string, on: boolean) {
    particular = on ? [...new Set([...particular, name])] : particular.filter((n) => n !== name);
    if (particular.length) siteMode = 'particular';
  }

  function handleMenu(cmd: string) {
    if (cmd === 'file.print') void printPage();
    if (cmd === 'edit.select_all') {
      const range = document.createRange();
      if (pre) range.selectNodeContents(pre);
      const sel = window.getSelection();
      sel?.removeAllRanges();
      sel?.addRange(range);
    }
    if (cmd === 'edit.copy') void api.clipboardWrite(window.getSelection()?.toString() || text);
  }

  onMount(() => {
    const unlisteners: (() => void)[] = [];
    (async () => {
      const p = await prefsReady;
      showReverse = p.smap_show_reverse;
      charSize = p.smap_char_size;
      perLine = p.smap_per_line;
      try {
        doc = await api.getDocument(id);
        enzymes = await api.enzymeList(id);
        if (!to) to = doc.bases.length;
      } catch {
        return;
      }
      unlisteners.push(await onDocChanged(id, (d) => (doc = d)));
      unlisteners.push(await onMenu(handleMenu));
      void api.noteFocus(id);
    })();
    window.addEventListener('focus', () => void api.noteFocus(id));
    return () => unlisteners.forEach((u) => u());
  });
</script>

<div class="smap">
  <div class="panels no-print">
    <fieldset class="search" data-help-name="Search" data-help="Find a sub-sequence or a restriction site in the map. Amino acids should be separated by two spaces (three in codon mode). Press Return or click the magnifying glass; another click finds the next occurrence, wrapping around.">
      <input type="text" placeholder="sequence or V  A  L" bind:value={query} onkeydown={(e) => { if (e.key === 'Enter') search(); }} />
      <select bind:value={enzymeQuery} onchange={search} title="restriction enzyme">
        <option value="">enzyme…</option>
        {#each enzymes as e (e.name)}
          <option value={e.name} class:unique={e.count === 1} class:absent={e.count === 0}>{e.name}</option>
        {/each}
      </select>
      <button type="button" onclick={search} title="Find">🔍</button>
    </fieldset>

    <fieldset data-help-name="Translation" data-help="Modify the display of the sequence translation. If 'Translate all' is unselected the nucleotides between which translation is displayed can be entered. The Frame checkboxes select the frames; with a single frame the codons can be highlighted.">
      <legend>Translation</legend>
      <label><input type="checkbox" bind:checked={translateAll} /> Translate all</label>
      <label>From <input type="number" min="1" bind:value={from} disabled={translateAll} /></label>
      <label>To <input type="number" min="1" bind:value={to} disabled={translateAll} /></label>
      <label><input type="checkbox" bind:checked={frames[0]} /> Frame 1</label>
      <label><input type="checkbox" bind:checked={frames[1]} /> Frame 2</label>
      <label><input type="checkbox" bind:checked={frames[2]} /> Frame 3</label>
      <label><input type="checkbox" bind:checked={codons} disabled={!singleFrame} /> Codons</label>
      <label><input type="checkbox" bind:checked={uppercaseOnly} /> Uppercase only</label>
    </fieldset>

    <fieldset data-help-name="Restriction sites" data-help="Modify the display of the restriction map: hide all sites, show all sites, only the enzymes cutting at most n times, or a particular list.">
      <legend>Restriction sites</legend>
      <label><input type="radio" name="sites" value="none" bind:group={siteMode} /> Do not show RE sites</label>
      <label><input type="radio" name="sites" value="all" bind:group={siteMode} /> Show all sites</label>
      <label><input type="radio" name="sites" value="max" bind:group={siteMode} /> Show RE cutting maximum <input type="number" min="1" bind:value={maxCuts} class="tiny" /> times</label>
      <label><input type="radio" name="sites" value="particular" bind:group={siteMode} /> Particular <button type="button" onclick={() => (chooser = !chooser)}>Choose…</button></label>
    </fieldset>

    <fieldset data-help-name="Display" data-help="Show the antisense strand and the nucleotide coordinates; set the character size and the number of nucleotides per line.">
      <legend>Display</legend>
      <label><input type="checkbox" bind:checked={showReverse} /> Show Reverse strand</label>
      <label><input type="checkbox" bind:checked={showCoords} /> Coord</label>
      <label>Character size <select bind:value={charSize}>{#each [8, 9, 10, 11, 12, 14] as s (s)}<option value={s}>{s} pts</option>{/each}</select></label>
      <label><input type="number" min="10" max="500" step="10" bind:value={perLine} class="small" /> nt/line</label>
      <label><input type="checkbox" bind:checked={showFeatures} /> Features</label>
    </fieldset>
  </div>

  {#if chooser}
    <div class="chooser no-print">
      <div class="chooser-head">
        <button type="button" onclick={() => (particular = [])}>Unselect all</button>
        <button type="button" onclick={() => (chooser = false)}>Close</button>
      </div>
      <div class="chooser-list">
        {#each enzymes as e (e.name)}
          <label class:unique={e.count === 1} class:absent={e.count === 0}>
            <input type="checkbox" checked={particular.includes(e.name)} onchange={(ev) => toggleParticular(e.name, ev.currentTarget.checked)} />
            {e.name} <span class="site">{e.site}</span>
          </label>
        {/each}
      </div>
    </div>
  {/if}

  <pre bind:this={pre} style:font-size="{charSize}px" data-help-name="Map" data-help="The sequence map, updated automatically whenever a parameter is modified. Select and copy all or part of it with the Edit menu.">{#each segments as s, i (i)}{#if s.m}<mark>{s.t}</mark>{:else}{s.t}{/if}{/each}</pre>

  <footer class="no-print">
    <span class="hint">{doc ? `${doc.name} — ${doc.bases.length} bp` : ''}</span>
    <button type="button" onclick={() => void printPage()} data-help-name="Print" data-help="Prints the map. 8 pt characters and 100 nt per line usually fit a landscape page.">Print</button>
  </footer>
</div>

<style>
  .smap {
    display: flex;
    flex-direction: column;
    height: 100vh;
    box-sizing: border-box;
    padding: 6px 8px;
    gap: 6px;
    position: relative;
  }
  .panels {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: stretch;
  }
  fieldset {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 4px 8px;
    margin: 0;
  }
  legend {
    color: var(--fg-muted);
    font-size: var(--fs-small);
    padding: 0 4px;
  }
  fieldset label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .search input {
    width: 160px;
    font: inherit;
    font-family: var(--font-mono);
  }
  input[type='number'] {
    width: 64px;
    font: inherit;
  }
  .tiny {
    width: 44px;
  }
  .small {
    width: 60px;
  }
  option.unique {
    font-weight: 700;
  }
  option.absent {
    font-style: italic;
  }
  pre {
    flex: 1;
    margin: 0;
    padding: 8px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: #fff;
    color: #111;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    line-height: 1.25;
    -webkit-user-select: text;
    user-select: text;
  }
  mark {
    background: #ffd54f;
  }
  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .hint {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  .chooser {
    position: absolute;
    top: 100px;
    right: 16px;
    z-index: 2;
    width: 300px;
    max-height: 55vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
  }
  .chooser-head {
    display: flex;
    gap: 4px;
    padding: 6px;
    border-bottom: 1px solid var(--border);
  }
  .chooser-list {
    overflow: auto;
    padding: 4px 6px;
  }
  .chooser-list label {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .chooser-list .unique {
    font-weight: 700;
  }
  .chooser-list .absent {
    font-style: italic;
    color: var(--fg-muted);
  }
  .site {
    font-family: var(--font-mono);
    color: var(--fg-muted);
  }
  @media print {
    .no-print,
    .chooser {
      display: none !important;
    }
    .smap {
      height: auto;
      padding: 0;
    }
    pre {
      border: none;
      overflow: visible;
      font-size: var(--smap-print-size, 8px) !important;
    }
  }
</style>
