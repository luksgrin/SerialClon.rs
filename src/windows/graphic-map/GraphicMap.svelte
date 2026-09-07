<script lang="ts">
  /**
   * The Graphic Map (User Manual §IV.C; reference/ui/06 and 07): circular or
   * linear map with Features and unique / double / particular sites, labels
   * you can drag, two-click fragment selection (green BEGIN, red END,
   * spacebar CW/CCW), Extract Fragment, save as PNG, print.
   */
  import { onMount } from 'svelte';
  import { Menu } from '@tauri-apps/api/menu';
  import { message } from '@tauri-apps/plugin-dialog';
  import { api, docIdFromUrl, onDocChanged, onMenu, type DocumentView, type EnzymeEntry, type MapSite, type SiteRefView } from '$lib/doc';

  const id = docIdFromUrl();
  const COLORS = { unique: '#f2e14c', double: '#f0a53a', particular: '#7cb3e8', begin: '#2ea043', end: '#d1242f' } as const;
  type Category = 'unique' | 'double' | 'particular';
  const PREVIOUS_KEY = 'gmap.previousParticular';

  let doc = $state<DocumentView | null>(null);
  let sites = $state<MapSite[]>([]);
  let enzymes = $state<EnzymeEntry[]>([]);
  let showFeatures = $state(true);
  let showUnique = $state(true);
  let showDouble = $state(false);
  let showParticular = $state(false);
  let particular = $state<string[]>([]);
  let chooser = $state(false);
  let chooserFilter = $state('');
  let width = $state(700);
  let height = $state(600);
  let offsets = $state<Record<string, { dx: number; dy: number }>>({});
  let begin = $state<string | null>(null);
  let end = $state<string | null>(null);
  let clockwise = $state(true);
  let selectAll = $state(false);
  let svgEl = $state<SVGSVGElement | undefined>();
  let container = $state<HTMLDivElement | undefined>();

  const keyOf = (s: MapSite) => `${s.name}@${s.position}${s.forward === false ? 'r' : ''}`;
  const length = $derived(doc?.bases.length ?? 0);
  const circular = $derived(doc?.circular ?? false);

  interface Placed {
    key: string;
    site: MapSite;
    category: Category;
    text: string;
    tick: { x1: number; y1: number; x2: number; y2: number };
    box: { x: number; y: number; w: number; h: number };
    leader: { x: number; y: number };
  }

  const shown = $derived.by(() => {
    const out: { site: MapSite; category: Category }[] = [];
    const isParticular = (n: string) => particular.includes(n);
    for (const s of sites) {
      if (showParticular && isParticular(s.name)) out.push({ site: s, category: 'particular' });
      else if (showUnique && s.count === 1) out.push({ site: s, category: 'unique' });
      else if (showDouble && s.count === 2) out.push({ site: s, category: 'double' });
    }
    return out;
  });

  // --- geometry -----------------------------------------------------------
  const TOP = 8;
  const CHAR = 6.6;
  const LABEL_H = 16;
  const center = $derived({ x: width / 2, y: TOP + (height - TOP) / 2 });
  const radius = $derived(Math.max(60, Math.min(width, height - TOP) / 2 - 150));
  const bar = $derived({ x0: 50, x1: width - 50, y: TOP + (height - TOP) * 0.62 });

  function angleOf(pos: number): number {
    return (pos / Math.max(1, length)) * Math.PI * 2 - Math.PI / 2;
  }
  function polar(r: number, a: number) {
    return { x: center.x + r * Math.cos(a), y: center.y + r * Math.sin(a) };
  }
  function xOf(pos: number): number {
    return bar.x0 + (pos / Math.max(1, length)) * (bar.x1 - bar.x0);
  }
  function labelText(s: MapSite): string {
    const arrow = s.forward === null ? '' : s.forward ? '>' : '<';
    return `${s.position} ${arrow}${s.name}`;
  }

  const placed = $derived.by((): Placed[] => {
    const items = shown.map(({ site, category }) => ({ site, category, key: keyOf(site), text: labelText(site) }));
    const result: Placed[] = [];
    if (circular) {
      const sides: { right: typeof items; left: typeof items } = { right: [], left: [] };
      for (const it of items) (Math.cos(angleOf(it.site.cut_top)) >= 0 ? sides.right : sides.left).push(it);
      for (const side of ['right', 'left'] as const) {
        const list = sides[side].map((it) => {
          const a = angleOf(it.site.cut_top);
          const anchor = polar(radius + 62, a);
          const w = it.text.length * CHAR + 10;
          return { it, a, w, x: side === 'right' ? anchor.x : anchor.x - w, y: anchor.y - LABEL_H / 2 };
        });
        list.sort((p, q) => p.y - q.y);
        const gap = LABEL_H + 3;
        for (let i = 1; i < list.length; i++) list[i]!.y = Math.max(list[i]!.y, list[i - 1]!.y + gap);
        const bottom = height - 10;
        for (let i = list.length - 1; i >= 0; i--) {
          const limit = i === list.length - 1 ? bottom - LABEL_H : list[i + 1]!.y - gap;
          list[i]!.y = Math.min(list[i]!.y, limit);
        }
        for (const p of list) {
          const o = offsets[p.it.key] ?? { dx: 0, dy: 0 };
          const box = { x: p.x + o.dx, y: Math.max(TOP, p.y) + o.dy, w: p.w, h: LABEL_H };
          const t1 = polar(radius, p.a);
          const t2 = polar(radius + 10, p.a);
          const leader = side === 'right' ? { x: box.x, y: box.y + LABEL_H / 2 } : { x: box.x + box.w, y: box.y + LABEL_H / 2 };
          result.push({ key: p.it.key, site: p.it.site, category: p.it.category, text: p.it.text, tick: { x1: t1.x, y1: t1.y, x2: t2.x, y2: t2.y }, box, leader });
        }
      }
    } else {
      const rows: { x: number; w: number }[][] = [];
      const sorted = [...items].sort((p, q) => p.site.cut_top - q.site.cut_top);
      for (const it of sorted) {
        const x = xOf(it.site.cut_top);
        const w = it.text.length * CHAR + 10;
        let left = Math.min(Math.max(bar.x0, x - w / 2), bar.x1 - w);
        let row = rows.findIndex((r) => r.every((b) => left + w + 4 <= b.x || b.x + b.w + 4 <= left));
        if (row < 0) {
          rows.push([]);
          row = rows.length - 1;
        }
        rows[row]!.push({ x: left, w });
        const o = offsets[it.key] ?? { dx: 0, dy: 0 };
        const y = bar.y - 34 - row * (LABEL_H + 4);
        const box = { x: left + o.dx, y: y + o.dy, w, h: LABEL_H };
        result.push({ key: it.key, site: it.site, category: it.category, text: it.text, tick: { x1: x, y1: bar.y - 6, x2: x, y2: bar.y - 16 }, box, leader: { x: box.x + w / 2, y: box.y + LABEL_H } });
        void left;
      }
    }
    return result;
  });

  interface FeatureShape {
    name: string;
    color: string;
    forward: boolean;
    path: string;
    label: { x: number; y: number };
    head: string;
  }
  const featureShapes = $derived.by((): FeatureShape[] => {
    if (!doc || !showFeatures || length === 0) return [];
    return doc.features
      .filter((f) => f.show)
      .map((f) => {
        const a = f.forward ? f.start : f.end;
        const b = f.forward ? f.end : f.start;
        const lo = a - 1;
        const span = a <= b ? b - a + 1 : length - a + 1 + b;
        if (circular) {
          const r = radius - 16;
          const a0 = angleOf(lo);
          const a1 = angleOf(lo + span);
          const p0 = polar(r, a0);
          const p1 = polar(r, a1);
          const large = span / length > 0.5 ? 1 : 0;
          const mid = polar(radius - 44, angleOf(lo + span / 2));
          const tip = f.forward ? p1 : p0;
          const dir = angleOf(lo + span) + (f.forward ? Math.PI / 2 : -Math.PI / 2);
          const headAt = f.forward ? angleOf(lo + span) : angleOf(lo);
          const hd = f.forward ? Math.PI / 2 : -Math.PI / 2;
          const h1 = polar(r + 7, headAt);
          const h2 = polar(r - 7, headAt);
          const h3 = polar(r, headAt + (f.forward ? 1 : -1) * (14 / r));
          void dir;
          void tip;
          void hd;
          return {
            name: f.name,
            color: f.color,
            forward: f.forward,
            path: `M ${p0.x} ${p0.y} A ${r} ${r} 0 ${large} 1 ${p1.x} ${p1.y}`,
            label: mid,
            head: f.arrow ? `M ${h1.x} ${h1.y} L ${h3.x} ${h3.y} L ${h2.x} ${h2.y} Z` : '',
          };
        }
        const x0 = xOf(lo);
        const x1 = xOf(Math.min(length, lo + span));
        const y = bar.y + 14;
        const h = 12;
        const head = f.arrow ? (f.forward ? `M ${x1} ${y} L ${x1 + 8} ${y + h / 2} L ${x1} ${y + h} Z` : `M ${x0} ${y} L ${x0 - 8} ${y + h / 2} L ${x0} ${y + h} Z`) : '';
        return { name: f.name, color: f.color, forward: f.forward, path: `M ${x0} ${y} H ${x1} V ${y + h} H ${x0} Z`, label: { x: (x0 + x1) / 2, y: y + h + 12 }, head };
      });
  });

  const selectionPath = $derived.by(() => {
    if (!doc) return '';
    const b = placed.find((p) => p.key === begin);
    const e = placed.find((p) => p.key === end);
    if (circular) {
      if (!b) return '';
      const from = b.site.cut_top;
      const to = e ? e.site.cut_top : b.site.cut_top;
      const [s, t] = clockwise ? [from, to] : [to, from];
      let span = (t - s + length) % length;
      if (span === 0) span = length;
      const p0 = polar(radius, angleOf(s));
      const p1 = polar(radius, angleOf(s + span - 0.0001));
      const large = span / length > 0.5 ? 1 : 0;
      return `M ${p0.x} ${p0.y} A ${radius} ${radius} 0 ${large} 1 ${p1.x} ${p1.y}`;
    }
    if (selectAll) return `M ${bar.x0} ${bar.y - 4} H ${bar.x1} V ${bar.y + 4} H ${bar.x0} Z`;
    if (!b || !e) return '';
    const x0 = xOf(Math.min(b.site.cut_top, e.site.cut_top));
    const x1 = xOf(Math.max(b.site.cut_top, e.site.cut_top));
    return `M ${x0} ${bar.y - 4} H ${x1} V ${bar.y + 4} H ${x0} Z`;
  });

  const hasSelection = $derived(!!begin || selectAll);
  const orientationLabel = $derived(circular ? (clockwise ? 'CW' : 'CCW') : clockwise ? 'Sense' : 'Antisense');

  // --- interaction --------------------------------------------------------
  let drag: { key: string; x: number; y: number; dx: number; dy: number; moved: boolean } | null = null;
  function onLabelDown(e: PointerEvent, key: string) {
    if (e.button !== 0) return;
    const o = offsets[key] ?? { dx: 0, dy: 0 };
    drag = { key, x: e.clientX, y: e.clientY, dx: o.dx, dy: o.dy, moved: false };
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    e.preventDefault();
  }
  function onPointerMove(e: PointerEvent) {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (Math.abs(dx) > 3 || Math.abs(dy) > 3) drag.moved = true;
    if (drag.moved) offsets = { ...offsets, [drag.key]: { dx: drag.dx + dx, dy: drag.dy + dy } };
  }
  function onPointerUp(e: PointerEvent) {
    if (!drag) return;
    const { key, moved } = drag;
    drag = null;
    if (moved) return;
    selectAll = false;
    if (e.shiftKey) end = key;
    else if (begin === key && end === null) begin = null;
    else begin = key;
  }
  function onLabelDblClick(key: string) {
    selectAll = false;
    end = key;
    if (begin === null) begin = key;
  }
  function clearSelection() {
    begin = null;
    end = null;
    selectAll = false;
  }
  function onKeyDown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement) return;
    if (e.key === ' ') {
      clockwise = !clockwise;
      e.preventDefault();
    } else if (e.key.toLowerCase() === 'a' && !e.metaKey && !e.ctrlKey && !circular) {
      selectAll = true;
      begin = null;
      end = null;
    } else if (e.key === 'Delete' || e.key === 'Backspace' || e.key === 'Escape') {
      clearSelection();
    }
  }

  function siteRef(key: string | null): SiteRefView | null {
    const p = placed.find((x) => x.key === key);
    return p ? { enzyme: p.site.name, position: p.site.position, cut_top: p.site.cut_top, cut_bottom: p.site.cut_bottom } : null;
  }
  async function extract() {
    if (!doc) return;
    try {
      if (selectAll && !circular) {
        await api.extractSelection(id, 0, length);
        return;
      }
      const b = siteRef(begin);
      const e = siteRef(end);
      if (!b) {
        await message('Click a site name box to choose the BEGIN site (green); double-click or shift-click another for the END site (red).', { title: 'Extract Fragment' });
        return;
      }
      if (!circular && !e) {
        await message('A linear sequence needs both a BEGIN and an END site (or press A to select the whole insert).', { title: 'Extract Fragment' });
        return;
      }
      await api.extractFragment(id, b, e, clockwise);
    } catch (err) {
      await message(String(err), { title: 'Extract Fragment', kind: 'error' });
    }
  }

  async function toggleTopology() {
    if (!doc) return;
    await api.edit(id, { op: 'set_topology', circular: !doc.circular });
  }

  async function savePng() {
    if (!svgEl || !doc) return;
    const clone = svgEl.cloneNode(true) as SVGSVGElement;
    clone.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
    const xml = new XMLSerializer().serializeToString(clone);
    const url = URL.createObjectURL(new Blob([xml], { type: 'image/svg+xml;charset=utf-8' }));
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error('cannot rasterise the map'));
      img.src = url;
    });
    const scale = 2;
    const canvas = document.createElement('canvas');
    canvas.width = width * scale;
    canvas.height = height * scale;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
    URL.revokeObjectURL(url);
    const data = canvas.toDataURL('image/png').split(',')[1] ?? '';
    await api.savePng(`Graphic Map of ${doc.name}`, data);
  }

  async function contextMenu() {
    if (!doc) return;
    const menu = await Menu.new({
      items: [
        { id: 'topo', text: doc.circular ? 'Linearize' : 'Circularize', action: () => void toggleTopology() },
        { id: 'png', text: 'Save Graphic Map As…', action: () => void savePng() },
        { id: 'print', text: 'Print…', action: () => window.print() },
        { item: 'Separator' },
        { id: 'extract', text: 'Extract Fragment', enabled: hasSelection, action: () => void extract() },
        { id: 'dir', text: circular ? (clockwise ? 'Select Counter-Clockwise' : 'Select Clockwise') : clockwise ? 'Select Antisense' : 'Select Sense', enabled: hasSelection, action: () => (clockwise = !clockwise) },
        { id: 'cancel', text: 'Cancel Selection', enabled: hasSelection, action: clearSelection },
        { item: 'Separator' },
        { id: 'parent', text: 'Show Sequence Window', action: () => void api.showParent(id) },
      ],
    });
    await menu.popup();
  }

  function handleMenu(cmd: string) {
    switch (cmd) {
      case 'file.save_as':
        void savePng();
        break;
      case 'file.print':
        window.print();
        break;
      case 'generate.extract':
        void extract();
        break;
      case 'seq.topology':
        void toggleTopology();
        break;
    }
  }

  // --- chooser ------------------------------------------------------------
  function toggleParticular(name: string, on: boolean) {
    particular = on ? [...new Set([...particular, name])] : particular.filter((n) => n !== name);
    if (particular.length) showParticular = true;
  }
  function usePrevious() {
    try {
      particular = JSON.parse(localStorage.getItem(PREVIOUS_KEY) ?? '[]');
      if (particular.length) showParticular = true;
    } catch {
      /* none */
    }
  }
  function closeChooser() {
    chooser = false;
    try {
      if (particular.length) localStorage.setItem(PREVIOUS_KEY, JSON.stringify(particular));
    } catch {
      /* private mode */
    }
  }
  const chooserList = $derived(enzymes.filter((e) => !chooserFilter || e.name.toLowerCase().startsWith(chooserFilter.toLowerCase())));

  async function load() {
    const d = await api.mapData(id);
    doc = d.doc;
    sites = d.sites;
    enzymes = d.enzymes;
    if (begin && !sites.some((s) => keyOf(s) === begin)) clearSelection();
  }

  onMount(() => {
    const unlisteners: (() => void)[] = [];
    (async () => {
      try {
        await load();
      } catch {
        return;
      }
      unlisteners.push(await onDocChanged(id, () => void load()));
      unlisteners.push(await onMenu(handleMenu));
      void api.noteFocus(id);
    })();
    const ro = new ResizeObserver(() => {
      if (container) {
        width = container.clientWidth;
        height = container.clientHeight;
      }
    });
    if (container) ro.observe(container);
    window.addEventListener('focus', () => void api.noteFocus(id));
    return () => {
      unlisteners.forEach((u) => u());
      ro.disconnect();
    };
  });
