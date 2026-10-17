# Design

SerialClon.rs is a faithful reimplementation of **Serial Cloner 2.6.1** (F. Perez, SerialBasics,
last release March 2013) in Rust and Tauri 2. The goal is not another sequence editor. It is the
same windows, the same interactions, the same keystrokes and the same file formats, on a stack
that still builds.

## 1. What Serial Cloner is

| | |
|---|---|
| Last version | 2.6.1, March 2013 |
| Original stack | REALbasic (`libRBAppearancePak.so`, `libRBBrowser.so`, `libRBXML.so` in the Linux tarball) |
| Licence | Freeware, closed source, unsupported |
| Platforms | macOS (signed for Mountain Lion), Windows, Linux (Ubuntu, "as is") |
| Native format | `.xdna`, an extended DNA Strider binary format |
| Enzyme data | `RELibrary`, REBASE `striderc.211` (October 2012), plain text |

The requirements were taken from the official site, the User Manual, the Features ReadMe, the
version history and the shipped sample files; the sample files are kept under `reference/`.
The distributed binaries were later examined to recover the built-in data and to compare
behaviour; see [Migration](migration.md) and `reference/docs/original-behaviour.md`.

### 1.1 The interactions that have to be preserved

These are what make Serial Cloner feel like Serial Cloner. They are the actual requirements; the
feature list is secondary.

1. **A floating-window desktop, not a single-page app.** A thin always-on-top Toolbar strip plus
   independent windows the user tiles. Several sequences open at once, each with its own window
   and its own maps.
2. **The permanent Help window.** A small window that shows a one-line explanation of whatever
   widget the mouse is over, in the front-most window. Every control carries a `data-help` text
   and a mouse-over bus delivers it.
3. **Drawers.** Find attaches to its Sequence Window and follows it when it moves.
4. **Numeric selection everywhere.** From, to and length are editable spinner fields. Type a
   start and a length and the selection follows.
5. **Keyboard-scrubbing enzyme lists.** Type "Bam" into any enzyme list and it jumps. Unique
   sites are bold, absent sites are italic, everywhere an enzyme list appears.
6. **Two-click fragment selection on the map.** Click a site box and it turns green (begin);
   double-click or shift-click another and it turns red (end). The spacebar toggles CW/CCW on a
   circular map or sense/antisense on a linear one. Extract Fragment drops the fragment, with its
   cohesive ends, into a new Sequence Window whose comment records how it was made.
7. **Draggable labels.** Site and feature boxes on the Graphic Map can be dragged before printing
   so nothing overlaps.
8. **Alt as the advanced modifier.** Alt-click Save is Save As; Alt before the minus button
   deletes features without confirmation; Alt-click a Show checkbox shows or hides all; Alt-click
   Import is Export.
9. **Everything is one context menu away.** Right-click the sequence field, the map, the sequence
   name (rename) or the topology field (linearize/circularize).
10. **The comment field is the lab notebook.** Every generated sequence arrives with a
    human-readable provenance note in its comments.

### 1.2 Window inventory

| Window | Shortcut | Purpose |
|---|---|---|
| Toolbar | Window menu | Icon strip, always available |
| Help | Window menu | Contextual hover help |
| Sequence Window | ⌘N / ⌘O | Name, length, type, topology, Lock, from/to/length spinners; tabs Sequence / Comments / Features / Extremities; Selection Information & Translation |
| Find drawer | ⌘F | Sequence / Site / ORFs / Peptide; both strands; sortable hit table; wraps around the origin |
| Graphic Map | ⌘G | Circular or linear map; Features, unique, double and particular sites, Choose site; draggable labels; cohesive ends; PNG export; print |
| Sequence Map | ⌘M | Text restriction map with multi-frame translation, site filters, display options, feature marks |
| Site Usage | Restriction menu | Live frequency table, filter against a second sequence; printable table |
| Enzyme Library | Restriction menu | Filters, isoschizomers, compatible ends, your own enzymes |
| Build a Construct | ⌘1 | Fragment slots, blunting, Ligate |
| Create an Adaptor | ⌘2 | Core dsDNA plus cohesive extensions, oligos to order |
| Run a PCR | ⌘3 | Template and two primers with tails, TA overhang, primer features |
| Make shRNA | ⌘4 | Stored layouts, siRNA core, point mutations |
| Gateway cloning | ⌘5 | BP and LR, att detection, homologous recombination |
| Extract Fragment | ⌘6 | From the selection or between two sites |
| Align two Sequences | ⌘7 | Local Smith-Waterman or NCBI BLAST2Seq, 3-frame translation, consensus, differences |
| Align Multiple Sequences | ⌘8 | MAFFT, new in SerialClon.rs |
| Virtual Cutter | Tools | Fragment sizes and a simulated gel |
| Silent Map | Tools | Sites introducible without changing the peptide |
| Web Access | Tools | NCBI and ENA import, BLAST submission |
| Manage Features | Features menu | Collections, simple and complex definitions, import and export |
| Codon Usage | Protein menu | Codon bias tables, import |
| Preferences | ⌘, | Colours, sizes, defaults, network, genetic code, pK tables |
| Genetic Code, DNA Alphabet, About | Window menu | Reference cards |

### 1.3 Functions

