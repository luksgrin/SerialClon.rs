/**
 * Layout tests, run with `npm run test:layout` (plain Node, type stripping).
 */
import assert from 'node:assert/strict';
import { angularOverlap, assignTracks, boxAround, budgetLabels, budgetRows, groupIsoschizomers, packAngular, packRows, reserveBand, spanLength, tickStep } from './layout.ts';

let passed = 0;
function test(name: string, fn: () => void) {
  fn();
  passed++;
  console.log(`ok - ${name}`);
}

test('overlapping features get distinct tracks, disjoint ones share', () => {
  const t = assignTracks(
    [
      { lo: 0, hi: 100, wraps: false },
      { lo: 50, hi: 150, wraps: false },
      { lo: 200, hi: 300, wraps: false },
      { lo: 120, hi: 130, wraps: false },
    ],
    1000,
  );
  assert.notEqual(t[0], t[1]);
  assert.equal(t[2], t[0]); // disjoint from the longest: same track
  assert.notEqual(t[3], t[1]); // inside the second
  assert.equal(t[3], t[0]);
});

test('a wrapping feature occupies both ends', () => {
  const total = 1000;
  const wrap = { lo: 900, hi: 50, wraps: true };
  assert.equal(spanLength(wrap, total), 150);
  const t = assignTracks([wrap, { lo: 10, hi: 20, wraps: false }, { lo: 950, hi: 960, wraps: false }, { lo: 500, hi: 600, wraps: false }], total);
  assert.notEqual(t[0], t[1]);
  assert.notEqual(t[0], t[2]);
  assert.equal(t[3], t[0]);
});

test('the longest feature is on the innermost track', () => {
  const t = assignTracks([{ lo: 10, hi: 20, wraps: false }, { lo: 0, hi: 900, wraps: false }], 1000);
  assert.equal(t[1], 0);
  assert.equal(t[0], 1);
});

test('angular packing separates colliding labels and keeps their order', () => {
  const ideal = [0.1, 0.11, 0.12, 2.0, 2.01, 4.0];
  const size = ideal.map(() => 0.2);
  const pos = packAngular(ideal, size, 0.02);
  assert.equal(pos.length, ideal.length);
  assert.ok(!angularOverlap(pos, size, 0.02), `overlap in ${pos}`);
  // Order preserved among the packed cluster.
  assert.ok(pos[0]! < pos[1]! && pos[1]! < pos[2]!);
  assert.ok(pos[3]! < pos[4]!);
  // Isolated labels stay where they are.
  assert.ok(Math.abs(pos[5]! - 4.0) < 1e-9);
  // A cluster is centred on its members, not trailing clockwise.
  const centre = (pos[0]! + pos[2]!) / 2;
  assert.ok(Math.abs(centre - 0.11) < 0.05, `cluster centre ${centre}`);
});

test('angular packing handles the wrap at the origin', () => {
  const ideal = [-0.05, 0.0, 0.05, Math.PI * 2 - 0.02];
  const size = ideal.map(() => 0.3);
  const pos = packAngular(ideal, size, 0.02);
  assert.ok(!angularOverlap(pos, size, 0.02), `overlap in ${pos}`);
});

test('a full circle degrades to even spacing without overlap', () => {
  const n = 40;
  const ideal = Array.from({ length: n }, (_, i) => (i / n) * 0.5); // all crammed in a small arc
  const size = ideal.map(() => 0.1); // total 4 rad + gaps < 2π
  const pos = packAngular(ideal, size, 0.01);
  assert.ok(!angularOverlap(pos, size, 0.01));
  const sorted = ideal.map((_, i) => i).sort((a, b) => ideal[a]! - ideal[b]!);
  for (let k = 1; k < n; k++) {
    const a = pos[sorted[k - 1]!]!;
    const b = pos[sorted[k]!]!;
    assert.ok(b > a, 'order kept');
  }
});

test('100 labels pack quickly', () => {
  const n = 100;
  const ideal = Array.from({ length: n }, () => Math.random() * Math.PI * 2);
  const size = ideal.map(() => 0.04);
  const t0 = performance.now();
  for (let i = 0; i < 20; i++) packAngular(ideal, size, 0.005);
  const ms = (performance.now() - t0) / 20;
  assert.ok(ms < 50, `${ms} ms`);
  assert.ok(!angularOverlap(packAngular(ideal, size, 0.005), size, 0.005));
});

test('row packing stacks only what collides', () => {
  const rows = packRows([{ x: 0, w: 50 }, { x: 40, w: 50 }, { x: 200, w: 50 }, { x: 45, w: 10 }], 4);
  assert.equal(rows[0], 0);
  assert.equal(rows[1], 1);
  assert.equal(rows[2], 0);
  assert.equal(rows[3], 2);
});

test('tick spacing adapts to the sequence size', () => {
  assert.deepEqual(tickStep(1191), { major: 100, minor: 20 });
  assert.deepEqual(tickStep(4733), { major: 500, minor: 100 });
  assert.deepEqual(tickStep(29988), { major: 5000, minor: 1000 });
  assert.deepEqual(tickStep(200000), { major: 20000, minor: 4000 });
});


// ---- band reservation, budget, rows, isoschizomers ----

