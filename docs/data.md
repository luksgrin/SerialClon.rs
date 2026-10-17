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

The five Collections (Generic, Yeast, Other, Tags & Cut, Gateway; 155 definitions) are Serial
Cloner 2.6.1's own built-in Feature database, verbatim. The original keeps it inside the
application and writes it to disk on first launch; those files were recovered from the 2.6.1
Linux build and converted field for field into `resources/features/*.json`. A default scan
therefore gives exactly the original's result on any sequence, and tests assert that on six of
its sample files.

A sixth Collection, *Additions*, holds definitions the original does not have (among them the
MultiSite Gateway att sites). It is shipped unchecked for scanning; tick it in Manage Features
to include it.

The original's definitions are all simple: one consensus, which may contain IUPAC letters and
the original's `#` gap wildcard. One definition is shipped on the opposite strand from the
original's file: attB1, which the original alone defines on the reverse strand, so that all att
sites read the same way. The verbatim dump and the conversion notes are under
`reference/original/features/` and `resources/features/README.md`.

An installation seeded by an earlier version keeps its old Collections until
*Manage Features ▸ Restore built-in collections* is used.

## Codon usage tables

The nine tables Serial Cloner 2.6.1 shipped, recovered from the application, plus five from the
Kazusa Codon Usage Database (source URL and retrieval date in each file): *Escherichia coli*
K-12, *Saccharomyces cerevisiae*, *Homo sapiens*, *Mus musculus*, *Drosophila melanogaster*.
Import others from the Codon Usage window.

## Genetic codes

The 27 NCBI translation tables, generated from NCBI's `gc.prt`, with their alternative start
codons.

## pK tables

The five tables of the original with its values: EMBOSS (default), Wikipedia, DTASelect, Rodwell
and Sillero. The pI is computed the way the original computes it, so the numbers match.

## DNA ladders

The four ladders of the original's virtual gel: GeneRuler 1 kb Plus, Smart Ladder, NEB 1 kb and
Invitrogen 1 kb plus, with their bright reference bands.

## The reference material

`reference/` in the repository holds what the reimplementation was measured against: the
original's sample files (the format fixtures), the 2012 enzyme database, and the data and golden
files recovered from the 2.6.1 application. See `reference/README.md` and `reference/docs/original-behaviour.md`.
