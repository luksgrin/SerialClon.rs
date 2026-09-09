# Data

## Restriction enzymes — REBASE

The enzyme database is [REBASE](http://rebase.neb.com), © Dr. Richard J. Roberts, New England
Biolabs, free for academic use, in the `striderc` flavour (the DNA Strider / Serial Cloner
format). The bundled file is `resources/RELibrary`; the one Serial Cloner 2.6.1 shipped (REBASE
211, October 2012) is kept as `reference/RELibrary-striderc.211` so that digestion results can be
compared with the original exactly.

To refresh the bundled database from REBASE before a release:

```sh
scripts/update-rebase.sh
```

The script downloads the current `striderc` file, checks its banner and entry count, and only
then overwrites the bundled copy. Enzymes you add in the Enzyme Library are stored separately and
merged on top at startup, so a refresh never loses them.

## Built-in Feature Collections

The original shipped ~150 Features in five Collections, but only their **names** are documented;
the Collection files themselves were never public. SerialClon's Collections are a reconstruction
from public records — **131 of the 150 names**:

| Collection | Present | Missing |
|---|---|---|
| Generic | 59 (+5 covered under another name) | 13: TetA, hGH polyA signal, EM7 promoter, mPGK Prom, tsr, oriS, oriRg, AML-P, EYFP, ECFP, AcGFP, IRES [2], Synthetic intron |
| Yeast | 24 | — |
| Other | 10 | — |
| Tags & Cut | 22 | — |
| Gateway | 11 | 6: attP3, attP4, attL3, attL4, attR3, attR4 (MultiSite) |

Rules that were followed: nothing typed from memory. Every DNA consensus was read out of an
annotated sample file the original shipped (which is exactly what its own scan found there) or
out of a GenBank / RefSeq / SGD record at the record's annotated coordinates; every peptide comes
from UniProt or a published sequence. The accession and coordinates are in each definition's
comment. Where two canonical sources differ (pBR322 versus pBluescript AmpR; the S288C reference
versus the alleles the pRS vectors carry; the *Streptomyces* `pac` versus the `pac` of pLKO.1) the
consensus is degenerate at those positions and says so. Yeast promoters and terminators are the
proximal genome regions (400 bp upstream of the ATG, 250 bp downstream of the stop), so any
longer fragment a vector carries still contains them.

The records behind the researched definitions are committed under `reference/sequences/genbank/`
and a test scans each and asserts that every definition lands at the record's own coordinates.

The 19 missing names have no verifiable public source: the MultiSite Gateway donor vectors are
not deposited, and the other elements' defining vectors are undeposited or unannotated. Manage
Features ▸ Import takes your own definitions; a copy of an old installation's `SerialCloner/Features/`
folder would settle the whole list.

## Codon usage tables

Five tables are bundled, from the Kazusa Codon Usage Database (source URL and retrieval date in
each file): *Escherichia coli* K-12 (the W3110 entry, 4332 CDS — Kazusa's "K12" entry has 14),
*Saccharomyces cerevisiae*, *Homo sapiens*, *Mus musculus*, *Drosophila melanogaster*. Import
others from the Codon Usage window.

## Genetic codes

The 27 NCBI translation tables, generated from NCBI's `gc.prt`, with their alternative start
codons.

## pK tables

Bjellqvist/ExPASy (default; 6.78 for the twenty amino acids, 5.92 for serum albumin, as ExPASy
computes them), EMBOSS, Sillero, Solomon, Lehninger, Rodwell.

## The reference material

`reference/` in the repository holds what the reimplementation was measured against: the
original's sample files (the format fixtures), its manuals and full changelog, the screenshots
used on this site, and the 2012 enzyme database. See `reference/README.md`.