test('band reservation keeps every placed label inside the drawing', () => {
  const W = 640;
  const H = 480;
  const labelW = 90;
  const labelH = 16;
  const ring = 34;
  const r = reserveBand(W, H, labelW, labelH, ring, 6);
  const labelR = r + ring;
  for (let k = 0; k < 72; k++) {
    const a = (k / 72) * Math.PI * 2 - Math.PI / 2;
    const { x, y } = boxAround(W / 2, H / 2, labelR, a, labelW, labelH);
    assert.ok(x >= 0 && x + labelW <= W, `x out of range at ${a}: ${x}`);
    assert.ok(y >= 0 && y + labelH <= H, `y out of range at ${a}: ${y}`);
  }
  assert.ok(r >= 40);
});

test('the budget never overlaps, respects pinned and hidden, prefers sites then longer features', () => {
  const gap = 0.01;
  const cands: { key: string; angle: number; size: number; priority: number; pinned?: boolean; hidden?: boolean }[] = [];
  for (let k = 0; k < 60; k++) cands.push({ key: `f${k}`, angle: (k / 60) * 0.6, size: 0.15, priority: 10 + k });
  cands.push({ key: 's1', angle: 0.3, size: 0.15, priority: 0 });
  cands.push({ key: 'pin', angle: 0.31, size: 0.15, priority: 50, pinned: true });
  cands.push({ key: 'hid', angle: 0.29, size: 0.15, priority: 0, hidden: true });
  const b = budgetLabels(cands, gap);
  assert.ok(b.angles.has('s1'), 'site shown first');
  assert.ok(b.angles.has('pin'), 'pinned shown');
  assert.ok(!b.angles.has('hid'), 'hidden never shown');
  assert.ok(!b.crowdedOut.includes('hid'), 'hidden not counted as crowded out');
  assert.ok(b.crowdedOut.length > 0, 'the cluster cannot all fit');
  const shownF = [...b.angles.keys()].filter((k) => k.startsWith('f')).map((k) => Number(k.slice(1)));
  const maxShown = Math.max(...shownF);
  for (const k of b.crowdedOut.filter((k) => k.startsWith('f')).map((k) => Number(k.slice(1)))) {
    assert.ok(k > maxShown, `feature ${k} crowded out while ${maxShown} shown`);
  }
  const keys = [...b.angles.keys()];
  const pos = keys.map((k) => b.angles.get(k)!);
  const sizes = keys.map((k) => cands.find((c) => c.key === k)!.size);
  assert.ok(!angularOverlap(pos, sizes, gap), 'shown labels overlap');
});

test('show-all places everything that is not hidden', () => {
  const cands: { key: string; angle: number; size: number; priority: number; hidden?: boolean }[] = Array.from({ length: 30 }, (_, k) => ({ key: `k${k}`, angle: (k / 30) * 0.4, size: 0.1, priority: k }));
  cands.push({ key: 'h', angle: 1, size: 0.1, priority: 0, hidden: true });
  const b = budgetLabels(cands, 0.01, true);
  assert.equal(b.angles.size, 30);
  assert.equal(b.crowdedOut.length, 0);
  assert.ok(!b.angles.has('h'));
});

test('row budget caps the rows and keeps pinned boxes', () => {
  const boxes: { x: number; w: number; priority: number; pinned?: boolean; hidden?: boolean }[] = Array.from({ length: 12 }, (_, k) => ({ x: 100, w: 50, priority: k }));
  boxes.push({ x: 100, w: 50, priority: 99, pinned: true });
  boxes.push({ x: 100, w: 50, priority: 0, hidden: true });
  const rows = budgetRows(boxes, 4, 3);
  assert.equal(rows.filter((r) => r >= 0).length, 3); // the pinned box takes one of the three rows
  assert.ok(rows[12]! >= 0, 'pinned shown');
  assert.equal(rows[13], -1, 'hidden not shown');
  assert.ok(rows[0]! >= 0 && rows[1]! >= 0 && rows[2] === -1, 'highest priorities shown, the next crowded out');
});

test('isoschizomer grouping keys on cut position and strand', () => {
  const sites = [
    { name: 'AsiSI', cut_top: 7763, cut_bottom: 7761, forward: null },
    { name: 'SgfI', cut_top: 7763, cut_bottom: 7761, forward: null },
    { name: 'BsaI', cut_top: 100, cut_bottom: 104, forward: true },
    { name: 'BsaI', cut_top: 100, cut_bottom: 104, forward: false },
    { name: 'RgaI', cut_top: 7763, cut_bottom: 7761, forward: null },
  ];
  const g = groupIsoschizomers(sites);
  assert.equal(g.length, 3);
  assert.equal(g[0]!.lead.name, 'AsiSI');
  assert.deepEqual(g[0]!.others.map((s) => s.name), ['SgfI', 'RgaI']);
  assert.equal(g[1]!.lead.forward, true);
  assert.equal(g[2]!.lead.forward, false);
  const g2 = groupIsoschizomers(sites, (s) => s.name === 'RgaI');
  assert.equal(g2[0]!.lead.name, 'RgaI');
});

console.log(`${passed} layout tests passed`);
