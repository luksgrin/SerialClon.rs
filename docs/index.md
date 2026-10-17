# SerialClon.rs

An open-source, maintained reimplementation of **Serial Cloner** — the molecular biology / DNA
cloning workbench by F. Perez (SerialBasics), whose last release was **2.6.1 in March 2013** —
built in **Rust + Tauri 2** and shipping installers for **Windows, macOS and Ubuntu**.

!!! note "Not affiliated"
    SerialClon.rs is an **independent reimplementation**. It is not affiliated with, endorsed by, or
    derived from the source code of Serial Cloner or SerialBasics. Serial Cloner is © F. Perez and
    was distributed as freeware; its manuals and sample files are used here only as compatibility
    and interface documentation.

## Why

Serial Cloner is abandonware. It was written in REALbasic/Xojo, the macOS build is a 32-bit-era
signed app, the Linux build was shipped "as is" with known interface problems, and there has been
no release in over a decade. There are plenty of alternatives — SnapGene, Benchling, ApE, Geneious —
and none of them work the way Serial Cloner works.

This project is not trying to build a better sequence editor. It is trying to rebuild **the same
one**: the same windows, the same layout, the same keystrokes, the same file formats, on a stack
that still builds today.

## What "the same" means

The feature list is the easy part. These interaction details are the actual requirements:

- The **floating-window desktop** — a thin toolbar plus independent windows you tile yourself, one
  Sequence Window per sequence, each with its own maps.
- The permanent **Help window** that narrates whatever your mouse is hovering over.
- **Drawers** (`Find`) that attach to a Sequence Window and follow it around.
- `from` / `to` / `length` as **editable spinners** — you select numerically, not just by dragging.
- Enzyme lists you **scrub by typing**, with unique sites in **bold** and absent sites in *italic*.
- Two-click fragment selection on the Graphic Map: click a site (green = begin), double- or
  shift-click another (red = end), **spacebar** toggles CW/CCW, then Extract Fragment — cohesive
  ends and all — into a new window whose comments record how it was made.
- **Draggable** site and feature labels, so a map is figure-ready before you print it.
- `Alt` as the universal "advanced" modifier (Alt-click Save = Save As, Alt + `−` = delete without
  confirmation, Alt-click a checkbox = apply to all, Alt-click Import = Export).
- Everything is one context menu away, and every generated sequence carries a provenance note in
  its comments.

## What is in 1.0

| Area | What you get |
|---|---|
| Sequences | Sequence Window with tabs, spinners, lock, Find drawer, Selection Information & Translation panel, undo, autosave and crash recovery |
| Maps | Graphic Map (circular / linear, draggable labels, two-click selection) and the text Sequence Map with multi-frame translation |
| Features | Scan against five built-in Collections, the Features tab, Manage Features, import / export |
| Cloning | Build a Construct, Create an Adaptor, Run a PCR, Make shRNA, Gateway™ BP/LR, Site Usage, Enzyme Library, Virtual Cutter, Silent Map |
| Alignment | Local Smith-Waterman pairwise alignment, NCBI BLAST2Seq, and **Align Multiple Sequences** on a pure-Rust MAFFT |
| Protein | Protein windows, reverse translation with codon usage, pI, alternative genetic codes, bisulfite conversion |
| Network | NCBI / ENA import and remote BLAST — off unless you ask |
| Files | `.xdna` read and write byte for byte, FASTA, GenBank with features and colours, and more — see [File formats](file-formats.md) |

Start with [Installation](installation.md), then [Getting started](getting-started.md). Coming
from the original? Read the [migration guide](migration.md).
