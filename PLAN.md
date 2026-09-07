# SerialClon.rs — Plan

A faithful, maintained reimplementation of **Serial Cloner 2.6.1** (F. Perez, SerialBasics, last
release March 2013) in **Rust + Tauri 2**, shipping signed installers for Windows, macOS and Ubuntu.

The goal is *not* "another sequence editor". It is: **the same windows, the same interactions, the
same keystrokes, the same file formats** — on a stack that still builds in 2026.

---

## 1. What Serial Cloner actually is (research findings)

| | |
|---|---|
| Last version | 2.6.1, March 2013 |
| Original stack | REALbasic/Xojo (`libRBAppearancePak.so`, `libRBBrowser.so`, `libRBXML.so` in the Linux tarball) |
| Licence | Freeware, closed source, unsupported. Linux build shipped "as is", "known interface problems" |
| Platforms | macOS (signed for Mountain Lion), Windows, Linux (Ubuntu, untested) |
| Native format | `.xdna` — an *extended DNA Strider* binary format |
| Enzyme data | `RELibrary` — REBASE `striderc.211` (Oct 2012), plain text |

Everything below was extracted from: the official site, the v1.2 User Manual PDF, the "Features
ReadMe" PDF (v2.1), `Serial-Cloner-History.txt` (full changelog beta10 → 2.6.1), and the shipped
Linux tarball (`RELibrary`, example `.xdna`/`.gb`/`.nucl`/`.ma4` sequences). Copies of these
reference artefacts are committed under `reference/` — they are the acceptance fixtures.

### 1.1 The UX that has to be preserved

These are the things that make Serial Cloner *feel* like Serial Cloner. They are the actual
requirements; the feature list below is secondary.

1. **Floating-window desktop, not a single-page app.** A thin always-on-top **Toolbar** strip
   (New, Open, Save, Print, Graph Map, Seq Map, Site Usage, Find, Construct, Adaptor, PCR, shRNA,
   Gateway, Extract, Align, Help) plus independent windows the user tiles themselves. Multiple
   sequences open simultaneously, each with its own window and its own maps.
2. **The permanent Help window.** A small window (default visible) that shows a one-line
   explanation of *whatever widget the mouse is currently over*, in the front-most window
   ("Sequence Window : Sequence Field — Enter here the actual sequence of the fragment…").
   This single detail is the biggest driver of the "gentle" feel and is trivially replicated with a
   `data-help` attribute + mouseover bus.
3. **Drawers.** `Find` and `Show Info` attach to their parent Sequence Window and follow it when it
   moves. One drawer per sequence window on macOS.
4. **Numeric selection everywhere.** `from` / `to` / `length` are *editable spinner fields*, not
   read-outs. Type 129 in `from`, 861 in `length`, the selection follows. This is how people
   actually work in it.
5. **Keyboard-scrubbing enzyme lists.** Type "Bam" into any enzyme list and it jumps. Unique sites
   are **bold**, absent sites are *italic*, everywhere an enzyme list appears.
6. **Two-click fragment selection on the map.** Click a site name box → it turns green (BEGIN).
   Double-click or shift-click another → red (END). **Spacebar** toggles CW/CCW (circular) or
   sense/antisense (linear). `A` selects the whole linear insert. `Delete` cancels. Then
   `Extract Fragment` (⌘6) drops the fragment — *with its cohesive ends* — into a new Sequence
   Window whose comment field records how it was made.
7. **Draggable labels.** Restriction-site and feature name boxes on the Graphic Map can be dragged
   before printing/copying so nothing overlaps. Users rely on this for figures.
8. **Alt as the "advanced" modifier.** Alt-click Save = Save As. Alt before the `−` button = delete
   features without confirmation (button turns red). Alt-click a Show checkbox = show/hide *all*.
   Alt-click toolbar = alternative commands.
9. **Everything is one context-menu away.** Right-click in sequence field, on the map, on the
   sequence name (rename), on the topology field (linearize/circularize).
10. **The comment field is the lab notebook.** Every generated sequence (PCR product, ligation,
    extracted fragment, Gateway product) arrives with a human-readable provenance note in its
    comments.

