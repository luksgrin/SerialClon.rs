<script lang="ts">
  /**
   * The Sequence Window (User Manual §IV.A, reference/ui/01-sequence-window.png):
   * name, size, type, format, topology, lock, from/to/length spinners, the
   * Sequence / Comments / Features / Extremities tabs, the sequence field,
   * and the Save / Sequence Map / Graphic Map / Find / Show Info buttons.
   */
  import { onMount } from 'svelte';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import { emitTo } from '@tauri-apps/api/event';
  import { Menu } from '@tauri-apps/api/menu';
  import { message, confirm } from '@tauri-apps/plugin-dialog';
  import SequenceCanvas from '$lib/components/SequenceCanvas.svelte';
  import {
    api,
    docIdFromUrl,
    fastaOf,
    formatLines,
    hasSelection,
    onDocChanged,
    onMenu,
    onSelectRequest,
    selEnd,
    selStart,
    type DocumentView,
    type EditOp,
    type EndsView,
    type Selection,
    type SelectionInfo,
    type FeatureView,
    FEATURE_KINDS,
  } from '$lib/doc';
  import { prefs } from '$lib/prefs';

  const id = docIdFromUrl();
  const win = getCurrentWindow();

  let doc = $state<DocumentView | null>(null);
  let sel = $state<Selection>({ anchor: 0, focus: 0 });
  let tab = $state<'sequence' | 'comments' | 'features' | 'extremities'>('sequence');
  let showFeatures = $state(true);
  let showInfo = $state(false);
  let firstOrf = $state(true);
  let info = $state<SelectionInfo | null>(null);
  let renaming = $state(false);
  let renameValue = $state('');
  let commentDraft = $state('');
  let ends = $state<EndsView>({ left_length: 0, left_bases: '', right_length: 0, right_bases: '' });
  let canvas = $state<SequenceCanvas | undefined>();
  let renameInput = $state<HTMLInputElement | null>(null);
  let editMode = $state(false);
  let selectedFeature = $state(-1);
  let draft = $state<FeatureView | null>(null);
  let altHeld = $state(false);
  let dragIndex = -1;

  const INFO_HEIGHT = 200;
  const length = $derived(doc?.bases.length ?? 0);
  const start = $derived(selStart(sel));
  const end = $derived(selEnd(sel));
  const selected = $derived(hasSelection(sel));
  const currentFeature = $derived.by(() => {
    if (!doc) return null;
    const pos = sel.focus + 1;
    return (
      doc.features.find((f) => {
        const a = f.forward ? f.start : f.end;
        const b = f.forward ? f.end : f.start;
        return a <= b ? pos >= a && pos <= b : pos >= a || pos <= b;
      }) ?? null
    );
  });

  function apply(d: DocumentView) {
    doc = d;
    commentDraft = d.comment;
    ends = { ...d.ends };
    const n = d.bases.length;
    if (sel.anchor > n || sel.focus > n) sel = { anchor: Math.min(sel.anchor, n), focus: Math.min(sel.focus, n) };
  }

  async function edit(op: EditOp): Promise<boolean> {
    if (!doc) return false;
    try {
      const r = await api.edit(id, op);
      apply(r.view);
      if (r.filtered > 0) await message(`${r.filtered} non-${doc.degenerate ? 'IUPAC' : 'ACGT'} characters were filtered out.`, { title: 'Characters filtered', kind: 'warning' });
      return true;
    } catch (e) {
      await message(String(e), { title: 'Cannot edit', kind: 'error' });
      return false;
    }
  }

  async function onEdit(from: number, to: number, text: string) {
    if (await edit({ op: 'replace', from, to, text })) {
      const caret = from + text.length;
      sel = { anchor: caret, focus: caret };
      canvas?.revealFocus();
    }
  }

  function setSel(s: Selection) {
    sel = s;
  }

  function select(from: number, to: number) {
    sel = { anchor: Math.max(0, Math.min(length, from)), focus: Math.max(0, Math.min(length, to)) };
    canvas?.revealFocus();
  }

  // --- spinners -----------------------------------------------------------
  function onFrom(v: string) {
    const n = Number(v);
    if (!Number.isFinite(n) || v === '') return;
    const s = Math.max(0, Math.min(length, Math.floor(n) - 1));
    select(s, Math.max(s, selected ? end : s));
  }
  function onTo(v: string) {
    const n = Number(v);
    if (!Number.isFinite(n) || v === '') return;
    const e = Math.max(0, Math.min(length, Math.floor(n)));
    select(Math.min(start, e), e);
  }
  function onLength(v: string) {
    const n = Number(v);
    if (!Number.isFinite(n) || v === '') return;
    select(start, Math.max(start, Math.min(length, start + Math.floor(n))));
  }

  // --- info panel ---------------------------------------------------------
  let infoTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    void sel;
    void doc;
    void firstOrf;
    if (!showInfo || !doc) return;
    clearTimeout(infoTimer);
    const [f, t] = selected ? [start, end] : [0, 0];
    infoTimer = setTimeout(async () => {
      try {
        info = selected ? await api.selectionInfo(id, f, t, firstOrf) : null;
      } catch {
        info = null;
      }
    }, 80);
  });

  async function toggleInfo() {
    showInfo = !showInfo;
    try {
      const size = await win.innerSize();
      const scale = await win.scaleFactor();
      const h = size.height / scale + (showInfo ? INFO_HEIGHT : -INFO_HEIGHT);
      await win.setSize(new LogicalSize(size.width / scale, h));
    } catch {
      /* browser */
    }
  }

  /** Back-selection: selecting residues in the peptide field selects their codons. */
  function onPeptideSelect(e: Event) {
    const ta = e.currentTarget as HTMLTextAreaElement;
    if (!info || ta.selectionStart === ta.selectionEnd) return;
    const base = start;
    const per = info.protein ? 1 : 3;
    select(base + ta.selectionStart * per, Math.min(end, base + ta.selectionEnd * per));
  }

  /** The peptide field's context menu: a Protein window or a FASTA copy of the translation. */
  async function peptideMenu() {
    if (!doc || !info) return;
    const [f, t] = selected ? [start, end] : [0, length];
    const menu = await Menu.new({
      items: [
        { id: 'open', text: 'Open as Protein Sequence', enabled: !info.protein, action: () => void api.peptideToDocument(id, f, t, firstOrf) },
        { id: 'copy', text: 'Copy Peptide as Fasta', action: () => void api.clipboardWrite(`> [${doc!.name}] translation ${f + 1}-${t} (${info!.genetic_code})\n${formatLines(info!.peptide)}`) },
        { id: 'raw', text: 'Copy Non Formatted', action: () => void api.clipboardWrite(info!.peptide) },
      ],
    });
    await menu.popup();
  }

  // --- clipboard ----------------------------------------------------------
  function activeTextField(): HTMLInputElement | HTMLTextAreaElement | null {
    const el = document.activeElement;
    return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement ? el : null;
  }

  async function copySelection(kind: 'plain' | 'fasta' | 'raw') {
    if (!doc) return;
    const [f, t] = selected ? [start, end] : [0, length];
    const text = kind === 'fasta' ? fastaOf(doc, f, t) : kind === 'raw' ? doc.bases.slice(f, t) : formatLines(doc.bases.slice(f, t));
    await api.clipboardWrite(text);
  }

  async function pasteClipboard() {
    if (!doc) return;
    let text = '';
    try {
      text = await api.clipboardRead();
    } catch {
      return;
    }
    if (!text) return;
    try {
      const r = await api.paste(id, start, end, text);
      const inserted = r.view.bases.length - doc.bases.length + (end - start);
      apply(r.view);
      const caret = start + Math.max(0, inserted);
      sel = { anchor: caret, focus: caret };
      if (r.filtered > 0) await message(`${r.filtered} characters were filtered out while pasting.`, { title: 'Characters filtered', kind: 'warning' });
    } catch (e) {
      await message(String(e), { title: 'Cannot paste', kind: 'error' });
    }
  }

  // --- menu commands ------------------------------------------------------
  async function transform(op: 'uppercase' | 'lowercase' | 'antiparallel' | 'reverse' | 'complement') {
    if (!doc) return;
    const [f, t] = selected ? [start, end] : [0, length];
    if (op !== 'uppercase' && op !== 'lowercase' && selected && !(f === 0 && t === length)) {
      const ok = await confirm(`Apply ${op} to the selected portion only (${f + 1}–${t})?`, { title: 'Partial selection', kind: 'warning' });
      if (!ok) return;
    }
    if (await edit({ op, from: f, to: t })) select(f, t);
  }

  async function findNextOrf() {
    if (!doc) return;
    const orfs = (await api.findOrfs(id, 1)).filter((o) => o.forward).sort((a, b) => a.start - b.start);
    if (!orfs.length) return;
    const next = orfs.find((o) => o.start - 1 > selection0()) ?? orfs[0];
    if (next) select(next.start - 1, next.end);
  }
  const selection0 = () => (selected ? start : sel.focus);

  async function handleMenu(cmd: string) {
    if (!doc) return;
    const field = activeTextField();
    switch (cmd) {
      case 'edit.undo':
        if (field) document.execCommand('undo');
        else apply(await api.undo(id));
        return;
      case 'edit.redo':
        if (field) document.execCommand('redo');
        else apply(await api.redo(id));
        return;
      case 'edit.select_all':
        if (field) field.select();
        else select(0, length);
        return;
      case 'edit.copy':
        if (field) await api.clipboardWrite(field.value.slice(field.selectionStart ?? 0, field.selectionEnd ?? 0));
        else await copySelection('plain');
        return;
      case 'edit.cut':
        if (field) {
          await api.clipboardWrite(field.value.slice(field.selectionStart ?? 0, field.selectionEnd ?? 0));
          field.setRangeText('', field.selectionStart ?? 0, field.selectionEnd ?? 0, 'end');
          field.dispatchEvent(new Event('input', { bubbles: true }));
        } else if (selected && !doc.locked) {
          await copySelection('plain');
          await onEdit(start, end, '');
        }
        return;
      case 'edit.paste':
        if (field) {
          const text = await api.clipboardRead();
          field.setRangeText(text, field.selectionStart ?? 0, field.selectionEnd ?? 0, 'end');
          field.dispatchEvent(new Event('input', { bubbles: true }));
        } else await pasteClipboard();
        return;
      case 'edit.copy_fasta':
        await copySelection('fasta');
        return;
      case 'edit.copy_formatted':
        await copySelection('raw');
        return;
      case 'edit.find_orf':
        await findNextOrf();
        return;
      case 'seq.info':
        await toggleInfo();
        return;
      case 'seq.copy_translation': {
        const [f, t] = selected ? [start, end] : [0, length];
        const i = await api.selectionInfo(id, f, t, firstOrf);
        await api.clipboardWrite(`> [${doc.name}] translation of ${f + 1}-${t}\n${formatLines(i.peptide)}`);
        return;
      }
      case 'seq.uppercase':
      case 'seq.lowercase':
      case 'seq.antiparallel':
      case 'seq.reverse':
      case 'seq.complement':
        await transform(cmd.slice(4) as 'uppercase');
        return;
      case 'seq.topology':
        await edit({ op: 'set_topology', circular: !doc.circular });
        return;
      case 'seq.set_origin':
        if (doc.circular) await edit({ op: 'set_origin', position: sel.focus + 1 });
        else await message('Set Origin applies to circular sequences.', { title: 'Set Origin' });
        return;
      case 'seq.degenerate':
        await edit({ op: 'set_degenerate', degenerate: !doc.degenerate });
        return;
      case 'protein.reverse_translate':
        try {
          await api.reverseTranslate(id, selected ? start : 0, selected ? end : 0);
        } catch (e) {
          await message(String(e), { title: 'Reverse Translate' });
        }
        return;
      case 'protein.blast':
      case 'seq.blast': {
        // Forwarded here only when the selection matters; the shell opens Web Access.
        await api.toolbarAction(cmd);
        return;
      }
      case 'seq.rename':
        startRename();
        return;
      case 'seq.read':
        readAloud();
        return;
      case 'restriction.sequence_map':
        await api.openSequenceMap(id, selected ? start + 1 : null, selected ? end : null);
        return;
      case 'generate.extract':
        if (selected) await api.extractSelection(id, start, end);
        else await message('Select a portion of the sequence first, or choose sites on the Graphic Map.', { title: 'Extract Fragment' });
        return;
      case 'tools.silent_map':
        if (selected) await api.openToolWindow('silent', id, start + 1, end);
        else await message('Select the reading frame (an ORF) to scan for silent sites first.', { title: 'Silent Map' });
        return;
      case 'file.print':
        await message('Print is available for the Graphic Map and the Sequence Map.', { title: 'Print' });
        return;
      case 'features.from_selection':
        if (selected) await addFeature();
        else await message('Select a portion of the sequence first.', { title: 'New Feature From Selection' });
        return;
    }
  }

  // --- context menus ------------------------------------------------------
  const COMP: Record<string, string> = { A: 'T', T: 'A', C: 'G', G: 'C', a: 't', t: 'a', c: 'g', g: 'c', N: 'N', n: 'n' };
  const revcomp = (s: string) => s.split('').reverse().map((c) => COMP[c] ?? c).join('');

  async function sequenceContextMenu() {
    if (!doc) return;
    const pcrOpen = await api.pcrWindowOpen().catch(() => false);
    const menu = await Menu.new({
      items: [
        { id: 'copy', text: 'Copy', enabled: selected, action: () => void copySelection('plain') },
        { id: 'copy_fasta', text: 'Copy as Fasta', enabled: selected, action: () => void copySelection('fasta') },
        { id: 'paste', text: 'Paste', enabled: !doc.locked, action: () => void pasteClipboard() },
        { item: 'Separator' },
        { id: 'upper', text: 'UPPERCASE', enabled: selected && !doc.locked, action: () => void transform('uppercase') },
        { id: 'lower', text: 'Lowercase', enabled: selected && !doc.locked, action: () => void transform('lowercase') },
        { id: 'anti', text: 'Antiparallel', enabled: selected && !doc.locked, action: () => void transform('antiparallel') },
        { item: 'Separator' },
        { id: 'extract', text: 'Extract Fragment', enabled: selected, action: () => void api.extractSelection(id, start, end) },
        { id: 'feature', text: 'Create New Feature From Selection', enabled: selected, action: () => void addFeature() },
        { item: 'Separator' },
        { id: 'all', text: 'Select All', action: () => select(0, length) },
        ...(pcrOpen && selected
          ? [
              { item: 'Separator' as const },
              { id: 'pfwd', text: 'Use selection as forward primer', action: () => void emitTo('pcr', 'pcr:primer', { which: 'forward', text: doc?.bases.slice(start, end) ?? '' }) },
              { id: 'prev', text: 'Use selection as reverse primer', action: () => void emitTo('pcr', 'pcr:primer', { which: 'reverse', text: revcomp(doc?.bases.slice(start, end) ?? '') }) },
            ]
          : []),
      ],
    });
    await menu.popup();
  }

  async function topologyMenu() {
    if (!doc) return;
    const menu = await Menu.new({
      items: [{ id: 'topo', text: doc.circular ? 'Linearize' : 'Circularize', action: () => void edit({ op: 'set_topology', circular: !doc!.circular }) }],
    });
    await menu.popup();
  }

  async function nameMenu() {
    const menu = await Menu.new({
      items: [
        { id: 'rename', text: 'Rename Sequence…', action: startRename },
        { id: 'reveal', text: 'Show File in Finder / Explorer', enabled: !!doc?.path, action: () => void api.revealFile(id) },
      ],
    });
    await menu.popup();
  }

  function startRename() {
    renameValue = doc?.name ?? '';
    renaming = true;
    queueMicrotask(() => renameInput?.select());
  }
  async function finishRename(commit: boolean) {
    renaming = false;
    if (commit && doc && renameValue.trim() && renameValue.trim() !== doc.name) await edit({ op: 'rename', name: renameValue.trim() });
  }

  // --- tabs ----------------------------------------------------------------
  let commentTimer: ReturnType<typeof setTimeout> | undefined;
  function onCommentInput() {
    clearTimeout(commentTimer);
    commentTimer = setTimeout(() => {
      if (doc && commentDraft !== doc.comment) void edit({ op: 'set_comment', text: commentDraft });
    }, 400);
  }

  function selectFeature(i: number) {
    const f = doc?.features[i];
    if (!f) return;
    const a = f.forward ? f.start : f.end;
    const b = f.forward ? f.end : f.start;
    tab = 'sequence';
    select(a - 1, b);
  }

  async function applyEnds() {
    await edit({ op: 'set_ends', ends: { ...ends, left_bases: ends.left_bases.toUpperCase(), right_bases: ends.right_bases.toUpperCase() } });
  }
  function endKind(len: number): 'blunt' | '5' | '3' {
    return len === 0 ? 'blunt' : len > 0 ? '5' : '3';
  }
  function setEndKind(side: 'left' | 'right', kind: string) {
    const bases = side === 'left' ? ends.left_bases : ends.right_bases;
    const len = kind === 'blunt' ? 0 : kind === '5' ? Math.max(1, bases.length) : -Math.max(1, bases.length);
    if (side === 'left') ends.left_length = len;
    else ends.right_length = len;
  }
  function setEndBases(side: 'left' | 'right', v: string) {
    const b = v.replace(/[^A-Za-z]/g, '');
    if (side === 'left') {
      ends.left_bases = b;
      if (ends.left_length !== 0) ends.left_length = Math.sign(ends.left_length) * Math.max(1, b.length);
    } else {
      ends.right_bases = b;
      if (ends.right_length !== 0) ends.right_length = Math.sign(ends.right_length) * Math.max(1, b.length);
    }
  }

  // --- read selection aloud (Sequence ▸ Read selection, Esc stops) ----------
  function readAloud() {
    if (!doc || typeof speechSynthesis === 'undefined') return;
    const [f, t] = selected ? [start, end] : [sel.focus, Math.min(length, sel.focus + 60)];
    const text = doc.bases.slice(f, t).toUpperCase().split('').join(' ');
    if (!text) return;
    speechSynthesis.cancel();
    const u = new SpeechSynthesisUtterance(text);
    u.rate = $prefs.read_speed;
    speechSynthesis.speak(u);
  }
  function stopReading() {
    if (typeof speechSynthesis !== 'undefined') speechSynthesis.cancel();
  }

  // --- features tab -------------------------------------------------------
  function pickFeature(i: number) {
    selectedFeature = i;
    draft = doc?.features[i] ? { ...doc.features[i] } : null;
  }
  async function addFeature() {
    if (!doc) return;
    const translation = showInfo && info && selected ? info.peptide : null;
    const [f, t] = selected ? [start, end] : [0, length];
    const v = await api.newFeatureFromSelection(id, f, t, translation);
    apply(v);
    editMode = true;
    tab = 'features';
    pickFeature(v.features.length - 1);
  }
  async function deleteFeature() {
    if (!doc || selectedFeature < 0) return;
    const f = doc.features[selectedFeature];
    if (!f) return;
    if (!altHeld) {
      const ok = await confirm(`Delete the Feature "${f.name}"? There is no undo for this in the original; here Edit ▸ Undo brings it back.`, { title: 'Delete Feature', kind: 'warning' });
      if (!ok) return;
    }
    await edit({ op: 'delete_feature', index: selectedFeature });
    pickFeature(Math.min(selectedFeature, (doc?.features.length ?? 1) - 1));
  }
  async function applyDraft() {
    if (!draft || selectedFeature < 0) return;
    await edit({ op: 'update_feature', index: selectedFeature, feature: { ...draft, start: Math.max(1, Math.min(length, draft.start)), end: Math.max(1, Math.min(length, draft.end)) } });
    pickFeature(selectedFeature);
  }
  async function scan() {
    try {
      const r = await api.scanFeatures(id);
      tab = 'features';
      if (r.found === 0) await message('No Feature of the enabled Collections was found in this sequence.', { title: 'Scan' });
    } catch (e) {
      await message(String(e), { title: 'Scan', kind: 'error' });
    }
  }
  async function dropOn(i: number) {
    if (dragIndex < 0 || dragIndex === i) return;
    await edit({ op: 'move_feature', from: dragIndex, to: i });
    pickFeature(i);
    dragIndex = -1;
  }
  function onWindowKey(e: KeyboardEvent) {
    altHeld = e.altKey;
    if (e.type === 'keydown' && e.key === 'Escape') stopReading();
    if (e.type === 'keydown' && tab === 'features' && e.key.toLowerCase() === 's' && !e.metaKey && !e.ctrlKey && !e.altKey && !activeTextField()) {
      showFeatures = !showFeatures;
      e.preventDefault();
    }
  }

  // --- buttons ------------------------------------------------------------
  async function save(saveAs: boolean) {
    const v = await api.save(id, saveAs);
    if (v) apply(v);
  }

  onMount(() => {
    const unlisteners: (() => void)[] = [];
    (async () => {
      try {
        apply(await api.getDocument(id));
      } catch {
        return;
      }
      unlisteners.push(await onDocChanged(id, apply));
      unlisteners.push(await onMenu((cmd) => void handleMenu(cmd)));
      unlisteners.push(await onSelectRequest((r) => select(r.from, r.to)));
      void api.noteFocus(id);
    })();
    window.addEventListener('focus', () => void api.noteFocus(id));
    return () => unlisteners.forEach((u) => u());
  });
