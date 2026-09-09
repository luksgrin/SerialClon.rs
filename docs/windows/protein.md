# Protein

## Protein windows

File ▸ **New Protein Sequence** and **Open Protein Sequence…** open the Sequence Window in
protein mode (Type: Protein): the amino-acid alphabet in the field, size, MW and **pI** in the
Info panel, Find on the peptide, Features scanned with the peptide definitions (Tags & Cut).
A peptide translated in a DNA window's Info panel can be opened as a protein window from its
context menu; pasting a GenBank protein record creates one too.

## The Protein menu

| Item | What it does |
|---|---|
| **Reverse Translate…** | Turns the front-most protein into DNA using the most frequent codon of the selected codon-usage table; the protein's Features are inherited. |
| **Scan for Peptide Features** | The Features scan with the peptide definitions. |
| **Protein Align…** | The Align window in protein mode. |
| **BLAST Protein Sequence…** | blastp at the NCBI (see [Web Access](web-access.md)). |
| **Codon Usage: …** | Shows the current table and opens the Codon Usage window. |
| **Import Codon Usage…** | A new table from pasted text (Kazusa / GCG format) or a URL. |
| **Choose Genetic Code** | One of the 27 NCBI translation tables (standard, vertebrate mitochondrial, yeast mitochondrial, bacterial, ciliate, …), with their alternative start codons. Applies to every translation. |
| **pK Table for pI** | Bjellqvist/ExPASy (default), EMBOSS, Sillero, Solomon, Lehninger, Rodwell. |

The choices are saved in `settings.json`; in 1.0 they are also in Preferences.

## Codon Usage window

The **Tables** list (five bundled: *E. coli* K-12, *S. cerevisiae*, *H. sapiens*, *M. musculus*,
*D. melanogaster*, from the Kazusa Codon Usage Database — source URL and date in each file — and
the ones you imported), the **Codons** of the selected table (fraction, per thousand, count),
**Use for reverse translation**, **Import**. A codon-usage table can also serve as the genetic
code for translation (an original 2.6 option, exposed in Preferences).

## Bisulfite conversion

Sequence ▸ Bisulfite Conversion ▸ **Sense Strand (C>>T)** or **Antisense Strand (C>>T)** opens
the converted sequence in a new window with a provenance comment — for designing bisulfite
sequencing primers.
