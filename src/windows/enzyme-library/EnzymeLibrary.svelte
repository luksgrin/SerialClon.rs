<script lang="ts">
  /**
   * The Enzyme Library (User Manual §III.D, changelog beta 15 / 2.5):
   * the library with filters, isoschizomers and compatible ends, check /
   * uncheck (unchecked enzymes are hidden from maps and lists), and adding
   * type II or IIs enzymes.
   */
  import { onMount } from 'svelte';
  import { message } from '@tauri-apps/plugin-dialog';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api, type LibraryEntry } from '$lib/doc';

  let entries = $state<LibraryEntry[]>([]);
  let selected = $state('');
  let search = $state('');
  let lengthFilter = $state<'all' | '4' | '5' | '6' | '7' | '8+'>('all');
  let endFilter = $state<'all' | 'blunt' | '5' | '3'>('all');
  let hideUnchecked = $state(false);
  let iso = $state<string[]>([]);
  let compat = $state<string[]>([]);
  let line = $state('');
  let name = $state('');
  let site = $state('');
  let iis = $state(false);
  let a = $state(1);
  let b = $state(5);

  const shown = $derived(
    entries.filter((e) => {
      if (search && !e.name.toLowerCase().startsWith(search.toLowerCase())) return false;
      if (hideUnchecked && e.hidden) return false;
      if (lengthFilter !== 'all' && (lengthFilter === '8+' ? e.length < 8 : e.length !== Number(lengthFilter))) return false;
      if (endFilter === 'blunt' && e.overhang !== 0) return false;
      if (endFilter === '5' && e.overhang <= 0) return false;
      if (endFilter === '3' && e.overhang >= 0) return false;
      return true;
    }),
  );
  const current = $derived(entries.find((e) => e.name === selected));

  async function load() {
    entries = await api.libraryEntries();
  }
  async function pick(n: string) {
    selected = n;
    [iso, compat] = await Promise.all([api.isoschizomers(n), api.compatibleEnds(n)]);
  }
  async function toggle(e: LibraryEntry) {
    await api.setEnzymesHidden([e.name], !e.hidden);
    e.hidden = !e.hidden;
  }
  async function setAll(hidden: boolean) {
    await api.setEnzymesHidden(shown.map((e) => e.name), hidden);
    for (const e of shown) e.hidden = hidden;
  }
  async function add() {
    const text = line.trim() || (iis ? `#${name},${site},${a},${b},` : `${name},${site},`);
    try {
      const e = await api.addEnzyme(text);
      entries = [...entries, e].sort((x, y) => x.name.toLowerCase().localeCompare(y.name.toLowerCase()));
      line = name = site = '';
      await pick(e.name);
    } catch (err) {
      await message(String(err), { title: 'Add an enzyme', kind: 'error' });
    }
  }
  async function remove() {
    if (!current) return;
    try {
      await api.removeEnzyme(current.name);
      entries = entries.filter((e) => e.name !== current.name);
      selected = '';
    } catch (err) {
      await message(String(err), { title: 'Remove an enzyme', kind: 'error' });
    }
  }
  const ov = (o: number) => (o === 0 ? 'blunt' : o > 0 ? `5' ${o}` : `3' ${-o}`);
  onMount(load);
</script>

<ToolWindow title="Enzyme Library" subtitle={`REBASE striderc — ${entries.length} enzymes; unchecked ones are hidden everywhere`}>
  <div class="grid">
    <div class="left">
      <fieldset data-help-name="Filters" data-help="Restrict the list to enzymes with a site of a given length, or generating particular ends (blunt, 5' or 3' cohesive). Type the first letters to jump.">
        <legend>Filter</legend>
        <input type="search" placeholder="name" bind:value={search} />
        <label>Site length <select bind:value={lengthFilter}><option value="all">all</option><option value="4">4</option><option value="5">5</option><option value="6">6</option><option value="7">7</option><option value="8+">8+</option></select></label>
        <label>Ends <select bind:value={endFilter}><option value="all">all</option><option value="blunt">blunt</option><option value="5">5' cohesive</option><option value="3">3' cohesive</option></select></label>
        <label><input type="checkbox" bind:checked={hideUnchecked} /> hide unchecked</label>
        <button type="button" onclick={() => void setAll(false)} data-help-name="Check all" data-help="Checks every listed enzyme.">Check all</button>
        <button type="button" onclick={() => void setAll(true)} data-help-name="Uncheck all" data-help="Unchecks every listed enzyme: they disappear from maps, Find, Site Usage and the Virtual Cutter until checked again.">Uncheck all</button>
      </fieldset>
      <div class="scroll">
        <table>
          <thead><tr><th></th><th>Enzyme</th><th>Site</th><th>Ends</th></tr></thead>
          <tbody>
            {#each shown as e (e.name)}
              <tr class:selected={e.name === selected} class:hidden-row={e.hidden} onclick={() => void pick(e.name)}>
                <td><input type="checkbox" checked={!e.hidden} onclick={(ev) => ev.stopPropagation()} onchange={() => void toggle(e)} /></td>
                <td>{e.name}{e.user ? ' •' : ''}</td>
                <td class="mono">{e.site}</td>
                <td>{ov(e.overhang)}{e.type_iis ? ' (IIs)' : ''}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
    <div class="right">
      {#if current}
        <fieldset data-help-name="Selected enzyme" data-help="Isoschizomers recognise the same site; compatible ends can be ligated to this enzyme's ends.">
          <legend>{current.name}</legend>
          <div class="wide">Site <span class="mono">{current.site}</span> — {ov(current.overhang)}</div>
          <div class="wide">Isoschizomers: {iso.length ? iso.join(', ') : '—'}</div>
          <div class="wide">Compatible ends: {compat.length ? compat.join(', ') : '—'}</div>
          {#if current.user}<button type="button" onclick={remove}>Remove this enzyme</button>{/if}
        </fieldset>
      {/if}
      <fieldset data-help-name="Add an enzyme" data-help="Adds a type II enzyme (site with the cut marked by /) or a type IIs enzyme (site plus the two cut offsets from its 3' end, REBASE notation), or paste a striderc line. Added enzymes are kept across runs.">
        <legend>Add an enzyme</legend>
        <label>Name <input type="text" bind:value={name} size="10" /></label>
        <label>Site <input type="text" class="mono" bind:value={site} size="14" placeholder={iis ? 'cacctgc' : 'g/aattc'} /></label>
        <label><input type="checkbox" bind:checked={iis} /> type IIs</label>
        {#if iis}<label>cuts <input type="number" bind:value={a} class="tiny" /> / <input type="number" bind:value={b} class="tiny" /></label>{/if}
        <label class="wide">or a striderc line <input type="text" class="mono" bind:value={line} placeholder="#AarI,cacctgc,4,8," /></label>
        <button type="button" class="primary" onclick={add} disabled={!line.trim() && !(name.trim() && site.trim())}>Add</button>
      </fieldset>
    </div>
  </div>
</ToolWindow>

<style>
  .grid {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 300px;
    gap: 10px;
    min-height: 0;
  }
  .left,
  .right {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }
  .hidden-row {
    color: var(--fg-muted);
  }
  .wide {
    width: 100%;
  }
  .tiny {
    width: 48px;
  }
</style>
