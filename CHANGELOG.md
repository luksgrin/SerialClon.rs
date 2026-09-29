# Changelog

All notable changes to SerialClon.rs. The project reimplements Serial Cloner 2.6.1
(F. Perez, SerialBasics, 2013) window for window; each entry names the milestone of
[`PLAN.md`](PLAN.md) it completed.

## Unreleased

- **SnapGene `.dna` files open and save.** Sequence with case and topology, features with
  colours and strands, primers as `primer_bind` features, the description as the comment. A
  file opened from SnapGene saves back to `.dna` byte for byte when unedited, and keeps its
  primers, history and traces when edited; File ▸ Export ▸ SnapGene writes any sequence as a
  `.dna` file. `.dna` is a file association and an Open filter. Verified on 24 public fixtures
  and cross-checked with Biopython's parser.
- Events sent to one window no longer reach every window. Tauri delivers a targeted event to
  every listener registered without a target, so Edit ▸ Paste in the PCR window (or the Find
  drawer, or any tool window) was *also* pasted into the sequence of every open Sequence Window.
  Every listener is now scoped to its own window.
- Run a PCR accepts a primer pair whose amplicon runs on the file's reverse strand (forward primer
  on the bottom strand, reverse on the top): the product is given 5'→3' from the forward primer
  and its comment says so. Each primer's message states its strand and Tm; two primers on the same
  strand are refused with the reverse-complement hint.

- Edit ▸ Undo / Redo / Select All / Cut / Copy / Paste now act on the window that has focus, as
  the original's Edit menu did. Before, in every window other than the Sequence Window (the PCR
  primers, the Find drawer, the Build a Construct slots…) ⌘V pasted the clipboard into the
  sequence field of the last Sequence Window instead of the field you were typing in.
- Run a PCR: a primer that matches the other strand as written is no longer just "does not
  anneal"; the window says where it matches, reminds that a reverse primer is given 5'→3' on the
  bottom strand, and offers **Reverse-complement** and **Use as forward/reverse primer** buttons.

## 1.0.0 — 2026-09-09

The first release. Every window of the original except the mini web browser (replaced by
Web Access on the NCBI and ENA APIs), plus a multiple-alignment window the original lacked.

### M7 — Polish (this release)
- **Preferences** (⌘,) with every knob the original had: colours of unique / double / particular
  sites, of the BEGIN / END selection and of new features; character sizes of the sequence field,
  the Graphic Map names and the Sequence Map (screen and print); Sequence Map defaults (nucleotides
  per line, reverse strand, amino acid under the first or middle base of the codon); Graphic Map
  boxing options; autolock of opened sequences; number of recent files (up to 25); the warning
  when importing non-Serial Cloner files (with "Don't show again"); minimum ORF size; PCR 3' A
  overhang default; autosave interval; update check; network proxy; Toolbar vertical shift; the
  width limit of Sequence windows (2.6.1); theme; read-out speed; genetic code, pK table and codon
  usage table, and "use the codon usage table as the genetic code" (2.6). Applied live, persisted
  to `prefs.json`; Reset Preferences button, `--reset-prefs` at launch.
- **Printing** on all three platforms through the native print dialog, File ▸ Page Setup
  (orientation, margins), print stylesheets for the Graphic Map, Sequence Map, Site Usage tables
  and the alignment windows, the Sequence Map's print character size.
- **Aqua Classic** theme reproducing the 2.6 look (pinstripe chrome, aqua buttons and tabs, the
  yellow / orange / blue site boxes), next to the default Modern theme.
- **File-type associations**: `.xdna` (editor), FASTA and GenBank (viewer); files double-clicked
  or dropped on any window open in Sequence Windows.
- **Read selection** aloud (Sequence menu, Esc stops), with a speed preference.
- **Update check** against GitHub Releases at start, at most once a day, with a Preferences switch.
- Licence: MIT OR Apache-2.0. Version 1.0.0.

### M6b — Align Multiple Sequences
- Generate ▸ Align Multiple Sequences (⌘8) on the pure-Rust MAFFT port (`mafft` crate): Auto,
  FFT-NS-2, FFT-NS-i, L-INS-i, G-INS-i, E-INS-i, adjust direction, conservation colouring,
  features above each row, consensus extraction, FASTA and Clustal export.

### M6 — Alignment, protein, web
- Align two Sequences (⌘7): local Smith-Waterman with affine gaps, 3-frame translation, features,
  consensus extraction, highlight differences, DNA ↔ protein switch; remote BLAST2Seq.
- Protein windows: New / Open Protein, pI with six pK tables, peptide-feature scanning, reverse
  translation with codon usage tables (five bundled from Kazusa), Codon Usage window, 27 NCBI
  genetic codes, bisulfite conversion on either strand.
- Web Access: NCBI E-utilities and ENA fetch by accession → Sequence Window with features;
  remote BLAST through the BLAST URL API. Off unless invoked.

### M5 — Cloning
- Build a Construct (⌘1) with fragment slots, blunting and cohesive-end checking; Create an
  Adaptor (⌘2); Run a PCR (⌘3) with tails, annealing check, primer features, TA overhang;
  Make shRNA (⌘4) with stored layouts; Gateway BP / LR and homologous recombination (⌘5);
  Site Usage interactive (with the second-sequence filter) and printable; Enzyme Library
  (filters, isoschizomers, compatible ends, add enzymes, hide unchecked ones everywhere);
  Virtual Cutter with a simulated gel; Silent Map.

### M4 — Features
- Features tab with Edit mode, drag to reorder, Alt-delete, S to show / hide; Scan against five
  built-in Collections (Generic, Yeast, Other, Tags & Cut, Gateway; 131 of the original's 150
  names, each sourced from a public record); simple and complex definitions (5'/3' ends,
  potential ends, context, DNA or peptide, sense-only); Manage Features window; import / export
  in SerialClon, generic-table and ApE formats; Remove duplicated Features.

### M3 — Maps
- Graphic Map (⌘G): circular and linear, unique / double / particular sites, Choose site,
  draggable labels, cohesive ends, two-click fragment selection with spacebar CW / CCW, PNG
  export, print. Sequence Map (⌘M): the text restriction map with multi-frame translation,
  reproducing the original's printed map line for line. Extract Fragment (⌘6) with cohesive ends.

### M2 — Sequence Window
- The Sequence Window (tabs, spinners that drive the selection, lock, topology, virtualised
  canvas, Selection Information & Translation), the Find drawer (sequence / site / ORFs, both
  strands, across the origin), the menu bar with every shortcut of the manual, Open / Save /
  Save As / Export / Revert, recent files, five-level undo, autosave with crash recovery, the
  contextual Help window, the Toolbar.

### M1 — Headless core
- `.xdna` read and write, byte-exact against every shipped sample; FASTA and GenBank in the
  original's dialects; REBASE `striderc` parsing; digestion with type IIs enzymes, verified
  against the original's printed map of pBluescript II SK(+); Tm, MW, ORFs, translation.

### M0 — Skeleton
- Cargo workspace, Tauri 2 shell, Svelte 5 frontend, icons, CI and installer workflows.
