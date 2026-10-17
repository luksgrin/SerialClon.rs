<script lang="ts">
  /**
   * Manage Features (Features ReadMe §IX):
   * Collections on the left, their Features in the middle, the definition
   * editor (Simple / Complex) on the right. OK saves everything.
   */
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Menu } from '@tauri-apps/api/menu';
  import { confirm, message } from '@tauri-apps/plugin-dialog';
  import { api, FEATURE_KINDS, type Collection, type FeatureDef } from '$lib/doc';

  let collections = $state<Collection[]>([]);
  let ci = $state(0);
  let fi = $state(-1);
  let search = $state('');
  let mode = $state<'simple' | 'complex'>('simple');
  let altHeld = $state(false);

  const collection = $derived(collections[ci]);
  const visible = $derived.by(() => {
    const c = collection;
    if (!c) return [] as { i: number; f: FeatureDef }[];
    const q = search.trim().toLowerCase();
    return c.features.map((f, i) => ({ i, f })).filter(({ f }) => !q || f.name.toLowerCase().includes(q));
  });
  const feature = $derived(collection?.features[fi] ?? null);

  function pickCollection(i: number) {
    ci = i;
    fi = -1;
    search = '';
  }
  function pickFeature(i: number) {
    fi = i;
    if (feature?.three || feature?.potential_five || feature?.potential_three || feature?.upstream || feature?.downstream) mode = 'complex';
  }
  function blank(): FeatureDef {
    return { name: 'New Feature', kind: 'misc_feature', color: '#84a4c0', arrow: true, sense_only: false, peptide: false, five: '', three: '', potential_five: '', potential_three: '', upstream: '', downstream: '', comment: '', scan: true };
  }
  function addCollection() {
    collections = [...collections, { name: `Collection ${collections.length + 1}`, scan: true, builtin: false, features: [] }];
    pickCollection(collections.length - 1);
  }
  async function deleteCollection() {
    if (!collection) return;
    if (!altHeld && !(await confirm(`Delete the Collection "${collection.name}" and its ${collection.features.length} Features? This cannot be undone; unchecking Scan is the safer choice.`, { title: 'Delete Collection', kind: 'warning' }))) return;
    collections = collections.filter((_, i) => i !== ci);
    pickCollection(Math.max(0, ci - 1));
  }
  function addFeature() {
    if (!collection) return;
    collection.features.push(blank());
    pickFeature(collection.features.length - 1);
  }
  async function deleteFeature() {
    if (!collection || !feature) return;
    if (!altHeld && !(await confirm(`Delete the Feature "${feature.name}"? Unchecking Scan is the safer choice.`, { title: 'Delete Feature', kind: 'warning' }))) return;
    collection.features.splice(fi, 1);
    fi = Math.min(fi, collection.features.length - 1);
  }
  function duplicateFeature() {
    if (!collection || !feature) return;
    collection.features.splice(fi + 1, 0, { ...feature, name: `${feature.name} copy` });
    fi = fi + 1;
  }
  function moveTo(target: number) {
    if (!collection || !feature || target === ci) return;
    const f = feature;
    collection.features.splice(fi, 1);
    collections[target]!.features.push(f);
    fi = -1;
  }
  async function importOrExport(e: MouseEvent) {
    if (e.altKey) {
      if (!collection) return;
      const menu = await Menu.new({
        items: [
          { id: 'json', text: 'Export as SerialClon collection (.json)', action: () => void api.exportCollection(collection.name, false) },
          { id: 'table', text: 'Export as generic table (.txt)', action: () => void api.exportCollection(collection.name, true) },
        ],
      });
      await menu.popup();
      return;
    }
    const r = await api.importCollection();
    if (r) {
      collections = r;
      pickCollection(collections.length - 1);
    }
  }
  async function restore() {
    if (!(await confirm('Restore the five built-in Collections to their shipped content? Your own Collections are kept.', { title: 'Restore built-in Collections' }))) return;
    collections = await api.restoreBuiltinCollections();
    pickCollection(0);
  }
  async function ok() {
    for (const c of collections) for (const f of c.features) if (!f.five.trim()) {
      await message(`"${f.name}" in ${c.name} has no sequence.`, { title: 'Incomplete Feature', kind: 'warning' });
      return;
    }
    await api.saveCollections($state.snapshot(collections));
    await getCurrentWindow().close();
  }
  function onKey(e: KeyboardEvent) {
    altHeld = e.altKey;
  }

  onMount(async () => {
    try {
      collections = await api.listCollections();
    } catch {
      collections = [];
    }
  });