- Sequence operations: upper and lower case, antiparallel, reverse, complement, linearize and
  circularize, set origin, lock, degenerate-DNA mode, read selection aloud.
- Translation: standard and alternative genetic codes, alternative start codons, codon table
  from the selected codon usage table, molecular weight, pI with selectable pK tables.
- Protein windows: open and edit, align, reverse-translate with codon bias, scan for peptide
  features, BLAST.
- Bisulfite conversion on either strand.
- Tm, with `[Na+] = 0.050 M`:
    - `n < 14`: `Tm = (A+T)·2 + (G+C)·4 − 16.6·log10(0.050) + 16.6·log10([Na+])`
    - `14 ≤ n ≤ 50`: `Tm = 100.5 + 41·(G+C)/n − 820/n + 16.6·log10([Na+])`
    - `n > 50`: `Tm = 81.5 + 41·(G+C)/n − 500/n + 16.6·log10([Na+])`
- Features: manual entry, from the selection, automatic scan against the built-in consensus
  sequences, per-feature colour, arrow and visibility, inheritance through PCR, ligation,
  Gateway, extraction and copy-paste.
- Multi-FASTA export of all open sequences, crash recovery, update check.

### 1.4 File formats

See [File formats](file-formats.md) for the byte layout of `.xdna`, the GenBank dialect with the
original's qualifiers, the SnapGene reader and writer, and the enzyme and feature file formats.

## 2. Product decisions

| Decision | Choice | Why |
|---|---|---|
| Name | SerialClon.rs, a clearly distinct product name | "Serial Cloner" is someone else's product. The README says plainly that this is an independent reimplementation. |
| Window model | Real OS windows through Tauri, one per Serial Cloner window | An in-app MDI would look like a web app. Real windows are the interaction. |
| Frontend | TypeScript, Vite and Svelte 5, plain CSS with design tokens | About twenty window types with real state; the bundle stays small and each window mounts cheaply. |
| Look | Neutral modern chrome by default, plus an Aqua Classic theme reproducing the 2.6 look | Muscle memory lives in layout and shortcuts; the theme costs one stylesheet. |
| Core logic | 100 % Rust, UI-free, in workspace crates | Testable without a GUI; reusable as a library. |
| Rendering | Graphic Map in SVG; Sequence Window and Sequence Map on a canvas with virtualised rows | Plasmids are 3 to 200 kb; one DOM node per base is not viable. |
| Network | Off by default; NCBI, BLAST and ENA fetches are explicit user actions | The 2013-era page scraping is dead; the E-utilities and BLAST URL APIs replace it. |

## 3. Architecture

```text
SerialClon.rs/
├── crates/
│   ├── sc-seq/          sequence model, topology, IUPAC, translation, genetic codes,
│   │                    Tm / MW / pI / %GC, ORFs, bisulfite, reverse translation
│   ├── sc-format/       .xdna (read + write), FASTA, GenBank/EMBL, SnapGene, raw text
│   ├── sc-enzyme/       REBASE striderc parser, enzymes (type II and IIs), digestion,
│   │                    site usage, isoschizomers, compatible ends, the text Sequence Map
│   ├── sc-clone/        extract, ligate, PCR, adaptor, shRNA, Gateway, silent map,
│   │                    virtual digest and gel migration
│   ├── sc-align/        Smith-Waterman, consensus, differences; multiple alignment on mafft
│   ├── sc-features/     Feature Collections and the scan engine
│   └── sc-net/          NCBI E-utilities, ENA, BLAST URL API
├── src-tauri/           the shell: commands, window manager, menu, preferences, autosave
├── src/windows/<name>/  one Vite entry and one Svelte component per window
├── src/lib/             the typed command API, the help bus, the shortcut map
├── resources/           RELibrary, the Feature Collections, the codon usage tables
├── reference/           the original's sample files, golden files, recovered data
└── docs/                this site
```

Key dependencies: `tauri` 2, `bio` (pairwise alignment), `mafft` (multiple alignment), `serde`,
`thiserror`, `memchr`. GenBank and FASTA are hand-written in `sc-format` so that the original's
dialects are matched byte for byte.

The Tauri command surface is coarse-grained and document-oriented (`open_sequence`,
`save_sequence`, `digest`, `scan_features`, `run_pcr`, `ligate`, `gateway`, `align`). Sequence
state lives in a Rust `DocumentStore` keyed by document id. Windows hold ids, never copies, and
receive a `doc:changed` event after every edit, so a map can never disagree with its Sequence
Window.

## 4. Build and release

Two GitHub Actions workflows:

- **CI** runs on every push and pull request: `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test --workspace`, `svelte-check` and the Vite build, on macOS, Ubuntu and Windows.
- **Installers** is manual. It refuses to run unless the version in `src-tauri/tauri.conf.json`,
  the workspace `Cargo.toml` and `package.json` agree and are newer than the latest release tag.
  It then builds `.dmg` (Apple Silicon, Intel and Universal), `.msi` and NSIS `.exe`, `.deb` and
  `.AppImage` with `tauri-action` and drafts the GitHub Release `v<version>`.

The installers are not code-signed. The app checks GitHub Releases for a new version once a day
and tells the user; there is no in-app updater.
