<script lang="ts">
  /**
   * The Graphic Map (User Manual §IV.C; reference/ui/06 and 07): circular or
   * linear map with Features and unique / double / particular sites, labels
   * you can drag, two-click fragment selection (green BEGIN, red END,
   * spacebar CW/CCW), Extract Fragment, save as PNG, print.
   */
  import { onMount } from 'svelte';
  import { Menu } from '@tauri-apps/api/menu';
  import { emitTo } from '@tauri-apps/api/event';
  import { message } from '@tauri-apps/plugin-dialog';
  import { prefs, printPage } from '$lib/prefs';
  import { api, docIdFromUrl, onDocChanged, onMenu, requestSelect, type ConstructSelection, type DocumentView, type EnzymeEntry, type MapSite, type SiteRefView } from '$lib/doc';
  import { assignTracks, formatCoord, packAngular, packRows, tickStep, type Span } from './layout';

  const id = docIdFromUrl();
  const COLORS = $derived({ unique: $prefs.color_unique, double: $prefs.color_double, particular: $prefs.color_particular, begin: $prefs.color_begin, end: $prefs.color_end });
  const RE_FONT = $derived($prefs.gmap_re_font_size);
  const FEATURE_FONT = $derived($prefs.gmap_feature_font_size);
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
  const LABEL_H = 16;
  const TAU = Math.PI * 2;
  /** Monospace label width per character at the RE font size. */
  const reChar = $derived(RE_FONT * 0.62);
  /** Proportional label width per character at the feature font size. */
  const featChar = $derived(FEATURE_FONT * 0.6);
  const siteLabelW = (text: string) => text.length * reChar + 10;
  const featLabelW = (name: string) => name.length * featChar + 10;

  function labelText(s: MapSite): string {
    const arrow = s.forward === null ? '' : s.forward ? '>' : '<';
    return `${s.position} ${arrow}${s.name}`;
  }

  /** Features to draw, with their spans (0-based, half-open). */
  const visibleFeatures = $derived.by(() => {
    if (!doc || !showFeatures || length === 0) return [] as { f: DocumentView['features'][number]; i: number; span: Span; len: number }[];
    return doc.features
      .map((f, i) => ({ f, i }))
      .filter(({ f }) => f.show)
      .map(({ f, i }) => {
        const a = f.forward ? f.start : f.end;
        const b = f.forward ? f.end : f.start;
        const lo = a - 1;
        const wraps = a > b;
        const len = wraps ? length - a + 1 + b : b - a + 1;
        return { f, i, span: { lo, hi: wraps ? b : lo + len, wraps }, len };
      });
  });
  const tracks = $derived(assignTracks(visibleFeatures.map((v) => v.span), Math.max(1, length)));
  const trackCount = $derived(tracks.reduce((m, t) => Math.max(m, t + 1), 0));

  /** The widest site label decides how much room the ring leaves outside. */
  const maxSiteW = $derived(shown.reduce((m, { site }) => Math.max(m, siteLabelW(labelText(site))), 0));

  const center = $derived({ x: width / 2, y: TOP + (height - TOP) / 2 });
  const radius = $derived(Math.max(60, Math.min(width / 2 - 44 - maxSiteW, (height - TOP) / 2 - 34 - 2 * LABEL_H - 8)));
  /** Ring geometry: ticks inside the backbone, feature tracks further in. */
  const TRACK_GAP = 30;
  const trackWidth = $derived(trackCount === 0 ? 12 : Math.max(6, Math.min(12, (radius - 48) / trackCount)));
  const trackRadius = (t: number) => radius - TRACK_GAP - t * trackWidth - trackWidth / 2;
  const LABEL_RING = 34;

  function angleOf(pos: number): number {
    return (pos / Math.max(1, length)) * TAU - Math.PI / 2;
  }
  function polar(r: number, a: number) {
    return { x: center.x + r * Math.cos(a), y: center.y + r * Math.sin(a) };
  }
  function arcPath(r: number, a0: number, a1: number, reverse = false): string {
    // a1 > a0, clockwise on screen. Full turns are split just short.
    const sweep = Math.min(a1 - a0, TAU - 1e-4);
    const large = sweep > Math.PI ? 1 : 0;
    if (!reverse) {
      const p0 = polar(r, a0);
      const p1 = polar(r, a0 + sweep);
      return `M ${p0.x.toFixed(2)} ${p0.y.toFixed(2)} A ${r} ${r} 0 ${large} 1 ${p1.x.toFixed(2)} ${p1.y.toFixed(2)}`;
    }
    const p0 = polar(r, a0 + sweep);
    const p1 = polar(r, a0);
    return `M ${p0.x.toFixed(2)} ${p0.y.toFixed(2)} A ${r} ${r} 0 ${large} 0 ${p1.x.toFixed(2)} ${p1.y.toFixed(2)}`;
  }

  // Linear geometry: site rows above the bar, ticks and feature rows below.
  const bar = $derived({ x0: 60, x1: Math.max(120, width - 60) });
  function xOf(pos: number): number {
    return bar.x0 + (pos / Math.max(1, length)) * (bar.x1 - bar.x0);
  }
  const ticks = $derived.by(() => {
    const { major, minor } = tickStep(Math.max(1, length), circular ? 12 : Math.max(4, Math.floor((bar.x1 - bar.x0) / 70)));
    const out: { pos: number; major: boolean }[] = [];
    for (let p = minor; p < length; p += minor) out.push({ pos: p, major: p % major === 0 });
    return out;
  });

  // ---- labels outside the map (sites, and features whose name does not fit) ----
  interface Placed {
    key: string;
    kind: 'site' | 'feature';
    site?: MapSite;
    category?: Category;
    feature?: { lo: number; hi: number; color: string };
    text: string;
    /** Leader from the anchor on the map to the box. */
    leader: { x1: number; y1: number; x2: number; y2: number };
    /** A short mark on the ring / bar at the site. */
    tick?: { x1: number; y1: number; x2: number; y2: number };
    box: { x: number; y: number; w: number; h: number };
  }

  interface FeatureShape {
    key: string;
    name: string;
    color: string;
    forward: boolean;
    path: string;
    /** Arrowhead polygon (circular). */
    head: string;
    /** Arc for the label's textPath, when the name fits along the feature. */
    textPath: string;
    lo: number;
    hi: number;
    track: number;
    /** Linear: the box, for the inside label. */
    box?: { x: number; y: number; w: number; h: number };
    /** Name drawn inside/along the feature (else it is in `placed`). */
    inside: boolean;
    strokeWidth: number;
  }

  function darker(hex: string): string {
    const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
    if (!m) return '#333';
    const n = parseInt(m[1]!, 16);
    const f = (c: number) => Math.max(0, Math.round(c * 0.55)).toString(16).padStart(2, '0');
    return `#${f(n >> 16)}${f((n >> 8) & 255)}${f(n & 255)}`;
  }

  type Cand = { key: string; kind: 'site' | 'feature'; text: string; w: number; angle: number; anchorR: number; site?: MapSite; category?: Category; feature?: { lo: number; hi: number; color: string } };

  /** Circular layout: features as arcs on tracks, labels packed around the ring. */
  const circularLayout = $derived.by((): { features: FeatureShape[]; labels: Placed[] } => {
    if (!circular) return { features: [], labels: [] };
    const features: FeatureShape[] = [];
    const cands: Cand[] = shown.map(({ site, category }) => ({ key: keyOf(site), kind: 'site' as const, text: labelText(site), w: siteLabelW(labelText(site)), angle: angleOf(site.cut_top), anchorR: radius, site, category }));
    visibleFeatures.forEach(({ f, i, span, len }, k) => {
      const t = tracks[k]!;
      const r = trackRadius(t);
      const a0 = angleOf(span.lo);
      const a1 = a0 + (len / length) * TAU;
      const arcPx = r * (a1 - a0);
      const key = `f:${i}`;
      const mid = (a0 + a1) / 2;
      const nameW = f.name.length * featChar;
      const inside = arcPx >= nameW + 8 && trackWidth >= 9;
      const small = arcPx < 4;
      let path: string;
      let head = '';
      if (small) {
        const p0 = polar(r - trackWidth / 2, mid);
        const p1 = polar(r + trackWidth / 2, mid);
        path = `M ${p0.x.toFixed(2)} ${p0.y.toFixed(2)} L ${p1.x.toFixed(2)} ${p1.y.toFixed(2)}`;
      } else if (f.arrow && arcPx > 6) {
        const headLen = Math.min(10, arcPx * 0.6);
        const dh = headLen / r;
        const [b0, b1] = f.forward ? [a0, a1 - dh] : [a0 + dh, a1];
        path = arcPath(r, b0, b1);
        const tipA = f.forward ? a1 : a0;
        const baseA = f.forward ? a1 - dh : a0 + dh;
        const tip = polar(r, tipA);
        const o = polar(r + trackWidth / 2 + 1, baseA);
        const n = polar(r - trackWidth / 2 - 1, baseA);
        head = `M ${o.x.toFixed(2)} ${o.y.toFixed(2)} L ${tip.x.toFixed(2)} ${tip.y.toFixed(2)} L ${n.x.toFixed(2)} ${n.y.toFixed(2)} Z`;
      } else {
        path = arcPath(r, a0, a1);
      }
      // Upright text: the path runs the other way on the lower half.
      const textPath = inside ? arcPath(r, a0, a1, Math.sin(mid) > 0) : '';
      features.push({ key, name: f.name, color: f.color, forward: f.forward, path, head, textPath, lo: span.lo, hi: span.hi, track: t, inside, strokeWidth: small ? 1.5 : Math.max(4, trackWidth - 3) });
      if (!inside) cands.push({ key, kind: 'feature', text: f.name, w: featLabelW(f.name), angle: mid, anchorR: r + trackWidth / 2, feature: { lo: span.lo, hi: span.hi, color: f.color } });
    });

    const labelR = radius + LABEL_RING;
    // A horizontal box's extent along the circle depends on where it sits:
    // stacked at the sides, side by side at the top and bottom.
    const sizes = cands.map((c) => (LABEL_H * Math.abs(Math.cos(c.angle)) + c.w * Math.abs(Math.sin(c.angle)) + 4) / labelR);
    const angles = packAngular(cands.map((c) => c.angle), sizes, 3 / labelR);
    const labels: Placed[] = cands.map((c, k) => {
      const a = angles[k]!;
      const anchor = polar(labelR, a);
      const cos = Math.cos(a);
      const sin = Math.sin(a);
      let x: number;
      let y: number;
      if (cos > 0.25) {
        x = anchor.x;
        y = anchor.y - LABEL_H / 2;
      } else if (cos < -0.25) {
        x = anchor.x - c.w;
        y = anchor.y - LABEL_H / 2;
      } else {
        x = anchor.x - c.w / 2;
        y = sin < 0 ? anchor.y - LABEL_H : anchor.y;
      }
      const o = offsets[c.key] ?? { dx: 0, dy: 0 };
      const box = { x: x + o.dx, y: y + o.dy, w: c.w, h: LABEL_H };
      const bx = Math.max(box.x, Math.min(box.x + box.w, anchor.x + o.dx));
      const by = Math.max(box.y, Math.min(box.y + box.h, anchor.y + o.dy));
      const t0 = polar(c.anchorR, c.angle);
      const from = polar(c.anchorR + (c.kind === 'site' ? 10 : 2), c.angle);
      return {
        key: c.key,
        kind: c.kind,
        site: c.site,
        category: c.category,
        feature: c.feature,
        text: c.text,
        leader: { x1: from.x, y1: from.y, x2: bx, y2: by },
        tick: c.kind === 'site' ? { x1: t0.x, y1: t0.y, x2: from.x, y2: from.y } : undefined,
        box,
      };
    });
    return { features, labels };
  });

  /** Linear layout: site rows above the bar, feature rows below, both packed. */
  const linearLayout = $derived.by((): { features: FeatureShape[]; labels: Placed[]; barY: number; total: number } => {
    if (circular) return { features: [], labels: [], barY: 0, total: height };
    const items = shown.map(({ site, category }) => {
      const text = labelText(site);
      const w = siteLabelW(text);
      const x = xOf(site.cut_top);
      const left = Math.min(Math.max(bar.x0 - 20, x - w / 2), bar.x1 + 20 - w);
      return { site, category, key: keyOf(site), text, w, x, left };
    });
    const rows = packRows(items.map((it) => ({ x: it.left, w: it.w })), 4);
    const rowCount = rows.reduce((m, r) => Math.max(m, r + 1), 0);
    const endsRoom = doc && (doc.ends.left_length !== 0 || doc.ends.right_length !== 0) ? 22 : 0;
    const barY = TOP + 10 + rowCount * (LABEL_H + 4) + 24 + endsRoom;
    const labels: Placed[] = items.map((it, k) => {
      const o = offsets[it.key] ?? { dx: 0, dy: 0 };
      const y = barY - 30 - endsRoom - rows[k]! * (LABEL_H + 4);
      const box = { x: it.left + o.dx, y: y + o.dy, w: it.w, h: LABEL_H };
      return {
        key: it.key,
        kind: 'site' as const,
        site: it.site,
        category: it.category,
        text: it.text,
        tick: { x1: it.x, y1: barY - 6, x2: it.x, y2: barY - 16 },
        leader: { x1: it.x, y1: barY - 16, x2: Math.max(box.x, Math.min(box.x + box.w, it.x + o.dx)), y2: box.y + box.h },
        box,
      };
    });
    const ROW_H = 18;
    const BOX_H = 12;
    const top = barY + 34;
    const features: FeatureShape[] = [];
    const outside: { key: string; text: string; x: number; y: number; color: string; lo: number; hi: number }[] = [];
    visibleFeatures.forEach(({ f, i, span }, k) => {
      const t = tracks[k]!;
      const y = top + t * ROW_H;
      const segs: [number, number][] = span.wraps ? [[span.lo, length], [0, span.hi]] : [[span.lo, span.hi]];
      const key = `f:${i}`;
      let d = '';
      segs.forEach(([lo, hi], si) => {
        const x0 = xOf(lo);
        const x1 = Math.max(xOf(hi), x0 + 1.5);
        const w = x1 - x0;
        const headW = Math.min(8, w * 0.6);
        const isTip = f.forward ? si === segs.length - 1 : si === 0;
        if (f.arrow && isTip && w > 3) {
          d += f.forward
            ? ` M ${x0.toFixed(2)} ${y} H ${(x1 - headW).toFixed(2)} L ${x1.toFixed(2)} ${y + BOX_H / 2} L ${(x1 - headW).toFixed(2)} ${y + BOX_H} H ${x0.toFixed(2)} Z`
            : ` M ${x1.toFixed(2)} ${y} H ${(x0 + headW).toFixed(2)} L ${x0.toFixed(2)} ${y + BOX_H / 2} L ${(x0 + headW).toFixed(2)} ${y + BOX_H} H ${x1.toFixed(2)} Z`;
        } else {
          d += ` M ${x0.toFixed(2)} ${y} H ${x1.toFixed(2)} V ${y + BOX_H} H ${x0.toFixed(2)} Z`;
        }
      });
      const [lo0, hi0] = segs[0]!;
      const x0 = xOf(lo0);
      const x1 = xOf(hi0);
      const inside = x1 - x0 >= f.name.length * featChar + 8;
      features.push({ key, name: f.name, color: f.color, forward: f.forward, path: d.trim(), head: '', textPath: '', lo: span.lo, hi: span.hi, track: t, box: { x: x0, y, w: x1 - x0, h: BOX_H }, inside, strokeWidth: 1 });
      if (!inside) outside.push({ key, text: f.name, x: (x0 + x1) / 2, y: y + BOX_H, color: f.color, lo: span.lo, hi: span.hi });
    });
    const labelTop = top + trackCount * ROW_H + 8;
    const boxes = outside.map((o) => {
      const w = featLabelW(o.text);
      return { w, x: Math.min(Math.max(bar.x0 - 20, o.x - w / 2), bar.x1 + 20 - w) };
    });
    const lrows = packRows(boxes, 4);
    outside.forEach((o, k) => {
      const oo = offsets[o.key] ?? { dx: 0, dy: 0 };
      const bx = boxes[k]!;
      const y = labelTop + lrows[k]! * (LABEL_H + 4);
      const box = { x: bx.x + oo.dx, y: y + oo.dy, w: bx.w, h: LABEL_H };
      labels.push({
        key: o.key,
        kind: 'feature',
        feature: { lo: o.lo, hi: o.hi, color: o.color },
        text: o.text,
        leader: { x1: o.x, y1: o.y, x2: Math.max(box.x, Math.min(box.x + box.w, o.x + oo.dx)), y2: box.y },
        box,
      });
    });
    const lrowCount = lrows.reduce((m, r) => Math.max(m, r + 1), 0);
    const total = Math.max(height, labelTop + lrowCount * (LABEL_H + 4) + 40);
    return { features, labels, barY, total };
  });

  const featureShapes = $derived(circular ? circularLayout.features : linearLayout.features);
  const placed = $derived(circular ? circularLayout.labels : linearLayout.labels);
  const barY = $derived(linearLayout.barY);
  const svgHeight = $derived(circular ? height : linearLayout.total);

  const selectionPath = $derived.by(() => {
    if (!doc) return '';
    const b = placed.find((p) => p.key === begin);
    const e = placed.find((p) => p.key === end);
    if (circular) {
      if (!b?.site) return '';
      const from = b.site.cut_top;
      const to = e?.site ? e.site.cut_top : b.site.cut_top;
      const [s, t] = clockwise ? [from, to] : [to, from];
      let span = (t - s + length) % length;
      if (span === 0) span = length;
      return arcPath(radius, angleOf(s), angleOf(s) + (span / length) * TAU);
    }
    if (selectAll) return `M ${bar.x0} ${barY - 5} H ${bar.x1} V ${barY + 5} H ${bar.x0} Z`;
    if (!b?.site || !e?.site) return '';
    const x0 = xOf(Math.min(b.site.cut_top, e.site.cut_top));
    const x1 = xOf(Math.max(b.site.cut_top, e.site.cut_top));
    return `M ${x0} ${barY - 5} H ${x1} V ${barY + 5} H ${x0} Z`;
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
    if (moved || key.startsWith('f:')) return;
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
    return p?.site ? { enzyme: p.site.name, position: p.site.position, cut_top: p.site.cut_top, cut_bottom: p.site.cut_bottom } : null;
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

  async function makeFeature() {
    const b = placed.find((x) => x.key === begin);
    const e = placed.find((x) => x.key === end) ?? b;
    if (!b?.site || !e?.site) {
      await message('Choose a BEGIN site (click) and an END site (double-click or shift-click) first.', { title: 'Make New Feature' });
      return;
    }
    await api.newFeatureFromSites(id, siteRef(b.key)!, siteRef(e.key)!, b.site.length, e.site.length, clockwise);
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
    canvas.height = svgHeight * scale;
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
        { id: 'print', text: 'Print…', action: () => void printPage() },
        { item: 'Separator' },
        { id: 'extract', text: 'Extract Fragment', enabled: hasSelection, action: () => void extract() },
        { id: 'feature', text: 'Make New Feature', enabled: !!begin, action: () => void makeFeature() },
        {
          id: 'construct',
          text: 'Use Selection in Build a Construct',
          enabled: !!begin || selectAll,
          action: () =>
            void emitTo('construct', 'construct:selection', {
              doc: id,
              name: doc?.name ?? '',
              begin: selectAll ? null : siteRef(begin),
              end: selectAll ? null : siteRef(end),
              clockwise,
            } satisfies ConstructSelection),
        },
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
        void printPage();
        break;
      case 'generate.extract':
        void extract();
        break;
      case 'seq.topology':
        void toggleTopology();
        break;
      case 'features.from_selection':
        void makeFeature();
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
        <button type="button" class="topo" onclick={toggleTopology} data-help-name="Topology" data-help="Draws the map as a circle or as a bar. Sequences opened from FASTA or text arrive linear; this is the same as Linearize / Circularize in the contextual menu.">{circular ? 'Circular' : 'Linear'} ▸ {circular ? 'linear' : 'circular'}</button>
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
      <svg bind:this={svgEl} {width} height={svgHeight} viewBox="0 0 {width} {svgHeight}" font-family="-apple-system, Segoe UI, system-ui, sans-serif" font-size="11">
        <rect x="0" y="0" {width} height={svgHeight} fill="#ffffff" />
        <defs>
          {#each featureShapes as f (f.key)}
            {#if f.textPath}<path id="tp-{f.key}" d={f.textPath} />{/if}
          {/each}
        </defs>

        {#if circular}
          <!-- backbone ring, ticks and coordinates -->
          <circle cx={center.x} cy={center.y} r={radius} fill="none" stroke="#dfe8f3" stroke-width="10" />
          <circle cx={center.x} cy={center.y} r={radius} fill="none" stroke="#5a8fd0" stroke-width="2" />
          {#each ticks as t (t.pos)}
            {@const a = angleOf(t.pos)}
            {@const p0 = polar(radius - 1, a)}
            {@const p1 = polar(radius - (t.major ? 9 : 5), a)}
            <line x1={p0.x} y1={p0.y} x2={p1.x} y2={p1.y} stroke="#5a8fd0" stroke-width={t.major ? 1.5 : 0.8} />
            {#if t.major}
              {@const p = polar(radius - 19, a)}
              <text x={p.x} y={p.y} text-anchor="middle" dominant-baseline="middle" font-size="9" fill="#555">{formatCoord(t.pos)}</text>
            {/if}
          {/each}
          {@const origin0 = polar(radius + 6, -Math.PI / 2)}
          {@const origin1 = polar(radius - 10, -Math.PI / 2)}
          <line x1={origin0.x} y1={origin0.y} x2={origin1.x} y2={origin1.y} stroke="#222" stroke-width="2" />
          <text x={center.x} y={center.y - 4} text-anchor="middle" font-size="13" font-weight="600" fill="#222">{doc.name}</text>
          <text x={center.x} y={center.y + 12} text-anchor="middle" font-size="11" fill="#555">{formatCoord(length)} bp</text>
        {:else}
          <!-- bar, ticks and coordinates -->
          <line x1={bar.x0} y1={barY} x2={bar.x1} y2={barY} stroke="#dfe8f3" stroke-width="12" stroke-linecap="round" />
          <line x1={bar.x0} y1={barY} x2={bar.x1} y2={barY} stroke="#5a8fd0" stroke-width="2" />
          {#each ticks as t (t.pos)}
            {@const x = xOf(t.pos)}
            <line x1={x} y1={barY + 6} x2={x} y2={barY + (t.major ? 14 : 10)} stroke="#5a8fd0" stroke-width={t.major ? 1.5 : 0.8} />
            {#if t.major}<text {x} y={barY + 25} text-anchor="middle" font-size="9" fill="#555">{formatCoord(t.pos)}</text>{/if}
          {/each}
          <text x={bar.x0} y={barY + 25} text-anchor="middle" font-size="9" fill="#555">1</text>
          <text x={bar.x1} y={barY + 25} text-anchor="middle" font-size="9" fill="#555">{formatCoord(length)}</text>
          {#if doc.ends.left_length !== 0}
            <line x1={bar.x0} y1={barY} x2={bar.x0 + 18} y2={barY} stroke="#d1242f" stroke-width="12" stroke-linecap="round" />
            <rect x={bar.x0 - 44} y={barY - 32} width={doc.ends.left_bases.length * reChar + 44} height="14" fill="#e6e6e6" stroke="#999" />
            <text x={bar.x0 - 40} y={barY - 21} font-size="10">{doc.ends.left_length > 0 ? "5'" : "3'"} : ({Math.abs(doc.ends.left_length)}){doc.ends.left_bases}</text>
          {/if}
          {#if doc.ends.right_length !== 0}
            <line x1={bar.x1 - 18} y1={barY} x2={bar.x1} y2={barY} stroke="#d1242f" stroke-width="12" stroke-linecap="round" />
            <rect x={bar.x1 - doc.ends.right_bases.length * reChar - 4} y={barY - 32} width={doc.ends.right_bases.length * reChar + 48} height="14" fill="#e6e6e6" stroke="#999" />
            <text x={bar.x1 - doc.ends.right_bases.length * reChar} y={barY - 21} font-size="10">{doc.ends.right_length > 0 ? "5'" : "3'"} : ({Math.abs(doc.ends.right_length)}){doc.ends.right_bases}</text>
          {/if}
          <text x={bar.x0} y={svgHeight - 14} font-size="12" fill="#222">{doc.name} - {formatCoord(length)} nt</text>
        {/if}

        {#if selectionPath}
          <path d={selectionPath} fill={circular ? 'none' : 'rgba(46,160,67,0.35)'} stroke={circular ? COLORS.begin : 'none'} stroke-width="14" stroke-opacity="0.55" />
        {/if}

        <!-- features on their tracks -->
        {#each featureShapes as f (f.key)}
          <g role="button" tabindex="-1" style="cursor: default" ondblclick={() => void requestSelect(id, f.lo, f.hi)} onclick={(e) => e.stopPropagation()} onkeydown={(e) => { if (e.key === 'Enter') void requestSelect(id, f.lo, f.hi); }}>
            <title>{f.name}</title>
            {#if circular}
              <path d={f.path} fill="none" stroke={f.color} stroke-width={f.strokeWidth} stroke-opacity="0.9" />
              {#if f.head}<path d={f.head} fill={f.color} stroke={darker(f.color)} stroke-width="0.6" />{/if}
              {#if f.textPath}
                <text font-size={FEATURE_FONT} fill="#111" dominant-baseline="middle" style="paint-order: stroke" stroke="#fff" stroke-width="2.5" stroke-opacity="0.85">
                  <textPath href="#tp-{f.key}" startOffset="50%" text-anchor="middle">{f.name}</textPath>
                </text>
              {/if}
            {:else}
              <path d={f.path} fill={f.color} fill-opacity="0.85" stroke={darker(f.color)} stroke-width="0.8" />
              {#if f.inside && f.box}
                <text x={f.box.x + f.box.w / 2} y={f.box.y + f.box.h / 2} text-anchor="middle" dominant-baseline="middle" font-size={FEATURE_FONT} fill="#111" style="paint-order: stroke" stroke="#fff" stroke-width="2" stroke-opacity="0.8">{f.name}</text>
              {/if}
            {/if}
          </g>
        {/each}

        <!-- site labels and outside feature labels -->
        {#each placed as p (p.key)}
          {@const selectedBox = p.kind === 'site' && (p.key === begin || p.key === end)}
          {@const isSite = p.kind === 'site'}
          {@const boxed = !isSite || selectedBox || ($prefs.gmap_box_re_names && (p.category !== 'unique' || $prefs.gmap_box_unique))}
          {@const fill = !isSite ? '#ffffff' : p.key === begin ? COLORS.begin : p.key === end ? COLORS.end : COLORS[p.category ?? 'unique']}
          {@const stroke = !isSite ? (p.feature?.color ?? '#555') : selectedBox ? fill : '#555'}
          {#if p.tick}<line x1={p.tick.x1} y1={p.tick.y1} x2={p.tick.x2} y2={p.tick.y2} stroke="#222" stroke-width="1.5" />{/if}
          <line x1={p.leader.x1} y1={p.leader.y1} x2={p.leader.x2} y2={p.leader.y2} stroke={isSite ? '#777' : (p.feature?.color ?? '#777')} stroke-width="0.8" />
          <g
            class="label"
            onpointerdown={(e) => onLabelDown(e, p.key)}
            ondblclick={() => (isSite ? onLabelDblClick(p.key) : p.feature && void requestSelect(id, p.feature.lo, p.feature.hi))}
            role="button"
            tabindex="-1"
            style="cursor: grab"
          >
            {#if boxed}
              <rect x={p.box.x} y={p.box.y} width={p.box.w} height={p.box.h} {fill} {stroke} rx="2" />
            {/if}
            {#if isSite}
              <text x={p.box.x + 5} y={p.box.y + 12} font-size={RE_FONT} fill={selectedBox ? '#fff' : '#111'} font-family="ui-monospace, Menlo, monospace">{p.text}</text>
            {:else}
              <text x={p.box.x + p.box.w / 2} y={p.box.y + 12} text-anchor="middle" font-size={FEATURE_FONT} fill="#111">{p.text}</text>
            {/if}
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
    overflow: auto;
    background: #fff;
  }
  .topo {
    padding: 2px 10px;
    border-radius: 12px;
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
