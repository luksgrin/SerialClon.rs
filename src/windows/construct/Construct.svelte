<script lang="ts">
  /**
   * Build a Construct (⌘1, User Manual §III.E): fragment slots, each chosen
   * between two restriction sites of an open sequence (or sent from the
   * Graphic Map's two-click selection), blunted if needed, then ligated
   * into a new construct.
   */
  import { onMount } from 'svelte';
    import { message } from '@tauri-apps/plugin-dialog';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api, type ConstructSelection, type FragmentPreview, type MapSite, type SiteRefView, listenHere } from '$lib/doc';

  interface Slot {
    doc: number | null;
    name: string;
    sites: MapSite[];
    circular: boolean;
    whole: boolean;
    begin: string;
    end: string;
    clockwise: boolean;
    bluntLeft: boolean;
    bluntRight: boolean;
    preview: FragmentPreview | null;
    error: string;
  }

  let docs = $state<[number, string][]>([]);
  let slots = $state<Slot[]>([blank(), blank()]);
  let active = $state(0);
  let circular = $state(true);
  let name = $state('Construct');
  let busy = $state(false);

  function blank(): Slot {
    return { doc: null, name: '', sites: [], circular: false, whole: false, begin: '', end: '', clockwise: true, bluntLeft: false, bluntRight: false, preview: null, error: '' };
  }
  const keyOf = (s: MapSite) => `${s.name}@${s.position}${s.forward === false ? 'r' : ''}`;
  function ref(slot: Slot, key: string): SiteRefView | null {
    const s = slot.sites.find((x) => keyOf(x) === key);
    return s ? { enzyme: s.name, position: s.position, cut_top: s.cut_top, cut_bottom: s.cut_bottom } : null;
  }
  function spec(slot: Slot) {
    if (slot.doc === null) return null;
    const whole = slot.whole && !slot.circular;
    const begin = whole ? null : ref(slot, slot.begin);
    const end = whole ? null : ref(slot, slot.end);
    if (!whole && !begin) return null;
    return { doc: slot.doc, begin, end, clockwise: slot.clockwise, blunt_left: slot.bluntLeft, blunt_right: slot.bluntRight };
  }

  async function refreshDocs() {
    docs = await api.listDocuments();
  }
  async function chooseDoc(i: number, id: number | null) {
    const slot = slots[i]!;
    slot.doc = id;
    slot.sites = [];
    slot.begin = '';
    slot.end = '';
    slot.preview = null;
    if (id === null) return;
    try {
      const d = await api.mapData(id);
      slot.name = d.doc.name;
      slot.circular = d.doc.circular;
      slot.sites = d.sites.filter((s) => s.count <= 2).sort((a, b) => a.position - b.position);
      slot.whole = !d.doc.circular;
    } catch (e) {
      slot.error = String(e);
    }
    await preview();
  }
  async function openFile(i: number) {
    const ids = await api.openFiles();
    await refreshDocs();
    if (ids[0] !== undefined) await chooseDoc(i, ids[0]);
  }
  async function preview() {
    for (const slot of slots) {
      const s = spec(slot);
      slot.error = '';
      if (!s) {
        slot.preview = null;
        continue;
      }
      try {
        slot.preview = (await api.previewFragments([s]))[0] ?? null;
      } catch (e) {
        slot.error = String(e);
        slot.preview = null;
      }
    }
  }
  function selectFullInsert(i: number) {
    const slot = slots[i]!;
    if (slot.circular) return;
    slot.whole = true;
    void preview();
  }
  async function ligate() {
    const specs = slots.map(spec);
    if (specs.some((s) => !s)) {
      await message('Every slot needs a sequence and a BEGIN site (and an END site on linear sequences), or "whole linear sequence".', { title: 'Ligate' });
      return;
    }
    busy = true;
    try {
      await api.ligateConstruct(specs as NonNullable<(typeof specs)[number]>[], circular, name);
    } catch (e) {
      await message(String(e), { title: 'Ligation impossible', kind: 'error' });
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    void refreshDocs();
    const timer = setInterval(() => void refreshDocs(), 3000);
    let unlisten: (() => void) | undefined;
    void listenHere<ConstructSelection>('construct:selection', async (e) => {
      const sel = e.payload;
      const slot = slots[active] ?? slots[0]!;
      await chooseDoc(active, sel.doc);
      if (sel.begin) slot.begin = `${sel.begin.enzyme}@${sel.begin.position}`;
      if (sel.end) slot.end = `${sel.end.enzyme}@${sel.end.position}`;
      // Site keys carry an orientation suffix for reverse hits; fall back by name+position.
      const fix = (v: string) => slot.sites.find((s) => keyOf(s).startsWith(v));
      if (sel.begin && !slot.sites.some((s) => keyOf(s) === slot.begin)) slot.begin = fix(slot.begin) ? keyOf(fix(slot.begin)!) : '';
      if (sel.end && !slot.sites.some((s) => keyOf(s) === slot.end)) slot.end = fix(slot.end) ? keyOf(fix(slot.end)!) : '';
      slot.clockwise = sel.clockwise;
      slot.whole = !sel.begin;
      await preview();
    }).then((u) => (unlisten = u));
    return () => {
      clearInterval(timer);
      unlisten?.();
    };
  });