</script>

<svelte:window onkeydown={onKeyDown} onpointermove={onPointerMove} onpointerup={onPointerUp} />

<div class="gmap" role="application" aria-label="Graphic Map">
  {#if doc}
    <header>
      <h1 data-help-name="Header" data-help="The name and size of the sequence.">{doc.name} - {length} nt</h1>
      <div class="controls no-print">
        <span>Show :</span>
        <label data-help-name="Features" data-help="Show Sequence features and annotations."><input type="checkbox" bind:checked={showFeatures} /> Features</label>
        <label data-help-name="Unique Sites" data-help="Display the restriction sites found once (yellow)."><input type="checkbox" bind:checked={showUnique} /> Unique Sites</label>
        <label data-help-name="Double Site" data-help="Display the restriction sites found twice (orange)."><input type="checkbox" bind:checked={showDouble} /> Double Site</label>
        <label data-help-name="Particular sites" data-help="Display the set of sites chosen with the Choose site button (blue)."><input type="checkbox" bind:checked={showParticular} /> Particular sites</label>
        <button type="button" class="choose" onclick={() => (chooser = !chooser)} data-help-name="Choose site" data-help="Pops up a window where the particular restriction sites to display can be selected. Unique sites are in bold, absent sites in italic.">Choose site</button>
        <span class="orientation" data-help-name="Direction" data-help="Direction of the selection between the BEGIN (green) and END (red) sites. Press the spacebar to toggle.">{hasSelection ? orientationLabel : ''}</span>
        <button type="button" class="house" title="Show Sequence Window" onclick={() => void api.showParent(id)} data-help-name="Sequence Window" data-help="Brings the parent Sequence Window to the front (Alt-click on the map does the same).">⌂</button>
      </div>
    </header>

    {#if chooser}
      <div class="chooser no-print" data-help-name="Choose site" data-help="Select sites by clicking their checkbox. Double-click 'Unselect all' to clear; 'Use previous selection' re-uses the list of a precedent Graphic Map. Click Close when done.">
        <div class="chooser-head">
          <input type="text" placeholder="type to jump…" bind:value={chooserFilter} />
          <button type="button" ondblclick={() => (particular = [])} onclick={() => {}}>Unselect all</button>
          <button type="button" onclick={usePrevious}>Use previous selection</button>
          <button type="button" onclick={closeChooser}>Close</button>
        </div>
        <div class="chooser-list">
          {#each chooserList as e (e.name)}
            <label class:unique={e.count === 1} class:absent={e.count === 0}>
              <input type="checkbox" checked={particular.includes(e.name)} onchange={(ev) => toggleParticular(e.name, ev.currentTarget.checked)} />
              {e.name} <span class="site">{e.site}</span> <span class="count">{e.count}</span>
            </label>
          {/each}
        </div>
      </div>
    {/if}

    <div class="map" bind:this={container} oncontextmenu={(e) => { e.preventDefault(); void contextMenu(); }} onclick={(e) => { if (e.altKey) void api.showParent(id); }} role="presentation" data-help-name="Map" data-help="Click a site name box to select the BEGIN site (green), double-click or shift-click another for the END site (red); spacebar toggles CW/CCW (or Sense/Antisense); A selects the whole linear insert; Delete cancels. Drag name boxes to arrange the map before printing. Right-click for the contextual menu.">
      <svg bind:this={svgEl} {width} {height} viewBox="0 0 {width} {height}" font-family="-apple-system, Segoe UI, system-ui, sans-serif" font-size="11">
        <rect x="0" y="0" {width} {height} fill="#ffffff" />
        {#if circular}
          <circle cx={center.x} cy={center.y} r={radius} fill="none" stroke="#9ec4ee" stroke-width="14" />
          <circle cx={center.x} cy={center.y} r={radius} fill="none" stroke="#5a8fd0" stroke-width="2" />
          <text x={center.x} y={center.y} text-anchor="middle" font-size="12" fill="#222">{doc.name} - {length} nt</text>
          <line x1={center.x - 6} y1={center.y + 14} x2={center.x + 6} y2={center.y + 14} stroke="#222" />
          <line x1={center.x} y1={center.y + 8} x2={center.x} y2={center.y + 20} stroke="#222" />
        {:else}
          <line x1={bar.x0} y1={bar.y} x2={bar.x1} y2={bar.y} stroke="#9ec4ee" stroke-width="12" stroke-linecap="round" />
          <line x1={bar.x0} y1={bar.y} x2={bar.x1} y2={bar.y} stroke="#5a8fd0" stroke-width="2" />
          {#if doc.ends.left_length !== 0}
            <line x1={bar.x0} y1={bar.y} x2={bar.x0 + 18} y2={bar.y} stroke="#d1242f" stroke-width="12" stroke-linecap="round" />
            <rect x={bar.x0 - 44} y={bar.y - 32} width={doc.ends.left_bases.length * CHAR + 44} height="14" fill="#e6e6e6" stroke="#999" />
            <text x={bar.x0 - 40} y={bar.y - 21} font-size="10">{doc.ends.left_length > 0 ? "5'" : "3'"} : ({Math.abs(doc.ends.left_length)}){doc.ends.left_bases}</text>
          {/if}
          {#if doc.ends.right_length !== 0}
            <line x1={bar.x1 - 18} y1={bar.y} x2={bar.x1} y2={bar.y} stroke="#d1242f" stroke-width="12" stroke-linecap="round" />
            <rect x={bar.x1 - doc.ends.right_bases.length * CHAR - 4} y={bar.y - 32} width={doc.ends.right_bases.length * CHAR + 48} height="14" fill="#e6e6e6" stroke="#999" />
            <text x={bar.x1 - doc.ends.right_bases.length * CHAR} y={bar.y - 21} font-size="10">{doc.ends.right_length > 0 ? "5'" : "3'"} : ({Math.abs(doc.ends.right_length)}){doc.ends.right_bases}</text>
          {/if}
          <text x={bar.x0} y={bar.y + 46} font-size="12" fill="#222">{doc.name} - {length} nt</text>
        {/if}

        {#if selectionPath}
          <path d={selectionPath} fill={circular ? 'none' : 'rgba(46,160,67,0.35)'} stroke={circular ? COLORS.begin : 'none'} stroke-width="14" stroke-opacity="0.55" />
        {/if}

        {#each featureShapes as f (f.name + f.path)}
          <path d={f.path} fill={circular ? 'none' : f.color} stroke={f.color} stroke-width={circular ? 10 : 1} />
          {#if f.head}<path d={f.head} fill={f.color} />{/if}
          <rect x={f.label.x - (f.name.length * CHAR) / 2 - 4} y={f.label.y - 9} width={f.name.length * CHAR + 8} height="14" fill="#ffffff" stroke={f.color} rx="2" />
          <text x={f.label.x} y={f.label.y + 2} text-anchor="middle" font-size="10" fill="#222">{f.name}</text>
        {/each}

        {#each placed as p (p.key)}
          {@const fill = p.key === begin ? COLORS.begin : p.key === end ? COLORS.end : COLORS[p.category]}
          {@const dark = p.key === begin || p.key === end}
          <line x1={p.tick.x1} y1={p.tick.y1} x2={p.tick.x2} y2={p.tick.y2} stroke="#222" stroke-width="1.5" />
          <line x1={p.tick.x2} y1={p.tick.y2} x2={p.leader.x} y2={p.leader.y} stroke="#777" stroke-width="0.8" />
          <g class="label" onpointerdown={(e) => onLabelDown(e, p.key)} ondblclick={() => onLabelDblClick(p.key)} role="button" tabindex="-1" style="cursor: grab">
            <rect x={p.box.x} y={p.box.y} width={p.box.w} height={p.box.h} {fill} stroke={dark ? fill : '#555'} rx="2" />
            <text x={p.box.x + 5} y={p.box.y + 12} font-size="11" fill={dark ? '#fff' : '#111'} font-family="ui-monospace, Menlo, monospace">{p.text}</text>
          </g>
        {/each}
      </svg>
    </div>
  {/if}
</div>

<style>
  .gmap {
    display: flex;
    flex-direction: column;
    height: 100vh;
    box-sizing: border-box;
  }
  header {
    padding: 6px 10px 4px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    margin: 0 0 4px;
    font-size: 18px;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .controls label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .choose {
    padding: 2px 16px;
    border-radius: 12px;
  }
  .orientation {
    margin-left: auto;
    font-weight: 700;
    color: var(--select-begin);
  }
  .house {
    width: 26px;
  }
  .chooser {
    position: absolute;
    top: 64px;
    right: 12px;
    z-index: 2;
    width: 320px;
    max-height: 60vh;
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
    flex-wrap: wrap;
  }
  .chooser-head input {
    flex: 1;
    min-width: 80px;
    font: inherit;
  }
  .chooser-list {
    overflow: auto;
    padding: 4px 6px;
  }
  .chooser-list label {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 1px 0;
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
  .count {
    margin-left: auto;
    color: var(--fg-muted);
    font-family: var(--font-mono);
  }
  .map {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: #fff;
  }
  svg {
    display: block;
    -webkit-user-select: none;
    user-select: none;
  }
  @media print {
    .no-print,
    .chooser {
      display: none !important;
    }
    .gmap {
      height: auto;
    }
  }
</style>
