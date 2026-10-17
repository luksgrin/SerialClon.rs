<script lang="ts">
  /**
   * Preferences (⌘,): every knob the original had (User Manual §III.A,
   * changelog), applied live. File ▸ Page Setup opens the Printing section.
   */
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { confirm, message } from '@tauri-apps/plugin-dialog';
  import { api } from '$lib/doc';
  import { checkForUpdates, prefs, prefsReady, resetPrefs, savePrefs, type Prefs } from '$lib/prefs';

  const section = new URLSearchParams(window.location.search).get('section');

  let draft = $state<Prefs | null>(null);
  let geneticCodes = $state<{ id: number; name: string; selected: boolean }[]>([]);
  let pkTables = $state<[string, boolean][]>([]);
  let codonTables = $state<{ slug: string; name: string; selected: boolean }[]>([]);
  let geneticCode = $state(1);
  let pkTable = $state('');
  let codonTable = $state('');
  let saving = false;
  let printing = $state<HTMLElement | undefined>();

  let timer: ReturnType<typeof setTimeout> | undefined;
  function queueSave() {
    clearTimeout(timer);
    timer = setTimeout(() => void save(), 250);
  }
  async function save() {
    if (!draft || saving) return;
    saving = true;
    try {
      draft = await savePrefs($state.snapshot(draft));
    } catch (e) {
      await message(String(e), { title: 'Preferences', kind: 'error' });
    } finally {
      saving = false;
    }
  }
  async function reset() {
    if (!(await confirm('Reset every preference to its default value?', { title: 'Reset Preferences', kind: 'warning' }))) return;
    draft = await resetPrefs();
  }
  async function updates() {
    const r = await checkForUpdates();
    void r; // the shell shows the outcome in a dialog
  }
  async function loadProtein() {
    try {
      const codes = await api.geneticCodes();
      geneticCodes = codes as unknown as typeof geneticCodes;
      geneticCode = geneticCodes.find((c) => c.selected)?.id ?? 1;
      pkTables = (await api.pkTables()) as unknown as typeof pkTables;
      pkTable = pkTables.find(([, on]) => on)?.[0] ?? '';
      const tables = await api.codonTables();
      codonTables = tables as unknown as typeof codonTables;
      codonTable = codonTables.find((t) => t.selected)?.slug ?? '';
    } catch {
      /* browser */
    }
  }

  onMount(async () => {
    draft = { ...(await prefsReady) };
    await loadProtein();
    if (section === 'printing') queueMicrotask(() => printing?.scrollIntoView({ block: 'start' }));
  });
</script>

