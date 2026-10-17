# What the original binaries told us

Serial Cloner 2.6.1's distributed binaries were examined in October 2026 for data recovery and
behaviour comparison. No source code exists or was consulted; no binary is committed. The
archives, their SHA-256 digests and their contents are listed in
[`../original/ARCHIVES.md`](../original/ARCHIVES.md). Everything recovered sits under
[`../original/`](../original/).

## Tools and method

- **Archives:** the five downloads from `serialbasics.free.fr` (Windows installer and zip, Intel
  and Universal Mac disk images, Linux tarball). Expanded with `innoextract`, `unzip`,
  `hdiutil`, `tar`. The Windows installer carries byte for byte the same `.exe` as the zip; the
  `RELibrary` file is identical in all archives and to `reference/RELibrary-striderc.211`. No
  archive ships a Features folder: the database lives inside the executable.
- **Strings:** Serial Cloner is a REALbasic 2012 application. Its string constants are stored as
  little objects (`u32 len+5`, `u32 len`, `00 01 00 08`, Pascal string) in the Linux and Windows
  executables and as `refcount, pointer, len+5, len` records in the Mac one. A small extractor
  listed 11 232 constants from the Linux binary, among them the whole feature database (name,
  sequence, colour and description constants in order), the fallback enzyme library, the DNA
  ladders, codon-usage tables, genetic-code names, pK-table names and the Features-file format
  header.
- **Decompilation:** [rea](https://github.com/morluto/rea) 4.0.1 over Ghidra 12.1.4 (natives built
  for macOS arm64) on the Intel Mac executable, whose symbol table keeps every REALbasic method
  name and signature (`_GeneralMethods.CalculateTm%f4%si4i4i4i4`, …). `rea analyze`,
  `rea trace`, `rea decompile` and `rea function` were used for the overview, string tracing
  and the first functions; a Ghidra headless script then decompiled a batch of ~40 named
  functions in one pass. REALbasic's calling convention hides most string arguments from the
  decompiler, so constants were read from the instruction listings and from the string objects
  they reference.
- **Running the original:** the Linux i386 build runs under Xvfb inside a `i386/debian:bookworm`
  Docker container (needs `libgtk2.0-0`); it was driven with `xdotool` through its menus and
  dialogs. On launch it writes its Feature database to `SerialCloner/Features_Temp/` next to the
  executable; the sample files were opened, scanned and saved; linear fragments with every
  single-end combination were made through *Sequence Window ▸ Extremities* and saved as `.xdna`
  and exported as GenBank. Those files are the golden files under `../original/golden/`.

## The Feature database

`../original/features/*.json` is the verbatim content of the five files the original wrote
(`raw/*.txt` are the files themselves): 79 Generic, 24 Yeast, 10 Other, 24 Tag & Cut and
18 Gateway definitions, 155 in all. Fourteen fields per definition
(`FeatureName,Seq5prime,Seq3prime,SeqUpstream,SeqDownstream,Additional5prime,Additional3prime,FeatureType,Comments,DNA_PEP,Color,Arrow,ToScan,OnlySenseStrand`);
only the name, 5' sequence, type, DNA/PEP and colour are ever non-default. The shipped
`resources/features/*.json` now carry exactly these definitions; `resources/features/README.md`
describes the conversion, the quirks (`#` gap wildcard, duplicate names, the reverse-strand
attB1) and the before/after coverage.

How a definition becomes a search (`GeneralMethods.ToRegEx`): the consensus is upper-cased and
each character mapped through a fixed table — `A C G T` → `a c g t`, `R`→`[ag]`, `Y`→`[ct]`,
`S`→`[gc]`, `W`→`[at]`, `K`→`[gt]`, `M`→`[ac]`, `B`→`[cgt]`, `D`→`[agt]`, `H`→`[act]`,
`V`→`[acg]`, `N`→`[acgt]`, `#`→`.*`; anything else is dropped. Peptide definitions go through
`AA_to_DNA_RegEx` / `Prot_To_DNA_RegEx`, which expand each amino acid to its codons
(`GC[TCAG]`, `(TT[AG]|CT[TACG])`, …). Both strands are searched (`FindAnnot` searches the
antiparallel sequence when nothing is found on the sense strand, unless OnlySenseStrand).

`../original/golden/scan/` holds the original's scan results for the shipped samples;
`crates/sc-features/tests/fixtures.rs` asserts that ours are identical, name by name and
coordinate by coordinate (pBluescript II KS(+) plain and annotated, pBR322-annotated,
pEGFP-N1, pDONR201, pDEST14).