### 1.2 Complete window inventory

| Window | Menu / shortcut | Purpose |
|---|---|---|
| Toolbar | Window ▸ Show/Hide Toolbar | Icon strip, always available |
| Help | Window ▸ Show/Hide Help | Contextual hover help |
| **Sequence Window** | ⌘N / ⌘O | Name, length, type, format, topology, Lock checkbox, from/to/length spinners, tabs: **Sequence / Comments / Features / Extremities**, buttons: Save, Sequence Map, Graphic Map, Find, Show Selection Information & Translation |
| Info drawer | ⌘I | Tm, A/C/G/T counts, %GC, translation (size, MW, pI), peptide field, "show only 1st ORF" toggle, back-select DNA from peptide |
| Find drawer | ⌘F | Tabs: **Sequence / Site / ORFs**; instant search, both strands, sortable hit table (position/size/strand), incomplete ORFs in italic, wraps around the origin of circular plasmids |
| **Graphic Map** | ⌘G | Circular or linear map; checkboxes Features / Unique / Double / Particular sites + "Choose site"; draggable labels; cohesive ends drawn in red with the overhang sequence boxed; resizable H and W; save as PNG; print |
| **Sequence Map** | ⌘M | Text restriction map + multi-frame translation. Panels: search (sub-sequence or enzyme), Translation (translate all / from–to / frames 1‑2‑3 / codon mode), Restriction sites (hide / all / cutting max *n* times / particular), Display (reverse strand, coordinates, char size, nt per line), Features toggle with `>`/`<` sense marks and a legend |
| Site Usage (interactive) | Restriction menu | Live frequency table; filter against a *second* sequence ("unique here, absent there") |
| Site Usage (table) | Restriction menu | Printable: all / unique / absent |
| Enzyme Library | Restriction menu | Manage RELibrary files, filter by site length / blunt / cohesive, find isoschizomers and compatible ends, add type II and IIs enzymes |
| **Build a Construct** | ⌘1 | Graphic map on the left + fragment slots; blunt buttons; **Ligate** |
| Create an Adaptor | ⌘2 | Core dsDNA + optional cohesive extensions → oligos to order + in-memory adaptor sequence |
| Run a PCR | ⌘3 | Matrix + 2 primers with un-annealed tails; optional 3' A overhang for TA cloning; primer Anchor/Core recognised as Features |
| Make shRNA | ⌘4 | Stored layouts (cohesive ends, prefix, loop, suffix), siRNA core, point mutations → oligos + insert |
| Gateway cloning | ⌘5 | BP and LR; att site detection; also yeast-style homologous recombination |
| Extract Fragment | ⌘6 | From selection or between two sites |
| Align two Sequences | ⌘7 | Local (Smith-Waterman) or NCBI BLAST2Seq; 3-frame translation; features shown; consensus extraction; DNA↔protein switch; highlight-differences toggle |
| Virtual Cutter | Tools | Predicted fragment count/sizes + simulated gel migration |
| Silent Map | Tools | Sites introducible without changing the peptide |
| Web Access | Tools | Mini browser; detects NCBI/EMBL entries → "Create a New Sequence Window"; direct BLAST submission |
| Manage Features | Features ▸ Manage | Collections (Generic, Yeast, Other, Tags & Cut, Gateway) × Simple/Complex feature definitions, import (Serial Cloner / generic table / ApE), export |
| Manage / Import Codon Usage | Protein menu | Codon bias tables, import from web |
| Preferences | ⌘, | Colours, char sizes, autolock, recent-file count, proxy, pK tables, genetic code, ORF minimum, printing sizes… |
| Standard Genetic Code, DNA Alphabet, About | Window menu | Reference cards |

### 1.3 Function inventory (from the changelog, beta10 → 2.6.1)

- Sequence ops: uppercase/lowercase, antiparallel (⌘/), reverse, complement, linearize/circularize,
  set origin, lock, degenerate-DNA mode ↔ normal, read selection aloud (TTS).
- Translation: standard + alternative genetic codes, alternative start codons, codon table from the
  selected codon-usage table, stop-through mode, MW, **pI** (selectable pK tables).
