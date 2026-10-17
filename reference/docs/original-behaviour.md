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
| ORF search (`FindAllORFs`, `TranslateORF`; golden `../original/golden/orf/pBR322-find-orfs.png`) | an ORF opens at every occurrence of the preference's start codon (`#StartCodon`: `ATG` default, `GTG`, `ATG|GTG` or `ATG|CTG|GTG|TTG`; the genetic code's alternative starts are never used) and runs to the first stop, so nested ORFs are all listed; both strands; Size counts the stop codon; the list is sorted by Size; Position is the lower coordinate on either strand (nested reverse ORFs share it); Frame is the start position modulo 3 on the scanned strand (`+2`, `-2`); `(*)` marks an ORF spanning the origin, italics an ORF without stop; Minimum ORF Size is in aa, empty/0 = no limit (the default) | used the NCBI table's CTG/TTG starts too, defaulted to 33 codons, showed the ATG coordinate for reverse ORFs, sorted by position | **fixed** (`sc-seq::translate::find_orfs` takes the start-codon set; `orf_start_codons` and `orf_min_codons = 0` preferences; Find drawer shows Position/Size/Strand/±Frame sorted by size). Test: the first ten rows of the original's pBR322 list (86/1191/+/+2 … 626/651/+/+2) |
| Build a Construct: extremities (`BluntFill5p/3p`, `BluntDel5p/3p`, `NonProcessed5p/3p`) | three radio buttons per extremity of each fragment: Non-processed, Blunt (Fill-In) (the overhang bases stay, label `-Blunt(filled)`), Blunt (Remove) (trimmed, `-Blunt(trimmed)`); offered for either overhang kind, never decided by the program | one "blunt" checkbox per end, polymerase-style (fill a 5', chew a 3') | **fixed** (`sc-clone::ligate::blunt_with(BluntMode)`, `FragmentSpec.blunt_left/right = none/fill/remove`, radios in the window) |
| Build a Construct: Ligate (`GoGenerateFragment`, `LigUpdateAnnot`) | "No ligation can proceed without a Fragment #1."; if only fragment 1 is set: "Only one fragment/vector was selected and no insert. Do you want to proceed anyway ?"; the product is circular when the 5' end of fragment 1 is compatible with the 3' end of the last fragment, otherwise "Ligation will produce a linear fragment and not a circular plasmid. Do you want to proceed anyway ?"; an incompatible junction gives "Serial Cloner cannot ligate fragments in the indicated order. Please check order, orientation and end compatibility"; every insert gets an `insert from <name>` annotation whose note is the fragment's "Fragment extracted from …" comment, then the junction region is rescanned (`SetAnnotations(seq, from, to, false)`); window `Unsaved Ligation #n` | a "circular construct" checkbox and an error when the closing ends did not match; no insert annotation, no rescan | **fixed** (`ligation_plan` command decides the topology and the two confirmations use the original's wording; `ligate` adds the `insert from` feature). Still differs: no rescan of the junction, comment wording (see above) |
| Gateway (`CheckReaction`, `GateWayAnnotate`, `FindHomolFragment`) | BP checks [attB1, attB2] on the insert and [attP1, attP2] on the donor, LR [attL1, attL2] / [attR1, attR2] ("Checking [attB1, attB2] : OK/Error"); "Use minimal att consensus" option; the product gets an `Insert from <insert>` annotation with note `Fragment inserted from <insert> into <vector> [BP|LR] reaction`, then a feature scan; the Other tab does homologous recombination ("Homologous Recombination possible (deletes N nt)", "sense/antisense insert of N nt", "Homologous Recombination sites not found"); only the EXPR/ENTRY product is made (no by-product window) | same site pairs and reactions, homologous recombination with a minimum-homology field; no insert annotation | **fixed** (the `Insert from` feature). Differs: no "minimal att consensus" option, no automatic rescan, our own check/HR messages |
| Feature inheritance on extraction (`Extract_subChain`, `LigUpdateAnnot`) | features of the source are re-located in the fragment by searching their sequence (`FindString`); only features found whole are kept | features lying entirely inside the extracted region are inherited with shifted coordinates; partial ones are dropped | same outcome for features inside the fragment (not verified dynamically: the Construct window could not be driven under Xvfb) |
| Scan merge / protect (`SetAnnotations`, `IsANewAnnotations`, `CleanUpAnnotations`) | a hit identical to an existing annotation (`Annot_to_Chain`: name, position, strand, type, colour) replaces it; an annotation the user edited "(like its name, color, etc.)" is kept and the hit is added beside it (Features ReadMe §VIII.D); `CleanUpAnnotations` removes exact duplicates | identity was name + position + strand only, so a recoloured feature was replaced | **fixed** (`sc-features::scan::same` compares type and colour too) |
| Find: Nucleotide / Site (`FindSeq`, `FindSite`) | the query goes through `ToRegEx` (IUPAC), is searched on the sense strand and then on the antiparallel strand ("On both strands" checkbox, default on); Position/Size/Strand rows, `(*)` for a hit spanning the origin; sites use the enzyme's regex, `#` marks type IIs; "Not found" | same (IUPAC, both strands, wrap) | same |
| Find: Peptide (`FindAA`) | the amino-acid string is searched in `TranslateAll` of the three frames of each strand; Position/Size/Strand/Frame rows | no Peptide tab | **fixed** (`find_peptide` command and tab; `X` matches any residue) |
| Default preferences (Preferences window, `InitGraphMapColors`; goldens `../original/golden/prefs/`) | General: lock on open ✓, limit window width ✓, sequence field 12 pt, reading speed 2, toolbar shift 0, auto-check ✓. Folder and Files: last folder ✓, 10 recent files, import alert ✓, `RELibrary`, pref file visible. Graphic Map: No Box ✗, Box around names ✓, Show Color Legend ✗, connectors ✓, Unique RGB(255,255,175), Double RGB(255,156,7), Particular RGB(127,191,255), Start RGB(84,186,127), Stop RGB(255,63,0), Extremities/Connectors grey, RE sites 9 pt, Features 9 pt. Restriction Map: frames 1-3 ✓, Separate codons ✓, unique Bold ✓ Underlined ✗, reverse strand ✓, screen 11 pt, print 8 pt, 100 nt/line, show enzymes cutting at most 1 time. Silent Map: warn above 1000 nt, sites ≥ 6 nt. PCR: TA cloning ✓, 50 mM NaCl. ORF/Protein: ATG, no size limit, EMBOSS, H. sapiens, add STOP ✓, codon table as code ✗. Network: system settings, Blast2Seq URL | different colours (#f2e14c …), 13 pt field, 11/10 pt map labels, 12 pt Sequence Map, lock off, width unlimited, TA off, 33-codon ORF minimum | **fixed** (shell and frontend defaults, CSS fallbacks). Not represented: Show Color Legend, connectors, print size per map, silent-map thresholds as preferences |
| shRNA layouts (`PopulateShMenu`, `ParseLayoutComponents`) | one built-in layout, `pSUPER,5GATC,CCC,TTCAAGAGA,TTTTTG,5AGCT` (name, left 5' end, prefix, loop, suffix, right 5' end); user layouts stored as `#shLayout` preferences; the window shows the domains separated by spaces with the siRNA in red, offers "Add Mismatches" | pSUPER with suffix `TTTTTGGAAA` | **fixed** (suffix `TTTTTG`, name `pSUPER`) |
| Help texts | 113 titled help texts (`../original/embedded/help-texts.txt`) | 181 `data-help` texts written from the manual | spot-checked (Scan, Lock, Find, ORF, Ligate): no contradiction; the ORF, Ligate and Find texts were rewritten with this pass; a line-by-line comparison remains to do |

### Not yet compared

The Sequence Map header, the junction rescan after a ligation and after a Gateway reaction
(`SetAnnotations(seq, from, to, false)`), the "Use minimal att consensus" option, the
Gateway/homologous-recombination messages, the original's `Extract_subChain` on a feature that
straddles a cut, and a line-by-line reading of the 113 help texts. The Graphic Map is
deliberately not compared. Dynamic goldens exist for the ORF list and the Preferences panels;
the Construct, Gateway and shRNA windows opened under Xvfb (screenshots in the scratchpad) but
could not be driven to a product in the time given.
