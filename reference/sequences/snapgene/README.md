# SnapGene `.dna` fixtures

Real SnapGene files, used as the acceptance tests of `crates/sc-format/src/snapgene.rs`:
every file must read, and `write` must give back its exact bytes when nothing was edited
(`every_fixture_round_trips_byte_for_byte`). `scripts/check-snapgene.py` compares our reading
of each file, and the files our writer produces, with Biopython's SnapGene parser.

All files were retrieved on 2026-09-29 from public repositories whose licences allow
redistribution. None was modified. Addgene's SnapGene downloads need a login and are not
included; SnapGene's own sample plasmids (© GSL Biotech) were deliberately left out.

## Biopython — `Tests/SnapGene/` (Biopython License Agreement / BSD-3-Clause)

<https://github.com/biopython/biopython/tree/master/Tests/SnapGene>

| File | What it is |
|---|---|
| `biopython-pFA-KanMX4.dna` | pFA6a-kanMX4, 3941 bp circular, export version 13: 9 features on both strands, one wrapping the origin (SP6 promoter 3925→2), one multi-segment (AmpR), custom map label, description with HTML markup, references |
| `biopython-sample-d.dna` | 1000 bp linear, version 14, lower-case sequence, 4 features, `Comments` in the notes |
| `biopython-sample-e.dna` | 1000 bp, version 13, forward and reverse features |
| `biopython-sample-f.dna` | 1000 bp, version 15: multi-segment features with named segments and `gap` segments, one primer |
| `biopython-sample-hybridization-params.dna` | 2414 bp, version 15: primers with several binding sites and `HybridizationParams` thresholds (Biopython issue #5053) |
| `biopython-linebreak_in_qualifier_text.dna` | 246 bp: a qualifier value containing line breaks; 3 primers |
| `biopython-looped_feature.dna` | 10 bp, version 15: a feature `range="3-2"` wrapping the whole circle, LZMA history |
| `biopython-looped_feature_origin.dna` | 10 bp: a feature spanning the origin, three history nodes |

## sgffp — `tests/data/` (MIT)

<https://github.com/merv1n34k/sgffp/tree/master/tests/data> — synthetic files created by the
sgffp project (its README states that all files except the SnapGene sample, which is not
included here, are its own).

| File | What it is |
|---|---|
| `sgffp-legacy_v13.dna` | 5618 bp, export version 13 (old SnapGene): 17 features, 8 primers, three sequencing traces (type 16), history |
| `sgffp-gibson_assembly.dna` + `.fasta` | 10558 bp Gibson product, version 15: 29 features (two multi-exon CDS with gap segments), 4 primers, six history nodes; the FASTA holds the product, the linearised vector and the fragment — the product is the cross-check for the sequence |
| `sgffp-origin_spanning_features.dna` | 44 bp circle: the same origin-spanning span with directionality none / forward / reverse, 2 primers |
| `sgffp-circularize_only.dna` | 141 bp, no features |
| `sgffp-test.dna`, `sgffp-test2.dna`, `sgffp-test3.dna` | small files: no features, 3 primers with an escaped name, a trace container (type 16) with a trace alignment (type 27) |
| `sgffp-test.prot.dna` | a **protein** file (molecule type 2, sequence packet type 21) |
| `sgffp-test.rna.dna` | an **RNA** file (molecule type 7, packet type 32) |
| `sgffp-degenerate-1.dna` | 16 bp with IUPAC letters |
| `sgffp-format31-mixed.dna`, `sgffp-format31-lowercase.dna` | outputs of SnapGene's newest format examples (mixed-case, lower-case runs) |

## seqparse — `src/examples/snapgene/` (MIT)

<https://github.com/Lattice-Automation/seqparse/tree/main/src/examples/snapgene>

| File | What it is |
|---|---|
| `seqparse-RV027028.dna` | 35 040 bp, version 13, 29 features on both strands — the large one |
| `seqparse-pBbB8c-GFP.dna` | 4165 bp BglBrick vector, 7 features |

## dotDNA — `tests/` (MIT)

<https://github.com/noatgnu/dotDNA/tree/master/tests>

| File | What it is |
|---|---|
| `dotdna-test.dna` | 1680 bp GAS1 CDS with 15 primers carrying descriptions |
