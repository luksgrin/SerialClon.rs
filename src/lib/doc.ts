/**
 * Typed wrappers over the document commands (`src-tauri/src/commands.rs`)
 * and the events the shell sends to document windows.
 */
import { invoke } from '@tauri-apps/api/core';
import { emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface FeatureView {
  name: string;
  description: string;
  kind: string;
  start: number;
  end: number;
  forward: boolean;
  show: boolean;
  protect: boolean;
  arrow: boolean;
  color: string;
}

export interface EndsView {
  left_length: number;
  left_bases: string;
  right_length: number;
  right_bases: string;
}

export interface DocumentView {
  id: number;
  name: string;
  bases: string;
  seq_type: string;
  degenerate: boolean;
  circular: boolean;
  comment: string;
  features: FeatureView[];
  ends: EndsView;
  source: string;
  locked: boolean;
  dirty: boolean;
  path: string | null;
  can_undo: boolean;
  can_redo: boolean;
}

/** Mirrors `documents::EditOp` (serde tag `op`, snake_case). */
export type EditOp =
  | { op: 'replace'; from: number; to: number; text: string }
  | { op: 'uppercase'; from: number; to: number }
  | { op: 'lowercase'; from: number; to: number }
  | { op: 'antiparallel'; from: number; to: number }
  | { op: 'reverse'; from: number; to: number }
  | { op: 'complement'; from: number; to: number }
  | { op: 'set_topology'; circular: boolean }
  | { op: 'set_origin'; position: number }
  | { op: 'set_comment'; text: string }
  | { op: 'rename'; name: string }
  | { op: 'set_locked'; locked: boolean }
  | { op: 'set_ends'; ends: EndsView }
  | { op: 'set_degenerate'; degenerate: boolean }
  | { op: 'set_feature_show'; index: number; show: boolean }
  | { op: 'add_feature'; feature: FeatureView }
  | { op: 'update_feature'; index: number; feature: FeatureView }
  | { op: 'delete_feature'; index: number }
  | { op: 'move_feature'; from: number; to: number }
  | { op: 'remove_duplicates' };

export interface EditResponse {
  view: DocumentView;
  filtered: number;
}

export interface SelectionInfo {
  a: number;
  c: number;
  g: number;
  t: number;
  other: number;
  gc_percent: number;
  tm: number;
  peptide: string;
  size_aa: number;
  mw: number;
}

export interface Hit {
  position: number;
  length: number;
  forward: boolean;
}

export interface OrfHit {
  start: number;
  end: number;
  forward: boolean;
  frame: number;
  length: number;
  complete: boolean;
}

export interface EnzymeEntry {
  name: string;
  site: string;
  count: number;
}

export interface MapSite {
  name: string;
  position: number;
  length: number;
  cut_top: number;
  cut_bottom: number;
  forward: boolean | null;
  count: number;
}

export interface MapData {
  doc: DocumentView;
  sites: MapSite[];
  enzymes: EnzymeEntry[];
}

export interface TextMapRequest {
  per_line: number;
  show_reverse: boolean;
  show_coords: boolean;
  translate: 'none' | 'all' | 'range';
  from: number | null;
  to: number | null;
  frames: [boolean, boolean, boolean];
  codons: boolean;
  uppercase_only: boolean;
  sites: 'none' | 'all' | 'max' | 'particular';
  max_cuts: number | null;
  particular: string[];
  features: boolean;
}

export interface SiteRefView {
  enzyme: string;
  position: number;
  cut_top: number;
  cut_bottom: number;
}

export interface FeatureDef {
  name: string;
  kind: string;
  color: string;
  arrow: boolean;
  sense_only: boolean;
  peptide: boolean;
  five: string;
  three: string;
  potential_five: string;
  potential_three: string;
  upstream: string;
  downstream: string;
  comment: string;
  scan: boolean;
}

export interface Collection {
  name: string;
  scan: boolean;
  builtin: boolean;
  features: FeatureDef[];
}

export interface ScanReport {
  added: number;
  found: number;
}

/** A selection: 0-based indices in [0, length]; a caret when equal. */
export interface Selection {
  anchor: number;
  focus: number;
}

export const selStart = (s: Selection): number => Math.min(s.anchor, s.focus);
export const selEnd = (s: Selection): number => Math.max(s.anchor, s.focus);
export const hasSelection = (s: Selection): boolean => s.anchor !== s.focus;

/** The `?id=` of a document window's URL. */
export function docIdFromUrl(): number {
  const id = new URLSearchParams(window.location.search).get('id');
  return id ? Number(id) : 0;
}

export const api = {
  newSequence: (degenerate: boolean) => invoke<DocumentView>('new_sequence', { degenerate }),
  openFiles: () => invoke<number[]>('open_files'),
  openPaths: (paths: string[]) => invoke<number[]>('open_paths', { paths }),
  getDocument: (id: number) => invoke<DocumentView>('get_document', { id }),
  edit: (id: number, op: EditOp) => invoke<EditResponse>('edit', { id, op }),
  undo: (id: number) => invoke<DocumentView>('undo', { id }),
  redo: (id: number) => invoke<DocumentView>('redo', { id }),
  paste: (id: number, from: number, to: number, text: string) => invoke<EditResponse>('paste', { id, from, to, text }),
  save: (id: number, saveAs: boolean) => invoke<DocumentView | null>('save', { id, saveAs }),
  export: (id: number, format: 'fasta' | 'genbank') => invoke<boolean>('export', { id, format }),
  revert: (id: number) => invoke<DocumentView | null>('revert', { id }),
  closeDocument: (id: number) => invoke<void>('close_document', { id }),
  toggleFindWindow: (id: number) => invoke<boolean>('toggle_find_window', { id }),
  openReference: (which: 'about' | 'genetic-code' | 'alphabet') => invoke<void>('open_reference_window', { which }),
  clipboardWrite: (text: string) => invoke<void>('clipboard_write', { text }),
  clipboardRead: () => invoke<string>('clipboard_read'),
  copyMultifasta: () => invoke<number>('copy_multifasta'),
  selectionInfo: (id: number, from: number, to: number, firstOrf: boolean) =>
    invoke<SelectionInfo>('selection_info', { id, from, to, firstOrf }),
  findText: (id: number, query: string) => invoke<Hit[]>('find_text', { id, query }),
  findSite: (id: number, enzyme: string) => invoke<Hit[]>('find_site', { id, enzyme }),
  findOrfs: (id: number, minCodons: number) => invoke<OrfHit[]>('find_orfs_cmd', { id, minCodons }),
  enzymeList: (id: number) => invoke<EnzymeEntry[]>('enzyme_list', { id }),
  recentFiles: () => invoke<string[]>('recent_files'),
  revealFile: (id: number) => invoke<void>('reveal_file', { id }),
  noteFocus: (id: number) => invoke<void>('note_focus', { id }),
  toolbarAction: (action: string) => invoke<void>('toolbar_action', { action }),
  toggleHelpWindow: () => invoke<boolean>('toggle_help_window'),
  mapData: (id: number) => invoke<MapData>('map_data', { id }),
  sequenceMap: (id: number, request: TextMapRequest) => invoke<string>('sequence_map', { id, request }),
  extractFragment: (id: number, begin: SiteRefView, end: SiteRefView | null, clockwise: boolean) =>
    invoke<DocumentView>('extract_fragment', { id, begin, end, clockwise }),
  extractSelection: (id: number, from: number, to: number) => invoke<DocumentView>('extract_selection', { id, from, to }),
  openGraphicMap: (id: number) => invoke<void>('open_graphic_map', { id }),
  openSequenceMap: (id: number, from: number | null, to: number | null) => invoke<void>('open_sequence_map', { id, from, to }),
  savePng: (name: string, dataBase64: string) => invoke<boolean>('save_png', { name, dataBase64 }),
  showParent: (id: number) => invoke<void>('show_parent', { id }),
  scanFeatures: (id: number) => invoke<ScanReport>('scan_features', { id }),
  newFeatureFromSelection: (id: number, from: number, to: number, translation: string | null) =>
    invoke<DocumentView>('new_feature_from_selection', { id, from, to, translation }),
  newFeatureFromSites: (id: number, begin: SiteRefView, end: SiteRefView, beginLength: number, endLength: number, clockwise: boolean) =>
    invoke<DocumentView>('new_feature_from_sites', { id, begin, end, beginLength, endLength, clockwise }),
  listCollections: () => invoke<Collection[]>('list_collections'),
  saveCollections: (collections: Collection[]) => invoke<void>('save_collections', { collections }),
  restoreBuiltinCollections: () => invoke<Collection[]>('restore_builtin_collections'),
  importCollection: () => invoke<Collection[] | null>('import_collection'),
  exportCollection: (name: string, table: boolean) => invoke<boolean>('export_collection', { name, table }),
  openReferenceAny: (which: string) => invoke<void>('open_reference_window', { which }),
};

/** GenBank feature keys offered in the editors. */
export const FEATURE_KINDS = [
  'misc_feature', 'CDS', 'gene', 'promoter', 'terminator', 'rep_origin', 'primer_bind', 'misc_binding', 'protein_bind',
  'misc_recomb', 'polyA_signal', 'polyA_site', 'enhancer', 'RBS', 'sig_peptide', 'mat_peptide', 'intron', 'exon',
  'mRNA', '5\'UTR', '3\'UTR', 'LTR', 'repeat_region', 'oriT', 'misc_RNA', 'ncRNA', 'tRNA', 'rRNA', 'mobile_element',
];

/** Fires whenever the shell changes a document (edits, saves, undo…). */
export function onDocChanged(id: number, handler: (doc: DocumentView) => void): Promise<UnlistenFn> {
  return listen<DocumentView>('doc:changed', (e) => {
    if (e.payload.id === id) handler(e.payload);
  });
}

/** Menu-bar and Toolbar commands forwarded to the front-most Sequence Window. */
export function onMenu(handler: (id: string) => void): Promise<UnlistenFn> {
  return listen<string>('menu', (e) => handler(e.payload));
}

/** A selection request from a drawer (Find hit, Info back-selection). */
export interface SelectRequest {
  from: number;
  to: number;
}

export function requestSelect(id: number, from: number, to: number): Promise<void> {
  return emitTo(`seq-${id}`, 'doc:select', { from, to } satisfies SelectRequest);
}

export function onSelectRequest(handler: (r: SelectRequest) => void): Promise<UnlistenFn> {
  return listen<SelectRequest>('doc:select', (e) => handler(e.payload));
}

/** Formats bases as 80-column lines (Edit ▸ Copy from a sequence window). */
export function formatLines(bases: string, width = 80): string {
  const out: string[] = [];
  for (let i = 0; i < bases.length; i += width) out.push(bases.slice(i, i + width));
  return out.join('\n') + (bases.length ? '\n' : '');
}

/** Edit ▸ Copy as Fasta for a selection (Features ReadMe §VII.A). */
export function fastaOf(doc: DocumentView, from: number, to: number): string {
  const comment = doc.comment.replace(/[\r\n]+/g, ' ').trim();
  const whole = from === 0 && to === doc.bases.length;
  const range = whole ? '' : ` - Selection From [${from + 1}] To [${to}]`;
  return `> [${doc.name} - ${doc.bases.length} bp] ${comment}${range}\n${formatLines(doc.bases.slice(from, to))}`;
}
