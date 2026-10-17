<script lang="ts">
  /** Virtual Cutter (changelog 1.3): fragment count and sizes upon restriction, and a simulated gel. */
  import { onMount } from 'svelte';
  import ToolWindow from '$lib/components/ToolWindow.svelte';
  import { api, docIdFromUrl, onDocChanged, type Digest, type DocumentView, type EnzymeEntry } from '$lib/doc';

  const id = docIdFromUrl();
  let doc = $state<DocumentView | null>(null);
  let enzymes = $state<EnzymeEntry[]>([]);
  let chosen = $state<string[]>([]);
  let search = $state('');
  let digest = $state<Digest | null>(null);
  /** The original's four ladders first; the first is the default. */
  let ladders = $state<string[]>([]);
  let ladder = $state<string | null>(null);
  const cutters = $derived(enzymes.filter((e) => e.count > 0 && (!search || e.name.toLowerCase().startsWith(search.toLowerCase()))));

  async function cut() {
    digest = chosen.length ? await api.virtualCut(id, chosen, ladder) : null;
  }
  function toggle(name: string, on: boolean) {
    chosen = on ? [...new Set([...chosen, name])] : chosen.filter((n) => n !== name);
    void cut();
  }
  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      doc = await api.getDocument(id);
      ladders = await api.ladders();
      ladder = ladders[0] ?? null;
      enzymes = await api.visibleEnzymes(id);
      unlisten = await onDocChanged(id, async (d) => {
        doc = d;
        enzymes = await api.visibleEnzymes(id);
        await cut();
      });
    })();
    return () => unlisten?.();
  });
  const GEL_H = 360;
</script>

<ToolWindow title="Virtual Cutter" subtitle={doc ? `${doc.name} — ${doc.bases.length} bp, ${doc.circular ? 'circular' : 'linear'}` : ''}>
  <div class="grid">
    <div class="left">
      <input type="search" placeholder="enzyme" bind:value={search} data-help-name="Enzymes" data-help="Check the enzymes of the digest (only those cutting the sequence are listed; unique cutters in bold)." />
      <div class="scroll">
        <table>
          <tbody>
            {#each cutters as e (e.name)}
              <tr class:bold={e.count === 1}><td><input type="checkbox" checked={chosen.includes(e.name)} onchange={(ev) => toggle(e.name, ev.currentTarget.checked)} /></td><td>{e.name}</td><td class="mono">{e.site}</td><td class="num">{e.count}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
      <span class="hint">{chosen.join(' + ') || 'choose enzymes'}</span>
    </div>
    <div class="right">
      {#if digest}
        <div class="sizes" data-help-name="Fragments" data-help="Number and sizes of the fragments, largest first (cohesive overhangs are counted on both fragments).">
          <b>{digest.sizes.length} fragment{digest.sizes.length === 1 ? '' : 's'}:</b> {digest.sizes.join(', ')} bp
        </div>
        <label class="ladder-pick" data-help-name="Ladder" data-help="The DNA ladder of the left lane: the four ladders of the original (GeneRuler 1 kb Plus, Smart Ladder, NEB 1 kb, Invitrogen 1 kb plus; bright reference bands are drawn bolder) and a generic 1 kb ladder.">Ladder <select bind:value={ladder} onchange={() => void cut()}>{#each ladders as name (name)}<option value={name}>{name}</option>{/each}</select></label>
        <svg class="gel" viewBox="0 0 260 {GEL_H + 30}" data-help-name="Gel" data-help="Simulated 1 % agarose gel: the chosen ladder on the left, the digest on the right; the migration scale is calibrated on the ladder's smallest and largest bands.">
          <rect x="0" y="0" width="260" height={GEL_H + 30} fill="#1d2a35" />
          <text x="60" y="16" fill="#cfd8e3" font-size="9" text-anchor="middle">{digest.ladder_name}</text>
          <text x="190" y="16" fill="#cfd8e3" font-size="11" text-anchor="middle">digest</text>
          {#each digest.ladder as size, i (size)}
            {@const y = 30 + (digest.ladder_migrations[i] ?? 0) * (GEL_H - 20)}
            {@const bright = digest.ladder_bright[i] ?? false}
            <rect x="30" y={y - (bright ? 2.5 : 1.5)} width="60" height={bright ? 5 : 3} fill="#ffd7a3" opacity={bright ? 1 : 0.65} />
            <text x="26" y={y + 3} fill="#cfd8e3" font-size="9" text-anchor="end">{size}</text>
          {/each}
          {#each digest.sizes as size, i (i)}
            {@const y = 30 + (digest.migrations[i] ?? 0) * (GEL_H - 20)}
            <rect x="160" y={y - 1.5} width="60" height="3" fill="#ffe9c2" />
            <text x="226" y={y + 3} fill="#cfd8e3" font-size="9">{size}</text>
          {/each}
        </svg>
      {:else}
        <p class="hint">Check one or more enzymes to see the fragments and the gel.</p>
      {/if}
    </div>
  </div>
</ToolWindow>

<style>
  .ladder-pick { display: block; font-size: 12px; margin: 4px 0; }
  .grid {
    flex: 1;
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: 10px;
    min-height: 0;
  }
  .left,
  .right {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
  }
  .left input {
    font: inherit;
  }
  .bold {
    font-weight: 700;
  }
  .gel {
    width: 100%;
    max-height: 100%;
    border-radius: var(--radius);
  }
</style>