<div class="prefs">
  {#if draft}
    <section data-help-name="General" data-help="General preferences: locking, recent files, import warning, autosave, updates, look.">
      <h2>General</h2>
      <label><input type="checkbox" bind:checked={draft.autolock} onchange={queueSave} /> Automatically lock newly opened sequences</label>
      <label>Recent files to remember <input type="number" min="1" max="25" bind:value={draft.recent_files} onchange={queueSave} /> (max 25)</label>
      <label><input type="checkbox" bind:checked={draft.warn_import} onchange={queueSave} /> Warn when opening a file that is not in Serial Cloner format</label>
      <label>Autosave every <input type="number" min="5" max="3600" bind:value={draft.autosave_secs} onchange={queueSave} /> seconds</label>
      <label><input type="checkbox" bind:checked={draft.check_updates} onchange={queueSave} /> Check for a new version at start (once a day) <button type="button" onclick={updates}>Check now</button></label>
      <label>Theme
        <select bind:value={draft.theme} onchange={queueSave}>
          <option value="modern">Modern</option>
          <option value="aqua">Aqua Classic (Serial Cloner 2.6 look)</option>
        </select>
      </label>
      <label><input type="checkbox" bind:checked={draft.limit_window_width} onchange={queueSave} /> Limit the width of Sequence and Protein windows</label>
      <label>Toolbar vertical shift <input type="number" min="-200" max="400" bind:value={draft.toolbar_shift} onchange={queueSave} /> px</label>
    </section>

    <section data-help-name="Sequence Window" data-help="Character size of the sequence field and the speed of Read selection.">
      <h2>Sequence Window</h2>
      <label>Character size <input type="number" min="8" max="32" bind:value={draft.seq_font_size} onchange={queueSave} /> px</label>
      <label>Read-out speed <input type="range" min="0.5" max="2" step="0.1" bind:value={draft.read_speed} onchange={queueSave} /> {draft.read_speed.toFixed(1)}×</label>
      <label>ORF start codon <select bind:value={draft.orf_start_codons} onchange={queueSave}>
        <option value="ATG">ATG</option>
        <option value="GTG">GTG</option>
        <option value="ATG|GTG">ATG | GTG</option>
        <option value="ATG|CTG|GTG|TTG">ATG | CTG | GTG | TTG</option>
      </select></label>
      <label>Minimum ORF size (Find ▸ ORFs) <input type="number" min="0" max="10000" bind:value={draft.orf_min_codons} onchange={queueSave} /> aa (0 = no limit)</label>
    </section>

    <section data-help-name="Graphic Map" data-help="Colours of the site categories and of the selection, character sizes, and whether names are boxed.">
      <h2>Graphic Map</h2>
      <div class="colors">
        <label>Unique sites <input type="color" bind:value={draft.color_unique} onchange={queueSave} /></label>
        <label>Double sites <input type="color" bind:value={draft.color_double} onchange={queueSave} /></label>
        <label>Particular sites <input type="color" bind:value={draft.color_particular} onchange={queueSave} /></label>
        <label>BEGIN site <input type="color" bind:value={draft.color_begin} onchange={queueSave} /></label>
        <label>END site <input type="color" bind:value={draft.color_end} onchange={queueSave} /></label>
        <label>New feature <input type="color" bind:value={draft.color_feature} onchange={queueSave} /></label>
      </div>
      <label>RE name character size <input type="number" min="7" max="24" bind:value={draft.gmap_re_font_size} onchange={queueSave} /> px</label>
      <label>Feature name character size <input type="number" min="7" max="24" bind:value={draft.gmap_feature_font_size} onchange={queueSave} /> px</label>
      <label><input type="checkbox" bind:checked={draft.gmap_box_re_names} onchange={queueSave} /> Box the restriction-site names</label>
      <label><input type="checkbox" bind:checked={draft.gmap_box_unique} onchange={queueSave} /> Box and colour unique sites</label>
    </section>

    <section data-help-name="Sequence Map" data-help="Defaults of the Sequence Map window: nucleotides per line, reverse strand, character sizes, amino-acid alignment.">
      <h2>Sequence Map</h2>
      <label>Nucleotides per line <input type="number" min="10" max="500" step="10" bind:value={draft.smap_per_line} onchange={queueSave} /></label>
      <label><input type="checkbox" bind:checked={draft.smap_show_reverse} onchange={queueSave} /> Show the reverse strand by default</label>
      <label>Character size on screen <input type="number" min="6" max="24" bind:value={draft.smap_char_size} onchange={queueSave} /> pt</label>
      <label>Character size when printing <input type="number" min="4" max="16" bind:value={draft.smap_print_size} onchange={queueSave} /> pt (8 fits a landscape page)</label>
      <label><input type="checkbox" bind:checked={draft.smap_aa_middle} onchange={queueSave} /> Align the amino acid with the middle of the codon</label>
    </section>

    <section data-help-name="Cloning" data-help="Defaults of the cloning windows.">
      <h2>Cloning</h2>
      <label><input type="checkbox" bind:checked={draft.pcr_ta_overhang} onchange={queueSave} /> Run a PCR: 3' A overhang by default (TA cloning)</label>
    </section>

    <section data-help-name="Protein" data-help="Genetic code and pK table used for translation and pI, the codon usage table for reverse translation, and whether that table also defines the genetic code.">
      <h2>Protein</h2>
      <label>Genetic code
        <select bind:value={geneticCode} onchange={() => void api.setGeneticCode(geneticCode)}>
          {#each geneticCodes as c (c.id)}<option value={c.id}>{c.id}. {c.name}</option>{/each}
        </select>
      </label>
      <label>pK table for pI
        <select bind:value={pkTable} onchange={() => void api.setPkTable(pkTable)}>
          {#each pkTables as [name] (name)}<option value={name}>{name}</option>{/each}
        </select>
      </label>
      <label>Codon usage table
        <select bind:value={codonTable} onchange={() => void api.selectCodonTable(codonTable)}>
          {#each codonTables as t (t.slug)}<option value={t.slug}>{t.name}</option>{/each}
        </select>
        <button type="button" onclick={() => void api.openReferenceAny('codon-usage')}>Manage…</button>
      </label>
      <label><input type="checkbox" bind:checked={draft.codon_as_code} onchange={queueSave} /> Use the codon usage table as the genetic code</label>
    </section>

    <section data-help-name="Network" data-help="Proxy used for NCBI, ENA and the update check. Leave empty to use the system settings.">
      <h2>Network</h2>
      <label>Proxy URL <input type="text" placeholder="system settings" bind:value={draft.proxy} onchange={queueSave} /></label>
    </section>

    <section bind:this={printing} data-help-name="Printing" data-help="File ▸ Page Setup: orientation and margins of printed maps.">
      <h2>Printing (Page Setup)</h2>
      <label><input type="radio" name="orient" checked={!draft.page_landscape} onchange={() => { if (draft) { draft.page_landscape = false; queueSave(); } }} /> Portrait</label>
      <label><input type="radio" name="orient" checked={draft.page_landscape} onchange={() => { if (draft) { draft.page_landscape = true; queueSave(); } }} /> Landscape</label>
      <label>Margins <input type="number" min="0" max="50" bind:value={draft.page_margin_mm} onchange={queueSave} /> mm</label>
    </section>

    <footer>
      <button type="button" onclick={reset} data-help-name="Reset" data-help="Puts every preference back to its default. Launching with --reset-prefs does the same, like the original's Ctrl+Shift at start.">Reset Preferences</button>
      <span class="hint">Changes apply immediately.</span>
      <button type="button" class="primary" onclick={() => void getCurrentWindow().close()}>Close</button>
    </footer>
  {/if}
</div>

<style>
  .prefs {
    height: 100vh;
    box-sizing: border-box;
    overflow: auto;
    padding: 10px 14px;
  }
  section {
    margin-bottom: 12px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  h2 {
    margin: 0 0 2px;
    font-size: var(--fs-ui);
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  label {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  input[type='number'] {
    width: 72px;
    font: inherit;
  }
  input[type='text'] {
    flex: 1;
    min-width: 200px;
    font: inherit;
  }
  select {
    font: inherit;
  }
  .colors {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px 12px;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-bottom: 10px;
  }
  .hint {
    flex: 1;
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  .primary {
    background: var(--accent);
    color: #fff;
    padding: 3px 18px;
  }
</style>
