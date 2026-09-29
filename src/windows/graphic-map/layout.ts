/**
 * Pure layout functions of the Graphic Map: feature track assignment, label
 * packing around a circle and along a bar, tick spacing. No DOM, so they run
 * under plain Node for the tests in `layout.test.ts`.
 */

/** A feature's extent in 0-based, half-open coordinates on a sequence of `total` bases. */
export interface Span {
  lo: number;
  hi: number;
  /** `lo > hi` in practice: the feature covers `[lo, total)` then `[0, hi)`. */
  wraps: boolean;
}

type Seg = [number, number];

function segments(s: Span, total: number): Seg[] {
  return s.wraps ? [[s.lo, total], [0, s.hi]] : [[s.lo, s.hi]];
}

function overlaps(a: Seg, b: Seg): boolean {
  return a[0] < b[1] && b[0] < a[1];
}

/**
 * Greedy interval packing: every span gets the lowest track where it overlaps
 * nothing already there. Spans are considered longest first, so the big
 * backbone features sit on the inner tracks and small ones fill the gaps.
 * Returns one track index per input span, in input order.
 */
export function assignTracks(spans: Span[], total: number): number[] {
  const order = spans.map((s, i) => ({ i, segs: segments(s, total), len: spanLength(s, total) }));
  order.sort((a, b) => b.len - a.len || a.i - b.i);
  const tracks: Seg[][] = [];
  const out = new Array<number>(spans.length).fill(0);
  for (const { i, segs } of order) {
    let t = tracks.findIndex((track) => !track.some((seg) => segs.some((s) => overlaps(s, seg))));
    if (t < 0) {
      tracks.push([]);
      t = tracks.length - 1;
    }
    tracks[t]!.push(...segs);
    out[i] = t;
  }
  return out;
}

/** Length in bases of a span. */
export function spanLength(s: Span, total: number): number {
  return s.wraps ? total - s.lo + s.hi : s.hi - s.lo;
}

const TAU = Math.PI * 2;

function norm(a: number): number {
  return ((a % TAU) + TAU) % TAU;
}

/**
 * Places labels around a circle. `ideal` are the wanted angles (radians, any
 * range), `size` each label's extent along the circumference (radians) and
 * `gap` the minimum spacing between two labels. Labels keep their angular
 * order and are pushed apart just enough not to overlap; when the circle is
 * too full for that, they are spread evenly. Returns angles in input order.
 */
export function packAngular(ideal: number[], size: number[], gap: number): number[] {
  const n = ideal.length;
  if (n === 0) return [];
  const idx = ideal.map((_, i) => i).sort((a, b) => norm(ideal[a]!) - norm(ideal[b]!) || a - b);
  const want = idx.map((i) => norm(ideal[i]!));
  const sz = idx.map((i) => size[i]!);
  const sep = (k: number, l: number) => (sz[k]! + sz[l]!) / 2 + gap;
  const need = sz.reduce((a, b) => a + b, 0) + n * gap;
  let pos: number[];
  if (need >= TAU) {
    // Nothing fits: even spacing keeps the order and makes the best of it.
    pos = want.map((_, k) => want[0]! + (k * TAU) / n);
  } else {
    pos = want.slice();
    // Forward pass: push later labels clockwise out of the way.
    for (let k = 1; k < n; k++) pos[k] = Math.max(pos[k]!, pos[k - 1]! + sep(k - 1, k));
    // Wrap: the last must also clear the first (one turn later).
    let over = pos[n - 1]! + sep(n - 1, 0) - (pos[0]! + TAU);
    if (over > 0) {
      // Slide the whole chain back and re-run: the chain fits (need < TAU),
      // so distributing the excess evenly resolves the wrap.
      const shift = over / 2;
      pos = want.map((w) => w - shift);
      for (let k = 1; k < n; k++) pos[k] = Math.max(pos[k]!, pos[k - 1]! + sep(k - 1, k));
      over = pos[n - 1]! + sep(n - 1, 0) - (pos[0]! + TAU);
      if (over > 0) pos = want.map((_, k) => pos[0]! + (k * (TAU - sep(n - 1, 0))) / Math.max(1, n - 1));
    }
    // Pull chains back toward their ideal positions where room allows, so a
    // cluster is centred on its members instead of trailing clockwise.
    for (let pass = 0; pass < 3; pass++) {
      let k = 0;
      while (k < n) {
        let e = k;
        while (e + 1 < n && pos[e + 1]! - pos[e]! <= sep(e, e + 1) + 1e-9) e++;
        let drift = 0;
        for (let m = k; m <= e; m++) drift += pos[m]! - want[m]!;
        drift /= e - k + 1;
        if (drift > 1e-9) {
          const prevLimit = k === 0 ? pos[n - 1]! - TAU + sep(n - 1, 0) : pos[k - 1]! + sep(k - 1, k);
          const room = pos[k]! - prevLimit;
          const back = Math.min(drift, Math.max(0, room));
          for (let m = k; m <= e; m++) pos[m]! -= back;
        }
        k = e + 1;
      }
    }
  }
  const out = new Array<number>(n).fill(0);
  idx.forEach((i, k) => (out[i] = pos[k]!));
  return out;
}

/** True if two labels placed by [`packAngular`] would overlap. */
export function angularOverlap(pos: number[], size: number[], gap: number): boolean {
  const n = pos.length;
  if (n < 2) return false;
  const idx = pos.map((_, i) => i).sort((a, b) => pos[a]! - pos[b]!);
  for (let k = 0; k < n; k++) {
    const a = idx[k]!;
    const b = idx[(k + 1) % n]!;
    const d = k + 1 < n ? pos[b]! - pos[a]! : pos[b]! + TAU - pos[a]!;
    if (d + 1e-9 < (size[a]! + size[b]!) / 2 + gap) return true;
  }
  return false;
}

/**
 * Packs horizontal boxes into rows: each box goes to the first row where it
 * clears every box already there by `gap`. Returns the row of each box.
 */
export function packRows(boxes: { x: number; w: number }[], gap: number): number[] {
  const order = boxes.map((b, i) => ({ b, i })).sort((p, q) => p.b.x - q.b.x || p.i - q.i);
  const rows: { x: number; w: number }[][] = [];
  const out = new Array<number>(boxes.length).fill(0);
  for (const { b, i } of order) {
    let r = rows.findIndex((row) => row.every((o) => b.x + b.w + gap <= o.x || o.x + o.w + gap <= b.x));
    if (r < 0) {
      rows.push([]);
      r = rows.length - 1;
    }
    rows[r]!.push(b);
    out[i] = r;
  }
  return out;
}

/**
 * Tick spacing for a sequence of `length` bases: the coarsest "nice" step
 * giving at most `maxMajor` labelled ticks, with minor ticks at a fifth.
 */
export function tickStep(length: number, maxMajor = 12): { major: number; minor: number } {
  const steps = [10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000, 200000, 500000, 1000000];
  let major = steps[steps.length - 1]!;
  for (const s of steps) {
    if (length / s <= maxMajor) {
      major = s;
      break;
    }
  }
  return { major, minor: major / 5 };
}

/** Formats a coordinate with thousands separators, as SnapGene and the original print them. */
export function formatCoord(n: number): string {
  return n.toLocaleString('en-US');
}