- Protein windows (2.5+): open/edit protein, protein align, reverse-translate with codon bias
  (features inherited), scan for peptide features, BLAST protein.
- Bisulfite conversion (sense and antisense strands).
- Tm formulas (exact, from the manual — must match to the decimal):
  - `n < 14`: `Tm = (A+T)*2 + (G+C)*4 − 16.6*log10(0.050) + 16.6*log10([Na+])`
  - `14 ≤ n ≤ 51`: `Tm = 100.5 + 41*(G+C)/n − 820/n + 16.6*log10([Na+])`
  - `n > 51`: `Tm = 81.5 + 41*(G+C)/n − 500/n + 16.6*log10([Na+])`
  (the manual labels these `<14`, `>51`, "larger" — resolve the boundary empirically against the
  original binary and document it.)
- Features: manual entry, from selection (sequence or graphic map), automatic **Scan** against
  ~150 built-in consensus sequences, per-feature colour/arrow/show, drag to reorder (= z-order),
  inheritance through PCR / ligation / Gateway / extract / copy-paste, dedup.
- Multi-FASTA export/copy of all open sequences (for CLUSTAL).
- Crash recovery of unsaved sequences and PCR windows.
- Auto-update check.

### 1.4 File formats

**Read + write (native): `.xdna`.** Byte layout verified against all seven shipped `.xdna`
files (M1; `crates/sc-format/src/xdna.rs` is the reference implementation and round-trips every
fixture byte for byte):

```
offset  size  meaning
0       1     version (0)
1       1     sequence type   (0 unknown, 1 DNA, 2 degenerate DNA, 3 RNA, 4 protein)
2       1     topology        (0 linear, 1 circular)
3       25    DNA Strider fields (timestamps, window geometry, selection) — zero in
              Serial Cloner files, populated in genuine Strider files; preserved verbatim
28      4     u32be sequence length
32      64    more Strider fields, preserved verbatim
96      4     u32be comment length
100     12    reserved; Serial Cloner writes 0xFF at offset 111, DNA Strider at offset 92
112     …     sequence ASCII (length bytes, case preserved)
        …     comment ISO-8859-1 (comment length bytes, lines separated by 0x0D)
        1     0x00 → Serial Cloner annotation block follows (absent in plain Strider files)
```
Annotation block (all strings are Pascal strings: `u8 len` + bytes):
```
pstr right overhang length (decimal text; negative = 3', positive = 5')
     overhang sequence (abs(len) bytes) if len != 0        ← not attested by any sample
pstr left overhang  (same)
u8   feature count  — ABSENT when there are no features (block ends after the overhangs)
per feature:
  pstr name
  pstr description (qualifiers separated by 0x0D)
  pstr type          e.g. "rep_origin", "CDS"
  pstr start (1-based decimal text, first base in feature direction)
  pstr end   (last base in feature direction; start > end for reverse features and for
              forward features that wrap the origin, e.g. pEGFP-N1 "New Feature" 4000 → 200)
  4 bytes  flags: [strand 1=fwd/0=rev][show][protect][arrow]
  pstr rgb  e.g. "70,130,180,"
```
Confirmed against pBR322: `ColE1 origin / rep_origin / 3144 / 2516 / 00 01 00 01 / "70,130,180,"`.
`protect` is the "user edited, do not replace on re-scan" bit (set on pEGFP-N1's hand-made
feature, exported as `/SerialCloner_Protect`). Lower-case in the sequence is meaningful (Serial
Cloner can restrict translation to uppercase) and must round-trip.

Other facts established in M1, each pinned by a test:
- Restriction maps print the **first base of the recognition site**, and non-palindromic sites get
  a `>`/`<` strand arrow — verified against the original's printed map of pBluescript II SK(+)
  (`reference/docs/pBluescript-II-SK+-unique-sites-map.txt`, 51 enzymes).
- REBASE `#Name,site,a,b,` offsets are counted from the 3' end of the site (REBASE `(a/b)`);
  negative values encode asymmetric type II sites such as AciI `ccgc,-3,-1`.
