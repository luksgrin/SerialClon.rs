/**
 * Typed wrappers over the document commands (`src-tauri/src/commands.rs`)
 * and the events the shell sends to document windows.
 */
import { invoke } from '@tauri-apps/api/core';
import { emitTo, listen, type Event, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

/**
 * `listen()` with the default target receives events emitted to *any*
 * window (Tauri's `EventTarget::Any` matches every `emit_to`), so a `menu`
 * event sent to the PCR window also reached every Sequence Window. This
 * registers the listener for the current window only; `emit` broadcasts
 * still arrive.
 */
export function listenHere<T>(event: string, handler: (e: Event<T>) => void): Promise<UnlistenFn> {
  try {
    return getCurrentWebviewWindow().listen<T>(event, handler);
  } catch {
    return listen<T>(event, handler); // plain browser
  }
}

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
  pi: number | null;
  protein: boolean;
  genetic_code: string;
}

export interface Hit {
  position: number;
  length: number;
  forward: boolean;
}

export interface OrfHit {
  /** Lower coordinate on either strand (the original's Position column). */
  position: number;
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
  aa_middle?: boolean;
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

export interface GeneticCodeView {
  id: number;
  name: string;
  starts: string[];
  stops: string[];
  selected: boolean;
}

export interface CodonTableView {
  name: string;
  slug: string;
  source: string;
  retrieved: string;
  cds: number | null;
  builtin: boolean;
  selected: boolean;
}

export interface CodonUsageRow {
  codon: string;
  amino_acid: string;
  fraction: number;
  per_thousand: number;
  count: number;
}

export interface Alignment {
  kind: 'Dna' | 'Protein';
  score: number;
  a_start: number;
  a_end: number;
  b_start: number;
  b_end: number;
  b_reverse: boolean;
  a_line: string;
  b_line: string;
  midline: string;
  matches: number;
  length: number;
  identity: number;
}

export interface AlignView {
  alignment: Alignment;
  a_name: string;
  b_name: string;
  a_len: number;
  b_len: number;
  a_features: FeatureView[];
  b_features: FeatureView[];
  swapped: boolean;
  a_id: number;
  b_id: number;
}

export type MsaMode = 'auto' | 'fft-ns-2' | 'fft-nsi' | 'l-insi' | 'g-insi' | 'e-insi';

export interface MsaOptions {
  mode: MsaMode;
  adjust_direction: boolean;
  keep_case: boolean;
}

export interface MsaRow {
  name: string;
  aligned: string;
  reversed: boolean;
  length: number;
}

export interface Msa {
  kind: 'Dna' | 'Protein';
  mode: MsaMode;
  rows: MsaRow[];
  marks: string;
  consensus: string;
  length: number;
  score: number;
}

export interface MsaCandidate {
  id: number;
  name: string;
  seq_type: string;
  protein: boolean;
  length: number;
}

export interface MsaRowView {
  id: number;
  features: FeatureView[];
}

export interface MultipleAlignmentView {
  msa: Msa;
  rows: MsaRowView[];
  elapsed_ms: number;
}

export interface RecordView {
  text: string;
  format: 'genbank' | 'genpept' | 'embl';
  name: string;
}

export interface BlastJobView {
  rid: string;
  rtoe: number;
}

export interface BlastStatusView {
  status: 'waiting' | 'ready' | 'nohits' | 'failed' | 'unknown';
  wait: number;
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
  export: (id: number, format: 'fasta' | 'genbank' | 'snapgene') => invoke<boolean>('export', { id, format }),
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
  findPeptide: (id: number, query: string) => invoke<OrfHit[]>('find_peptide', { id, query }),
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
  // ---- M6: protein, codon usage, alignment, web ----
  geneticCodes: () => invoke<GeneticCodeView[]>('genetic_codes'),
  setGeneticCode: (id: number) => invoke<void>('set_genetic_code', { id }),
  pkTables: () => invoke<[string, boolean][]>('pk_tables'),
  setPkTable: (name: string) => invoke<void>('set_pk_table', { name }),
  codonTables: () => invoke<CodonTableView[]>('codon_tables'),
  codonTableRows: (slug: string) => invoke<CodonUsageRow[]>('codon_table_rows', { slug }),
  selectCodonTable: (slug: string) => invoke<void>('select_codon_table', { slug }),
  importCodonTable: (name: string, text: string | null, url: string | null) =>
    invoke<CodonTableView>('import_codon_table', { name, text, url }),
  deleteCodonTable: (slug: string) => invoke<void>('delete_codon_table', { slug }),
  newProtein: () => invoke<DocumentView>('new_protein'),
  openProteinFiles: () => invoke<number[]>('open_protein_files'),
  peptideToDocument: (id: number, from: number, to: number, firstOrf: boolean) =>
    invoke<DocumentView>('peptide_to_document', { id, from, to, firstOrf }),
  reverseTranslate: (id: number, from: number, to: number) => invoke<DocumentView>('reverse_translate', { id, from, to }),
  bisulfite: (id: number, antisense: boolean) => invoke<DocumentView>('bisulfite', { id, antisense }),
  translateFrames: (text: string) => invoke<[string, string, string]>('translate_frames', { text }),
  alignDocuments: (a: number, b: number, protein: boolean, short: boolean) =>
    invoke<AlignView>('align_documents', { a, b, protein, short }),
  alignConsensus: (alignment: Alignment, aName: string, bName: string) =>
    invoke<DocumentView>('align_consensus', { alignment, aName, bName }),
  msaCandidates: () => invoke<MsaCandidate[]>('msa_candidates'),
  alignMultiple: (ids: number[], options: MsaOptions) => invoke<MultipleAlignmentView>('align_multiple', { ids, options }),
  msaText: (msa: Msa, clustal: boolean) => invoke<string>('msa_text', { msa, clustal }),
  extractMsaConsensus: (msa: Msa) => invoke<DocumentView>('extract_msa_consensus', { msa }),
  exportMsa: (msa: Msa, clustal: boolean) => invoke<boolean>('export_msa', { msa, clustal }),
  webFetch: (source: 'ncbi' | 'ncbi-protein' | 'ena', accession: string) => invoke<RecordView>('web_fetch', { source, accession }),
  webCreateDocument: (text: string, name: string, protein: boolean) =>
    invoke<DocumentView>('web_create_document', { text, name, protein }),
  blastSubmit: (id: number, from: number, to: number, program: string, database: string | null, subject: number | null) =>
    invoke<BlastJobView>('blast_submit', { id, from, to, program, database, subject }),
  blastStatus: (rid: string) => invoke<BlastStatusView>('blast_status', { rid }),
  blastResult: (rid: string) => invoke<string>('blast_result', { rid }),
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
  printWindow: () => invoke<void>('print_window'),
  listDocuments: () => invoke<[number, string][]>('list_documents'),
  // --- M5: cloning ---
  previewFragments: (fragments: FragmentSpec[]) => invoke<FragmentPreview[]>('preview_fragments', { fragments }),
  ligateConstruct: (fragments: FragmentSpec[], circular: boolean, name: string) =>
    invoke<DocumentView>('ligate_construct', { fragments, circular, name }),
  ligationPlan: (fragments: FragmentSpec[]) => invoke<LigationPlan>('ligation_plan', { fragments }),
  runPcr: (id: number, forward: string, reverse: string, ta: boolean) => invoke<DocumentView>('run_pcr', { id, forward, reverse, ta }),
  checkPrimer: (id: number, primer: string, reverse: boolean) => invoke<PrimerCheck>('check_primer', { id, primer, reverse }),
  pcrWindowOpen: () => invoke<boolean>('pcr_window_open'),
  makeAdaptor: (name: string, core: string, ends: { left_length: number; left_bases: string; right_length: number; right_bases: string }, open: boolean) =>
    invoke<Oligos>('make_adaptor', { name, core, ends, open }),
  shrnaLayouts: () => invoke<ShrnaLayout[]>('shrna_layouts'),
  saveShrnaLayouts: (layouts: ShrnaLayout[]) => invoke<void>('save_shrna_layouts', { layouts }),
  makeShrna: (name: string, layout: ShrnaLayout, core: string, mutations: number[], open: boolean) =>
    invoke<Oligos>('make_shrna', { name, layout, core, mutations, open }),
  gateway: (insert: number, vector: number, reaction: 'bp' | 'lr' | 'hr', minHomology: number | null) =>
    invoke<DocumentView>('gateway', { insert, vector, reaction, minHomology }),
  siteUsage: (id: number, other: number | null) => invoke<UsageRow[]>('site_usage', { id, other }),
  libraryEntries: () => invoke<LibraryEntry[]>('library_entries'),
  isoschizomers: (name: string) => invoke<string[]>('isoschizomers', { name }),
  compatibleEnds: (name: string) => invoke<string[]>('compatible_ends', { name }),
  addEnzyme: (line: string) => invoke<LibraryEntry>('add_enzyme', { line }),
  removeEnzyme: (name: string) => invoke<void>('remove_enzyme', { name }),
  setEnzymesHidden: (names: string[], hidden: boolean) => invoke<void>('set_enzymes_hidden', { names, hidden }),
  virtualCut: (id: number, enzymes: string[]) => invoke<Digest>('virtual_cut', { id, enzymes }),
  silentMap: (id: number, from: number, to: number) => invoke<SilentRow[]>('silent_map', { id, from, to }),
  applySilentMutation: (id: number, position: number, mutated: string) => invoke<DocumentView>('apply_silent_mutation', { id, position, mutated }),
  openToolWindow: (which: string, id: number | null, from: number | null, to: number | null) =>
    invoke<void>('open_tool_window', { which, id, from, to }),
  visibleEnzymes: (id: number) => invoke<EnzymeEntry[]>('visible_enzymes', { id }),
};

export interface FragmentSpec {
  doc: number;
  begin: SiteRefView | null;
  end: SiteRefView | null;
  clockwise: boolean;
  /** Non-processed / Blunt (Fill-In) / Blunt (Remove), as the original's radios. */
  blunt_left: BluntChoice;
  blunt_right: BluntChoice;
}

export type BluntChoice = 'none' | 'fill' | 'remove';

export interface LigationPlan {
  circular: boolean;
  single: boolean;
}

export interface FragmentPreview {
  name: string;
  length: number;
  left: string;
  right: string;
  features: number;
}

export interface PrimerCheck {
  start: number;
  end: number;
  tail: number;
  tm: number;
  /** The primer anneals only to the other strand: it belongs in the other box. */
  other_strand: boolean;
}

export interface Oligos {
  top: string;
  bottom: string;
  view: DocumentView | null;
}

export interface ShrnaLayout {
  name: string;
  left_overhang: string;
  prefix: string;
  loop_seq: string;
  suffix: string;
  right_overhang: string;
}

export interface UsageRow {
  name: string;
  site: string;
  count: number;
  other: number | null;
}

export interface LibraryEntry {
  name: string;
  site: string;
  length: number;
  overhang: number;
  type_iis: boolean;
  user: boolean;
  hidden: boolean;
}

export interface Digest {
  sizes: number[];
  migrations: number[];
  ladder: number[];
  ladder_migrations: number[];
}

export interface SilentRow {
  enzyme: string;
  position: number;
  changes: number;
  mutated: string;
}

/** A Graphic Map selection handed to Build a Construct. */
export interface ConstructSelection {
  doc: number;
  name: string;
  begin: SiteRefView | null;
  end: SiteRefView | null;
  clockwise: boolean;
}

/** GenBank feature keys offered in the editors. */
export const FEATURE_KINDS = [
  'misc_feature', 'CDS', 'gene', 'promoter', 'terminator', 'rep_origin', 'primer_bind', 'misc_binding', 'protein_bind',
  'misc_recomb', 'polyA_signal', 'polyA_site', 'enhancer', 'RBS', 'sig_peptide', 'mat_peptide', 'intron', 'exon',
  'mRNA', '5\'UTR', '3\'UTR', 'LTR', 'repeat_region', 'oriT', 'misc_RNA', 'ncRNA', 'tRNA', 'rRNA', 'mobile_element',
];

/** Fires whenever the shell changes a document (edits, saves, undo…). */
export function onDocChanged(id: number, handler: (doc: DocumentView) => void): Promise<UnlistenFn> {
  return listenHere<DocumentView>('doc:changed', (e) => {
    if (e.payload.id === id) handler(e.payload);
  });
}

/** Menu-bar and Toolbar commands forwarded to the front-most Sequence Window. */
export function onMenu(handler: (id: string) => void): Promise<UnlistenFn> {
  return listenHere<string>('menu', (e) => handler(e.payload));
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
  return listenHere<SelectRequest>('doc:select', (e) => handler(e.payload));
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
