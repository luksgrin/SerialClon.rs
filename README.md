# SerialClon.rs

An open-source, maintained reimplementation of **Serial Cloner** — the molecular biology / DNA
cloning workbench by F. Perez (SerialBasics), whose last release was **2.6.1 in March 2013** — built
in **Rust + Tauri 2** and shipping installers for **Windows, macOS and Ubuntu**.

> **Status: M2 (Sequence Window) done.** The research and the implementation plan live in
> [`PLAN.md`](PLAN.md). The app now has the menu bar with the original's shortcuts, the Sequence
> Window (tabs, spinners, lock, topology and name context menus, virtualised sequence canvas,
> Selection Information & Translation panel), the Find drawer (sequence / site / ORFs on both
> strands, wrapping the origin), Open / Save / Save As / Export / Revert, recent files, five-level
> undo, autosave with crash recovery, and the Standard Genetic Code and DNA Alphabet cards.
> **M3** adds the Graphic Map and Sequence Map.

---

## Why

Serial Cloner is abandonware. It was written in REALbasic/Xojo, the macOS build is a 32-bit-era
signed app, the Linux build was shipped "as is" with known interface problems, and there has been
no release in over a decade. There are plenty of alternatives — SnapGene, Benchling, ApE, Geneious —
and none of them work the way Serial Cloner works.

This project is not trying to build a better sequence editor. It is trying to rebuild **the same
one**: the same windows, the same layout, the same keystrokes, the same file formats, on a stack
that still builds today.

## What "the same" means

The feature list is the easy part. The plan treats these interaction details as the actual
requirements ([`PLAN.md` §1.1](PLAN.md)):

- The **floating-window desktop** — a thin toolbar plus independent windows you tile yourself, one
  Sequence Window per sequence, each with its own maps.
- The permanent **Help window** that narrates whatever your mouse is hovering over.
- **Drawers** (`Find`, `Show Info`) that attach to a Sequence Window and follow it around.
- `from` / `to` / `length` as **editable spinners** — you select numerically, not just by dragging.
- Enzyme lists you **scrub by typing**, with unique sites in **bold** and absent sites in *italic*.
- Two-click fragment selection on the Graphic Map: click a site (green = begin), double- or
  shift-click another (red = end), **spacebar** toggles CW/CCW, then Extract Fragment — cohesive
  ends and all — into a new window whose comments record how it was made.
- **Draggable** site and feature labels, so a map is figure-ready before you print it.
- `Alt` as the universal "advanced" modifier (Alt-click Save = Save As, Alt + `−` = delete without
  confirmation, Alt-click a checkbox = apply to all).

## Compatibility

Your existing files keep working. This is a hard requirement, not a nice-to-have.

| | |
|---|---|
| **Native, read + write** | `.xdna` (extended DNA Strider) — byte layout reverse-engineered and documented in [`PLAN.md` §1.4](PLAN.md), including the Serial Cloner annotation block (features, colours, cohesive overhangs) |
| **Import** | FASTA (incl. degenerate and multi-FASTA), GenBank / EMBL with features, ApE, Vector NTI `.ma4`, MacVector `.nucl`, DNAStar EditSeq `.seq`, pDRAW32 `.pdw`, raw text |
| **Export** | `.xdna`, FASTA, GenBank with features and colours, PNG maps, printable text maps |
| **Enzymes** | REBASE `striderc` — the exact format Serial Cloner's `RELibrary` used, refreshable with `scripts/update-rebase.sh` |

## Stack

- **Rust core** — seven UI-free crates (`sc-seq`, `sc-format`, `sc-enzyme`, `sc-clone`, `sc-align`,
  `sc-features`, `sc-net`) holding all the science, testable without a GUI.
- **Tauri 2** — a thin shell over real OS windows (`WebviewWindow`), one per Serial Cloner window.
  An in-app MDI would read as a web page; real windows are the point.
- **TypeScript + Svelte 5 + Vite** — Graphic Map in SVG (draggable labels, clean print/export),
  sequence views on virtualised canvas so 200 kb plasmids stay smooth.
- **GitHub Actions** — `.dmg`/`.app` (arm64 + x86_64), `.msi` + NSIS, `.deb` + `.AppImage`.