</script>

<ToolWindow title="Build a Construct" subtitle="choose the fragments, then Ligate">
  <div class="slots">
    {#each slots as slot, i (i)}
      <fieldset class:active={i === active} onfocusin={() => (active = i)} data-help-name="Fragment {i + 1}" data-help="A fragment of the construct: pick an open sequence (or Select DNA… to open a file), then its BEGIN and END sites — or select them on the Graphic Map and use 'Use Selection in Build a Construct'. The first slot is usually the vector.">
        <legend>{i === 0 ? 'Vector' : `Insert ${i}`}</legend>
        <label>Sequence
          <select value={slot.doc ?? ''} onchange={(e) => void chooseDoc(i, e.currentTarget.value === '' ? null : Number(e.currentTarget.value))}>
            <option value="">—</option>
            {#each docs as [id, n] (id)}<option value={id}>{n}</option>{/each}
          </select>
        </label>
        <button type="button" onclick={() => void openFile(i)} data-help-name="Select DNA…" data-help="Opens a sequence file into a new Sequence Window and uses it here.">Select DNA…</button>
        <button type="button" onclick={() => slot.doc !== null && void api.openGraphicMap(slot.doc)} disabled={slot.doc === null} data-help-name="Graphic Map" data-help="Shows the Graphic Map of the sequence to choose the sites by clicking (BEGIN) and double-clicking (END), then right-click ▸ Use Selection in Build a Construct.">Graphic Map</button>
        {#if slot.doc !== null}
          <label>BEGIN
            <select bind:value={slot.begin} onchange={preview} disabled={slot.whole}>
              <option value="">—</option>
              {#each slot.sites as s (keyOf(s))}<option value={keyOf(s)} class:unique={s.count === 1}>{s.position} {s.forward === null ? '' : s.forward ? '>' : '<'}{s.name}</option>{/each}
            </select>
          </label>
          <label>END
            <select bind:value={slot.end} onchange={preview} disabled={slot.whole}>
              <option value="">{slot.circular ? '(same site: whole plasmid)' : '—'}</option>
              {#each slot.sites as s (keyOf(s))}<option value={keyOf(s)} class:unique={s.count === 1}>{s.position} {s.forward === null ? '' : s.forward ? '>' : '<'}{s.name}</option>{/each}
            </select>
          </label>
          <label><input type="checkbox" bind:checked={slot.clockwise} onchange={preview} /> {slot.circular ? 'CW' : 'Sense'}</label>
          {#if !slot.circular}
            <label><input type="checkbox" bind:checked={slot.whole} onchange={preview} /> whole linear sequence</label>
            <button type="button" onclick={() => selectFullInsert(i)} data-help-name="Select full insert" data-help="Uses the whole linear sequence with its own ends (changelog 2.6).">Full insert</button>
          {/if}
          <label data-help-name="Blunt left" data-help="Blunts the left end before ligation: a 5' overhang is filled in, a 3' overhang is chewed back."><input type="checkbox" bind:checked={slot.bluntLeft} onchange={preview} /> blunt left</label>
          <label data-help-name="Blunt right" data-help="Blunts the right end before ligation."><input type="checkbox" bind:checked={slot.bluntRight} onchange={preview} /> blunt right</label>
        {/if}
        {#if slot.preview}
          <span class="preview mono">{slot.preview.name}: {slot.preview.length} bp, left {slot.preview.left}, right {slot.preview.right}, {slot.preview.features} features</span>
        {/if}
        {#if slot.error}<span class="error">{slot.error}</span>{/if}
      </fieldset>
    {/each}
  </div>
  <div class="row">
    <button type="button" onclick={() => (slots = [...slots, blank()])}>+ slot</button>
    <button type="button" onclick={() => { if (slots.length > 1) slots = slots.slice(0, -1); }}>− slot</button>
    <label><input type="checkbox" bind:checked={circular} /> circular construct (last end ligates to the first)</label>
    <label>Name <input type="text" bind:value={name} /></label>
    <button type="button" class="primary" onclick={ligate} disabled={busy} data-help-name="Ligate" data-help="Ligates the fragments in order. Ends must be compatible (both blunt, or complementary overhangs of the same kind); the product opens in a new Sequence Window with its provenance in the comments and the inherited Features.">Ligate</button>
  </div>
</ToolWindow>

<style>
  .slots {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  fieldset.active {
    border-color: var(--accent);
  }
  .preview {
    width: 100%;
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  option.unique {
    font-weight: 700;
  }
</style>
