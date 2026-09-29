<script lang="ts">
  /**
   * The Graphic Map (User Manual §IV.C; reference/ui/06 and 07): circular or
   * linear map with Features and unique / double / particular sites, labels
   * you can drag, two-click fragment selection (green BEGIN, red END,
   * spacebar CW/CCW), Extract Fragment, save as PNG, print.
   *
   * Labels are budgeted the way SnapGene does it: only as many as fit without
   * overlapping are shown (sites first, then the longest features), the rest
   * are counted as hidden and reachable by hovering, by pinning them from the
   * Labels panel, or with Show all, which lays every label out.
   *
   * The drawing has its own coordinate space (the SVG viewBox) and the SVG
   * fills the window with `preserveAspectRatio="meet"`, so the whole map is
   * always visible and centred and grows with the window: in automatic mode
   * the layout space is the window itself; in Show all it is as large as the
   * labels need and is scaled down to fit.
   */
  import { onMount, tick } from 'svelte';
  import { Menu } from '@tauri-apps/api/menu';
  import { emitTo } from '@tauri-apps/api/event';
  import { message } from '@tauri-apps/plugin-dialog';
  import { prefs, printPage } from '$lib/prefs';
  import { api, docIdFromUrl, onDocChanged, onMenu, requestSelect, type ConstructSelection, type DocumentView, type EnzymeEntry, type MapSite, type SiteRefView } from '$lib/doc';
  import { assignTracks, boxAround, budgetLabels, budgetRows, fitCircle, formatCoord, groupIsoschizomers, tickStep, type BudgetCand, type Span } from './layout';

  const id = docIdFromUrl();
  const COLORS = $derived({ unique: $prefs.color_unique, double: $prefs.color_double, particular: $prefs.color_particular, begin: $prefs.color_begin, end: $prefs.color_end });
  const RE_FONT = $derived($prefs.gmap_re_font_size);
  const FEATURE_FONT = $derived($prefs.gmap_feature_font_size);
  type Category = 'unique' | 'double' | 'particular';
  const PREVIOUS_KEY = 'gmap.previousParticular';
  const PANEL_KEY = 'gmap.labelsPanel';
  const GROUP_KEY = 'gmap.groupIsoschizomers';

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

  // Label visibility: display-only state of this map window.
  let labelMode = $state<'auto' | 'all'>('auto');
  /** Per label key: pinned always visible, or hidden. Absent = automatic. */
  let labelState = $state<Record<string, 'pin' | 'hide'>>({});
  let panelOpen = $state(readSession(PANEL_KEY) === '1');
  let panelFilter = $state('');
  let groupIso = $state(readLocal(GROUP_KEY) !== '0');
  let hover = $state<{ key: string; text: string; x: number; y: number } | null>(null);

  function readSession(k: string): string | null {
    try {
      return sessionStorage.getItem(k);
    } catch {
      return null;
    }
  }
  function readLocal(k: string): string | null {
    try {
      return localStorage.getItem(k);
    } catch {
      return null;
    }
  }
  $effect(() => {
    try {
      sessionStorage.setItem(PANEL_KEY, panelOpen ? '1' : '0');
    } catch {
      /* private mode */
    }
  });
  $effect(() => {
    try {
      localStorage.setItem(GROUP_KEY, groupIso ? '1' : '0');
    } catch {
      /* private mode */
    }
  });

  const keyOf = (s: MapSite) => `${s.name}@${s.position}${s.forward === false ? 'r' : ''}`;
  const length = $derived(doc?.bases.length ?? 0);
  const circular = $derived(doc?.circular ?? false);

  /** Sites the checkboxes display, with their category. */
  const shown = $derived.by(() => {
    const out: (MapSite & { category: Category })[] = [];
    const isParticular = (n: string) => particular.includes(n);
    for (const s of sites) {
      if (showParticular && isParticular(s.name)) out.push({ ...s, category: 'particular' });
      else if (showUnique && s.count === 1) out.push({ ...s, category: 'unique' });
      else if (showDouble && s.count === 2) out.push({ ...s, category: 'double' });
    }
    return out;
  });

  /** One box per cut when isoschizomers are grouped; a pinned member leads. */
  interface Group {
    key: string;
    lead: MapSite & { category: Category };
    others: (MapSite & { category: Category })[];
    text: string;
    tooltip: string;
    pinned: boolean;
    hidden: boolean;
  }
  const siteGroups = $derived.by((): Group[] => {
    const groups = groupIso ? groupIsoschizomers(shown, (s) => labelState[keyOf(s)] === 'pin') : shown.map((s) => ({ lead: s, others: [] as typeof shown }));
    return groups.map(({ lead, others }) => {
      const members = [lead, ...others];
      const key = keyOf(lead);
      const base = labelText(lead);
      return {
        key,
        lead,
        others,
        text: others.length ? `${base} +${others.length}` : base,
        tooltip: others.length ? `${base}, ${others.map((o) => o.name).join(', ')}` : base,
        pinned: members.some((m) => labelState[keyOf(m)] === 'pin'),
        hidden: members.every((m) => labelState[keyOf(m)] === 'hide'),
      };
    });
  });
  const siteByKey = $derived(new Map(siteGroups.map((g) => [g.key, g.lead])));

  // --- geometry -----------------------------------------------------------
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

  /** Features to draw, with their spans (0-based, half-open) and a length rank. */
  const visibleFeatures = $derived.by(() => {
    if (!doc || !showFeatures || length === 0) return [] as { f: DocumentView['features'][number]; i: number; span: Span; len: number; rank: number }[];
    const list = doc.features
      .map((f, i) => ({ f, i }))
      .filter(({ f }) => f.show)
      .map(({ f, i }) => {
        const a = f.forward ? f.start : f.end;
        const b = f.forward ? f.end : f.start;
        const lo = a - 1;
        const wraps = a > b;
        const len = wraps ? length - a + 1 + b : b - a + 1;
        return { f, i, span: { lo, hi: wraps ? b : lo + len, wraps }, len, rank: 0 };
      });
    const byLen = list.map((v, k) => ({ k, len: v.len })).sort((p, q) => q.len - p.len || p.k - q.k);
    byLen.forEach(({ k }, rank) => (list[k]!.rank = rank));
    return list;
  });
  const tracks = $derived(assignTracks(visibleFeatures.map((v) => v.span), Math.max(1, length)));
  const trackCount = $derived(tracks.reduce((m, t) => Math.max(m, t + 1), 0));

  const LABEL_RING = 34;
  const TRACK_GAP = 30;
  const MARGIN = 6;

  function angleAt(pos: number): number {
    return (pos / Math.max(1, length)) * TAU - Math.PI / 2;
  }
  /** Extent of a horizontal box along the circle at its angle, in pixels. */
  const extentPx = (angle: number, w: number) => LABEL_H * Math.abs(Math.cos(angle)) + w * Math.abs(Math.sin(angle)) + 4;

  /** Every label that may appear outside the ring, with its width. */
  const outsideLabels = $derived.by(() => {
    const out: { key: string; w: number }[] = [];
    for (const g of siteGroups) if (!g.hidden) out.push({ key: g.key, w: siteLabelW(g.text) });
    for (const v of visibleFeatures) if (labelState[`f:${v.i}`] !== 'hide') out.push({ key: `f:${v.i}`, w: featLabelW(v.f.name) });
    return out;
  });

  /**
   * Drawing size and ring radius (see the module comment). The ring floor
   * leaves room for the feature tracks; labels wider than the band a small
   * window allows are dropped (counted as hidden for room).
   */
  const geom = $derived.by(() => {
    if (!circular) return { svgW: width, svgH: height, cx: width / 2, cy: height / 2, radius: 0, labelR: 0, dropped: [] as string[] };
    const floor = 48 + Math.max(1, trackCount) * 6 + TRACK_GAP;
    let minLabelR = 0;
    if (labelMode === 'all') {
      let need = 0;
      for (const g of siteGroups) {
        if (g.hidden) continue;
        need += extentPx(angleAt(g.lead.cut_top), siteLabelW(g.text)) + 3;
      }
      for (const v of visibleFeatures) {
        if (labelState[`f:${v.i}`] === 'hide') continue;
        const mid = angleAt(v.span.lo) + (v.len / Math.max(1, length)) * Math.PI;
        need += extentPx(mid, featLabelW(v.f.name)) + 3;
      }
      minLabelR = need / (TAU * 0.92);
    }
    const fit = fitCircle(width, height, outsideLabels, LABEL_H, LABEL_RING, MARGIN, floor, minLabelR);
    return { svgW: fit.lw, svgH: fit.lh, cx: fit.cx, cy: fit.cy, radius: fit.radius, labelR: fit.labelR, dropped: fit.dropped };
  });
  const center = $derived({ x: geom.cx, y: geom.cy });
  const radius = $derived(geom.radius);
  const trackWidth = $derived(trackCount === 0 ? 12 : Math.max(6, Math.min(12, (radius - 48) / trackCount)));
  const trackRadius = (t: number) => radius - TRACK_GAP - t * trackWidth - trackWidth / 2;

  function angleOf(pos: number): number {
    return angleAt(pos);
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
    tooltip: string;
    /** Leader from the anchor on the map to the box. */
    leader: { x1: number; y1: number; x2: number; y2: number };
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
    /** Name drawn inside/along the feature (else it is in `placed` or hidden). */
    inside: boolean;
    strokeWidth: number;
  }

  /** A short mark on the ring / bar for every displayed site, shown or not. */
  interface SiteTick {
    key: string;
    tooltip: string;
    x1: number;
    y1: number;
    x2: number;
    y2: number;
  }

  function darker(hex: string): string {
    const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
    if (!m) return '#333';
    const n = parseInt(m[1]!, 16);
    const f = (c: number) => Math.max(0, Math.round(c * 0.55)).toString(16).padStart(2, '0');
    return `#${f(n >> 16)}${f((n >> 8) & 255)}${f(n & 255)}`;
  }

  const sitePriority = (c: Category) => (c === 'particular' ? 0 : c === 'unique' ? 1 : 2);

  type Cand = BudgetCand & { kind: 'site' | 'feature'; text: string; tooltip: string; w: number; anchorR: number; site?: MapSite; category?: Category; feature?: { lo: number; hi: number; color: string } };

  /** Circular layout: features as arcs on tracks, labels budgeted and packed around the ring. */
  const circularLayout = $derived.by((): { features: FeatureShape[]; labels: Placed[]; ticks: SiteTick[]; crowded: number } => {
    if (!circular) return { features: [], labels: [], ticks: [], crowded: 0 };
    const labelR = geom.labelR;
    const features: FeatureShape[] = [];
    const siteTicks: SiteTick[] = [];
    const cands: Cand[] = [];
    for (const g of siteGroups) {
      const a = angleOf(g.lead.cut_top);
      const t0 = polar(radius, a);
      const t1 = polar(radius + 10, a);
      siteTicks.push({ key: g.key, tooltip: g.tooltip, x1: t0.x, y1: t0.y, x2: t1.x, y2: t1.y });
      const w = siteLabelW(g.text);
      cands.push({ key: g.key, kind: 'site', text: g.text, tooltip: g.tooltip, w, angle: a, size: extentPx(a, w) / labelR, priority: sitePriority(g.lead.category), pinned: g.pinned, hidden: g.hidden, anchorR: radius, site: g.lead, category: g.lead.category });
    }
    visibleFeatures.forEach(({ f, i, span, len, rank }, k) => {
      const t = tracks[k]!;
      const r = trackRadius(t);
      const a0 = angleOf(span.lo);
      const a1 = a0 + (len / length) * TAU;
      const arcPx = r * (a1 - a0);
      const key = `f:${i}`;
      const mid = (a0 + a1) / 2;
      const nameW = f.name.length * featChar;
      const inside = arcPx >= nameW + 8 && trackWidth >= 9 && labelState[key] !== 'hide';
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
      if (!inside) {
        const w = featLabelW(f.name);
        cands.push({ key, kind: 'feature', text: f.name, tooltip: f.name, w, angle: mid, size: extentPx(mid, w) / labelR, priority: 10 + rank, pinned: labelState[key] === 'pin', hidden: labelState[key] === 'hide', anchorR: r + trackWidth / 2, feature: { lo: span.lo, hi: span.hi, color: f.color } });
      }
    });

    const droppedSet = new Set(geom.dropped);
    for (const c of cands) if (droppedSet.has(c.key) && !c.pinned) c.hidden = true;
    const budget = budgetLabels(cands, 3 / labelR, labelMode === 'all');
    const labels: Placed[] = [];
    for (const c of cands) {
      const a = budget.angles.get(c.key);
      if (a === undefined) continue;
      const anchor = polar(labelR, a);
      const { x, y } = boxAround(center.x, center.y, labelR, a, c.w, LABEL_H);
      const o = offsets[c.key] ?? { dx: 0, dy: 0 };
      const box = { x: x + o.dx, y: y + o.dy, w: c.w, h: LABEL_H };
      const bx = Math.max(box.x, Math.min(box.x + box.w, anchor.x + o.dx));
      const by = Math.max(box.y, Math.min(box.y + box.h, anchor.y + o.dy));
      const from = polar(c.anchorR + (c.kind === 'site' ? 10 : 2), c.angle);
      labels.push({ key: c.key, kind: c.kind, site: c.site, category: c.category, feature: c.feature, text: c.text, tooltip: c.tooltip, leader: { x1: from.x, y1: from.y, x2: bx, y2: by }, box });
    }
    const crowded = budget.crowdedOut.length + cands.filter((c) => droppedSet.has(c.key) && !c.pinned).length;
    return { features, labels, ticks: siteTicks, crowded };
  });

  /** Linear layout: site rows above the bar, feature rows below, both budgeted. */
  const linearLayout = $derived.by((): { features: FeatureShape[]; labels: Placed[]; ticks: SiteTick[]; barY: number; total: number; crowded: number } => {
    if (circular) return { features: [], labels: [], ticks: [], barY: 0, total: height, crowded: 0 };
    const ROW_H = 18;
    const BOX_H = 12;
    const rowH = LABEL_H + 4;
    const endsRoom = doc && (doc.ends.left_length !== 0 || doc.ends.right_length !== 0) ? 22 : 0;
    // Room for label rows in automatic mode: what the container leaves once
    // the bar, ticks and feature rows are drawn, split above / below.
    const fixed = 10 + 24 + endsRoom + 34 + trackCount * ROW_H + 8 + 40;
    const spare = Math.max(rowH, height - fixed);
    const maxSiteRows = labelMode === 'all' ? Infinity : Math.max(1, Math.floor((spare * 0.55) / rowH));
    const maxFeatRows = labelMode === 'all' ? Infinity : Math.max(1, Math.floor((spare * 0.45) / rowH));
    const clampX = (x: number, w: number) => Math.min(Math.max(4, x), width - 4 - w);

    const items = siteGroups.map((g) => {
      const w = siteLabelW(g.text);
      const x = xOf(g.lead.cut_top);
      return { g, w, x, left: clampX(x - w / 2, w) };
    });
    const rows = budgetRows(items.map((it) => ({ x: it.left, w: it.w, priority: sitePriority(it.g.lead.category), pinned: it.g.pinned, hidden: it.g.hidden })), 4, maxSiteRows);
    const rowCount = rows.reduce((m, r) => Math.max(m, r + 1), 0);
    const barY = 10 + rowCount * rowH + 24 + endsRoom;
    let crowded = items.filter((it, k) => rows[k]! < 0 && !it.g.hidden).length;
    const labels: Placed[] = [];
    const siteTicks: SiteTick[] = [];
    items.forEach((it, k) => {
      siteTicks.push({ key: it.g.key, tooltip: it.g.tooltip, x1: it.x, y1: barY - 6, x2: it.x, y2: barY - 16 });
      if (rows[k]! < 0) return;
      const o = offsets[it.g.key] ?? { dx: 0, dy: 0 };
      const y = barY - 30 - endsRoom - rows[k]! * rowH;
      const box = { x: it.left + o.dx, y: y + o.dy, w: it.w, h: LABEL_H };
      labels.push({
        key: it.g.key,
        kind: 'site',
        site: it.g.lead,
        category: it.g.lead.category,
        text: it.g.text,
        tooltip: it.g.tooltip,
        leader: { x1: it.x, y1: barY - 16, x2: Math.max(box.x, Math.min(box.x + box.w, it.x + o.dx)), y2: box.y + box.h },
        box,
      });
    });
    const top = barY + 34;
    const features: FeatureShape[] = [];
    const outside: { key: string; text: string; x: number; y: number; color: string; lo: number; hi: number; rank: number }[] = [];
    visibleFeatures.forEach(({ f, i, span, rank }, k) => {
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
      const inside = x1 - x0 >= f.name.length * featChar + 8 && labelState[key] !== 'hide';
      features.push({ key, name: f.name, color: f.color, forward: f.forward, path: d.trim(), head: '', textPath: '', lo: span.lo, hi: span.hi, track: t, box: { x: x0, y, w: x1 - x0, h: BOX_H }, inside, strokeWidth: 1 });
      if (!inside) outside.push({ key, text: f.name, x: (x0 + x1) / 2, y: y + BOX_H, color: f.color, lo: span.lo, hi: span.hi, rank });
    });
    const labelTop = top + trackCount * ROW_H + 8;
    const boxes = outside.map((o) => {
      const w = featLabelW(o.text);
      return { w, x: clampX(o.x - w / 2, w), priority: 10 + o.rank, pinned: labelState[o.key] === 'pin', hidden: labelState[o.key] === 'hide' };
    });
    const lrows = budgetRows(boxes, 4, maxFeatRows);
    crowded += outside.filter((o, k) => lrows[k]! < 0 && labelState[o.key] !== 'hide').length;
    outside.forEach((o, k) => {
      if (lrows[k]! < 0) return;
      const oo = offsets[o.key] ?? { dx: 0, dy: 0 };
      const bx = boxes[k]!;
      const y = labelTop + lrows[k]! * rowH;
      const box = { x: bx.x + oo.dx, y: y + oo.dy, w: bx.w, h: LABEL_H };
      labels.push({
        key: o.key,
        kind: 'feature',
        feature: { lo: o.lo, hi: o.hi, color: o.color },
        text: o.text,
        tooltip: o.text,
        leader: { x1: o.x, y1: o.y, x2: Math.max(box.x, Math.min(box.x + box.w, o.x + oo.dx)), y2: box.y },
        box,
      });
    });
    const lrowCount = lrows.reduce((m, r) => Math.max(m, r + 1), 0);
    // The layout space is at least the window; when the rows need more, the
    // viewBox scales the drawing down instead of scrolling.
    const total = Math.max(height, labelTop + lrowCount * rowH + 40);
    return { features, labels, ticks: siteTicks, barY, total, crowded };
  });

  const featureShapes = $derived(circular ? circularLayout.features : linearLayout.features);
  const placed = $derived(circular ? circularLayout.labels : linearLayout.labels);
  const siteTicks = $derived(circular ? circularLayout.ticks : linearLayout.ticks);
  const barY = $derived(linearLayout.barY);
  const svgW = $derived(geom.svgW);
  const svgHeight = $derived(circular ? geom.svgH : linearLayout.total);
  const crowdedCount = $derived(circular ? circularLayout.crowded : linearLayout.crowded);
  /** Keys whose label is on the map right now (outside box or name along the feature). */
  const shownKeys = $derived.by(() => {
    const set = new Set<string>();
    for (const p of placed) set.add(p.key);
    for (const f of featureShapes) if (f.inside) set.add(f.key);
    return set;
  });
  /** A site's group lead key, so grouped members know whether their box is shown. */
  const leadOf = $derived.by(() => {
    const m = new Map<string, string>();
    for (const g of siteGroups) {
      m.set(g.key, g.key);
      for (const o of g.others) m.set(keyOf(o), g.key);
    }
    return m;
  });
  const userHiddenCount = $derived.by(() => {
    let n = siteGroups.filter((g) => g.hidden).length;
    for (const v of visibleFeatures) if (labelState[`f:${v.i}`] === 'hide') n++;
    return n;
  });

  const selectionPath = $derived.by(() => {
    if (!doc) return '';
    const b = begin ? siteByKey.get(begin) : undefined;
    const e = end ? siteByKey.get(end) : undefined;
    if (circular) {
      if (!b) return '';
      const from = b.cut_top;
      const to = e ? e.cut_top : b.cut_top;
      const [s, t] = clockwise ? [from, to] : [to, from];
      let span = (t - s + length) % length;
      if (span === 0) span = length;
      return arcPath(radius, angleOf(s), angleOf(s) + (span / length) * TAU);
    }
    if (selectAll) return `M ${bar.x0} ${barY - 5} H ${bar.x1} V ${barY + 5} H ${bar.x0} Z`;
    if (!b || !e) return '';
    const x0 = xOf(Math.min(b.cut_top, e.cut_top));
    const x1 = xOf(Math.max(b.cut_top, e.cut_top));
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
    const k = svgScale();
    const dx = (e.clientX - drag.x) / k;
    const dy = (e.clientY - drag.y) / k;
    if (Math.abs(dx) * k > 3 || Math.abs(dy) * k > 3) drag.moved = true;
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

  // Hover tooltips (SVG coordinates), pinning by click.
  /** Client → layout coordinates, through the SVG's own transform (the viewBox may scale). */
  function svgPoint(e: MouseEvent): { x: number; y: number } {
    const ctm = svgEl?.getScreenCTM();
    if (!svgEl || !ctm) return { x: 0, y: 0 };
    const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(ctm.inverse());
    return { x: p.x, y: p.y };
  }
  /** Screen pixels per layout unit. */
  function svgScale(): number {
    const ctm = svgEl?.getScreenCTM();
    return ctm ? Math.max(1e-6, ctm.a) : 1;
  }
  function showHover(e: MouseEvent, key: string, text: string) {
    const p = svgPoint(e);
    hover = { key, text, x: p.x, y: p.y };
  }
  function clearHover() {
    hover = null;
  }
  const tooltipW = $derived(hover ? hover.text.length * featChar + 12 : 0);
  const tooltipX = $derived(hover ? Math.max(2, Math.min(svgW - tooltipW - 2, hover.x + 12)) : 0);
  const tooltipY = $derived(hover ? Math.max(2, Math.min(svgHeight - LABEL_H - 2, hover.y - LABEL_H - 6)) : 0);

  /** What the panel shows for a label. */
  type PanelState = 'pinned' | 'hidden' | 'auto-shown' | 'auto-hidden';
  function panelState(key: string): PanelState {
    const s = labelState[key];
    if (s === 'pin') return 'pinned';
    if (s === 'hide') return 'hidden';
    const lead = leadOf.get(key) ?? key;
    return shownKeys.has(lead) || shownKeys.has(key) ? 'auto-shown' : 'auto-hidden';
  }
  /** Click: auto-shown → pinned, pinned → hidden, hidden → auto, auto-hidden → pinned. */
  function cycleLabel(key: string) {
    const st = panelState(key);
    const copy = { ...labelState };
    if (st === 'pinned') copy[key] = 'hide';
    else if (st === 'hidden') delete copy[key];
    else copy[key] = 'pin';
    labelState = copy;
  }
  function togglePin(key: string) {
    const copy = { ...labelState };
    if (copy[key] === 'pin') delete copy[key];
    else copy[key] = 'pin';
    labelState = copy;
  }
  function onFeatureClick(e: MouseEvent, key: string) {
    e.stopPropagation();
    if (e.altKey) void api.showParent(id);
    else togglePin(key);
  }

  // Labels panel.
  interface PanelItem {
    key: string;
    text: string;
    pos: number;
  }
  const panelFeatures = $derived.by((): PanelItem[] =>
    visibleFeatures.map((v) => ({ key: `f:${v.i}`, text: v.f.name, pos: v.span.lo })).sort((a, b) => a.pos - b.pos),
  );
  const panelSites = $derived.by((): PanelItem[] => shown.map((s) => ({ key: keyOf(s), text: labelText(s), pos: s.cut_top })).sort((a, b) => a.pos - b.pos));
  const filterItems = (items: PanelItem[]) => {
    const q = panelFilter.trim().toLowerCase();
    return q ? items.filter((it) => it.text.toLowerCase().includes(q)) : items;
  };
  /** Panel buttons: Show all lays every label out, Hide all hides them, Auto is the budget. */
  function setAll(state: 'all' | 'hide' | 'auto') {
    if (state === 'hide') {
      const copy: Record<string, 'pin' | 'hide'> = {};
      for (const it of [...panelFeatures, ...panelSites]) copy[it.key] = 'hide';
      labelState = copy;
      return;
    }
    labelState = {};
    labelMode = state === 'all' ? 'all' : 'auto';
  }

  function siteRef(key: string | null): SiteRefView | null {
    const s = key ? siteByKey.get(key) : undefined;
    return s ? { enzyme: s.name, position: s.position, cut_top: s.cut_top, cut_bottom: s.cut_bottom } : null;
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
    const b = begin ? siteByKey.get(begin) : undefined;
    const e = (end ? siteByKey.get(end) : undefined) ?? b;
    if (!b || !e) {
      await message('Choose a BEGIN site (click) and an END site (double-click or shift-click) first.', { title: 'Make New Feature' });
      return;
    }
    await api.newFeatureFromSites(id, siteRef(begin)!, siteRef(end ?? begin)!, b.length, e.length, clockwise);
  }

  async function toggleTopology() {
    if (!doc) return;
    await api.edit(id, { op: 'set_topology', circular: !doc.circular });
  }

  async function savePng() {
    if (!svgEl || !doc) return;
    hover = null;
    await tick();
    const clone = svgEl.cloneNode(true) as SVGSVGElement;
    clone.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
    clone.setAttribute('width', String(svgW));
    clone.setAttribute('height', String(svgHeight));
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
    canvas.width = svgW * scale;
    canvas.height = svgHeight * scale;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
    URL.revokeObjectURL(url);
    const data = canvas.toDataURL('image/png').split(',')[1] ?? '';
    await api.savePng(`Graphic Map of ${doc.name}`, data);
  }

  /** Print = the Show-all layout, so nothing is missing on paper. */
  async function printMap() {
    const prev = labelMode;
    labelMode = 'all';
    hover = null;
    await tick();
    try {
      await printPage();
    } finally {
      labelMode = prev;
    }
  }

  async function contextMenu() {
    if (!doc) return;
    const menu = await Menu.new({
      items: [
        { id: 'topo', text: doc.circular ? 'Linearize' : 'Circularize', action: () => void toggleTopology() },
        { id: 'png', text: 'Save Graphic Map As…', action: () => void savePng() },
        { id: 'print', text: 'Print…', action: () => void printMap() },
        { item: 'Separator' },
        { id: 'labels', text: labelMode === 'all' ? 'Automatic labels' : 'Show all labels', action: () => (labelMode = labelMode === 'all' ? 'auto' : 'all') },
        { id: 'panel', text: panelOpen ? 'Hide Labels panel' : 'Show Labels panel', action: () => (panelOpen = !panelOpen) },
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
        void printMap();
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

  // Development self-check: nothing may be drawn outside the viewBox.
  $effect(() => {
    if (!import.meta.env.DEV || !svgEl) return;
    void placed;
    void featureShapes;
    const el = svgEl;
    const w = svgW;
    const h = svgHeight;
    queueMicrotask(() => {
      try {
        const b = el.getBBox();
        if (b.x < -1 || b.y < -1 || b.x + b.width > w + 1 || b.y + b.height > h + 1) {
          console.warn(`Graphic Map: drawing ${b.x.toFixed(0)},${b.y.toFixed(0)} ${b.width.toFixed(0)}×${b.height.toFixed(0)} exceeds the viewBox ${w}×${h}`);
        }
      } catch {
        /* detached */
      }
    });
  });

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
        <label data-help-name="Group isoschizomers" data-help="Enzymes cutting at exactly the same place on the same strand share one box, named after the first with '+N' (the others are in the box's tooltip and in the Labels panel, where checking one of them shows its name instead)."><input type="checkbox" bind:checked={groupIso} /> Group isoschizomers</label>
        <button type="button" class="topo" onclick={toggleTopology} data-help-name="Topology" data-help="Draws the map as a circle or as a bar. Sequences opened from FASTA or text arrive linear; this is the same as Linearize / Circularize in the contextual menu.">{circular ? 'Circular' : 'Linear'} ▸ {circular ? 'linear' : 'circular'}</button>
        <button type="button" class="topo" class:active={panelOpen} onclick={() => (panelOpen = !panelOpen)} data-help-name="Labels" data-help="Opens the Labels panel: search, pin or hide the label of any Feature or displayed site on this map. This only affects the display of this map; the Features tab's Show checkbox is what hides a Feature from every map.">Labels</button>
        <span class="orientation" data-help-name="Direction" data-help="Direction of the selection between the BEGIN (green) and END (red) sites. Press the spacebar to toggle.">{hasSelection ? orientationLabel : ''}</span>
        <button type="button" class="house" title="Show Sequence Window" onclick={() => void api.showParent(id)} data-help-name="Sequence Window" data-help="Brings the parent Sequence Window to the front (Alt-click on the map does the same).">⌂</button>
      </div>
      <div class="status no-print" data-help-name="Hidden labels" data-help="Only the labels that fit without overlapping are shown (chosen sites first, then the longest Features). Hover a Feature or a site mark to read its name, click a Feature to pin its label, or use Show all to lay every label out — the drawing is then scaled to the window. Enlarge the window to enlarge the map.">
        {#if crowdedCount > 0 || userHiddenCount > 0}
          <span>{crowdedCount} label{crowdedCount === 1 ? '' : 's'} hidden for room{userHiddenCount ? `, ${userHiddenCount} hidden in the Labels panel` : ''}.</span>
        {:else}
          <span>All labels shown.</span>
        {/if}
        <button type="button" class:active={labelMode === 'all'} onclick={() => (labelMode = 'all')} data-help-name="Show all" data-help="Lays out every label; the whole drawing is scaled to fit the window (enlarge the window to read it). Print uses this layout.">Show all</button>
        <button type="button" class:active={labelMode === 'auto'} onclick={() => (labelMode = 'auto')} data-help-name="Auto" data-help="Shows only the labels that fit in the window without overlapping.">Auto</button>
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

    <div class="body">
      <div class="map" bind:this={container} oncontextmenu={(e) => { e.preventDefault(); void contextMenu(); }} onclick={(e) => { if (e.altKey) void api.showParent(id); }} role="presentation" data-help-name="Map" data-help="Click a site name box to select the BEGIN site (green), double-click or shift-click another for the END site (red); spacebar toggles CW/CCW (or Sense/Antisense); A selects the whole linear insert; Delete cancels. Hover a Feature or a site mark for its name; click a Feature to pin its label. Drag name boxes to arrange the map before printing. Right-click for the contextual menu.">
        <svg bind:this={svgEl} width="100%" height="100%" viewBox="0 0 {svgW} {svgHeight}" preserveAspectRatio="xMidYMid meet" font-family="-apple-system, Segoe UI, system-ui, sans-serif" font-size="11">
          <rect x="0" y="0" width={svgW} height={svgHeight} fill="#ffffff" />
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

          <!-- features on their tracks: hover for the name, click to pin its label -->
          {#each featureShapes as f (f.key)}
            {@const hot = hover?.key === f.key}
            {@const pinned = labelState[f.key] === 'pin'}
            <g
              role="button"
              tabindex="-1"
              style="cursor: default"
              onpointerenter={(e) => showHover(e, f.key, f.name)}
              onpointermove={(e) => showHover(e, f.key, f.name)}
              onpointerleave={clearHover}
              onclick={(e) => onFeatureClick(e, f.key)}
              ondblclick={() => void requestSelect(id, f.lo, f.hi)}
              onkeydown={(e) => { if (e.key === 'Enter') void requestSelect(id, f.lo, f.hi); }}
            >
              {#if circular}
                <path d={f.path} fill="none" stroke={hot || pinned ? darker(f.color) : f.color} stroke-width={hot ? f.strokeWidth + 2 : f.strokeWidth} stroke-opacity={hot ? 1 : 0.9} />
                {#if f.head}<path d={f.head} fill={f.color} stroke={darker(f.color)} stroke-width={hot ? 1.4 : 0.6} />{/if}
                {#if f.textPath}
                  <text font-size={FEATURE_FONT} fill="#111" dominant-baseline="middle" style="paint-order: stroke" stroke="#fff" stroke-width="2.5" stroke-opacity="0.85">
                    <textPath href="#tp-{f.key}" startOffset="50%" text-anchor="middle">{f.name}</textPath>
                  </text>
                {/if}
              {:else}
                <path d={f.path} fill={f.color} fill-opacity={hot ? 1 : 0.85} stroke={darker(f.color)} stroke-width={hot ? 1.6 : 0.8} />
                {#if f.inside && f.box}
                  <text x={f.box.x + f.box.w / 2} y={f.box.y + f.box.h / 2} text-anchor="middle" dominant-baseline="middle" font-size={FEATURE_FONT} fill="#111" style="paint-order: stroke" stroke="#fff" stroke-width="2" stroke-opacity="0.8">{f.name}</text>
                {/if}
              {/if}
            </g>
          {/each}

          <!-- site marks on the ring / bar, for every displayed site (labelled or not) -->
          {#each siteTicks as t (t.key)}
            <g role="presentation" onpointerenter={(e) => showHover(e, t.key, t.tooltip)} onpointermove={(e) => showHover(e, t.key, t.tooltip)} onpointerleave={clearHover}>
              <line x1={t.x1} y1={t.y1} x2={t.x2} y2={t.y2} stroke="transparent" stroke-width="9" />
              <line x1={t.x1} y1={t.y1} x2={t.x2} y2={t.y2} stroke={hover?.key === t.key ? '#000' : '#222'} stroke-width={hover?.key === t.key ? 3 : 1.5} />
            </g>
          {/each}

          <!-- site labels and outside feature labels that made the budget -->
          {#each placed as p (p.key)}
            {@const selectedBox = p.kind === 'site' && (p.key === begin || p.key === end)}
            {@const isSite = p.kind === 'site'}
            {@const boxed = !isSite || selectedBox || ($prefs.gmap_box_re_names && (p.category !== 'unique' || $prefs.gmap_box_unique))}
            {@const fill = !isSite ? '#ffffff' : p.key === begin ? COLORS.begin : p.key === end ? COLORS.end : COLORS[p.category ?? 'unique']}
            {@const stroke = !isSite ? (p.feature?.color ?? '#555') : selectedBox ? fill : '#555'}
            <line x1={p.leader.x1} y1={p.leader.y1} x2={p.leader.x2} y2={p.leader.y2} stroke={isSite ? '#777' : (p.feature?.color ?? '#777')} stroke-width="0.8" />
            <g
              class="label"
              onpointerdown={(e) => onLabelDown(e, p.key)}
              ondblclick={() => (isSite ? onLabelDblClick(p.key) : p.feature && void requestSelect(id, p.feature.lo, p.feature.hi))}
              onpointerenter={(e) => { if (p.tooltip !== p.text) showHover(e, p.key, p.tooltip); }}
              onpointerleave={clearHover}
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

          {#if hover}
            <g class="tooltip" pointer-events="none">
              <rect x={tooltipX} y={tooltipY} width={tooltipW} height={LABEL_H} fill="#222" fill-opacity="0.92" rx="3" />
              <text x={tooltipX + 6} y={tooltipY + 12} font-size={FEATURE_FONT} fill="#fff">{hover.text}</text>
            </g>
          {/if}
        </svg>
      </div>

      {#if panelOpen}
        <aside class="labels no-print" data-help-name="Labels panel" data-help="Every Feature and displayed site of this map. Click a checkbox to cycle: checked = always shown, unchecked = hidden, dash = automatic. This is only the display of this map window and is not saved with the sequence; use the Features tab's Show checkbox to hide a Feature from every map.">
          <div class="labels-head">
            <input type="search" placeholder="search" bind:value={panelFilter} />
            <button type="button" onclick={() => setAll('all')} data-help-name="Show all" data-help="Lays every label out (the drawing is scaled to the window).">Show all</button>
            <button type="button" onclick={() => setAll('hide')} data-help-name="Hide all" data-help="Hides every label; the Features and site marks stay on the map.">Hide all</button>
            <button type="button" onclick={() => setAll('auto')} data-help-name="Auto" data-help="Back to the automatic budget: what fits is shown, nothing pinned or hidden.">Auto</button>
            <p class="legend" data-help-name="Legend" data-help="Blue check: pinned by you, always shown. Grey check: shown automatically because it fits. Empty: hidden for room. Struck through: hidden by you. Click to cycle: shown → pinned → hidden → automatic.">
              <span class="sw pin"></span> pinned <span class="sw auto"></span> shown automatically <span class="sw off"></span> hidden for room <span class="sw user"></span> hidden by you
            </p>
          </div>
          <div class="labels-list">
            <h2>Features</h2>
            {#each filterItems(panelFeatures) as it (it.key)}
              {@const st = panelState(it.key)}
              <label class:pinned={st === 'pinned'} class:hidden-label={st === 'hidden'} class:auto-shown={st === 'auto-shown'} title={st === 'auto-shown' ? 'shown automatically' : st === 'auto-hidden' ? 'hidden for room' : st}>
                <input type="checkbox" checked={st === 'pinned' || st === 'auto-shown'} onclick={(e) => { e.preventDefault(); cycleLabel(it.key); }} />
                <span class="text">{it.text}</span>
                <span class="pos">{formatCoord(it.pos + 1)}</span>
              </label>
            {:else}
              <p class="hint">No Features shown.</p>
            {/each}
            <h2>Sites</h2>
            {#each filterItems(panelSites) as it (it.key)}
              {@const st = panelState(it.key)}
              <label class:pinned={st === 'pinned'} class:hidden-label={st === 'hidden'} class:auto-shown={st === 'auto-shown'} title={st === 'auto-shown' ? 'shown automatically' : st === 'auto-hidden' ? 'hidden for room' : st}>
                <input type="checkbox" checked={st === 'pinned' || st === 'auto-shown'} onclick={(e) => { e.preventDefault(); cycleLabel(it.key); }} />
                <span class="text mono">{it.text}</span>
              </label>
            {:else}
              <p class="hint">No sites displayed: tick Unique, Double or Particular sites.</p>
            {/each}
          </div>
        </aside>
      {/if}
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
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  .status button,
  .topo {
    padding: 1px 10px;
    border-radius: 12px;
  }
  .choose {
    padding: 2px 16px;
    border-radius: 12px;
  }
  .active {
    background: var(--accent);
    color: #fff;
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
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .map {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: #fff;
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
    -webkit-user-select: none;
    user-select: none;
  }
  .labels {
    width: 260px;
    flex: none;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    background: var(--bg-raised);
  }
  .labels-head {
    display: flex;
    gap: 4px;
    padding: 6px;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }
  .labels-head input {
    flex: 1 1 100%;
    font: inherit;
  }
  .labels-head button {
    padding: 1px 8px;
  }
  .labels-list {
    overflow: auto;
    padding: 4px 6px;
  }
  .labels-list h2 {
    margin: 6px 0 2px;
    font-size: var(--fs-small);
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .labels-list label {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 1px 0;
  }
  .labels-list .text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .labels-list .pinned .text {
    font-weight: 700;
  }
  .labels-list .pinned input {
    accent-color: var(--accent);
  }
  .labels-list .auto-shown input {
    accent-color: #9a9a9a;
  }
  .legend {
    flex: 1 1 100%;
    margin: 4px 0 0;
    display: flex;
    flex-wrap: wrap;
    gap: 2px 6px;
    align-items: center;
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  .sw {
    display: inline-block;
    width: 10px;
    height: 10px;
    border: 1px solid var(--border);
    border-radius: 2px;
  }
  .sw.pin {
    background: var(--accent);
  }
  .sw.auto {
    background: #9a9a9a;
  }
  .sw.off {
    background: transparent;
  }
  .sw.user {
    background: repeating-linear-gradient(45deg, transparent, transparent 2px, var(--fg-muted) 2px, var(--fg-muted) 3px);
  }
  .labels-list .hidden-label .text {
    color: var(--fg-muted);
    text-decoration: line-through;
  }
  .labels-list .pos {
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: var(--fs-small);
  }
  .mono {
    font-family: var(--font-mono);
  }
  .hint {
    margin: 2px 0;
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  @media print {
    .no-print,
    .chooser,
    .labels {
      display: none !important;
    }
    .gmap {
      height: auto;
    }
    .map {
      overflow: visible;
      height: auto;
    }
    .map svg {
      width: 100%;
      height: auto;
    }
  }
</style>
