# Codon-usage tables

One JSON file per table, loaded by `sc_seq::codon::builtin_tables()` in the
order below and listed by the Codon Usage window. Every table has 64 entries
with the codon, its amino acid, the fraction among synonymous codons and the
frequency per thousand codons; the five Kazusa tables also carry the raw
counts.

## Serial Cloner 2.6.1's tables (first, in the original's order)

Recovered verbatim from the original's string table
(`reference/original/embedded/codon-usage.txt`, see
`reference/docs/original-behaviour.md`). The original stores them as
`codon|aa|per-thousand|fraction` with the Kazusa retrieval month and the
genetic code (`Standard` for all nine); the JSON keeps the values unchanged,
`count` is 0 and `cds`/`codons_total` are unknown. Names are the original's
abbreviated binomials, so these are the tables a Serial Cloner user will
look for; `H. sapiens` is the default, as in the original.

| File | Name | Kazusa snapshot |
|---|---|---|
| `h-sapiens.json` | H. sapiens | Feb 2011 |
| `m-musculus.json` | M. musculus | Feb 2011 |
| `r-norvegicus.json` | R. norvegicus | Feb 2011 |
| `e-coli.json` | E. coli | Feb 2011 |
| `s-cerevisiae.json` | S. cerevisiae | Jan 2012 |
| `s-pombe.json` | S. pombe | Jan 2012 |
| `a-thaliana.json` | A. thaliana | Jan 2012 |
| `p-pastoris.json` | P. pastoris | Jan 2012 |
| `d-melanogaster.json` | D. melanogaster | Jan 2012 |

## Newer Kazusa tables (after them)

Retrieved from the Codon Usage Database on 2026-09-08 with their counts, under
the species' full names, so a 2026 snapshot stays available beside the
original's 2011–2012 one for the same species.

| File | Name |
|---|---|
| `ecoli-k12.json` | Escherichia coli K-12 |
| `yeast.json` | Saccharomyces cerevisiae |
| `human.json` | Homo sapiens |
| `mouse.json` | Mus musculus |
| `drosophila.json` | Drosophila melanogaster |

Tables imported by the user (Manage Codon Usage, Import Codon Usage) are
stored in the application data directory and listed after the built-in ones.
