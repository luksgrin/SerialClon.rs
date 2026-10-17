# Reference material

Everything here comes from **Serial Cloner 2.6.1** (F. Perez / SerialBasics, March 2013,
distributed as freeware) and is kept for one purpose: it is the **acceptance specification** for
this reimplementation — file-format fixtures, interface documentation, and the behaviour we are
matching. No Serial Cloner source code is included or was consulted, and no binary is committed.
The formats below were derived from the application's published documentation and from the
sample files it ships; the manuals and screenshots themselves are not redistributed here. In October 2026 the distributed binaries were additionally examined for
interoperability and data recovery: the built-in Feature database and other embedded tables were
read out of them, a few routines were decompiled to compare behaviour, and the Linux build was run
(under Xvfb in a 32-bit Debian container) to produce golden files; what was found is under
`original/` and in `docs/original-behaviour.md`. SerialClon.rs is an independent reimplementation
and is not affiliated with or endorsed by the author of Serial Cloner.

Sources, all retrieved 2026-09-06:

- <http://serialbasics.free.fr/Serial_Cloner.html> — product page
- <http://serialbasics.free.fr/Serial_Cloner-Download.html> — downloads
- `SerialCloner2-6.tar.gz` — the Linux distribution, which contains `RELibrary`, the sample
  sequences, the changelog, and the `Older files/` manuals

## `original/` — data recovered from the binaries (2026-10-17)

`ARCHIVES.md` and `SHA256SUMS.txt` inventory the five downloads. `features/` is the original's
built-in Feature database as it writes it on first launch (JSON field for field, plus the raw
files); `embedded/` the other tables read out of the executable (fallback enzyme library, DNA
ladders, codon-usage tables, genetic code, feature-type groups, defaults); `golden/` the files the
original itself produced when driven under Xvfb: its scan results for the sample sequences and
linear fragments with every cohesive-end combination, as `.xdna` and GenBank. How, and what was
compared, is in `docs/original-behaviour.md`.

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
| `snapgene/*.dna` | SnapGene | 24 real files from the Biopython, sgffp, seqparse and dotDNA projects (sources and licences in `snapgene/README.md`): the byte-exact round-trip fixtures of the SnapGene reader/writer |

Lower case in a sequence is meaningful (Serial Cloner can restrict translation to uppercase) and
must survive a round trip. Unknown header bytes must be preserved verbatim rather than zeroed.

## `docs/` — golden values and the behaviour comparison

| File | What it is |
|---|---|
| `pBluescript-II-SK+-unique-sites-map.txt` | The Sequence Map of pBluescript II SK(+) as printed by the original, "cutting maximum 1 time": the digestion golden values — 51 enzymes with positions and strand arrows |
| `original-behaviour.md` | The function-by-function comparison with the original and how the data under `original/` was obtained |

## `sequences/genbank/` — public records behind the built-in Features

GenBank records (NCBI, public domain) downloaded on 2026-09-07 with E-utilities, named
`<accession>-<vector>.gb`. They are the sources of the researched definitions in
`resources/features/` (see `resources/features/README.md`) and the fixtures of
`crates/sc-features/tests/researched.rs`, which checks that a scan finds each definition at the
record's own annotated coordinates: pGEX-4T-1 (U13853), pRS303/304/305/316/424 (U03435, U03436,
U03437, U03442, U03453), YCp50 (X70276), pBI121 (AF485783), pLKO.1 (PX675714), pTrc99A (U13872),
pBAD24 (X81837), pAN7-1 (Z32698), pIRES1neo (U89673).

## `RELibrary-striderc.211`

The enzyme database Serial Cloner 2.6.1 shipped: REBASE version 211, October 2012. Kept so that
digestion results can be compared against the original application exactly. The runtime default
lives at `../resources/RELibrary` and tracks the current REBASE release
(`scripts/update-rebase.sh`).

REBASE is © Dr. Richard J. Roberts, New England Biolabs, and is free for academic use —
<http://rebase.neb.com>.
