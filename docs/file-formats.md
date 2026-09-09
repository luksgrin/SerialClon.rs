# File formats

Your existing files keep working — that was a requirement, not a feature. Every format below is
read by content, and the fixtures shipped inside the original are the round-trip tests.

## `.xdna` — DNA Strider, extended by Serial Cloner (native, read + write)

The byte layout, verified against every `.xdna` sample the original shipped and round-tripped
byte for byte:

```text
offset  size  meaning
0       1     version (0)
1       1     sequence type   (0 unknown, 1 DNA, 2 degenerate DNA, 3 RNA, 4 protein)
2       1     topology        (0 linear, 1 circular)
3       25    DNA Strider fields (timestamps, window geometry, selection) — zero in
              Serial Cloner files, populated in genuine Strider files; preserved verbatim
28      4     u32 big-endian sequence length
32      64    more Strider fields, preserved verbatim
96      4     u32 big-endian comment length
100     12    reserved; Serial Cloner writes 0xFF at offset 111, DNA Strider at offset 92
112     …     sequence ASCII (case preserved — lower case is data)
        …     comment ISO-8859-1 (lines separated by 0x0D)
        1     0x00 → Serial Cloner annotation block follows (absent in plain Strider files)
```

Annotation block (all strings are Pascal strings: `u8` length + bytes):

```text
pstr right overhang length (decimal text; negative = 3', positive = 5')
     overhang sequence (abs(len) bytes) if len != 0
pstr left overhang  (same)
u8   feature count  — ABSENT when there are no features
per feature:
  pstr name
  pstr description (qualifiers separated by 0x0D)
  pstr type          e.g. "rep_origin", "CDS"
  pstr start         1-based, first base in feature direction
  pstr end           last base in feature direction (start > end for reverse features and for
                     forward features that wrap the origin)
  4 bytes  flags: [strand 1=fwd/0=rev][show][protect][arrow]
  pstr rgb  e.g. "70,130,180,"
```

A plain DNA Strider file that gains no features or cohesive ends is written back as plain
Strider; everything else gets the Serial Cloner block. The overhang encoding beyond `0` (blunt)
follows the plan's description — no shipped sample has cohesive ends; a `.xdna` saved from an
extracted fragment in the original would settle it.

## GenBank / EMBL / DDBJ (read + write)

The reader takes NCBI records (`join()` locations, CR line endings, EMBL two-letter codes) and
Serial Cloner's own export. Features keep their colours, visibility, protection and arrows
through these qualifiers, which the original invented:

```text
COMMENT     Serial Cloner Genbank Format
COMMENT     SerialCloner_Type=DNA
COMMENT     SerialCloner_Comments=<comment, one continuation line per comment line>
COMMENT     SerialCloner_Ends=0,0,,0,
FEATURES             Location/Qualifiers
     primer_bind     complement(771..790)
                     /label=T3
                     /SerialCloner_Color=&h008B8B
                     /SerialCloner_Show=True
                     /SerialCloner_Protect=False
                     /SerialCloner_Arrow=True
```

Importing a record without those qualifiers colours the features by kind, and — as the original
did — copies the LOCUS and DEFINITION lines into the comment. `source` features are skipped;
`join()` features become their outer span (a `join(a..len,1..b)` is a feature crossing the
origin).

## FASTA (read + write)

Both header forms the original wrote are read; the first is written:

```text
> [pBluescript II SK(+) - 2961 bp] gi|58063|… phagemid excised from lambda ZAPII
> [pBluescript II KS(+)] gi|58061|… - 2961 bp
```

Generic `>name comment` headers work too. Sequences are written in 80-column lines. Multi-FASTA
opens one window per record; Edit ▸ Copy All Sequences To Multifasta and File ▸ Export ▸ Save
All Sequences To Multifasta write one. Degenerate letters are kept and the window becomes
degenerate DNA; a record that is mostly non-nucleotide letters opens as a protein.

## Raw text (read)

Everything that is not one of the above: the IUPAC letters are kept, whitespace and digits
dropped, and the count of excluded characters reported, as the manual describes.

## REBASE `striderc` (enzymes)

The exact format Serial Cloner's `RELibrary` used, refreshed from REBASE:

```text
AatII,gacgt/c,
AccI,gt/mkac,
#AarI,cacctgc,4,8,
#AciI,ccgc,-3,-1,
```

A plain entry marks the top-strand cut with `/`; the bottom strand is cut symmetrically. A `#`
entry gives REBASE's `(a/b)` offsets from the 3' end of the site — negative values encode
asymmetric type II sites such as AciI. The same syntax adds enzymes in the Enzyme Library.

## Feature Collections

- **SerialClon JSON** — one object per Collection: `name`, `scan`, `builtin`, `features[]` with
  `name`, `kind`, `color` (`#rrggbb`), `arrow`, `sense_only`, `peptide`, `five`, `three`,
  `potential_five`, `potential_three`, `upstream`, `downstream`, `comment`, `scan`.
- **Generic table** — tab-separated `name  sequence  type  colour  DNA|Peptide` (the last two
  optional, `#` lines are comments).
- **ApE** — ApE's `Default_Features.txt` layout (`name  sequence  type  fwd colour  rev colour …`).

The original's own Collection files are not documented anywhere public; the JSON above is
SerialClon's equivalent.

## Codon usage tables

JSON with `name`, `taxid`, `source`, `retrieved`, `cds`, `codons_total` and 64 `codons`
(`codon`, `amino_acid`, `fraction`, `per_thousand`, `count`). Import accepts the Kazusa / GCG
"codon frequency" text format, pasted or fetched from a URL.

## Not yet

ApE `.ape` sequence files (its GenBank dialect opens already), Vector NTI `.ma4`, MacVector
`.nucl`, DNAStar EditSeq `.seq`, pDRAW32 `.pdw` and SnapGene `.dna` are on the list; the
sample files for the first three are in the repository as fixtures.
