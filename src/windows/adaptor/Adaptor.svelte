<script lang="ts">
  /** Create an Adaptor (⌘2): core dsDNA + cohesive extensions → oligos and an in-memory sequence. */
  import { message } from '@tauri-apps/plugin-dialog';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api, type Oligos } from '$lib/doc';

  let name = $state('Adaptor');
  let core = $state('');
  let leftKind = $state<'blunt' | '5' | '3'>('5');
  let leftBases = $state('GATC');
  let rightKind = $state<'blunt' | '5' | '3'>('5');
  let rightBases = $state('AATT');
  let result = $state<Oligos | null>(null);

  const len = (kind: string, bases: string) => (kind === 'blunt' ? 0 : kind === '5' ? Math.max(1, bases.length) : -Math.max(1, bases.length));

  async function make(open: boolean) {
    try {
      result = await api.makeAdaptor(name, core, { left_length: len(leftKind, leftBases), left_bases: leftBases, right_length: len(rightKind, rightBases), right_bases: rightBases }, open);
    } catch (e) {
      await message(String(e), { title: 'Create an Adaptor', kind: 'error' });
    }
  }
</script>

<ToolWindow title="Create an Adaptor" subtitle="the core is double-stranded; extensions are single-stranded">
  <fieldset data-help-name="Core" data-help="Enter the double-stranded core of the adaptor (top strand, 5'→3').">
    <legend>Core (double-stranded)</legend>
    <textarea rows="3" class="mono wide" bind:value={core} placeholder="e.g. GCTAGCGGATCC"></textarea>
  </fieldset>
  <div class="ends">
    <fieldset data-help-name="Left extension" data-help="Optional cohesive extension on the left: 5' (carried by the top strand) or 3' (carried by the bottom strand), read 5'→3' on its own strand.">
      <legend>Left (5') end</legend>
      <select bind:value={leftKind}><option value="blunt">Blunt</option><option value="5">5' overhang</option><option value="3">3' overhang</option></select>
      <input type="text" class="mono" bind:value={leftBases} disabled={leftKind === 'blunt'} placeholder="bases" />
    </fieldset>
    <fieldset data-help-name="Right extension" data-help="Optional cohesive extension on the right.">
      <legend>Right (3') end</legend>
      <select bind:value={rightKind}><option value="blunt">Blunt</option><option value="5">5' overhang</option><option value="3">3' overhang</option></select>
      <input type="text" class="mono" bind:value={rightBases} disabled={rightKind === 'blunt'} placeholder="bases" />
    </fieldset>
  </div>
  <div class="row">
    <label>Name <input type="text" bind:value={name} /></label>
    <button type="button" class="primary" onclick={() => void make(false)} data-help-name="Make" data-help="Computes the two oligonucleotides to order.">Make oligos</button>
    <button type="button" onclick={() => void make(true)} data-help-name="Create Sequence Window" data-help="Also puts the adaptor, with its cohesive ends, in a Sequence Window so that it can be used directly in Build a Construct ('in memory').">Create Sequence Window</button>
  </div>
  {#if result}
    <fieldset data-help-name="Oligos" data-help="The oligonucleotides to order, 5'→3'. Select and copy.">
      <legend>Oligonucleotides</legend>
      <div class="oligo"><span>Top 5'-</span><span class="mono sel">{result.top}</span><span>-3'</span> <span class="hint">({result.top.length} nt)</span></div>
      <div class="oligo"><span>Bottom 5'-</span><span class="mono sel">{result.bottom}</span><span>-3'</span> <span class="hint">({result.bottom.length} nt)</span></div>
    </fieldset>
  {/if}
</ToolWindow>

<style>
  .wide {
    width: 100%;
    box-sizing: border-box;
  }
  .ends {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
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
