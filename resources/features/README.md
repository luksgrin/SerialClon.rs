# Built-in Feature Collections

The original shipped ~150 Features in five Collections (Features ReadMe v2.1, "Internal database of
Features"), but only their **names** are in the reference material: the Collection files themselves
(`~/Library/Application Support/SerialCloner/Features/`) are not. The JSON files in this directory
are therefore a **partial reconstruction**: every DNA consensus here was either read out of the
annotated sample files in `reference/sequences/` (which is exactly what the original's scan found
in them) or is a textbook motif, and every peptide tag is its published sequence. Nothing was
guessed; what could not be verified is listed as missing below.

**Follow-up step:** research the missing consensus sequences (from the plasmids the original
annotated them in, GenBank entries, or a copy of the original's Feature folder if one turns up)
and add them here. A user-provided `SerialCloner/Features/` folder would settle the whole list.
Until then, users can import their own Collections (Manage Features ▸ Import: SerialClon JSON,
generic table, ApE).

Schema: one JSON object per Collection — `name`, `scan`, `builtin`, `features[]` with `name`,
`kind` (GenBank key), `color` (`#rrggbb`), `arrow`, `sense_only`, `peptide`, `five`, `three`,
`potential_five`, `potential_three`, `upstream`, `downstream`, `comment`, `scan`. See
`crates/sc-features/src/lib.rs`.

## Coverage: 57 of 150 original names present

### Generic:

Present (23): T3, M13-fwd, M13-rev, T7, SP6, ColE1 origin, F1 ori, LacZ alpha, LacO, AmpR, Kan/neoR, CmR, TetR, SV40 late polyA, CMV promoter, SV40 early promoter, FRT, LoxP, TRE, EGFP, Lac Promoter, tac promoter, bGH PA

**Missing (54):** 2u ori, M13 origin, LacI, KanR, ZeoR, BlastiR, PuroR, TetA, GentaR, HygroR, hGH polyA signal, U6 promoter, EM7 promoter, mPGK Prom, bar, Spec, dhfr, APH(3')-IIA, aadA, tsr, pat, aph, oriVR6Kgamma, oriS, oriRg, R6K ori, 35S CMV, PBAD, AML-P, NOS-P, TrpC, CaMV 35S, GST, mRFP1, mCherry, EYFP, ECFP, hRluc1, Rluc, AcGFP, IRES [1], IRES [2], Amp Promoter, HIV-1 5LTR, HIV-1 psi, HIV RRE, Delta U3, SV40 intron, Synthetic intron, SV40 ORI, lacI, rrnB, rrnB T1, rrnBT2

### Yeast

Present (0): —

**Missing (24):** KanMX, URA3, TRP1, MET15, LYS2, LEU2, HIS3, ADE2, 2 micron origin, CEN-ARS pRS, CEN-ARS YCp-lac, CEN-ARS YCp50, GAL4 terminator, CYC1 terminator, ADH1 terminator, MET15 promoter, GAL1-10 promoter, GAL1 promoter, ADH1 promoter, Gal4 activation domain, LexA DNA-binding domain, Gal4 DNA-binding domain, HIS1, PDR5

### Other

Present (1): IL2 ss

**Missing (9):** human IgG1 Fc, human IgG2 Fc, human IgG3 Fc, human IgG4 Fc, mouse IgG2A Fc, mouse IgG2B Fc, mouse IgG3 Fc, rabbit IgG Fc, rat IgG2B Fc

### Tags & Cut

Present (22): 6His, Cys, C-Myc tag, HA tag, VSV-G tag, HSV tag, Flag tag, T7 tag, GluGlu tag, E- tag, S-tag, PC tag, Strp tag II, SBP tag, Avitag, TEV tag [1], TEV tag [2], PreScission site, Factor Xa site, Thrombin site, SV40 NLS1, SV40 NLS2

**Missing (0):** —

### Gateway

Present (11): attB4, attB3, AttB2, AttB1, AttP2, AttP1, AttL2, AttL1, AttR2, AttR1, ccdB

**Missing (6):** attP4, attP3, attL4, attL3, attR4, attR3

## Added beyond the original list

- Generic: Kozak
- Other: Ig kappa ss
- Tags & Cut: V5 tag, Enterokinase site
- Yeast: ARS consensus, GAL4 UAS

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
