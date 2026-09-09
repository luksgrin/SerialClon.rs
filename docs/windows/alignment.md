# Alignment

## Align two Sequences (⌘7)

Pick **Sequence 1** and **Sequence 2** among the open windows (the two front-most by default),
then:

- **Local alignment** — Smith-Waterman with affine gaps, in-process. DNA is aligned against both
  strands (the best one is reported) with degenerate letters allowed; proteins use BLOSUM62.
- **NCBI BLAST2Seq** — the sequences are submitted to NCBI's BLAST service and the alignment
  displayed here; needs the network and NCBI's queue (see [Web Access](web-access.md)).
- **Align short sequences** — as in the original, the smaller one becomes Sequence 1.

The **Alignment** view shows the two sequences with match marks, the **Alignment summary** gives
the identity and the % of aligned nucleotides, **3-frame translation** adds the translation of the
aligned DNA, the Features of both sequences are drawn above their rows and a click reports the
coordinates and Features under it, **Highlight differences** dims what is identical, **Extract
consensus** opens the consensus as a new sequence, and **Protein** switches the same pair to a
protein alignment (Protein ▸ Protein Align… does the same for protein windows).

## Align Multiple Sequences (⌘8)

A window the original never had, but its "copy all sequences as multi-FASTA and take them to
CLUSTAL" workflow points at. The engine is
[rust-MAFFT](https://github.com/luksgrin/rust-MAFFT), a pure-Rust port of MAFFT 7.526 that is
byte-identical to the C program — in-process, no external binary, works offline.

- **Sequences** — tick any of the open windows (all by default, at least two). DNA or protein is
  detected; mixed sets are refused.
- **Strategy** — Auto (MAFFT's `--auto` rule: L-INS-i for up to 200 sequences of up to 2 kb,
  FFT-NS-2 otherwise), FFT-NS-2, FFT-NS-i, L-INS-i, G-INS-i, E-INS-i.
- **Adjust direction** — MAFFT's `--adjustdirection`: plasmids pasted in either orientation are
  flipped as needed and the flipped ones are reported. **Keep case** re-applies your input's case.
- **Align** — runs on a worker thread; the window shows a busy state. The input is capped at
  two million residues.

The **Alignment** view has a sticky name gutter, a ruler, conservation colouring (`*` identical,
`:` similar, as Clustal marks them) or **Highlight differences**, the Features of each sequence as
bars above its row, and a click on a column shows each sequence's coordinate and Features there,
with a button to select it in its window. **Copy alignment** puts Clustal text on the clipboard;
**Extract consensus** opens the majority consensus (IUPAC on DNA ties, `X` on protein ties) as a
new sequence; **Save as FASTA** / **Save as Clustal** write files.
