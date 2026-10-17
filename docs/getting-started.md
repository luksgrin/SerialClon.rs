# Getting started

A ten-minute tour along a normal working day: open a plasmid, look at it, find things, map it,
cut a fragment out, annotate it, save it. Anything you hover shows a one-line explanation in the
**Help** window — that is the fastest way to learn any window.

## 1. Open a plasmid

File ▸ Open… (⌘O / Ctrl+O) or the Toolbar's **Open** button. `.xdna`, FASTA, GenBank / EMBL and
plain text are recognised by content, not by extension. Each sequence gets its own **Sequence
Window**; a multi-FASTA or a multi-record GenBank file opens one window per record.

The **Format** field tells you what was read (DNA Strider, Serial Cloner, FASTA, GenBank, Text);
the **Type** field is DNA, Degenerate DNA or Protein. Tick **Locked** if you want to be sure you
cannot edit by accident.

## 2. Select numerically

Drag in the sequence field as you would anywhere, or type numbers: enter `129` in **from** and
`861` in **length** and the selection follows. The arrows go to the first and last base. With
no selection, **from** shows the insertion point.

## 3. Find

**Find** (⌘F) opens the Find drawer attached to the right edge of the window; it follows the
window when you move it. Three tabs:

- **Sequence** — type a string (IUPAC letters accepted), hits on both strands appear as you type;
  click one to select it.
- **Site** — pick an enzyme (type the first letters to jump; unique sites in **bold**, absent ones
  in *italic*) and press Find Site.
- **ORFs** — every open reading frame on both strands, longest first, incomplete ones in italic.

Hits wrap around the origin of circular plasmids.

## 4. The Info panel

**Show Selection Information & Translation** (⌘I) extends the window downwards with the Tm,
A/C/G/T counts and %GC of the selection, and its translation: size, MW, pI and the peptide.
Select residues in the peptide field to select their codons back in the sequence ("back-selection").

## 5. Graphic Map

**Graphic Map** (⌘G) draws the plasmid — circular or linear — with unique sites in yellow,
double sites in orange and, if you choose them, particular sites in blue, plus the Features.

## 6. Cut a fragment out

On the map, click a site name box: it turns **green** (BEGIN). Double-click or shift-click
another: it turns **red** (END). The **spacebar** flips the selection between clockwise and
counter-clockwise (or sense / antisense on a linear map); `A` selects a whole linear insert;
Delete cancels. Then **Extract Fragment** (⌘6, the Toolbar, or the context menu): a new Sequence
Window appears with the fragment, its **cohesive ends** (see its Extremities tab) and a comment
recording exactly how it was made.

## 7. Sequence Map

**Sequence Map** (⌘M) is the text restriction map with multi-frame translation. If a selection was
made, it is translated; otherwise everything is. The four panels choose which enzymes to show
(none, all, cutting at most *n* times, a particular list), the frames, codon mode, the reverse
strand and coordinates, and the layout for printing.

## 8. Annotate

**Scan** (Features ▸ Scan Sequence, ⌥⌘S, or the Features tab) looks for the built-in Collections
— origins, resistance genes, promoters, tags, att sites… — and adds what it finds. The Features
tab lets you edit, colour, reorder and delete Features; anything you edit is protected from the
next scan. See [Features](windows/features.md).

## 9. Save

**Save** (⌘S) writes `.xdna` — the DNA Strider format extended by Serial Cloner, readable by
Strider and by every Serial Cloner. Alt-click Save (or ⇧⌘S) is Save As…. File ▸ Export writes
FASTA or GenBank with the features and their colours.

Unsaved windows are snapshotted every 30 seconds; after a crash they come back at the next start.
