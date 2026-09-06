# SerialClon.rs

An open-source, maintained reimplementation of **Serial Cloner** — the molecular biology / DNA
cloning workbench by F. Perez (SerialBasics), whose last release was **2.6.1 in March 2013** — built
in **Rust + Tauri 2** and shipping installers for **Windows, macOS and Ubuntu**.

> **Status: planning.** The research and the implementation plan are done and live in
> [`PLAN.md`](PLAN.md). No application code has been written yet; the scaffold is milestone **M0**.

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
resources/RELibrary      current REBASE striderc enzyme database (runtime default)
scripts/update-rebase.sh refresh it from rebase.neb.com
reference/               see reference/README.md
  sequences/             original .xdna/.gb/.fasta/.nucl/.ma4 files — format fixtures
  docs/                  Serial Cloner manuals and full changelog (PDF + extracted text)
  ui/                    screenshots of every major window — the UI acceptance target
  RELibrary-striderc.211 the enzyme database Serial Cloner 2.6.1 actually shipped
```

## Roadmap

Seven milestones, each ending in installers on all three platforms
([`PLAN.md` §4](PLAN.md)). First genuinely usable build at **M3** — Sequence Window, Find/Info
drawers, Graphic Map and Sequence Map — which already covers most of a normal working day.

## Building

Nothing to build yet. Once M0 lands:

```sh
npm install
npm run tauri dev      # development
npm run tauri build    # platform installers
```

Prerequisites will be Rust (stable), Node 22+, and on Ubuntu the usual Tauri 2 system packages
(`libwebkit2gtk-4.1-dev`, `libxdo-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, …).

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