- Tm uses `[Na+] = 0.050 M` and the split `<14 / 14–50 / >50` (the manual's "<14 / >51 / larger"
  is a typo); the Info-panel screenshot (666 nt, 50 %GC, 79.7 °C) pins it.
- Protein MW is the sum of **integer** average residue masses with no water term (24.12 kDa for
  the 221-aa peptide in the same screenshot; fractional tables give 24.13–24.17).
- GenBank import puts the LOCUS and DEFINITION lines, 0x0D-terminated, into the comment.

**Import:** FASTA (incl. degenerate + multi-FASTA), GenBank/EMBL (with features, incl. Serial
Cloner's colour qualifiers) — both done in M1 — then ApE, Vector NTI `.ma4` (single and
multi-sequence), MacVector `.nucl`, DNAStar EditSeq `.seq`, pDRAW32 `.pdw`, raw text.
**Export:** `.xdna`, FASTA, GenBank (with features + colours) — done in M1 — PNG maps, printable
text maps.
*Stretch:* SnapGene `.dna` import — the format most collaborators send today.

**Enzymes:** REBASE `striderc.NNN`, format `Name,site_with_/[,cut5,cut3],` where a leading `#`
marks type IIs and the two integers are the downstream cut offsets:
```
AatII,gacgt/c,
#AarI,cacctgc,4,8,
AasI,gacnnnn/nngtc,
```

---

## 2. Product decisions

| Decision | Choice | Why |
|---|---|---|
| Name | working title **SerialClon.rs**; ship as a clearly distinct product name | "Serial Cloner" is someone else's product name. Say plainly in the README that this is an independent reimplementation, not affiliated with SerialBasics. |
| Window model | **Real OS windows** via Tauri `WebviewWindow`, one per Serial Cloner window | An in-app MDI would look like a web app. Real windows are the UX. |
| Frontend | **TypeScript + Vite + Svelte 5**, plain CSS with design tokens | ~20 window types with real state; vanilla JS (as in `flow-cytometry-analyzer`) would not scale here. Svelte keeps the bundle small and per-window mount cheap. |
| Look | Neutral modern chrome by default, **plus an "Aqua Classic" theme** reproducing the 2.6 look | Muscle memory lives in *layout and shortcuts*, not in brushed metal — but the option costs one stylesheet and makes the switch painless. |
| Core logic | 100 % Rust, UI-free, in workspace crates | Testable without a GUI; reusable as a CLI/library later. |
| Rendering | Graphic Map = **SVG** (draggable labels, crisp print/export); Sequence Window + Sequence Map = **canvas with virtualised rows** | Plasmids are 3–200 kb; DOM-per-base is not viable. |
| Network | Off by default; NCBI/BLAST/EMBL/codon-usage fetches are explicit user actions | 2013-era HTML scraping is dead. Use the NCBI E-utilities and BLAST URL APIs, not page parsing. |

---

## 3. Architecture

```
SerialClon.rs/
├── crates/
│   ├── sc-seq/          # sequence model, topology, IUPAC, translation, genetic codes,
│   │                    # Tm / MW / pI / %GC, ORF, bisulfite, reverse translation
│   ├── sc-format/       # xdna (read+write), fasta, genbank/embl, ape, vntI ma4,
│   │                    # macvector nucl, dnastar seq, pdraw — behind one `SeqFile` trait
│   ├── sc-enzyme/       # REBASE striderc parser, enzyme model (type II & IIs),
│   │                    # digestion, site usage, isoschizomers, compatible ends
│   ├── sc-clone/        # extract, ligate, PCR, adaptor, shRNA, Gateway BP/LR,
│   │                    # silent map, virtual digest + gel migration
│   ├── sc-align/        # Smith-Waterman local align (rust-bio), consensus, diff highlight
│   ├── sc-features/     # feature collections, scan engine (simple + complex definitions)
│   └── sc-net/          # NCBI E-utilities, BLAST URL API, EMBL, codon-usage import
├── src-tauri/           # thin: commands, window manager, prefs, recent files, autosave
├── src/                 # Svelte frontend, one directory per window
│   ├── windows/{sequence,graphic-map,sequence-map,find,info,construct,pcr,adaptor,
│   │            shrna,gateway,align,site-usage,enzyme-library,virtual-cutter,
│   │            silent-map,features,codon-usage,prefs,help,toolbar}/
│   ├── lib/help-bus.ts  # the contextual Help window
│   └── lib/shortcuts.ts # the full ⌘/Ctrl map
├── resources/
│   ├── RELibrary        # REBASE striderc snapshot + `scripts/update-rebase.sh`
│   └── features/*.json  # the 5 built-in feature collections
├── reference/           # original .xdna/.gb/.ma4/.nucl fixtures + extracted manuals
└── .github/workflows/
```

