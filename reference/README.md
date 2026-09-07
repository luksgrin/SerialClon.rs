# Reference material

Everything here comes from **Serial Cloner 2.6.1** (F. Perez / SerialBasics, March 2013,
distributed as freeware) and is kept for one purpose: it is the **acceptance specification** for
this reimplementation — file-format fixtures, interface documentation, and the behaviour we are
matching. No Serial Cloner source code is included or was consulted; the binary was not
disassembled. The formats below were derived from the published documentation and from the sample
files the application itself ships.

Sources, all retrieved 2026-09-06:

- <http://serialbasics.free.fr/Serial_Cloner.html> — product page
- <http://serialbasics.free.fr/Serial_Cloner-Download.html> — downloads
- `SerialCloner2-6.tar.gz` — the Linux distribution, which contains `RELibrary`, the sample
  sequences, the changelog, and the `Older files/` manuals

## `sequences/` — format fixtures

The example sequences shipped inside the application, used as round-trip tests for `sc-format`.

| File | Format | Notes |
|---|---|---|
| `pBluescript-II-KS+.xdna` | Xdna | circular, 2961 bp, comment only, no annotation block |
| `pBluescript-II-KS+-annotated.xdna` | Xdna | with features |
| `pBR322.xdna`, `pBR322-annotated.xdna` | Xdna | 4361 bp; the annotated one is the worked example for the annotation block in `PLAN.md` §1.4 (`ColE1 origin` / `rep_origin` / 3144 / 2516 / `70,130,180,`) |
| `pBR322.gb` | GenBank | same plasmid, for cross-format comparison |
| `pBR322.nucl` | MacVector | binary import target |
| `Gateway-vectors.ma4` | Vector NTI | multi-sequence import target |
| `pDONR201.xdna`, `pDEST14.xdna` | Xdna | Gateway donor/destination vectors — BP and LR test material |
| `pEGFP-N1.xdna` / `.fasta` | Xdna / FASTA | feature-scan test material (EGFP, KanR, SV40 polyA…) |
| `*.fasta` | FASTA | the same plasmids, to check import/export agreement |
| `pBluescript-II-SK+.gb` | GenBank (Serial Cloner export) | transcribed from the Features ReadMe §VII.A "Copy as GenBank" example — the only sample of the original's own GenBank dialect (`SerialCloner_*` comments and qualifiers) |

Lower case in a sequence is meaningful (Serial Cloner can restrict translation to uppercase) and
must survive a round trip. Unknown header bytes must be preserved verbatim rather than zeroed.

## `docs/` — documentation and changelog

| File | What it is |
|---|---|
| `SerialCloner-User-Manual-v1.2.pdf` / `.txt` | The only full user manual — menus, every shortcut, the Sequence Window field by field, Find, Graphic Map, Sequence Map, and the exact Tm formulas |
| `SerialCloner-Features-ReadMe-v2.1.pdf` / `.txt` | The features/annotation system: data model, display, scanning, collections, and the complete list of ~150 built-in features |
| `Serial-Cloner-History.txt` | Full changelog, beta10 → 2.6.1 — the most complete function inventory that exists |
| `version_2-6-1-details.txt` | Final release notes |
| `pBluescript-II-SK+-unique-sites-map.txt` | The Sequence Map of pBluescript II SK(+) printed by Serial Cloner 2.1 (Features ReadMe §IV), "cutting maximum 1 time": the digestion golden values — 51 enzymes with positions and strand arrows |

`.txt` files are `pdftotext -layout` extractions of the PDFs, kept because they are greppable.

## `ui/` — interface acceptance targets

Screenshots extracted from the manuals. These are what "same UI" is measured against.

| File | Window |
|---|---|
| `00-desktop-overview.png` | The whole application: toolbar strip, Sequence Window with the Info panel open, Find drawer, circular Graphic Map, RE Library, Help window |
| `01-sequence-window.png` | Sequence Window, all 20 numbered elements |
| `02-…-comments-tab.png` | Comments tab and the 255-character DNA Strider limit |
| `03-…-extremities-tab.png` | Cohesive 5' / 3' end editor |
| `04-selection-info-and-translation.png` | Tm, base counts, %GC, size, MW, pI, peptide field |
| `05-find-drawer.png` | Find drawer — Sequence / Site / ORFs tabs, hit table |
| `06-graphic-map-linear.png` | Linear map: stacked draggable labels, cohesive ends in red |
| `07-graphic-map-circular.png` | Circular map with the Features / Unique / Double / Particular controls |
| `08-sequence-map.png` | Text map: the four control panels and multi-frame translation |
| `09-manage-features.png` | Feature Collection editor, Simple / Complex definitions |

## `RELibrary-striderc.211`

The enzyme database Serial Cloner 2.6.1 shipped: REBASE version 211, October 2012. Kept so that
digestion results can be compared against the original application exactly. The runtime default
lives at `../resources/RELibrary` and tracks the current REBASE release
(`scripts/update-rebase.sh`).

REBASE is © Dr. Richard J. Roberts, New England Biolabs, and is free for academic use —
<http://rebase.neb.com>.
