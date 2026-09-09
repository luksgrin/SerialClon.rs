# Preferences

SerialClon ▸ Preferences… on macOS, File ▸ Preferences… on Windows and Linux (⌘, / Ctrl+,).
Every change applies to the open windows immediately and is saved in `prefs.json`; the Protein
menu's genetic-code, pK-table and codon-usage choices are mirrored in `settings.json` (see
[where your data lives](../installation.md#where-your-data-lives)).

## General

- **Autolock** newly opened sequences.
- Number of **recent files** (up to 25).
- **Warn when importing** non-Serial Cloner files.
- **Check for updates** at startup (once a day, against the GitHub releases; a dialog offers the
  release page when a newer version exists).
- **Autosave interval** for crash recovery (30 s by default).
- **Look**: neutral modern chrome (default) or the **Aqua Classic** theme reproducing the 2.6 look.
- **Toolbar vertical shift**, in pixels, as in the original.

## Sequence Window

- Character size of the sequence field.
- Limit or not the width of Sequence and Protein windows (2.6.1).
- Minimum ORF size for "find all ORFs".
- Read-out speed for Sequence ▸ Read selection.

## Maps

- Colours of unique, double and particular sites; of the BEGIN (green) and END (red) selection.
- Character sizes of restriction-site names and of Feature names on the Graphic Map; box or not
  the names; box and colour unique-site names.
- Sequence Map defaults: show the reverse strand, character size on screen and for printing,
  nucleotides per line, amino acid aligned with the first or the middle base of the codon.

## Translation and protein

- **Genetic code** (the 27 NCBI tables and alternative start codons).
- **pK table** for pI.
- **Codon usage table** for reverse translation, and *use the codon usage table as the genetic
  code*. (Translating only upper-case bases is a checkbox of the Sequence Map window itself.)

## Cloning

- 3' A overhang by default on PCR products (TA cloning).

## Printing

- Page setup for the maps and tables: landscape or portrait, and the margins (applied as the
  page style when printing). The Sequence Map's print character size is under *Maps*.

## Network

- Proxy: the system proxy is used by default; a manual proxy URL can be given when that is not
  enough.

## Reset

The **Reset Preferences** button restores every default. Launching the application with the
`--reset-prefs` flag does the same before any window opens — it stands in for the original's
Ctrl+Shift-at-launch, which cannot be read before the event loop exists.