Key dependencies: `tauri 2.11`, `bio 4.0` (pairwise alignment), `gb-io 0.9` (GenBank read/write),
`noodles-fasta 0.66`, `serde`, `thiserror`, `nom` (binary/format parsing), `memchr`.

**Tauri command surface** — coarse-grained and document-oriented (`open_sequence`, `save_sequence`,
`digest`, `scan_features`, `run_pcr`, `ligate`, `gateway`, `align`, …). Sequence state lives in a
Rust `DocumentStore` keyed by document id; windows hold ids, not copies, so a Graphic Map and its
parent Sequence Window can never diverge. Cross-window sync via Tauri events.

---

## 4. Milestones

Each milestone ends with installers on all three OSes and a demo you can actually use.

### M0 — Skeleton (1 week)
Repo, workspace, Tauri 2 app, Vite/Svelte, icons, CI matrix (macOS 14, ubuntu-22.04,
windows-latest), release workflow producing `.dmg`/`.app` (arm64 + x86_64), `.msi` + NSIS,
`.deb` + `.AppImage`. Commit `reference/` fixtures. **Exit:** `cargo test` + three green installers.

### M1 — The core, headless (2–3 weeks)
`sc-seq`, `sc-format` (xdna read/write + FASTA + GenBank), `sc-enzyme` (REBASE + digestion incl.
type IIs). **Exit:** byte-exact round-trip of every `reference/*.xdna`; digestion of pBluescript
and pBR322 matches the site counts in the original manual's printed maps.

### M2 — Sequence Window + Toolbar + Help (2–3 weeks)
The real one: tabs, spinners, lock, topology context menu, virtualised sequence canvas, Find
drawer (sequence/site/ORF, both strands, origin-spanning), Info drawer (Tm/%GC/translation/MW/pI,
peptide back-selection), undo/redo (5 levels), contextual menus, the Help window, the full shortcut
map, Open/Save/Save As/Export, recent files, autosave + crash recovery.
**Exit:** a molecular biologist can do a day's sequence inspection in it.

### M3 — Maps (3–4 weeks)
Graphic Map (circular + linear, unique/double/particular, Choose site, draggable labels, cohesive
ends, resize, PNG export, print) and Sequence Map (all four control panels, multi-frame
translation, codon mode, features with `>`/`<` and legend, in-map search). Two-click fragment
selection + spacebar CW/CCW + Extract Fragment.
**Exit:** side-by-side screenshot diff against the manual's figures for pBluescript II KS(+).

### M4 — Features (2 weeks)
Features tab, colour/arrow/show, drag-to-reorder, `+`/`−` with the Alt no-confirm behaviour, the
5 built-in collections, the scan engine (simple = one consensus; complex = 5'/3' + potential 5'/3'
+ up/downstream context, DNA or peptide, sense-only option), Manage Features window,
import/export (Serial Cloner, generic table, ApE), scan-preserves-user-edits semantics.

**M4 follow-up — the built-in Feature database is incomplete.** The original's Collection files
are not in the reference material, only the ~150 names; `resources/features/README.md` lists what
is shipped (63 definitions, each verified against a sample file or a textbook motif) and what is
missing (mostly long marker genes, yeast markers, promoters and reporter ORFs). Researching and
adding the missing consensus sequences is a separate step; a copy of the original's
`SerialCloner/Features/` folder from any old installation would settle it at once.

