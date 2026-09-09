# Sequence Window

One window per sequence. Menu and Toolbar commands act on the front-most Sequence Window (or the
last one you used, when the Toolbar or a map has focus).

## Header

| Field | Meaning |
|---|---|
| **File Name** | The sequence name. Right-click to **Rename** or to **Show File in Finder / Explorer**; double-click to rename inline. A `•` marks unsaved changes. |
| **Length** | Bases (or residues for a protein). |
| **Type** | DNA, Degenerate DNA, RNA or Protein — the DNA Strider type codes. Sequence ▸ *Transform to / from Degenerate DNA* switches; leaving the degenerate alphabet filters non-ACGT letters and reports how many. |
| **Format** | What the file was: DNA Strider, Serial Cloner, FASTA, GenBank, Text, or New. |
| **Topology** | Linear or Circular. Right-click (or click) for **Linearize / Circularize**. |
| **Locked** | No edit can touch the sequence while ticked. Comments, name and features stay editable. |

## Selection

`from`, `to` and `length` are editable: type a value or use the arrows and the selection follows.
The two arrow buttons jump to the beginning and the end. When the insertion point lies inside a
Feature its name is shown above the field.

The sequence field itself is a virtualised canvas — a 200 kb plasmid scrolls like an oligo.
Numbered rows, click and drag or shift-click to select, arrows / Home / End / Page keys to move,
type letters to insert (only the window's alphabet is accepted), Backspace and Delete to remove.
Right-click for the context menu: Copy, Copy as Fasta, Paste, UPPERCASE, Lowercase,
Antiparallel, Extract Fragment, Create New Feature From Selection, Select All, and — while a Run
a PCR window is open — *use selection as forward / reverse primer*.

## Tabs

- **Sequence** — the field described above.
- **Comments** — free text. DNA Strider only reads the first 255 characters; Serial Cloner files
  keep the whole text, and so do we. Every generated sequence (PCR product, ligation, extracted
  fragment, Gateway product, bisulfite conversion, consensus) arrives with a provenance note here.
- **Features** — see [Features](features.md).
- **Extremities** — the cohesive ends of a linear fragment: blunt, 5' or 3' overhang, and the
  protruding bases. Editable, so you can add a protruding A to a PCR product or a T to a TA vector.

## Buttons

- **Save** — Save, or Save As… when the sequence was never saved. Alt-click is always Save As….
- **Sequence Map** and **Graphic Map** — open the maps for this window.
- **Find** — opens (and, pressed again, closes) the Find drawer.
- **Show / Hide Selection Information & Translation** — the Info panel.

## Find drawer

Attached to the right edge and following the window. **Sequence** searches as you type on both
strands with IUPAC letters; **Site** lists the enzymes (type to jump, unique in bold, absent in
italic) and finds every occurrence with its orientation; **ORFs** lists every open reading frame of
at least 33 codons, longest first, incomplete ones in italic. Columns sort on click; a click on a
hit selects it in the window. Everything wraps around the origin of circular sequences.

## Selection Information & Translation

| Item | Detail |
|---|---|
| Tm | The three formulas of the original manual: Wallace rule below 14 nt, `100.5 + 41·GC/n − 820/n + 16.6·log[Na+]` from 14 to 50 nt, `81.5 + 41·GC/n − 500/n + 16.6·log[Na+]` above, with [Na+] = 50 mM. |
| A, C, G, T, %GC | Counts of the selection. |
| Size (aa), MW | The translation from the first selected base. MW is the sum of integer residue masses, as the original computed it. |
| pI | Isoelectric point with the pK table chosen in Protein ▸ *pK Table for pI* (Bjellqvist/ExPASy by default). |
| Show only 1st ORF | Stop at the first stop codon, or read through (`*`). |
| Peptide field | Select residues to select their codons in the sequence. Right-click to save the peptide as FASTA or open it as a protein window. |

## Edit and Sequence menus

Undo and Redo keep five levels, like the original. Copy formats the selection in 80-column lines;
**Copy as Fasta** (⌘K) adds a header; **Copy Non Formatted** (⇧⌘K) copies the bare letters;
**Copy All Sequences To Multifasta** (⇧⌘J) gathers every open window for CLUSTAL and friends.
Paste recognises FASTA and GenBank text — into an empty window the name, comment and features
come along.

Sequence ▸ **UPPERCASE**, **Lowercase**, **Antiparallel** (⌘/), **Reverse**, **Complement** act on
the selection or the whole sequence (with the original's warning when only a portion is
selected); **Set Origin** rotates a circular sequence so the insertion point becomes base 1;
**Bisulfite Conversion** (sense or antisense strand, C→T) opens the converted sequence in a new
window; **BLAST at the NCBI…** sends the sequence to BLAST (see [Web Access](web-access.md)).

## Protein windows

File ▸ New Protein Sequence / Open Protein Sequence…, or a peptide from the Info panel, open the
Sequence Window in protein mode: amino-acid alphabet, MW and pI in the Info panel, and the
Protein menu — see [Protein](protein.md).