## Other embedded data (`../original/embedded/`)

| File | Content |
|---|---|
| `fallback-relibrary.txt` | the 316-enzyme library compiled into the application, used when the `RELibrary` file is missing or malformed ("Enzyme names should be < 9 characters"); striderc syntax, a `#` prefix on 107 entries (the shipped 730-enzyme `RELibrary` uses the same marker on 187) |
| `ladders.txt` | the four DNA ladders (GeneRuler 1 kb Plus, Smart Ladder, NEB 1 kb, Invitrogen 1 kb plus); a leading `-` marks a bright reference band |
| `codon-usage.txt` | nine codon-usage tables (H. sapiens, M. musculus, R. norvegicus, E. coli, S. cerevisiae, S. pombe, A. thaliana, P. pastoris, D. melanogaster) as `codon|aa|per-thousand|fraction` |
| `genetic-code.txt` | the standard code as the original stores it (64 codons, 64 letters) |
| `feature-type-groups.txt` | the GenBank-key groups of the Features window (Genes, Signals, Binding, Variation, Repeats, RNA, Ig, Misc) |
| `misc.txt` | collection order, Open-dialog file types, ORF start-codon options (`ATG`, `GTG`, `ATG|GTG`, `ATG|CTG|GTG|TTG`), monospace font candidates, pK-table names, the 13 genetic-code names, version-check URLs, the Features-file header |

## Behaviour: same or differs

