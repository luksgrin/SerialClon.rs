# Migration from Serial Cloner 2.6.1

If you have used Serial Cloner, you already know SerialClon. This page is the honest list of what
is identical, what is different and why, and what was learned about the original along the way.

## Identical

- **Your files.** `.xdna` files open and save byte for byte, including DNA Strider's own fields,
  the comments, the features with their colours, and the cohesive ends. GenBank exports with the
  `SerialCloner_*` qualifiers round-trip. See [File formats](file-formats.md).
- **The windows.** Toolbar, Help window, Sequence Window (tabs, spinners, lock, Info panel, Find),
  Graphic Map, Sequence Map, Site Usage, Enzyme Library, Build a Construct, Create an Adaptor,
  Run a PCR, Make shRNA, Gateway, Extract Fragment, Align two Sequences, Virtual Cutter, Silent
  Map, Web Access, Manage Features, Codon Usage, Standard Genetic Code, DNA Alphabet, About.
- **The keystrokes.** Every shortcut of the manual, the spacebar / A / Delete keys on the map,
  typing to scrub enzyme lists, and the Alt modifier everywhere it existed
  (see [Keyboard shortcuts](shortcuts.md)).
- **The formulas.** Tm, protein MW, %GC, the site-count conventions and the map layouts were
  matched to the original's own printed output, not re-derived (table below).
- **The enzyme database.** REBASE in the `striderc` format the original used — a newer release,
  with the 2012 one it shipped kept in the repository for comparison.
- **Provenance comments.** Every product still says how it was made.

## Different, and why

| Original | SerialClon | Why |
|---|---|---|
| REALbasic windows; Find and Info as macOS drawers, floating windows on Windows | Real OS windows on all three platforms. Find is a window snapped to the right edge that follows its Sequence Window; the Info panel extends the window downwards | An in-app MDI would read as a web page. The Windows version of the original behaved this way already. |
| Build a Construct with an embedded graphic map on its left | Fragment slots plus the real Graphic Map: make the two-click selection there and *Use Selection in Build a Construct* | One map component instead of two; the map keeps all its features (drag, chooser, PNG). |
| A mini web browser scraping NCBI and EMBL pages | Web Access on the NCBI E-utilities, the ENA API and the BLAST URL API | The 2013-era pages no longer exist. Network stays off unless you press a button. |
| Several `RELibrary` files, switchable | One library: the bundled REBASE plus the enzymes you add; unchecked enzymes are hidden everywhere | Refreshing from REBASE is a script; per-file juggling is not needed. |
| ~150 built-in Features | 131 of them, reconstructed from public records; 19 missing | The original's Collection files are not public. Import your own; see [Data](data.md). |
| Auto-update download | A check that tells you a new release exists | The installers are unsigned; you install them yourself. |
| Signed macOS application (2.6) | Unsigned installers | See [Installation](installation.md) for the one-time unblocking. |
| Read selection aloud | Not implemented | No cross-platform speech API worth the dependency; may return. |
| Align two Sequences: BLAST2Seq first, local second | Local Smith-Waterman first, BLAST2Seq second, plus **Align Multiple Sequences** on MAFFT | The local one works offline and instantly; multiple alignment is new. |
| One genetic code + alternative starts | The 27 NCBI tables, six pK tables for pI | The 2.6 options, completed. |

Preferences knobs that existed in the original are listed on the
[Preferences](windows/preferences.md) page.

## Facts established from the original

These were not guessed: each was pinned by a test against something the original printed or
saved. They matter because a lab trusts a number only if it matches what it got before.

| Fact | Evidence |
|---|---|
| Tm uses `[Na+] = 0.050 M` and the split `< 14 / 14–50 / > 50` nt (the manual's "< 14 / > 51 / larger" is a typo) | The manual's Info-panel screenshot: 666 nt, 50.0 %GC → 79.7 °C only with that salt and that branch |
| Protein MW is the sum of **integer** average residue masses with no water term | The same screenshot: 221 aa → 24.12 kDa; fractional tables give 24.13–24.17 |
| Restriction maps print the **first base** of the recognition site | The original's printed map of pBluescript II SK(+): 51 enzymes at their printed coordinates |
| Non-palindromic sites and every offset-notation (type IIs) enzyme get a `>` / `<` arrow, even when the degenerate site is palindromic (HaeIV) | Same map |
| Features are labelled on the Sequence Map at their **midpoint** | Same map: F1 ori 13..453 labelled at 233, T3 771..790 at 780 |
| In `.xdna`, the feature-count byte is absent when there are no features; the third flag byte is "protect" | All seven shipped `.xdna` files |
| DNA Strider files carry populated reserved fields that must survive a save | The genuine Strider file among the samples |
| GenBank import copies the LOCUS and DEFINITION lines into the comment | `pBR322.xdna` as shipped, next to `pBR322.gb` |
| The original's AmpR consensus starts at the internal Met and must be degenerate at two bases | Its annotations of pBR322 and pBluescript are both 660 bp and differ at two positions |
| REBASE `#Name,site,a,b,` offsets count from the 3' end of the site; negative values are asymmetric type II sites | The `striderc` file itself (AciI `ccgc,-3,-1`) |

## Things we could not verify

- The `.xdna` encoding of cohesive ends beyond "blunt", and the `SerialCloner_Ends` GenBank
  field beyond `0,0,,0,`: no shipped sample has overhangs. A file saved from an extracted
  fragment in the original would settle both.
- The exact stacking order of colliding labels on the Sequence Map when two sites share a
  column (ours is alphabetical).
- The 19 missing built-in Features.

If you still have Serial Cloner running somewhere, files with cohesive ends and a copy of its
`Features` folder are the two most valuable things you could contribute.
