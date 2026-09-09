# Cloning

The Generate menu (⌘1 … ⌘6), the Restriction menu and the Tools menu. Every product opens in a
new Sequence Window, marked unsaved, with a comment that records how it was made and which
Features it inherited.

## Build a Construct (⌘1)

Fragment **slots** — a vector and one or more inserts. For each slot pick an open sequence (or
**Select DNA…** to open a file), its BEGIN and END sites (unique sites in bold), the direction
(CW / sense), or *Select full insert* for a whole linear fragment; **Blunt left** and **Blunt
right** fill in or trim each end. A live preview shows the length and the ends of each fragment.
The **Graphic Map** button opens the map for the two-click selection; its context menu's *Use
Selection in Build a Construct* fills the slot.

**Ligate** checks that the ends are compatible (both blunt, or complementary overhangs of the
same kind), joins the fragments, regenerates the sites, inherits the Features of every fragment
without duplicates, and never produces degenerate DNA. The comment names the parents, the sites
and the ends.

## Create an Adaptor (⌘2)

The core double-stranded sequence plus optional 5' / 3' / blunt extensions on each side →
**Make** shows the two oligonucleotides to order; **Create Sequence Window** opens the adaptor
in memory with its cohesive ends, ready for Build a Construct.

## Run a PCR (⌘3)

The matrix (an open sequence) and two primers, which may carry un-annealed tails: the longest
unique 3' core of at least 10 nt is used to find the annealing site, on either strand, with the
Tm shown live. Overlapping, reversed or ambiguous primers are reported. Circular matrices work.
The primers' **Anchor** and **Core** are added as Features of the product; **3' A overhang**
gives a TA-cloning product. While this window is open, a selection in a Sequence Window can be
sent here from its context menu as the forward or the reverse primer.

## Make shRNA (⌘4)

A stored **layout** (cohesive ends, prefix, loop, suffix — pSUPER is shipped; edit and save your
own), the **siRNA** core, optional point mutations in the sense arm → the oligos to order and,
with **Create insert window**, the insert with its Features.

## Gateway™ cloning (⌘5)

**BP** and **LR** reactions between the chosen vector and insert, modelled at the core of the
att sites found by the Gateway Collection (the matching is tolerant: real vectors differ by a
base in the arms). Antisense partners are flipped, circular partners rotated, Features inherited.
The **homologous recombination** option does the yeast-style gap repair instead.

## Site Usage

Restriction ▸ **Site Usage (interactive)**: a live table of every enzyme with its number of cuts,
a frequency **Filter**, and **Compare** against a second open sequence — "unique here, absent
there" is the classic use. Restriction ▸ **Site Usage (table)**: the printable tables — all
sites, unique sites (bold), absent sites (italic).

## Enzyme Library

The bundled REBASE database (`striderc` format — the same file Serial Cloner used, refreshed).
**Filters** by site length and end type (blunt, 5', 3'); the **Selected enzyme** panel shows its
site with the cut, its isoschizomers and the enzymes with compatible ends. **Check all** /
**Uncheck all**: unchecked enzymes disappear from the maps, Find, Site Usage, the Virtual Cutter
and the Silent Map, and stay unchecked across runs. **Add an enzyme** takes `striderc` syntax
(`Name,site/with/cut,` or `#Name,site,a,b,` for type IIs); enzymes you added can be removed, the
REBASE ones cannot.

## Virtual Cutter (Tools)

Pick **Enzymes**; the **Fragments** table lists count and sizes, and the **Gel** shows their
migration on a simulated 1 % agarose gel next to a 1 kb ladder.

## Silent Map (Tools)

Choose the **Reading frame** of the selected ORF; the **Silent sites** table lists the
restriction sites that can be introduced by mutations that do not change the peptide, with the
minimal codon changes; **Apply** makes the mutation in the sequence.