| Topic | Original (from the decompiled code or the golden files) | SerialClon.rs | Verdict |
|---|---|---|---|
| Tm (`CalculateTm`) | `len < 14`: (A+T)·2 + (G+C)·4 − 16.6·log10(0.05) + 16.6·log10(Na); `14 ≤ len ≤ 50`: 100.5 + 41·GC/len − 820/len + 16.6·log10(Na); `len > 50`: 81.5 + 41·GC/len − 500/len + 16.6·log10(Na); Na given in mM and divided by 1000; result is a single-precision float | same formula and boundaries, Na 50 mM default | same |
| Protein MW (`MWProtAveraged`) | sum of integer residue masses (A 71 … W 186), no water, single precision | same table, no water | same |
| pI (`FindIsoelectricPoint`) | Henderson–Hasselbalch charge from precomputed tables at pH 0.00–14.00 in 0.01 steps; starts at pH 7.00 and walks down (or up) to the first step where the charge changes sign; five pK tables: EMBOSS (default) 3.6/3.9/4.1/8.5/10.1/6.5/8.6/10.8/12.5, Wikipedia 3.65/3.9/4.07/8.18/10.46/6.04/8.2/10.54/12.48, DTASelect 3.1/4.4/4.4/8.5/10.0/6.5/8.0/10.0/12.0, Rodwell 3.1/3.68/4.25/8.33/10.07/6.0/8.0/11.5/11.5, Sillero 3.2/4.0/4.5/9.0/10.0/6.4/8.2/10.4/12.0 (order: C-term, D, E, C, Y, H, N-term, K, R) | had the same charge formula but bisection and six other tables (Bjellqvist default, EMBOSS, Sillero, Solomon, Lehninger, Rodwell) | **fixed** (`sc-seq::pi`): the original's five tables, values, order, EMBOSS default and 0.01-step walk from pH 7; a 1.0.0 settings file naming a dropped table falls back to EMBOSS |
| PCR (`PCRWindow.DoPCR`, `FindPrimer`) | each primer has a 5' tail field and an annealing field; the annealing field is turned into a regex (`ToRegEx`, IUPAC-aware) and must match the template *in full*, first on the sense strand, then on the antiparallel one; errors: primer 1/2 not found, both primers on the same strand, primers overlapping (distance shorter than a primer), reversed order on a linear template; product = tail 1 + template between the primers + antiparallel(tail 2) | the whole primer is typed in one field and the longest 3' core ≥ 10 nt that matches decides the annealing site (chosen after the 2026-09-29 field test, where users paste full primers with tails) | differs by design; recommendation: keep ours, optionally add the original's separate "5' tail" field as an alternative entry mode |
| Extract comment (`CircGraphMap.GoGenerateFragment`) | `Fragment extracted from <name> [<size label>] : (<enzyme 5'>[<pos>] / <enzyme 3'>[<pos>])`, the enzyme labels suffixed `-Blunt(filled)` / `-Blunt(trimmed)` when an end was blunted; a selection extracted from the Sequence Window gets `Fragment extracted from <name> [between <from> and <to>]`; new windows are named `Unsaved DNA Fragment #n` / `DNA Fragment #n` / `Unsaved Fragment #n` | `Fragment extracted from {name} [{n} bp] between {enz} ({pos}) and {enz} ({pos}), {CW/CCW}` plus a line describing each end | differs cosmetically; recommendation: adopt the original's wording once the label contents are confirmed on a saved fragment |
| Ligation comment | `Ligation of : <frag 1> [<size>] : (<enz>[<pos>] / <enz>[<pos>]) to <frag 2> …` (` and to <frag 3> …` for a third), window `Unsaved Ligation #n`; refusals: "No ligation can proceed without a Fragment #1", "Serial Cloner cannot ligate fragments in the indicated order. Please check order, orientation and end compatibility", warnings "Only one fragment/vector was selected and no insert" and "Ligation will produce a linear fragment and not a circular plasmid" (both "Do you want to proceed anyway ?") | own wording | differs cosmetically; recommendation as above |
| `.xdna` cohesive ends | annotation block starts with an *ends flag* byte: 0 blunt, 5 left end cohesive, 3 right end cohesive, 35 both; then right overhang (Pascal string of the signed length, + = 5', − = 3', then the bases 5'→3' on the strand that carries them), then left overhang, then the feature count (always written by 2.6.1, absent in older files without features) | assumed a 0x00 marker and omitted the count when there were no features | **fixed** (`sc-format::xdna`), goldens in `../original/golden/ends/` |
| GenBank `SerialCloner_Ends=` | `flag,left-length,left-bases,right-length,right-bases` (e.g. `35,4,GATC,4,TCGA`, `5,-4,CTAG,0,`, `3,0,,4,TCGA`) | wrote `left,right,left-bases,0,right-bases` | **fixed** (`sc-format::genbank`) |
| GenBank export header | `LOCUS       <file name with extension>    <n> bp      DNA     circular   <short date>`; blank DEFINITION/ACCESSION/VERSION/SOURCE/ORGANISM lines, no KEYWORDS; sequence lines end with a trailing space; LF line endings; the comment keeps its embedded CRs | standard-width LOCUS with `SYN` and a `DD-MON-YYYY` date, `.` placeholders, no trailing space | differs cosmetically; left as is (both read each other) |
| GenBank feature block | `/label=`, `/SerialCloner_Color=&hRRGGBB`, `/SerialCloner_Show`, `/SerialCloner_Protect`, `/SerialCloner_Arrow`, `complement(a..b)` | same | same |
| Feature scan on the shipped samples | see `../original/golden/scan/` | identical sets | same (after adding the `#` wildcard) |
| Gateway att definitions | attP1/attP2 (232 bp) and attL/attR definitions differ by one base around the core | detection tolerated mismatches and called a runner-up within one mismatch "ambiguous" | **fixed**: only a tie is ambiguous |
| Genetic codes | 13 named codes (Standard … Blepharisma Nuclear) | 27 NCBI codes | superset; same translation for the shared ones (not individually verified) |
| Codon usage | nine built-in tables | not shipped | missing — tables now in `../original/embedded/codon-usage.txt` for a later milestone |
| DNA ladders | four tables | virtual gel uses its own | to compare |

### Not yet compared

ORF search (`TranslateORF`, `FindAllORFs`), the ligation and blunting mechanics themselves
(`LigUpdateAnnot`, `BluntFill*`, `BluntDel*`), Gateway reaction mechanics (`GateWayAnnotate`,
`CheckReaction`, `FindHomolFragment`), feature inheritance on extraction, scan merge/protect
(`SetAnnotations`, `ExtendFoundAnnot`), Find semantics (`FindSeq`, `FindIt`, `FindSite`),
Graphic Map tick/position convention (`LinTickPos`, `CircTickPos`), default preferences
(`InitVariables`), shRNA layouts, the Sequence Map header, Help texts (the long UI strings are
in the string table and can be dumped with the same extractor). Their decompilations were
produced and are available on request; comparing them is the next step. The dynamic golden
files for PCR/ligation products and a Graphic Map screenshot were not produced: the Graphic Map
window did not open under Xvfb from either the menu or its button in the time given, and typing
into the Sequence field under xdotool only delivers the first character (the Extremities and
dialog fields accept typed text, which is how the cohesive-end goldens were made).
