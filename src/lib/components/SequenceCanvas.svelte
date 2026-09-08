<script lang="ts">
  /**
   * The sequence field: a virtualised canvas, numbered rows, one selection.
   *
   * Only the rows in view are drawn, so a 200 kb plasmid scrolls as smoothly
   * as a 200 bp oligo. The parent owns the bases and the selection; typing
   * and deleting are reported through `onedit` and applied by the shell.
   */
  import { onMount } from 'svelte';
  import { hasSelection, selEnd, selStart, type FeatureView, type Selection } from '$lib/doc';

  interface Props {
    bases: string;
    features: FeatureView[];
    showFeatures: boolean;
    selection: Selection;
    locked: boolean;
    degenerate: boolean;
    /** Protein window: the amino-acid alphabet is accepted. */
    protein?: boolean;
    onselect: (s: Selection) => void;
    onedit: (from: number, to: number, text: string) => void;
    oncontextmenu?: (e: MouseEvent) => void;
  }
  let { bases, features, showFeatures, selection, locked, degenerate, protein = false, onselect, onedit, oncontextmenu }: Props = $props();

  const FONT = '13px ui-monospace, "SF Mono", Menlo, Consolas, monospace';
  const ROW_H = 22;
  const GUTTER_CHARS = 7;

  let container: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let width = $state(0);
  let height = $state(0);
  let scrollTop = $state(0);
  let charW = $state(8);
  let focused = $state(false);
  let dragging = false;

  const gutter = $derived(charW * GUTTER_CHARS + 10);
  const perLine = $derived(Math.max(10, Math.floor((width - gutter - 12) / charW / 10) * 10));
  const rows = $derived(Math.max(1, Math.ceil((bases.length + 1) / perLine)));
  const totalHeight = $derived(rows * ROW_H + 8);

  function measure() {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.font = FONT;
    charW = ctx.measureText('G').width;
  }

  function indexAt(x: number, y: number): number {
    const row = Math.max(0, Math.floor((y + scrollTop - 4) / ROW_H));
    const col = Math.max(0, Math.min(perLine, Math.round((x - gutter) / charW)));
    return Math.min(bases.length, row * perLine + col);
  }

  /** Scrolls so that the focus end of the selection is visible. */
  export function revealFocus() {
    const row = Math.floor(selection.focus / perLine);
    const top = row * ROW_H;
    if (top < scrollTop) container.scrollTop = top;
    else if (top + ROW_H > scrollTop + height) container.scrollTop = top + ROW_H - height + 8;
  }

  function draw() {
    const ctx = canvas.getContext('2d');
    if (!ctx || width === 0) return;
    const dpr = window.devicePixelRatio || 1;
    if (canvas.width !== Math.floor(width * dpr) || canvas.height !== Math.floor(height * dpr)) {
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const css = getComputedStyle(container);
    const fg = css.getPropertyValue('--fg').trim() || '#000';
    const muted = css.getPropertyValue('--fg-muted').trim() || '#888';
    const selBg = focused ? css.getPropertyValue('--accent').trim() || '#2f6fbf' : muted;
    ctx.clearRect(0, 0, width, height);
    ctx.font = FONT;
    ctx.textBaseline = 'middle';

    const first = Math.max(0, Math.floor((scrollTop - 4) / ROW_H));
    const last = Math.min(rows - 1, Math.ceil((scrollTop + height) / ROW_H));
    const s = selStart(selection);
    const e = selEnd(selection);
    const n = bases.length;

    // Feature bars (drawn first, under the text).
    const spans: { lo: number; hi: number; color: string }[] = [];
    if (showFeatures) {
      for (const f of features) {
        if (!f.show) continue;
        const a = f.forward ? f.start : f.end;
        const b = f.forward ? f.end : f.start;
        if (a <= b) spans.push({ lo: a - 1, hi: b, color: f.color });
        else {
          spans.push({ lo: a - 1, hi: n, color: f.color });
          spans.push({ lo: 0, hi: b, color: f.color });
        }
      }
    }

    for (let r = first; r <= last; r++) {
      const rowStart = r * perLine;
      const rowEnd = Math.min(n, rowStart + perLine);
      const y = 4 + r * ROW_H - scrollTop + ROW_H / 2;
      const x0 = gutter;

      for (const sp of spans) {
        const lo = Math.max(sp.lo, rowStart);
        const hi = Math.min(sp.hi, rowEnd);
        if (lo < hi) {
          ctx.fillStyle = sp.color;
          ctx.globalAlpha = 0.28;
          ctx.fillRect(x0 + (lo - rowStart) * charW, y - ROW_H / 2 + 2, (hi - lo) * charW, ROW_H - 4);
          ctx.globalAlpha = 1;
        }
      }

      if (s !== e) {
        const lo = Math.max(s, rowStart);
        const hi = Math.min(e, rowEnd);
        if (lo < hi) {
          ctx.fillStyle = selBg;
          ctx.globalAlpha = 0.35;
          ctx.fillRect(x0 + (lo - rowStart) * charW, y - ROW_H / 2 + 1, (hi - lo) * charW, ROW_H - 2);
          ctx.globalAlpha = 1;
        }
      }

      ctx.fillStyle = muted;
      ctx.textAlign = 'right';
      ctx.fillText(String(rowStart + 1), gutter - 10, y);
      ctx.textAlign = 'left';
      ctx.fillStyle = fg;
      if (rowEnd > rowStart) ctx.fillText(bases.slice(rowStart, rowEnd), x0, y);

      // Caret: exactly one row owns each index (the row after a full one
      // owns its end), so a caret at the very end still shows.
      if (s === e && focused && s >= rowStart && s - rowStart < perLine) {
        const cx = x0 + (s - rowStart) * charW;
        ctx.fillStyle = fg;
        ctx.fillRect(Math.round(cx), y - ROW_H / 2 + 2, 1.5, ROW_H - 4);
      }
    }
  }

  function setSelection(anchor: number, focus: number) {
    onselect({ anchor, focus });
  }

  function onMouseDown(e: MouseEvent) {
    if (e.button !== 0) return;
    container.focus();
    const rect = canvas.getBoundingClientRect();
    const idx = indexAt(e.clientX - rect.left, e.clientY - rect.top);
    if (e.shiftKey) setSelection(selection.anchor, idx);
    else setSelection(idx, idx);
    dragging = true;
    e.preventDefault();
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragging) return;
    const rect = canvas.getBoundingClientRect();
    const idx = indexAt(e.clientX - rect.left, e.clientY - rect.top);
    if (idx !== selection.focus) setSelection(selection.anchor, idx);
  }

  function onMouseUp() {
    dragging = false;
  }

  function accepts(ch: string): boolean {
    if (protein) return /^[A-Za-z*]$/.test(ch);
    return degenerate ? /^[ACGTRYSWKMBDHVNUacgtryswkmbdhvnu]$/.test(ch) : /^[ACGTacgt]$/.test(ch);
  }

  function onKeyDown(e: KeyboardEvent) {
    const n = bases.length;
    const s = selStart(selection);
    const en = selEnd(selection);
    const primary = e.metaKey || e.ctrlKey;
    const move = (to: number) => {
      to = Math.max(0, Math.min(n, to));
      if (e.shiftKey) setSelection(selection.anchor, to);
      else setSelection(to, to);
      queueMicrotask(revealFocus);
    };
    switch (e.key) {
      case 'ArrowLeft':
        move(primary ? 0 : hasSelection(selection) && !e.shiftKey ? s : selection.focus - 1);
        break;
      case 'ArrowRight':
        move(primary ? n : hasSelection(selection) && !e.shiftKey ? en : selection.focus + 1);
        break;
      case 'ArrowUp':
        move(selection.focus - perLine);
        break;
      case 'ArrowDown':
        move(selection.focus + perLine);
        break;
      case 'Home':
        move(primary ? 0 : Math.floor(selection.focus / perLine) * perLine);
        break;
      case 'End':
        move(primary ? n : Math.min(n, Math.floor(selection.focus / perLine) * perLine + perLine));
        break;
      case 'PageUp':
        move(selection.focus - perLine * Math.max(1, Math.floor(height / ROW_H) - 1));
        break;
      case 'PageDown':
        move(selection.focus + perLine * Math.max(1, Math.floor(height / ROW_H) - 1));
        break;
      case 'Backspace':
        if (locked) return;
        if (hasSelection(selection)) onedit(s, en, '');
        else if (selection.focus > 0) onedit(selection.focus - 1, selection.focus, '');
        break;
      case 'Delete':
        if (locked) return;
        if (hasSelection(selection)) onedit(s, en, '');
        else if (selection.focus < n) onedit(selection.focus, selection.focus + 1, '');
        break;
      case 'Escape':
        setSelection(selection.focus, selection.focus);
        break;
      default:
        if (primary || e.altKey || e.key.length !== 1) return;
        if (locked || !accepts(e.key)) return;
        onedit(s, en, e.key);
    }
    e.preventDefault();
  }

  onMount(() => {
    measure();
    const ro = new ResizeObserver(() => {
      width = container.clientWidth;
      height = container.clientHeight;
    });
    ro.observe(container);
    width = container.clientWidth;
    height = container.clientHeight;
    return () => ro.disconnect();
  });

  $effect(() => {
    // Redraw on any input change.
    void bases;
    void features;
    void showFeatures;
    void selection;
    void width;
    void height;
    void scrollTop;
    void focused;
    void charW;
    draw();
  });
</script>

<svelte:window onmouseup={onMouseUp} onmousemove={onMouseMove} />

<div
  class="seqview"
  bind:this={container}
  tabindex="0"
  role="textbox"
  aria-multiline="true"
  aria-readonly={locked}
  aria-label="Sequence"
  onscroll={() => (scrollTop = container.scrollTop)}
  onmousedown={onMouseDown}
  onkeydown={onKeyDown}
  onfocus={() => (focused = true)}
  onblur={() => (focused = false)}
  oncontextmenu={(e) => {
    e.preventDefault();
    oncontextmenu?.(e);
  }}
>
  <div class="spacer" style:height="{totalHeight}px"></div>
  <canvas bind:this={canvas} style:top="{scrollTop}px" style:width="{width}px" style:height="{height}px"></canvas>
</div>

<style>
  .seqview {
    position: relative;
    flex: 1;
    min-height: 120px;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    outline: none;
    cursor: text;
  }
  .seqview:focus {
    border-color: var(--accent);
  }
  .spacer {
    width: 1px;
  }
  canvas {
    position: absolute;
    left: 0;
    pointer-events: none;
  }
</style>
