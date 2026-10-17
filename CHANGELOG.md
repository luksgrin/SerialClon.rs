# Changelog

All notable changes to SerialClon.rs. The project reimplements Serial Cloner 2.6.1
(F. Perez, SerialBasics, 2013) window for window.

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

The first release: every window of the original except its mini web browser, which is replaced
by Web Access on the NCBI and ENA APIs, plus a multiple-alignment window the original lacked.

### Sequences
- Sequence Window with Sequence, Comments, Features and Extremities tabs; spinners that drive the
  selection; lock; topology; a virtualised sequence field; Selection Information & Translation.
- Find drawer: sequence, site and ORF search on both strands, across the origin of circular
  sequences.
- The menu bar with every shortcut of the manual; Open, Save, Save As, Export, Revert, recent
  files, five-level undo, autosave with crash recovery, the contextual Help window, the Toolbar.
- Edit ▸ Undo / Redo / Select All / Cut / Copy / Paste act on the window that has focus.
- Read selection aloud with a speed preference.

### Files
- `.xdna` read and write, byte-exact against every sample the original shipped.
- FASTA, multi-FASTA and GenBank / EMBL in the original's dialects, with features and colours.
- SnapGene `.dna` read and write: sequence, topology, features, primers and the description. A
  file opened from SnapGene saves back byte for byte when unedited and keeps its history and
  traces when edited. Verified on 24 public fixtures.
- File-type associations for `.xdna`, FASTA, GenBank and `.dna`; files dropped on any window open
  in Sequence Windows.

### Maps
- Graphic Map: circular and linear; unique, double and particular sites; Choose site; draggable
  labels; cohesive ends; two-click fragment selection with spacebar CW / CCW; PNG export; print.
  Feature tracks, tick marks and packed labels; only the labels that fit are shown, with Show all,
  hover tooltips and a Labels panel to search, pin or hide any label; isoschizomers cutting at the
  same place share one box. The drawing always fits the window and scales with it.
- Sequence Map: the text restriction map with multi-frame translation, reproducing the original's
  printed map line for line.
- Extract Fragment with cohesive ends.

### Features
- Features tab with Edit mode, drag to reorder, Alt-delete, show / hide.
- Scan against the five built-in Collections; simple and complex definitions (5' / 3' ends,
  potential ends, context, DNA or peptide, sense-only); Manage Features; import and export in
  SerialClon, generic-table and ApE formats; Remove duplicated Features.

### Cloning
- Build a Construct with fragment slots, blunting and cohesive-end checking.
- Create an Adaptor.
- Run a PCR with tails, annealing check, primer features and TA overhang. A primer pair whose
  amplicon runs on the file's reverse strand is accepted and the product is given 5'→3' from the
  forward primer; a primer that matches the other strand as written is reported with its position
  and Reverse-complement / swap buttons.
- Make shRNA with stored layouts.
- Gateway BP / LR and homologous recombination.
- Site Usage, interactive (with the second-sequence filter) and printable.
- Enzyme Library: filters, isoschizomers, compatible ends, your own enzymes, hiding unchecked
  enzymes everywhere.
- Virtual Cutter with a simulated gel; Silent Map.

### Alignment and protein
- Align two Sequences: local Smith-Waterman with affine gaps, 3-frame translation, features,
  consensus extraction, highlight differences, DNA ↔ protein switch; remote BLAST2Seq.
- Align Multiple Sequences on the pure-Rust MAFFT port: Auto, FFT-NS-2, FFT-NS-i, L-INS-i,
  G-INS-i, E-INS-i, adjust direction, conservation colouring, consensus extraction, FASTA and
  Clustal export.
- Protein windows: New / Open Protein, pI, peptide-feature scanning, reverse translation with
  codon usage tables, Codon Usage window, 27 NCBI genetic codes, bisulfite conversion on either
  strand.

### Network
- Web Access: NCBI E-utilities and ENA fetch by accession into a Sequence Window with features;
  remote BLAST through the BLAST URL API. Off unless invoked.

### Preferences and polish
- Preferences with every knob the original had: colours, character sizes, Sequence Map and Graphic
  Map defaults, autolock, recent files, import warning, minimum ORF size, TA overhang default,
  autosave interval, update check, proxy, Toolbar shift, Sequence Window width limit, theme,
  read-out speed, genetic code, pK table and codon usage table. Applied live, persisted to
  `prefs.json`, with a Reset button and a `--reset-prefs` launch flag.
- Printing on all three platforms through the native print dialog, with Page Setup and print
  stylesheets for the maps, Site Usage and the alignment windows.
- Aqua Classic theme reproducing the 2.6 look, next to the default Modern theme.
- Update check against GitHub Releases, at most once a day.
- Licence: MIT OR Apache-2.0.
