# Built-in Feature Collections

The five collections `generic.json`, `yeast.json`, `other.json`, `tags-and-cut.json` and
`gateway.json` are **Serial Cloner 2.6.1's own internal Feature database**, recovered from the
application in October 2026 (see `reference/original/features/` for the verbatim dump and
`reference/docs/original-behaviour.md` for how it was obtained). On first launch the original
writes that database to disk as five text files (`#V1.0#` / `#Format=…` / `#CollectionName=…`
header, one comma-separated definition per line); the Linux build was run under Xvfb in a
container and the files it wrote were converted field for field into our JSON schema. The sixth
file, `additions.json`, holds the definitions of ours that the original does not have; it is
shipped **unchecked for scanning** so that a default scan gives exactly the original's result.

Coverage is therefore **155 of 155** original definitions (79 Generic, 24 Yeast, 10 Other,
24 Tag & Cut, 18 Gateway), replacing the 1.0.0 reconstruction from public records, which had
131 of the ~150 names listed in the Features ReadMe v2.1 and, as it turned out, reproduced only
22 of the original's consensus sequences exactly.

An install seeded by 1.0.0 keeps its old files under `{app data}/features/` until
*Manage Features ▸ Restore built-in collections* is used.

## What the original's database looks like

- Every definition is *simple*: one consensus, no 3' part, no upstream/downstream context and
  no "additional" ends — the complex-definition fields exist in the file format but are empty
  in all 155 records. All have Arrow and ToScan set and OnlySenseStrand unset; the Comments
  field is empty.
- Feature types are the GenBank keys the original uses, verbatim, including its capitalised
  `Promoter` and `Misc`.
- Colours are the file's `&hRRGGBB` values, written here as `#rrggbb`.
- A consensus may contain IUPAC letters (`N`, `R`, `Y`, `M`, `S`, `W`, `K`) and the original's
  gap wildcard **`#`**, which its scanner turns into `.*` (any run of bases, greedy). `KanR`,
  `M13 origin`, `LEU2`, `HIS3`, `2 micron origin` and `ADH1 promoter` carry one; the scanner
  in `sc-features` implements it. Any other character (`2 micron origin` has a stray `>`) is
  dropped by the original's regex builder and by ours.
- Names repeat: `TetR` (two different sequences), `IRES` (two), `TEV site` (two), `mPGK Prom`
  (one of which is a copy of the EM7 promoter sequence), `Avitag` (identical twice). They are
  kept as they are.
- The peptide tags are `PEP` definitions matched against the translation; `SV40 NLS2` is a DNA
  definition in the original although it sits in the Tag & Cut collection.
- Several sequences differ from the published vectors (the original's `CMV promoter` carries an
  `N` that pEGFP-N1 does not match, its `PuroR` is not pLKO.1's); they are the original's and
  are shipped unchanged, because the aim is to annotate exactly what the original annotates.
  `reference/original/golden/scan/` holds the original's own scan results for the shipped
  sample files and `crates/sc-features/tests/fixtures.rs` checks that ours are identical.

## The one deliberate deviation

`AttB1` is defined in the original on the opposite strand to `AttB2`, `AttP1`, `AttL1` and
`AttR1` (`AGCCWRCTTTTTTGTACAAACTTGT`, the reverse complement of the attB1 everybody writes).
`gateway.json` ships it reverse-complemented (`ACAAGTTTGTACAAAAAAGYWGGCT`) so that an
attB1–insert–attB2 fragment is recognised in the sense orientation by the Gateway window; the
scan finds the same sites, on the other strand. The verbatim definition is in
`reference/original/features/gateway.json` and in the definition's own comment.

## Additions (`additions.json`, not scanned by default)

Kozak, hPGK promoter, NOS terminator, trc promoter, Ig kappa ss, Enterokinase site,
ARS consensus, GAL4 UAS, TEF promoter, TEF terminator — researched definitions from 1.0.0 whose
provenance (accession and coordinates) is in each definition's comment and whose records are
under `reference/sequences/genbank/`. The other 1.0.0 extras were other spellings of
definitions the original has and were dropped (`Lac Promoter` → `lac`, `Amp Promoter` →
`Amp prom`, `HIV-1 5LTR` → `HIV-1 5 LTR`, `HIV RRE` → `RRE`, `SV40 intron` → `SV40 int`,
`rrnBT2` → `rrnB T2`, `E- tag` → `E-tag`, `Strp tag II` → `Strep tag II`, `TEV tag [1]/[2]` →
`TEV site`, `Factor Xa site` → `FactorXa site`, `IRES [1]/[2]` → `IRES`, `Synthetic intron` →
`synth int`).

## Schema

One JSON object per collection — `name`, `scan`, `builtin`, `features[]` with `name`, `kind`
(GenBank key), `color` (`#rrggbb`), `arrow`, `sense_only`, `peptide`, `five`, `three`,
`potential_five`, `potential_three`, `upstream`, `downstream`, `comment`, `scan`. See
`crates/sc-features/src/lib.rs`. The original's fourteen fields map onto it as
FeatureName → `name`, Seq5prime → `five`, Seq3prime → `three`, SeqUpstream → `upstream`,
SeqDownstream → `downstream`, Additional5prime → `potential_five`, Additional3prime →
`potential_three`, FeatureType → `kind`, Comments → `comment`, DNA_PEP → `peptide`,
Color → `color`, Arrow → `arrow`, ToScan → `scan`, OnlySenseStrand → `sense_only`.
