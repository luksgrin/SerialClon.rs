<script lang="ts">
  /** Make shRNA (⌘4): stored layouts, an siRNA core, optional point mutations → oligos + insert. */
  import { onMount } from 'svelte';
  import { confirm, message } from '@tauri-apps/plugin-dialog';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api, type Oligos, type ShrnaLayout } from '$lib/doc';

  let layouts = $state<ShrnaLayout[]>([]);
  let li = $state(0);
  let core = $state('');
  let mutations = $state('');
  let name = $state('shRNA');
  let result = $state<Oligos | null>(null);
  const layout = $derived(layouts[li]);

  function parseMutations(): number[] {
    return mutations.split(/[ ,;]+/).map((s) => Number(s)).filter((n) => Number.isInteger(n) && n > 0);
  }
  async function make(open: boolean) {
    if (!layout) return;
    try {
      result = await api.makeShrna(name, $state.snapshot(layout), core, parseMutations(), open);
    } catch (e) {
      await message(String(e), { title: 'Make shRNA', kind: 'error' });
    }
  }
  async function save() {
    await api.saveShrnaLayouts($state.snapshot(layouts));
  }
  function addLayout() {
    layouts = [...layouts, { name: `Layout ${layouts.length + 1}`, left_overhang: '', prefix: '', loop_seq: 'TTCAAGAGA', suffix: 'TTTTT', right_overhang: '' }];
    li = layouts.length - 1;
  }
  async function deleteLayout() {
    if (!layout || !(await confirm(`Delete the layout "${layout.name}"?`, { title: 'Make shRNA' }))) return;
    layouts = layouts.filter((_, i) => i !== li);
    li = Math.max(0, li - 1);
    await save();
  }
  onMount(async () => {
    layouts = await api.shrnaLayouts();
  });
</script>

<ToolWindow title="Make shRNA" subtitle="top oligo = left overhang + prefix + sense + loop + antisense + suffix">
  <div class="grid">
    <fieldset class="layout" data-help-name="Layout" data-help="A stored layout describes the vector: the cohesive ends of the oligos, the prefix before the sense arm, the loop and the suffix (usually the pol III terminator). Save keeps your layouts across runs.">
      <legend>Layout</legend>
      <select bind:value={li}>{#each layouts as l, i (i)}<option value={i}>{l.name}</option>{/each}</select>
      <button type="button" onclick={addLayout}>+</button>
      <button type="button" onclick={deleteLayout} disabled={!layout}>−</button>
      {#if layout}
        <label>Name <input type="text" bind:value={layout.name} /></label>
        <label>Left overhang (top oligo 5') <input type="text" class="mono" bind:value={layout.left_overhang} /></label>
        <label>Prefix <input type="text" class="mono" bind:value={layout.prefix} /></label>
        <label>Loop <input type="text" class="mono" bind:value={layout.loop_seq} /></label>
        <label>Suffix <input type="text" class="mono" bind:value={layout.suffix} /></label>
        <label>Right overhang (bottom oligo 5') <input type="text" class="mono" bind:value={layout.right_overhang} /></label>
        <button type="button" onclick={save}>Save layouts</button>
      {/if}
    </fieldset>
    <fieldset data-help-name="siRNA" data-help="The siRNA core (19–21 nt, sense). Point mutations (1-based positions in the sense arm, comma-separated) swap a base to weaken the hairpin without touching the antisense arm.">
      <legend>siRNA</legend>
      <label>Core (sense, 19–21 nt) <input type="text" class="mono" bind:value={core} size="30" /></label>
      <label>Point mutations (sense positions) <input type="text" bind:value={mutations} placeholder="e.g. 3, 10" /></label>
      <label>Name <input type="text" bind:value={name} /></label>
      <div class="row">
        <button type="button" class="primary" onclick={() => void make(false)} disabled={!layout}>Make oligos</button>
        <button type="button" onclick={() => void make(true)} disabled={!layout} data-help-name="Create insert window" data-help="Also puts the insert, with its cohesive ends and the sense / loop / antisense Features, in a Sequence Window for Build a Construct.">Create insert window</button>
      </div>
    </fieldset>
  </div>
  {#if result}
    <fieldset>
      <legend>Oligonucleotides</legend>
      <div class="oligo">Top 5'-<span class="mono sel">{result.top}</span>-3' <span class="hint">({result.top.length} nt)</span></div>
      <div class="oligo">Bottom 5'-<span class="mono sel">{result.bottom}</span>-3' <span class="hint">({result.bottom.length} nt)</span></div>
    </fieldset>
  {/if}
</ToolWindow>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .layout label,
  fieldset label {
    width: 100%;
    justify-content: space-between;
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .oligo {
    width: 100%;
    word-break: break-all;
  }
  .sel {
    -webkit-user-select: text;
    user-select: text;
  }
</style>
