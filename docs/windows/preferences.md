# Preferences

!!! info "Reflects 1.0"
    The Preferences window is part of the 1.0 release. This page lists the settings as the
    original defined them (User Manual and changelog), which is what 1.0 implements; until it
    lands, the Protein-menu settings (genetic code, pK table, codon usage) are the ones that
    exist.

SerialClon ▸ Preferences… on macOS, File ▸ Preferences… on Windows and Linux (⌘, / Ctrl+,).
Settings are saved in `settings.json` (see [where your data lives](../installation.md#where-your-data-lives)).

## General

- **Autolock** newly opened sequences.
- Number of **recent files** (up to 25).
- **Warn when importing** non-Serial Cloner files.
- **Check for updates** at startup.
- **Look**: neutral modern chrome (default) or the **Aqua Classic** theme reproducing the 2.6 look.

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
  code*.
- Translate only upper-case bases in the Sequence Map.

## Cloning

- 3' A overhang by default on PCR products (TA cloning).

## Network

- Proxy settings, when the system's are not enough.
- The `email` sent with NCBI requests.