## Repository layout

```
PLAN.md                  research findings + the full implementation plan
Cargo.toml               workspace: crates/* + src-tauri
crates/
  sc-seq/                sequence model, topology, alphabets, translation, Tm/MW/pI   (M1)
  sc-format/             .xdna, FASTA, GenBank/EMBL, ApE, Vector NTI, MacVector, …    (M1)
  sc-enzyme/             REBASE striderc parser, digestion, site usage                (M1)
  sc-features/           feature collections + scan engine                           (M4)
  sc-clone/              extract, ligate, PCR, adaptor, shRNA, Gateway, virtual cut  (M5)
  sc-align/              Smith-Waterman, consensus, diff highlight                   (M6)
  sc-net/                NCBI E-utilities, BLAST URL API, EMBL, codon usage          (M6)
src-tauri/               the Tauri shell: commands, window manager, bundling config
src-tauri/src/
  documents.rs           the DocumentStore: sequences, undo/redo, edits
  menu.rs                the menu bar with the original's shortcuts
  windows.rs             window manager: Toolbar, Help, seq-{id}, find-{id} drawers
  io.rs / recent.rs / autosave.rs
src/
  windows/<name>/        one Vite entry + Svelte component per Serial Cloner window
  lib/components/SequenceCanvas.svelte  the virtualised sequence field
  lib/help-bus.ts        the contextual Help window bus (`data-help` → Help window)
  lib/shortcuts.ts       the full ⌘/Ctrl shortcut map from the manual
  app.css                design tokens (the Aqua Classic theme will override these)
assets/app-icon.svg      icon source; `cargo tauri icon assets/app-icon.svg -o src-tauri/icons`
resources/RELibrary      current REBASE striderc enzyme database, bundled with the app
scripts/update-rebase.sh refresh it from rebase.neb.com
reference/               see reference/README.md
  sequences/             original .xdna/.gb/.fasta/.nucl/.ma4 files — format fixtures
  docs/                  Serial Cloner manuals and full changelog (PDF + extracted text)
  ui/                    screenshots of every major window — the UI acceptance target
  RELibrary-striderc.211 the enzyme database Serial Cloner 2.6.1 actually shipped
.github/workflows/
  ci.yml                 fmt, clippy (-D warnings), tests, svelte-check, Vite build × 3 OSes
  release.yml            installers via tauri-action; draft GitHub Release on a `v*` tag
```

## Roadmap

Seven milestones, each ending in installers on all three platforms
([`PLAN.md` §4](PLAN.md)). First genuinely usable build at **M3** — Sequence Window, Find/Info
drawers, Graphic Map and Sequence Map — which already covers most of a normal working day.

## Building

Prerequisites: Rust (stable, with `rustfmt` and `clippy`), Node 22+, and on Ubuntu the usual
Tauri 2 system packages (`libwebkit2gtk-4.1-dev`, `libxdo-dev`, `libayatana-appindicator3-dev`,
`librsvg2-dev`, `patchelf`, …).

```sh
npm install
npm run tauri dev      # development: Toolbar + Help window, hot reload
npm run tauri build    # platform installers under src-tauri/target/release/bundle/

cargo test --workspace                                   # the core crates
cargo clippy --workspace --all-targets -- -D warnings    # what CI enforces
npm run check                                            # svelte-check
```

Installers for every platform are built by the `Installers` workflow on each push and pull
request, and attached to a draft GitHub Release when a `v*` tag is pushed.

## Licence and attribution

This project is an **independent reimplementation** and is **not affiliated with, endorsed by, or
derived from the source code of** Serial Cloner or SerialBasics. Serial Cloner is © F. Perez and was
distributed as freeware; its manuals and sample files are included under `reference/` solely as
compatibility and interface documentation — see [`reference/README.md`](reference/README.md).

The restriction enzyme database is REBASE, © Dr. Richard J. Roberts, New England Biolabs, free for
academic use — <http://rebase.neb.com>.

"Gateway" is a trademark of Thermo Fisher Scientific and is used here descriptively, to name the
cloning chemistry the corresponding window automates.

Project licence: **TBD** (MIT or Apache-2.0 — see [`PLAN.md` §7](PLAN.md)).
