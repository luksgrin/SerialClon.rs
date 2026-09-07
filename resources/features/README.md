# Built-in Feature Collections

The original shipped ~150 Features in five Collections (Features ReadMe v2.1, "Internal database of
Features"), but only their **names** are in the reference material: the Collection files themselves
(`~/Library/Application Support/SerialCloner/Features/`) are not. The JSON files in this directory
are therefore a **reconstruction from public records**: every DNA consensus was read out of an
annotated sample file in `reference/sequences/` or a public GenBank / SGD record (its accession and
coordinates are in the definition's `comment`), and every peptide comes from a UniProt or published
sequence. Nothing was typed from memory; what could not be verified is listed as missing below.

Where two canonical sources differ (pBR322 vs pBluescript AmpR, the S288C reference vs the allele
carried by the pRS vectors, the Streptomyces pac vs the pac of pLKO.1) the consensus is degenerate
at those positions and the comment says so. Promoter and terminator fragments from the yeast genome
are the *proximal* regions (400 bp upstream of the ATG, 250 bp downstream of the stop) so that any
longer fragment a vector carries still contains them.

The records behind the researched definitions are committed under `reference/sequences/genbank/`
and `crates/sc-features/tests/researched.rs` checks that a scan finds each definition at the
record's own annotated coordinates.

**Still to research:** the names listed as missing below. A copy of the original's
`SerialCloner/Features/` folder from any old installation would settle the whole list at once.
Until then, users can import their own Collections (Manage Features ▸ Import: SerialClon JSON,
generic table, ApE).

Schema: one JSON object per Collection — `name`, `scan`, `builtin`, `features[]` with `name`,
`kind` (GenBank key), `color` (`#rrggbb`), `arrow`, `sense_only`, `peptide`, `five`, `three`,
`potential_five`, `potential_three`, `upstream`, `downstream`, `comment`, `scan`. See
`crates/sc-features/src/lib.rs`.

## Coverage: 131 of 150 original names present

### Generic

Present (59): T3, M13-fwd, M13-rev, T7, SP6, ColE1 origin, F1 ori, 2u ori, M13 origin, LacZ alpha, LacO, AmpR, LacI, Kan/neoR, KanR, CmR, TetR, ZeoR, BlastiR, PuroR, GentaR, HygroR, SV40 late polyA, CMV promoter, SV40 early promoter, FRT, LoxP, TRE, U6 promoter, bar, dhfr, aadA, pat, R6K ori, PBAD, NOS-P, TrpC, CaMV 35S, EGFP, GST, mRFP1, mCherry, hRluc1, Rluc, IRES [1], Amp Promoter, Lac Promoter, HIV-1 5LTR, HIV-1 psi, HIV RRE, Delta U3, SV40 intron, SV40 ORI, lacI, tac promoter, rrnB, rrnB T1, rrnBT2, bGH PA

Covered under another name (5): Spec (= aadA), APH(3')-IIA (= Kan/neoR), aph (= KanR), oriVR6Kgamma (= R6K ori), 35S CMV (= CaMV 35S)

**Missing (13):** TetA, hGH polyA signal, EM7 promoter, mPGK Prom, tsr, oriS, oriRg, AML-P, EYFP, ECFP, AcGFP, IRES [2], Synthetic intron

### Yeast

Present (24): KanMX, URA3, TRP1, MET15, LYS2, LEU2, HIS3, ADE2, 2 micron origin, CEN-ARS pRS, CEN-ARS YCp-lac, CEN-ARS YCp50, GAL4 terminator, CYC1 terminator, ADH1 terminator, MET15 promoter, GAL1-10 promoter, GAL1 promoter, ADH1 promoter, Gal4 activation domain, LexA DNA-binding domain, Gal4 DNA-binding domain, HIS1, PDR5

**Missing (0):** —

### Other

Present (10): IL2 ss, human IgG1 Fc, human IgG2 Fc, human IgG3 Fc, human IgG4 Fc, mouse IgG2A Fc, mouse IgG2B Fc, mouse IgG3 Fc, rabbit IgG Fc, rat IgG2B Fc

**Missing (0):** —

### Tags & Cut

Present (22): 6His, Cys, C-Myc tag, HA tag, VSV-G tag, HSV tag, Flag tag, T7 tag, GluGlu tag, E- tag, S-tag, PC tag, Strp tag II, SBP tag, Avitag, TEV tag [1], TEV tag [2], PreScission site, Factor Xa site, Thrombin site, SV40 NLS1, SV40 NLS2

**Missing (0):** —

### Gateway

Present (11): attB4, attB3, AttB2, AttB1, AttP2, AttP1, AttL2, AttL1, AttR2, AttR1, ccdB

**Missing (6):** attP4, attP3, attL4, attL3, attR4, attR3

## Added beyond the original list

- Generic: Kozak, hPGK promoter, NOS terminator, trc promoter
- Other: Ig kappa ss
- Tags & Cut: V5 tag, Enterokinase site
- Yeast: ARS consensus, GAL4 UAS, TEF promoter, TEF terminator

## Why the remaining names are missing

- **Gateway attP3/attP4/attL3/attL4/attR3/attR4** (MultiSite): the pDONR P4-P1R / P2R-P3 vectors are not in a public sequence database.
- **EM7 promoter, hGH polyA signal, mPGK Prom, Synthetic intron, IRES [2], AML-P, oriS, oriRg, tsr, TetA, M13 origin variants, EYFP / ECFP / AcGFP, SV40 ORI variants**: the vectors that define them (pcDNA3.1/Zeo, pSecTag, pMSCV, pCI, pBeloBAC11 as annotated, Clontech FP vectors) are either not deposited or deposited without feature annotation, so no verifiable span exists. hPGK promoter is shipped instead of the mouse PGK promoter.
- **Yeast promoters/terminators** are shipped as proximal genome regions (see above); the exact fragments of specific vectors (pYES2, pESC, Mumberg) are not deposited.
- **Gal4 / LexA domains** are shipped as peptides (UniProt spans), which detect any codon usage.

## Provenance of the present DNA entries

| Feature | Source |
|---|---|
| T3, M13-fwd, M13-rev, T7, ColE1 origin, F1 ori, LacZ alpha, LacO | `pBluescript-II-KS+-annotated.xdna` annotations |
| AmpR | KS(+) and pBR322 annotations merged; degenerate (R, Y) where they differ |
| TetR | `pBR322-annotated.xdna` annotation |
| Kan/neoR, EGFP, CMV promoter | `pEGFP-N1.xdna` at Clontech's coordinates (2629..3423, 679..1398, 1..589) |
| CmR, ccdB, AttP1, AttP2, AttR1, AttR2 | `pDONR201.xdna` / `pDEST14.xdna` |
| SV40 early promoter, SV40 late polyA, Lac Promoter, Kozak | motif verified in a fixture |
| SP6, bGH PA, FRT, LoxP, TRE, tac promoter, AttB1–4, AttL1, AttL2, ARS consensus, GAL4 UAS | textbook motifs, not in any fixture |
| Tags & Cut, IL2 ss, Ig kappa ss | published peptide sequences |
| Every other entry | the GenBank / RefSeq / SGD / UniProt accession and coordinates given in its `comment` |