</script>

<svelte:window onkeydown={onWindowKey} onkeyup={onWindowKey} />

{#if doc}
  <div class="window" class:with-info={showInfo}>
    <header class="head">
      <div class="field name" data-help-name="Sequence name" data-help="The name of the opened sequence. Right-click to rename it or to show the file on disk." oncontextmenu={(e) => { e.preventDefault(); void nameMenu(); }} role="group">
        <span class="label">File Name</span>
        {#if renaming}
          <input bind:this={renameInput} bind:value={renameValue} onblur={() => finishRename(true)} onkeydown={(e) => { if (e.key === 'Enter') finishRename(true); if (e.key === 'Escape') finishRename(false); }} />
        {:else}
          <span class="value" ondblclick={startRename} role="button" tabindex="-1">{doc.name}{doc.dirty ? ' •' : ''}</span>
        {/if}
      </div>
      <div class="field" data-help-name="Length" data-help="The size of the sequence, in nucleotides.">
        <span class="label">Length</span><span class="value num">{length}</span>
      </div>
      <div class="field" data-help-name="Type" data-help="The type as defined by the DNA Strider format: DNA, Degenerate DNA, RNA or Protein.">
        <span class="label">Type</span><span class="value">{doc.seq_type}</span>
      </div>
      <div class="field" data-help-name="Format" data-help="The file format the sequence was read from: DNA Strider, Serial Cloner, FASTA, GenBank or text.">
        <span class="label">Format</span><span class="value">{doc.source}</span>
      </div>
      <div class="field topology" data-help-name="Topology" data-help="Topology of the sequence. Right-click (or click) to display a menu allowing to change it: Linearize / Circularize." oncontextmenu={(e) => { e.preventDefault(); void topologyMenu(); }} role="group">
        <span class="label">Topology</span>
        <button type="button" class="link" onclick={topologyMenu}>{doc.circular ? 'Circular' : 'Linear'}</button>
      </div>
      <label class="field lock" data-help-name="Locked" data-help="Use this checkbox to lock or unlock the sequence. A locked sequence cannot be inadvertently modified.">
        <input type="checkbox" checked={doc.locked} onchange={(e) => void edit({ op: 'set_locked', locked: e.currentTarget.checked })} />
        Locked
      </label>
    </header>

    <div class="selection-row">
      <div class="nav">
        <button type="button" data-help-name="Go to beginning" data-help="Use these arrows to go directly at the beginning or at the end of the sequence." onclick={() => select(0, 0)}>⇤</button>
        <button type="button" data-help-name="Go to end" data-help="Use these arrows to go directly at the beginning or at the end of the sequence." onclick={() => select(length, length)}>⇥</button>
      </div>
      <label data-help-name="from" data-help="When a selection is made, indicates the position of the first nucleotide selected. A value can be directly entered, or the up and down arrows used, to set the beginning of the selection. With no selection, indicates the insertion point.">
        from
        <input type="number" min="1" max={length} value={selected ? start + 1 : sel.focus + 1} onchange={(e) => onFrom(e.currentTarget.value)} />
      </label>
      <label data-help-name="to" data-help="When a selection is made, indicates the position of the last nucleotide selected. A value can be directly entered, or the up and down arrows used, to set the end of the selection.">
        to
        <input type="number" min="1" max={length} value={selected ? end : ''} onchange={(e) => onTo(e.currentTarget.value)} />
      </label>
      <label data-help-name="length" data-help="When a selection is made, indicates the length of the selected fragment. A value can be directly entered, or the up and down arrows used, to set the length of the selection.">
        length
        <input type="number" min="0" max={length} value={selected ? end - start : ''} onchange={(e) => onLength(e.currentTarget.value)} />
      </label>
      <span class="feature-name" data-help-name="Current feature" data-help="When the insertion point is located within a Feature, the corresponding Feature's name is indicated here.">
        {currentFeature ? currentFeature.name : ''}
      </span>
      <label class="show-features" data-help-name="Show Features" data-help="Show or hide Features (coloured) in the sequence field.">
        <input type="checkbox" bind:checked={showFeatures} /> Show Features
      </label>
    </div>

    <div class="tabs" role="tablist">
      <button type="button" role="tab" aria-selected={tab === 'sequence'} onclick={() => (tab = 'sequence')} data-help-name="Sequence tab" data-help="Use this tab to show the nucleotide sequence.">Sequence</button>
      <button type="button" role="tab" aria-selected={tab === 'comments'} onclick={() => (tab = 'comments')} data-help-name="Comments tab" data-help="Use this tab to show the comments associated to the sequence. DNA Strider limits comments to 255 characters.">Comments</button>
      <button type="button" role="tab" aria-selected={tab === 'features'} onclick={() => (tab = 'features')} data-help-name="Features tab" data-help="Use this tab to see the Features of the sequence. Double-click a Feature to select its nucleotides.">Features</button>
      <button type="button" role="tab" aria-selected={tab === 'extremities'} onclick={() => (tab = 'extremities')} data-help-name="Extremities tab" data-help="Use this tab to access the extremities of a linear insert. Cohesive ends can be directly modified here.">Extremities</button>
    </div>

    <section class="pane">
      {#if tab === 'sequence'}
        <div class="seq-wrap" data-help-name="Sequence Field" data-help="Enter here the actual sequence of the fragment (ATGC letters only are accepted, other letters are filtered out). Ctrl-Click or Right-click displays a contextual menu.">
          <SequenceCanvas
            bind:this={canvas}
            bases={doc.bases}
            features={doc.features}
            {showFeatures}
            selection={sel}
            locked={doc.locked}
            degenerate={doc.degenerate}
            fontSize={$prefs.seq_font_size}
            protein={doc.seq_type === 'Protein'}
            onselect={setSel}
            onedit={onEdit}
            oncontextmenu={() => void sequenceContextMenu()}
          />
        </div>
      {:else if tab === 'comments'}
        <div class="comments">
          <textarea bind:value={commentDraft} oninput={onCommentInput} spellcheck="false" data-help-name="Comments" data-help="The comments respect the Strider format and are thus limited to 255 characters when read by DNA Strider. Serial Cloner files keep longer text."></textarea>
          <div class="hint">
            {commentDraft.length} characters{commentDraft.length > 255 ? ' — DNA Strider will only read the first 255' : ''}
          </div>
        </div>
      {:else if tab === 'features'}
        <div class="features-pane">
          <div class="features" data-help-name="Feature list" data-help="The Features of the sequence, in drawing order. Double-click one to select its nucleotides. In Edit mode, click to edit it, drag to reorder (the order dictates how overlapping Features are drawn), + and − add and delete. Shown Features are listed in bold.">
            <table>
              <thead><tr><th>Show</th><th>Name</th><th>Type</th><th>From</th><th>To</th><th>Strand</th><th></th></tr></thead>
              <tbody>
                {#each doc.features as f, i (i)}
                  <tr
                    class:hidden-feature={!f.show}
                    class:selected={editMode && selectedFeature === i}
                    draggable={editMode}
                    ondragstart={() => (dragIndex = i)}
                    ondragover={(e) => e.preventDefault()}
                    ondrop={() => void dropOn(i)}
                    onclick={() => { if (editMode) pickFeature(i); }}
                    ondblclick={() => selectFeature(i)}
                  >
                    <td><input type="checkbox" checked={f.show} onclick={(e) => { if (e.altKey) { e.preventDefault(); const all = !f.show; doc?.features.forEach((_, k) => void edit({ op: 'set_feature_show', index: k, show: all })); } }} onchange={(e) => void edit({ op: 'set_feature_show', index: i, show: e.currentTarget.checked })} /></td>
                    <td class:bold={f.show}>{f.name}</td>
                    <td>{f.kind}</td>
                    <td class="num">{f.start}</td>
                    <td class="num">{f.end}</td>
                    <td>{f.forward ? 'CW' : 'CCW'}</td>
                    <td><span class="swatch" style:background={f.color}></span></td>
                  </tr>
                {:else}
                  <tr><td colspan="7" class="empty">No Features. Press Scan, or + to add one from the selection.</td></tr>
                {/each}
              </tbody>
            </table>
          </div>
          <div class="feature-tools">
            <button type="button" class:active={editMode} onclick={() => { editMode = !editMode; if (!editMode) pickFeature(-1); }} data-help-name="Edit" data-help="Toggles between the protected mode, where no unwanted modification can be done, and the Edit mode.">{editMode ? 'Done' : 'Edit'}</button>
            <button type="button" onclick={addFeature} disabled={!editMode || doc.locked} data-help-name="+" data-help="Adds a Feature. The coordinates of the current selection are proposed; with the Selection Information shown, its translation goes to the comments.">+</button>
            <button type="button" class:danger={altHeld} onclick={deleteFeature} disabled={!editMode || selectedFeature < 0} data-help-name="−" data-help="Deletes the selected Feature after confirmation. Press [Alt] before clicking (the button turns red) to delete without confirmation.">−</button>
            <button type="button" onclick={scan} disabled={doc.locked} data-help-name="Scan" data-help="Scans the sequence against the enabled Feature Collections. Features you modified are protected and kept.">Scan</button>
            <button type="button" onclick={() => void api.openReferenceAny('features')} data-help-name="Manage" data-help="Opens the Feature Collection window to add, delete or modify the Features to scan for.">Manage…</button>
            <span class="hint">S shows/hides Features</span>
          </div>
          {#if editMode && draft}
            <div class="feature-editor" data-help-name="Feature editor" data-help="Modify the selected Feature: name, type, coordinates (first and last base in the direction of the Feature), strand, arrow, colour and comments, then Apply. A modified Feature is protected from re-scans.">
              <label>Name <input type="text" bind:value={draft.name} /></label>
              <label>Type <select bind:value={draft.kind}>{#each FEATURE_KINDS as k (k)}<option value={k}>{k}</option>{/each}</select></label>
              <label>From <input type="number" min="1" max={length} bind:value={draft.start} /></label>
              <label>To <input type="number" min="1" max={length} bind:value={draft.end} /></label>
              <label><input type="radio" name="strand" checked={draft.forward} onchange={() => { if (draft) draft.forward = true; }} /> CW</label>
              <label><input type="radio" name="strand" checked={!draft.forward} onchange={() => { if (draft) draft.forward = false; }} /> CCW</label>
              <label><input type="checkbox" bind:checked={draft.show} /> Show</label>
              <label><input type="checkbox" bind:checked={draft.arrow} /> Arrow</label>
              <label>Colour <input type="color" bind:value={draft.color} /></label>
              <label class="wide">Comments <input type="text" bind:value={draft.description} placeholder="qualifier=value, separated by returns in files" /></label>
              <button type="button" onclick={applyDraft}>Apply</button>
            </div>
          {/if}
        </div>
      {:else}
        <div class="ends" data-help-name="Extremities" data-help="Visualisation and modification of the cohesive ends of a linear fragment: for example add a protruding A at the extremity of a PCR fragment.">
          {#if doc.circular}
            <p class="hint">A circular sequence has no extremities. Linearize it to edit cohesive ends.</p>
          {:else}
            {#each [['left', "5' end (left)"], ['right', "3' end (right)"]] as [side, title] (side)}
              <fieldset>
                <legend>{title}</legend>
                <select value={endKind(side === 'left' ? ends.left_length : ends.right_length)} onchange={(e) => setEndKind(side as 'left', e.currentTarget.value)}>
                  <option value="blunt">Blunt</option>
                  <option value="5">5' overhang</option>
                  <option value="3">3' overhang</option>
                </select>
                <input type="text" placeholder="protruding bases" value={side === 'left' ? ends.left_bases : ends.right_bases} disabled={(side === 'left' ? ends.left_length : ends.right_length) === 0} oninput={(e) => setEndBases(side as 'left', e.currentTarget.value)} />
              </fieldset>
            {/each}
            <button type="button" onclick={applyEnds} disabled={doc.locked}>Apply</button>
          {/if}
        </div>
      {/if}
    </section>

    <footer class="buttons">
      <button type="button" data-help-name="Save" data-help="Save or Save As… the sequence. Press [Alt] before clicking to Save As… and change the name of the sequence." onclick={(e) => void save(e.altKey || !doc?.path)} disabled={!doc.dirty && !!doc.path}>
        {doc.path ? 'Save' : 'Save As…'}
      </button>
      <button type="button" data-help-name="Sequence Map" data-help="Shows the textual sequence map (restriction map and translation) in a separate window. If a selection has been made, its coordinates are used to translate only this selection." onclick={() => void api.openSequenceMap(id, selected ? start + 1 : null, selected ? end : null)}>Sequence Map</button>
      <button type="button" data-help-name="Graphic Map" data-help="Shows the Graphic map of the sequence in a separate window." onclick={() => void api.openGraphicMap(id)}>Graphic Map</button>
      <button type="button" data-help-name="Find" data-help="Opens the Find drawer: search for a sub-sequence, a restriction site or an ORF. Press again to close it." onclick={() => void api.toggleFindWindow(id)}>Find</button>
      <button type="button" class="info-toggle" data-help-name="Show Selection Information & Translation" data-help="Shows additional data about the selection: Tm, %GC, base counts, translation, MW." onclick={toggleInfo}>
        {showInfo ? 'Hide' : 'Show'} Selection Information & Translation
      </button>
    </footer>

    {#if showInfo}
      <section class="info">
        {#if doc.seq_type !== 'Protein'}
          <div class="info-block" data-help-name="Selected DNA" data-help="Information about the selected DNA: Tm, number of A, C, G and T, and %GC. Tm uses the three formulas of the manual with [Na+] = 50 mM.">
            <h2>Selected DNA</h2>
            {#if info}
              <div class="stats">
                <span>Tm: <b>{info.tm.toFixed(1)}</b></span>
                <span>A: {info.a}</span><span>C: {info.c}</span><span>G: {info.g}</span><span>T: {info.t}</span>
                <span>%GC: <b>{info.gc_percent.toFixed(1)}%</b></span>
              </div>
            {:else}
              <div class="stats hint">Select a portion of the sequence.</div>
            {/if}
          </div>
        {/if}
        <div class="info-block" data-help-name={doc.seq_type === 'Protein' ? 'Selected protein' : 'Translation'} data-help={doc.seq_type === 'Protein' ? 'Size, MW and isoelectric point of the selected residues (pK table chosen in the Protein menu). Right-click the field to copy it as FASTA.' : 'Information about the translation of the selected sequence with the chosen genetic code: size, calculated MW, pI and sequence of the protein. Select residues in the peptide field to back-select the corresponding nucleotides; right-click it to open it as a Protein window or copy it as FASTA.'}>
          <h2>{doc.seq_type === 'Protein' ? 'Selected protein' : `Translation (${info?.genetic_code ?? 'Standard'})`}</h2>
          <div class="stats">
            <span>Size (aa): <b>{info?.size_aa ?? ''}</b></span>
            <span>MW: <b>{info ? (info.mw / 1000).toFixed(2) + ' kDa' : ''}</b></span>
            <span>pI: <b>{info?.pi != null ? info.pi.toFixed(2) : '–'}</b></span>
            {#if doc.seq_type !== 'Protein'}
              <label><input type="checkbox" bind:checked={firstOrf} /> Show only 1st ORF</label>
            {/if}
          </div>
          <textarea class="peptide" readonly value={info?.peptide ?? ''} onselect={onPeptideSelect} oncontextmenu={(e) => { e.preventDefault(); void peptideMenu(); }} spellcheck="false"></textarea>
        </div>
      </section>
    {/if}
  </div>
{/if}

<style>
  .window {
    display: flex;
    flex-direction: column;
    height: 100vh;
    box-sizing: border-box;
    padding: 8px 10px;
    gap: 6px;
  }
  .head {
    display: grid;
    grid-template-columns: 1fr auto auto auto auto auto;
    gap: 4px 12px;
    align-items: end;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .label {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  .value {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name .value {
    -webkit-user-select: text;
    user-select: text;
  }
  .name input {
    font: inherit;
  }
  .num {
    font-family: var(--font-mono);
    text-align: right;
  }
  .link {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
    cursor: default;
  }
  .lock {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-bottom: 3px;
  }
  .selection-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .selection-row label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .selection-row input[type='number'] {
    width: 72px;
    font: inherit;
    font-family: var(--font-mono);
  }
  .nav button {
    width: 28px;
    height: 22px;
    padding: 0;
  }
  .feature-name {
    flex: 1;
    color: var(--accent);
    font-weight: 600;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    padding: 3px 12px;
    border: 1px solid transparent;
    border-bottom: none;
    border-radius: var(--radius) var(--radius) 0 0;
    background: transparent;
  }
  .tabs button[aria-selected='true'] {
    border-color: var(--border);
    background: var(--bg-raised);
    margin-bottom: -1px;
  }
  .pane {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .seq-wrap {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .comments {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .comments textarea {
    flex: 1;
    font: inherit;
    resize: none;
  }
  .hint {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  .features-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 0;
  }
  .feature-tools {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .feature-tools .active {
    background: var(--accent);
    color: #fff;
  }
  .feature-tools .danger {
    background: var(--select-end);
    color: #fff;
  }
  .feature-editor {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: center;
    padding: 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
  }
  .feature-editor label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .feature-editor .wide {
    flex: 1;
  }
  .feature-editor .wide input {
    flex: 1;
  }
  .feature-editor input[type='number'] {
    width: 72px;
  }
  tr.selected {
    background: var(--accent);
    color: #fff;
  }
  .features {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-raised);
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 2px 6px;
    text-align: left;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }
  th {
    position: sticky;
    top: 0;
    background: var(--bg);
  }
  .hidden-feature {
    color: var(--fg-muted);
  }
  .bold {
    font-weight: 700;
  }
  .swatch {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 1px solid var(--border);
    border-radius: 3px;
  }
  .empty {
    color: var(--fg-muted);
    text-align: center;
  }
  .ends {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-content: flex-start;
  }
  .ends fieldset {
    display: flex;
    gap: 6px;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .ends input {
    font-family: var(--font-mono);
    text-transform: uppercase;
  }
  .buttons {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .buttons button {
    padding: 3px 10px;
  }
  .info-toggle {
    margin-left: auto;
  }
  .info {
    display: grid;
    grid-template-columns: 1fr;
    gap: 6px;
    border-top: 1px solid var(--border);
    padding-top: 6px;
  }
  .info h2 {
    margin: 0 0 2px;
    font-size: var(--fs-small);
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .stats {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    align-items: center;
  }
  .peptide {
    width: 100%;
    height: 84px;
    box-sizing: border-box;
    margin-top: 4px;
    font-family: var(--font-mono);
    font-size: var(--fs-ui);
    resize: none;
    -webkit-user-select: text;
    user-select: text;
  }
</style>