</script>

<svelte:window onkeydown={onKey} onkeyup={onKey} />

<div class="manage">
  <section class="col collections">
    <h2>Collection</h2>
    <div class="list" data-help-name="Collections" data-help="The Feature Collections. The checkbox indicates whether a Collection is used when scanning; unselect the ones you never need to speed scans up.">
      <table>
        <thead><tr><th>Scan</th><th>Name</th></tr></thead>
        <tbody>
          {#each collections as c, i (i)}
            <tr class:selected={i === ci} onclick={() => pickCollection(i)}>
              <td><input type="checkbox" bind:checked={c.scan} onclick={(e) => e.stopPropagation()} /></td>
              <td>{c.name}{c.builtin ? '' : ' •'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="tools">
      <button type="button" onclick={addCollection} data-help-name="+" data-help="Adds an empty Collection.">+</button>
      <button type="button" class:danger={altHeld} onclick={deleteCollection} disabled={!collection} data-help-name="−" data-help="Deletes the selected Collection (Alt: without confirmation). Not undoable.">−</button>
      <button type="button" onclick={importOrExport} data-help-name="Import" data-help="Imports a Collection saved by SerialClon (.json), as a generic table (name, sequence, type, colour, DNA|Peptide, tab-separated) or by ApE. Press [Alt] to Export the selected Collection instead.">{altHeld ? 'Export' : 'Import'}</button>
      <button type="button" onclick={restore} data-help-name="Restore" data-help="Puts the five built-in Collections back.">Restore built-in</button>
    </div>
  </section>

  <section class="col features">
    <h2>Feature <input type="search" placeholder="search" bind:value={search} /></h2>
    <div class="list" data-help-name="Features" data-help="The Features of the selected Collection. The checkbox indicates whether the Feature is used when scanning.">
      <table>
        <thead><tr><th>Scan</th><th>Name</th></tr></thead>
        <tbody>
          {#each visible as { i, f } (i)}
            <tr class:selected={i === fi} onclick={() => pickFeature(i)}>
              <td><input type="checkbox" bind:checked={f.scan} onclick={(e) => e.stopPropagation()} /></td>
              <td>{f.name}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="tools">
      <button type="button" onclick={addFeature} disabled={!collection}>+</button>
      <button type="button" class:danger={altHeld} onclick={deleteFeature} disabled={!feature}>−</button>
      <button type="button" onclick={duplicateFeature} disabled={!feature} data-help-name="Duplicate" data-help="Duplicates the Feature, e.g. to store it in another Collection scanned at other times.">Duplicate</button>
    </div>
  </section>

  <section class="col editor">
    {#if collection && feature}
      <div class="tabs" role="tablist">
        <button type="button" role="tab" aria-selected={mode === 'simple'} onclick={() => (mode = 'simple')}>Simple</button>
        <button type="button" role="tab" aria-selected={mode === 'complex'} onclick={() => (mode = 'complex')}>Complex</button>
      </div>
      <div class="grid">
        <div class="left">
          <label>Name <input type="text" bind:value={feature.name} /></label>
          <div class="row">
            <label>Colour <input type="color" bind:value={feature.color} /></label>
            <label><input type="checkbox" bind:checked={feature.arrow} /> Arrow</label>
          </div>
          {#if mode === 'simple'}
            <label class="seq" data-help-name="Sequence of the feature" data-help="The consensus to scan for: DNA (degenerate IUPAC letters allowed) or a peptide, matched against the translation of both strands in the three frames.">Sequence of the feature <textarea rows="6" bind:value={feature.five}></textarea></label>
          {:else}
            <label class="seq" data-help-name="5' Sequence" data-help="Used alone it is the whole consensus; with a 3' sequence it is the 5' end, anything between the two being part of the Feature.">5' Sequence of the feature <textarea rows="4" bind:value={feature.five}></textarea></label>
            <label class="seq" data-help-name="3' Sequence" data-help="The 3' end of the Feature; the intervening sequence belongs to the Feature.">3' Sequence of the feature <textarea rows="2" bind:value={feature.three}></textarea></label>
            <label class="seq" data-help-name="Potential 5' sequence" data-help="May or may not be present; incorporated in the Feature when it is (e.g. the START codon of EGFP).">Potential 5' Sequence of the feature <textarea rows="1" bind:value={feature.potential_five}></textarea></label>
            <label class="seq" data-help-name="Potential 3' sequence" data-help="May or may not be present; incorporated when it is (e.g. the STOP codon).">Potential 3' Sequence of the feature <textarea rows="1" bind:value={feature.potential_three}></textarea></label>
            <label class="seq" data-help-name="Upstream sequence" data-help="Must be present for the Feature to be scored, but is not part of it.">Sequence upstream of the feature <textarea rows="1" bind:value={feature.upstream}></textarea></label>
            <label class="seq" data-help-name="Downstream sequence" data-help="Must be present for the Feature to be scored, but is not part of it (the CVLL of a prenylated cysteine).">Sequence downstream of the feature <textarea rows="1" bind:value={feature.downstream}></textarea></label>
          {/if}
          <label class="seq">Comments <textarea rows="2" bind:value={feature.comment}></textarea></label>
        </div>
        <div class="right">
          <fieldset>
            <label><input type="radio" checked={!feature.peptide} onchange={() => { if (feature) feature.peptide = false; }} /> DNA</label>
            <label><input type="radio" checked={feature.peptide} onchange={() => { if (feature) feature.peptide = true; }} /> Peptide</label>
          </fieldset>
          <label><input type="checkbox" bind:checked={feature.sense_only} /> Scan only Sense Strand</label>
          <label>Type <select bind:value={feature.kind}>{#each FEATURE_KINDS as k (k)}<option value={k}>{k}</option>{/each}{#if !FEATURE_KINDS.includes(feature.kind)}<option value={feature.kind}>{feature.kind}</option>{/if}</select></label>
          <label>Collection <select value={ci} onchange={(e) => moveTo(Number(e.currentTarget.value))}>{#each collections as c, i (i)}<option value={i}>{c.name}</option>{/each}</select></label>
          <label><input type="checkbox" bind:checked={feature.scan} /> Scan for this feature</label>
        </div>
      </div>
    {:else}
      <p class="hint">Select a Feature to edit it, or press + to add one.</p>
    {/if}
    <div class="ok">
      <button type="button" onclick={() => void getCurrentWindow().close()}>Cancel</button>
      <button type="button" class="primary" onclick={ok} data-help-name="OK" data-help="Saves every Collection and closes the window.">OK</button>
    </div>
  </section>
</div>

<style>
  .manage {
    display: grid;
    grid-template-columns: 200px 220px 1fr;
    gap: 10px;
    height: 100vh;
    box-sizing: border-box;
    padding: 8px;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  h2 {
    margin: 0;
    font-size: var(--fs-ui);
    color: var(--fg-muted);
    display: flex;
    gap: 6px;
    align-items: center;
  }
  h2 input {
    flex: 1;
    font: inherit;
  }
  .list {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 2px 6px;
    text-align: left;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }
  th {
    position: sticky;
    top: 0;
    background: var(--bg);
    font-size: var(--fs-small);
  }
  tr.selected {
    background: var(--accent);
    color: #fff;
  }
  .tools {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .danger {
    background: var(--select-end);
    color: #fff;
  }
  .tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    padding: 3px 14px;
    border: 1px solid transparent;
    border-bottom: none;
    border-radius: var(--radius) var(--radius) 0 0;
    background: transparent;
  }
  .tabs button[aria-selected='true'] {
    border-color: var(--border);
    background: var(--bg-raised);
    margin-bottom: -1px;
  }
  .grid {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 200px;
    gap: 10px;
    overflow: auto;
  }
  .left,
  .right {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .left label,
  .right label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .right label,
  .row label {
    flex-direction: row;
    align-items: center;
    gap: 4px;
  }
  .row {
    display: flex;
    gap: 14px;
  }
  .seq textarea {
    font-family: var(--font-mono);
    font-size: var(--fs-ui);
    resize: vertical;
  }
  fieldset {
    display: flex;
    gap: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 4px 8px;
  }
  .ok {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  .primary {
    background: var(--accent);
    color: #fff;
    padding: 3px 18px;
  }
  .hint {
    color: var(--fg-muted);
  }
</style>