### M5 — Cloning (3–4 weeks)
Build a Construct (graphic selection → slots → blunting → Ligate), Create an Adaptor, Run a PCR
(tails, TA overhang, primer features, right-click "use selection as forward/reverse primer"),
Make shRNA (stored layouts), Gateway BP/LR + homologous recombination, Site Usage (interactive with
second-sequence filter, and printable), Enzyme Library, Virtual Cutter + gel, Silent Map.
Feature inheritance through every one of these operations.
**Exit:** reproduce a real subcloning from the user's own lab notebook, end to end.

### M6 — Alignment, protein, web (2–3 weeks)
Local Smith-Waterman align with translation/features/consensus/diff-highlight, protein windows,
reverse translation + codon usage tables, pI, bisulfite, NCBI/EMBL import via E-utilities, remote
BLAST via the BLAST URL API, multi-FASTA export.

### M7 — Polish & ship 1.0 (2–3 weeks)
Preferences (every knob the original had), printing on all three platforms, Aqua Classic theme,
file-type associations for `.xdna`, code signing + notarisation (macOS) and Authenticode (Windows),
Tauri updater, docs site (mkdocs, mirroring the reference repo), migration guide.

**Total: ~4–5 months of focused part-time work**, first genuinely usable build at M3 (~2 months).

---

## 5. Build & release

Start from `luksgrin/flow-cytometry-analyzer`'s two workflows, with three changes:

1. Replace the hand-rolled per-OS jobs with **`tauri-apps/tauri-action`** driven by a matrix
   (`macos-14` × {aarch64, x86_64}, `ubuntu-22.04`, `windows-latest`); it creates the draft GitHub
   Release and uploads bundles + `latest.json` for the updater in one step.
2. Keep the existing Linux deps block verbatim (`libwebkit2gtk-4.1-dev`, `libxdo-dev`,
   `libayatana-appindicator3-dev`, `librsvg2-dev`, …) — it is correct for Tauri 2 on 22.04.
3. Add `cargo test --workspace` and `cargo clippy -- -D warnings` to the check job; the core crates
   are where the science lives.

Targets: `dmg` + `app` (both arches, plus a universal build), `msi` + `nsis`, `deb` + `appimage`.
Signing secrets: `APPLE_CERTIFICATE`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`/`APPLE_PASSWORD`/
`APPLE_TEAM_ID` for notarisation; `WINDOWS_CERTIFICATE` + password. Without them the app still
builds — users just get the Gatekeeper/SmartScreen warning the original had.

---

## 6. Risks and how to defuse them

| Risk | Mitigation |
|---|---|
| `.xdna` reserved-byte fields we don't understand | Round-trip tests on real files; preserve unknown bytes verbatim instead of zeroing them |
| Tm / pI / MW must match the original to the decimal or users will distrust it | Golden-value tests generated by running the original 2.6.1 binary on fixed inputs, checked in as CSV |
| Graphic Map label layout (the collision-avoidance stacking in the screenshots) is non-trivial | Dedicated spike in M3; greedy row-packing + user drag as the escape hatch |
| NCBI page scraping is dead | E-utilities + BLAST URL API instead; degrade gracefully offline |
| Codon-usage import (Kazusa) may be gone | Bundle common tables; make import a pluggable URL |
| REBASE licensing | Data is free for academic use, © R. J. Roberts — ship with attribution + an `update-rebase.sh` that fetches a fresh `striderc.NNN` |
| Trademark ("Serial Cloner", "Gateway") | Distinct product name, explicit non-affiliation notice, "Gateway™ cloning" referenced descriptively |
| Scope: 20 windows is a lot | M2+M3 alone already replace 80 % of daily use — ship early, iterate |

---

## 7. Open questions for you

1. **Priority order** — is M5 (cloning: Build a Construct, PCR, Gateway) more urgent to you than M4
   (features/scan)? Your daily workflow should set the order, not this document.
2. **Look** — modern-neutral by default with Aqua Classic as an option, or pixel-faithful 2.6 from
   day one?
3. **Licence and visibility** — MIT/Apache-2.0 public repo, like `flow-cytometry-analyzer`?
4. **SnapGene `.dna` import** — worth pulling into M1, or a post-1.0 stretch?
5. Do you have **your own `.xdna` corpus** (lab plasmids) we can use as private round-trip
   fixtures? That is the strongest correctness signal available.
