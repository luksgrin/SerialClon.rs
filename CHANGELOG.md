# Changelog

All notable changes to SerialClon.rs. The project reimplements Serial Cloner 2.6.1
(F. Perez, SerialBasics, 2013) window for window; each entry names the milestone of
[`PLAN.md`](PLAN.md) it completed.

## Unreleased

### Behaviour aligned with the original, third pass (2026-10-17)

- Extract Fragment and Build a Construct write the original's comments: `Fragment extracted
  from <name> [<n>  nt] : (<enzyme 5'>[<pos>] / <enzyme 3'>[<pos>])` (an end made blunt is
  relabelled `-Blunt(filled)` / `-Blunt(trimmed)`), `Fragment extracted from <name> [between
  <from> and <to>]` for a Sequence Window selection, and `Ligation of : <fragment 1> to
  <fragment 2> and to <fragment 3>`. Products are named `DNA Fragment #n`, `Fragment #n` and
  `Ligation #n` and open as `Unsaved <name>`, as the original titles its windows.
- After a ligation and after a Gateway reaction the junctions are rescanned for features, as the
  original does: a site rebuilt by compatible ends or a tag completed across a junction is
  annotated, under the same merge/protect rules as a scan.
- Gateway: *Use minimal att consensus* (the original's checkbox, remembered in the preferences)
  locates the att sites with the original's short core consensus instead of the full
  definitions, for vectors with unusual att arms.
- Codon Usage lists the original's nine tables first (H. sapiens, M. musculus, R. norvegicus,
  E. coli, S. cerevisiae, S. pombe, A. thaliana, P. pastoris, D. melanogaster, verbatim from the
  original), with H. sapiens as the default; the five 2026 Kazusa tables follow.
- The Virtual Cutter's gel offers the original's four ladders (GeneRuler 1 kb Plus, Smart
  Ladder, NEB 1 kb, Invitrogen 1 kb plus), GeneRuler by default, bright reference bands drawn
  bolder and the migration scale calibrated on the chosen ladder; the generic 1 kb ladder stays.

### Data recovered from the original binaries (2026-10-17)

- The built-in Feature collections are now Serial Cloner 2.6.1's own database, all 155
  definitions, recovered by running the original's Linux build and reading the files it writes
  on first launch (`reference/original/features/`). The 1.0.0 reconstruction had 131 names and
  only 22 sequences right; the previous extras live on in a "SerialClon additions" collection
  that is not scanned by default. The scanner gained the original's `#` gap wildcard. An install
  seeded by 1.0.0 picks the new database up with *Manage Features ▸ Restore built-in
  collections*.
- `.xdna` cohesive ends and the GenBank `SerialCloner_Ends=` comment are now written the way the
  original writes them (ends flag 0/5/3/35, overhang lengths signed by 5'/3', bases 5'→3' on
  their strand), checked against files the original produced (`reference/original/golden/`).
- Gateway site detection no longer reports an ambiguity when attP1 and attP2 (which differ by
  one base) are both present.
- The isoelectric point is computed the way the original computes it: its five pK tables
  (EMBOSS, Wikipedia, DTASelect, Rodwell, Sillero, with its values and EMBOSS as the default)
  and its 0.01-pH walk from pH 7 instead of a bisection over six other tables. A settings file
  naming one of the dropped tables falls back to EMBOSS.
- `reference/original/` also holds the archive inventory with SHA-256 digests, the fallback
  enzyme library, DNA ladders, codon-usage tables, genetic-code names and pK tables read out
  of the executable, and `reference/docs/original-behaviour.md` records the same/differs
  verdicts of the behaviour comparison (Tm, protein MW, pI, scan, GenBank export).

### Behaviour aligned with the original, second pass (2026-10-17)

- Find ▸ ORFs works as Serial Cloner's: an ORF opens at every occurrence of the preference's
  start codon (ATG by default; GTG, ATG|GTG or ATG|CTG|GTG|TTG in Preferences) and never at
  the genetic code's alternative starts; nested ORFs are listed; the list is sorted by size;
  Position is the lower end of the ORF on either strand and Frame carries the strand sign;
  the minimum size is in amino acids with 0 (the default) meaning no limit. The first ten
  rows of the original's pBR322 list are a test (`reference/original/golden/orf/`).
- Find has the original's Peptide tab: an amino-acid string searched in the three frames of
  both strands (X matches any residue), with Position / Size / Strand / Frame.
- Build a Construct offers the original's three choices per extremity (Non-processed,
  Blunt (Fill-In), Blunt (Remove)) instead of one polymerase-style checkbox, decides the
  topology from the outer ends like the original (asking before a linear product or a lone
  vector, with its wording), and marks every insert with an `insert from <name>` feature.
  Gateway products get the original's `Insert from <insert>` feature too.
- A feature whose name, colour or type was edited is no longer replaced by a new scan: the hit
  is added beside it, as the Features ReadMe documents.
- Preference defaults are the original's (read from its Preferences window, screenshots under
  `reference/original/golden/prefs/`): map colours, 12 pt sequence field, 9 pt map labels,
  11 pt Sequence Map, sequences locked when opened, limited window width, TA cloning on.
- The built-in pSUPER shRNA layout is the original's (`CCC … TTCAAGAGA … TTTTTG` between
  BglII and HindIII ends); the suffix was wrong.
- `reference/original/embedded/help-texts.txt` holds the original's 113 help texts.

## 1.0.0 — 2026-10-12

The first release. Everything below is in it; the milestone work was done between
2026-09-07 and 2026-09-09, the fixes after the first field tests between 2026-09-29 and 2026-10-12.

### Fixes after the first field tests

- Graphic Map: the drawing always fits the window and scales with it, in Auto and in Show all
  (the map has its own coordinate space and the window shows all of it); no more clipped labels,
  off-centre circles or scrolling. Pinning a label adds it to the automatically shown ones
  instead of replacing them; the Labels panel shows what is on the map (blue check = pinned,
  grey check = shown automatically, empty = hidden for room, struck = hidden by you).

- Graphic Map: labels are no longer clipped at the window edge and the circle stays centred at
  every window size. Only the labels that fit without overlapping are shown (chosen sites first,
  then the longest Features); the status line counts the rest, with Show all / Auto. Hover a
  Feature or a site mark for its name, click a Feature to pin its label, or open the new Labels
  panel to search, pin or hide any label of the map. Isoschizomers cutting at the same place share
  one box ("7763 AsiSI +3"). Print uses the Show-all layout.
- Maps, Site Usage, the Virtual Cutter and the Silent Map follow their document again after an
  edit or a Linearize / Circularize (a regression of the event-scoping fix: the change
  notification only reached the Sequence Window and its Find drawer).

- Toolbar: the window sizes itself to the strip, so button hover boxes are no longer cut at the
  window edges. Textareas everywhere can be resized in height only and never wider than their
  window; tool windows scroll when their content grows.

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

### Milestones

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
